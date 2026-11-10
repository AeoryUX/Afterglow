use std::{
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tiny_http::{Header, Method, Response, Server, StatusCode};

#[derive(Default, serde::Serialize)]
struct Diagnostics {
    requests: u64,
    last_status: u16,
    last_origin: String,
    last_issue: String,
}
pub struct MediaServer {
    diagnostics: Arc<Mutex<Diagnostics>>,
    prefix: String,
}
fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("static media header")
}

pub fn byte_range(value: Option<&str>, size: u64) -> Result<(u64, u64, bool), ()> {
    if size == 0 {
        return Err(());
    }
    let Some(value) = value else {
        return Ok((0, size - 1, false));
    };
    let (start, end) = value
        .strip_prefix("bytes=")
        .ok_or(())?
        .split_once('-')
        .ok_or(())?;
    if end.contains(',') {
        return Err(());
    }
    if start.is_empty() {
        let suffix: u64 = end.parse().map_err(|_| ())?;
        if suffix == 0 {
            return Err(());
        }
        return Ok((size.saturating_sub(suffix), size - 1, true));
    }
    let start: u64 = start.parse().map_err(|_| ())?;
    let end = if end.is_empty() {
        size - 1
    } else {
        end.parse::<u64>().map_err(|_| ())?.min(size - 1)
    };
    if start >= size || end < start {
        return Err(());
    }
    Ok((start, end, true))
}
impl MediaServer {
    pub fn start(root: PathBuf, cache: PathBuf) -> Result<Self, String> {
        let server = Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
        let address = server
            .server_addr()
            .to_ip()
            .ok_or("No local media address")?;
        let mut random = [0_u8; 32];
        getrandom::fill(&mut random).map_err(|e| e.to_string())?;
        let token = random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let prefix = format!("http://{address}/{token}/");
        let limit = Arc::new(AtomicUsize::new(0));
        let diagnostics = Arc::new(Mutex::new(Diagnostics::default()));
        let stats = diagnostics.clone();
        std::thread::spawn(move || {
            for request in server.incoming_requests() {
                if limit.fetch_add(1, Ordering::SeqCst) >= 8 {
                    limit.fetch_sub(1, Ordering::SeqCst);
                    let _ = request.respond(Response::empty(503));
                    continue;
                }
                let root = root.clone();
                let stats = stats.clone();
                let cache = cache.clone();
                let token = token.clone();
                let limit = limit.clone();
                std::thread::spawn(move || {
                    serve(request, &root, &cache, &token, &address.to_string(), &stats);
                    limit.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
        Ok(Self {
            prefix,
            diagnostics,
        })
    }
    pub fn diagnostics(&self) -> String {
        self.diagnostics
            .lock()
            .ok()
            .and_then(|d| serde_json::to_string(&*d).ok())
            .unwrap_or_else(|| "unavailable".into())
    }
    pub fn preview_url(&self) -> String {
        format!("{}preview.webm", self.prefix)
    }
    pub fn url(&self, name: &str) -> String {
        format!(
            "{}{}",
            self.prefix,
            percent_encoding::utf8_percent_encode(name, percent_encoding::NON_ALPHANUMERIC)
        )
    }
}
fn note(d: &Mutex<Diagnostics>, status: u16, origin: &str, issue: &str) {
    if let Ok(mut d) = d.lock() {
        d.requests += 1;
        d.last_status = status;
        d.last_origin = origin.chars().take(120).collect();
        d.last_issue = issue.into();
    }
}
fn serve(
    request: tiny_http::Request,
    root: &std::path::Path,
    cache: &std::path::Path,
    token: &str,
    address: &str,
    diagnostics: &Mutex<Diagnostics>,
) {
    let hosts = request
        .headers()
        .iter()
        .filter(|h| h.field.equiv("Host"))
        .collect::<Vec<_>>();
    if hosts.len() != 1 || hosts[0].value.as_str() != address {
        note(diagnostics, 403, "", "Host rejected");
        let _ = request.respond(Response::empty(403));
        return;
    }
    // The URL's random 256-bit token is the authorization gate. WebKit can
    // serialize custom-scheme origins as opaque "null" on newer Linux builds.
    let expected = format!("/{token}/");
    let Some(encoded) = request.url().strip_prefix(&expected) else {
        note(diagnostics, 404, "", "Unknown media token");
        let _ = request.respond(Response::empty(404));
        return;
    };
    let origins = request
        .headers()
        .iter()
        .filter(|h| h.field.equiv("Origin"))
        .collect::<Vec<_>>();
    let origin = origins.first().map(|h| h.value.as_str()).unwrap_or("");
    let allowed = matches!(
        origin,
        "" | "null" | "tauri://localhost" | "http://tauri.localhost" | "https://tauri.localhost"
    ) || cfg!(debug_assertions) && origin == "http://localhost:1420";
    if origins.len() > 1 || !allowed {
        note(diagnostics, 403, origin, "Origin rejected");
        let _ = request.respond(Response::empty(403));
        return;
    }
    if request.method() != &Method::Get && request.method() != &Method::Head {
        note(diagnostics, 405, origin, "Method rejected");
        let _ = request.respond(Response::empty(405));
        return;
    }
    let Ok(name) = percent_encoding::percent_decode_str(encoded).decode_utf8() else {
        note(diagnostics, 404, origin, "Invalid media name");
        let _ = request.respond(Response::empty(404));
        return;
    };
    let preview = name == "preview.webm";
    let source = if preview {
        crate::private_fs::Directory::open(cache)
            .and_then(|dir| dir.read("preview.webm"))
            .map_err(|e| e.to_string())
    } else {
        crate::clips::open_clip(root, &name)
    };
    let Ok(mut file) = source else {
        note(diagnostics, 404, origin, "Media file unavailable");
        let _ = request.respond(Response::empty(404));
        return;
    };
    let Ok(meta) = file.metadata() else {
        note(diagnostics, 404, origin, "Media metadata unavailable");
        let _ = request.respond(Response::empty(404));
        return;
    };
    let size = meta.len();
    let range = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Range"))
        .map(|h| h.value.as_str());
    let Ok((start, end, partial)) = byte_range(range, size) else {
        note(diagnostics, 416, origin, "Range rejected");
        let _ = request.respond(
            Response::empty(416).with_header(header("Content-Range", &format!("bytes */{size}"))),
        );
        return;
    };
    if file.seek(SeekFrom::Start(start)).is_err() {
        note(diagnostics, 500, origin, "Seek failed");
        let _ = request.respond(Response::empty(500));
        return;
    }
    let length = end - start + 1;
    let mut headers = vec![
        header(
            "Content-Type",
            if preview { "video/webm" } else { "video/mp4" },
        ),
        header("Accept-Ranges", "bytes"),
        header("Cache-Control", "no-store"),
        header("X-Content-Type-Options", "nosniff"),
        header("Cross-Origin-Resource-Policy", "cross-origin"),
    ];
    if let Some(origin) = origins.first() {
        headers.push(header("Access-Control-Allow-Origin", origin.value.as_str()));
        headers.push(header("Vary", "Origin"));
    }
    if partial {
        headers.push(header(
            "Content-Range",
            &format!("bytes {start}-{end}/{size}"),
        ));
    }
    let status = StatusCode(if partial { 206 } else { 200 });
    note(
        diagnostics,
        status.0,
        origin,
        if preview { "WebM preview" } else { "MP4 clip" },
    );
    let response = Response::new(
        status,
        headers,
        file.take(length),
        Some(length as usize),
        None,
    );
    let _ = request.respond(response);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handles_seek_ranges() {
        assert_eq!(byte_range(None, 100), Ok((0, 99, false)));
        assert_eq!(byte_range(Some("bytes=10-19"), 100), Ok((10, 19, true)));
        assert_eq!(byte_range(Some("bytes=90-"), 100), Ok((90, 99, true)));
        assert_eq!(byte_range(Some("bytes=-10"), 100), Ok((90, 99, true)));
        assert_eq!(byte_range(Some("bytes=10-900"), 100), Ok((10, 99, true)));
    }
    #[test]
    fn rejects_invalid_ranges() {
        for range in [
            "bytes=100-",
            "bytes=50-10",
            "bytes=-0",
            "bytes=0-1,3-4",
            "words=0-1",
            "",
        ] {
            assert!(byte_range(Some(range), 100).is_err(), "{range}");
        }
        assert!(byte_range(None, 0).is_err());
    }
    #[test]
    fn streams_ranges_and_rejects_other_paths() {
        use std::{
            fs,
            io::Write,
            net::TcpStream,
            time::{SystemTime, UNIX_EPOCH},
        };
        let root = std::env::temp_dir().join(format!(
            "afterglow-http-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("A play.mp4"), b"0123456789").unwrap();
        let server = MediaServer::start(root.clone(), root.join("cache")).unwrap();
        let request = |url: &str, range: &str| {
            let (address, path) = url
                .strip_prefix("http://")
                .unwrap()
                .split_once('/')
                .unwrap();
            let mut socket = TcpStream::connect(address).unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            write!(
                socket,
                "GET /{path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n{range}\r\n"
            )
            .unwrap();
            let mut out = String::new();
            socket.read_to_string(&mut out).unwrap();
            out
        };
        assert!(request(
            &server.url("A play.mp4"),
            "Origin: https://example.invalid\r\n"
        )
        .starts_with("HTTP/1.1 403"));
        assert!(
            request(&server.url("A play.mp4"), "Host: example.invalid\r\n")
                .starts_with("HTTP/1.1 403")
        );
        std::os::unix::fs::symlink("/etc/passwd", root.join("link.mp4")).unwrap();
        assert!(request(&server.url("link.mp4"), "").starts_with("HTTP/1.1 404"));
        fs::create_dir(root.join("cache")).unwrap();
        fs::write(root.join("cache/preview.webm"), b"webm-preview").unwrap();
        let preview = request(
            &server.preview_url(),
            "Origin: tauri://localhost\r\nRange: bytes=0-3\r\n",
        );
        assert!(preview.starts_with("HTTP/1.1 206"));
        assert!(preview.contains("video/webm"));
        assert!(preview.ends_with("webm"));
        for origin in ["null", "http://tauri.localhost", "https://tauri.localhost"] {
            let opaque = request(
                &server.url("A play.mp4"),
                &format!("Origin: {origin}\r\nRange: bytes=0-2\r\n"),
            );
            assert!(opaque.starts_with("HTTP/1.1 206"));
            assert!(opaque.ends_with("012"));
            assert!(opaque
                .to_lowercase()
                .contains(&format!("access-control-allow-origin: {origin}")));
        }
        let result = request(&server.url("A play.mp4"), "Range: bytes=2-5\r\n");
        assert!(result.starts_with("HTTP/1.1 206"));
        assert!(result.ends_with("2345"));
        assert!(
            request(&server.url("A play.mp4"), "Range: bytes=100-\r\n").starts_with("HTTP/1.1 416")
        );
        assert!(request(&server.url("../escape.mp4"), "").starts_with("HTTP/1.1 404"));
        let address = server
            .prefix
            .strip_prefix("http://")
            .unwrap()
            .split('/')
            .next()
            .unwrap();
        assert!(request(
            &format!("http://{address}/wrong-token/A.mp4"),
            "Origin: null\r\n"
        )
        .starts_with("HTTP/1.1 404"));
        assert!(request(
            &server.url("A play.mp4"),
            "Origin: null\r\nOrigin: tauri://localhost\r\n"
        )
        .starts_with("HTTP/1.1 403"));
        assert!(!server.diagnostics().contains(&server.prefix));
        fs::remove_dir_all(root).unwrap();
    }
}
