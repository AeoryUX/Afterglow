//! Short PipeWire playback on confirmed saves, independent of the webview.
use std::{
    io::Write,
    process::Stdio,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
static PLAYING: AtomicBool = AtomicBool::new(false);
// Smaller than PIPE_BUF: the initial audio write fits even before pw-play reads.
const CLIP_SOUND: &[u8] = include_bytes!("../sounds/clip.wav");
pub fn play(app: tauri::AppHandle) {
    if PLAYING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let result = (|| -> std::io::Result<()> {
            let mut child = crate::system::command("pw-play")
                .args(["--media-role", "Notification", "-"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            let write = child
                .stdin
                .take()
                .ok_or_else(|| std::io::Error::other("Missing audio input"))
                .and_then(|mut input| input.write_all(CLIP_SOUND));
            if let Err(e) = write {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e);
            }
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                match child.try_wait()? {
                    Some(exit) if exit.success() => return Ok(()),
                    Some(_) => {
                        return Err(std::io::Error::other(
                            "PipeWire could not play the clip sound",
                        ))
                    }
                    None if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(25))
                    }
                    None => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(std::io::Error::other("Clip sound playback timed out"));
                    }
                }
            }
        })();
        if let Err(e) = result {
            crate::activity::event(&app, &format!("Clip saved; sound unavailable: {e}"));
        }
        PLAYING.store(false, Ordering::SeqCst);
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_tone_is_a_small_wave() {
        assert!(super::CLIP_SOUND.len() < 4096);
        assert_eq!(&super::CLIP_SOUND[..4], b"RIFF");
        assert_eq!(&super::CLIP_SOUND[8..12], b"WAVE");
    }
}
