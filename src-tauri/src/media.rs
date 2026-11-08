use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tiny_http::{Header, Method, Response, Server, StatusCode};

pub struct MediaServer {
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
    pub fn start(root: PathBuf) -> Result<Self, String> {
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
        std::thread::spawn(move || {
            for request in server.incoming_requests() {
                if limit.fetch_add(1, Ordering::SeqCst) >= 8 {
                    limit.fetch_sub(1, Ordering::SeqCst);
                    let _ = request.respond(Response::empty(503));
                    continue;
                }
                let root = root.clone();
                let token = token.clone();
                let limit = limit.clone();
                std::thread::spawn(move || {
                    serve(request, &root, &token);
                    limit.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
        Ok(Self { prefix })
    }
    pub fn url(&self, name: &str) -> String {
        format!(
            "{}{}",
            self.prefix,
            percent_encoding::utf8_percent_encode(name, percent_encoding::NON_ALPHANUMERIC)
        )
    }
}
fn serve(request: tiny_http::Request, root: &std::path::Path, token: &str) {
    if request.method() != &Method::Get && request.method() != &Method::Head {
        let _ = request.respond(Response::empty(405));
        return;
    }
    let expected = format!("/{token}/");
    let Some(encoded) = request.url().strip_prefix(&expected) else {
        let _ = request.respond(Response::empty(404));
        return;
    };
    let Ok(name) = percent_encoding::percent_decode_str(encoded).decode_utf8() else {
        let _ = request.respond(Response::empty(404));
        return;
    };
    let Ok(path) = crate::clips::clip_path(root, &name) else {
        let _ = request.respond(Response::empty(404));
        return;
    };
    let Ok(mut file) = File::open(path) else {
        let _ = request.respond(Response::empty(404));
        return;
    };
    let Ok(meta) = file.metadata() else {
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
        let _ = request.respond(
            Response::empty(416).with_header(header("Content-Range", &format!("bytes */{size}"))),
        );
        return;
    };
    if file.seek(SeekFrom::Start(start)).is_err() {
        let _ = request.respond(Response::empty(500));
        return;
    }
    let length = end - start + 1;
    let mut headers = vec![
        header("Content-Type", "video/mp4"),
        header("Accept-Ranges", "bytes"),
        header("Cache-Control", "no-store"),
        header("Cross-Origin-Resource-Policy", "cross-origin"),
    ];
    if partial {
        headers.push(header(
            "Content-Range",
            &format!("bytes {start}-{end}/{size}"),
        ));
    }
    let status = StatusCode(if partial { 206 } else { 200 });
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
        let server = MediaServer::start(root.clone()).unwrap();
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
        assert!(
            request(&format!("http://{address}/wrong-token/A.mp4"), "").starts_with("HTTP/1.1 404")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
