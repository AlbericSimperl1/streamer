use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Owns a running `ffmpeg -i - … out.mp4` process
pub struct Encoder {
    child: Child,
    width: u32,
    height: u32,
}

impl Encoder {
    /// Spawn ffmpeg reading raw BGR0 frames from stdin and streaming them as
    /// H.264 / MPEG-TS over UDP — mirrors the standalone command:
    ///
    /// ```text
    /// ffmpeg -re -f lavfi -i testsrc=size=1920x1080:rate=60 \
    ///   -c:v libx264 -preset ultrafast -tune zerolatency \
    ///   -g 30 -keyint_min 30 \
    ///   -f mpegts udp://192.168.0.119:5000?pkt_size=1316
    /// ```
    ///
    /// `output_path` is only kept for API compatibility with the caller; the
    /// stream is the single UDP output (writing an extra `.mp4` alongside the
    /// UDP url breaks the keyframe structure VLC needs to actually decode).
    pub fn start(width: u32, height: u32, fps: u32, _output_path: &str) -> Result<Self, String> {
        let size = format!("{width}x{height}");
        let rate = format!("{fps}");
        let gop = format!("{fps}");
        let ipad_udp_url = "udp://192.168.0.119:5000?pkt_size=1316";

        let mut child = Command::new("ffmpeg")
            .args([
                // --- INPUT: raw BGR0 frames from stdin (Rust) ---
                "-y",
                "-f",
                "rawvideo",
                "-pixel_format",
                "bgr0",
                "-video_size",
                &size,
                "-framerate",
                &rate,
                "-i",
                "-",
                // --- ENCODER: match the working standalone command ---
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-tune",
                "zerolatency",
                // bgr0 must be converted to yuv420p for H.264 compatibility.
                // (testsrc in the standalone command already outputs yuv420p,
                //  so it doesn't need this flag — we do.)
                "-pix_fmt",
                "yuv420p",
                "-g",
                &gop,
                "-keyint_min",
                &gop,
                // --- OUTPUT: single MPEG-TS over UDP, must be last arg ---
                "-f",
                "h264",
                ipad_udp_url,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| e.to_string())?;

        // Sanity: ffmpeg may exit instantly if the path is bad. Read stderr
        // lazily; we only inspect it on finalize failure.
        let _ = child.stderr.take();

        Ok(Self {
            child,
            width,
            height,
        })
    }

    /// Push one tightly-packed BGR0 frame (`width * height * 4` bytes).
    pub fn push_frame(&mut self, bytes: &[u8]) -> Result<(), String> {
        let expected = (self.width as usize) * (self.height as usize) * 4;
        if bytes.len() < expected {
            return Err(format!(
                "Short frame: got {} bytes, expected {expected}",
                bytes.len()
            ));
        }
        if let Some(stdin) = self.child.stdin.as_mut() {
            stdin
                .write_all(&bytes[..expected])
                .map_err(|e| format!("ffmpeg stdin write failed: {e}"))
        } else {
            Err("ffmpeg stdin closed".into())
        }
    }

    /// Close stdin so ffmpeg flushes and finalizes. Gives FFmpeg up to 3
    /// seconds to exit gracefully; after that it gets SIGKILL'd.
    pub fn finish(&mut self) -> Result<(), String> {
        // 1. Sluit de stdin pipe — stuurt EOF naar FFmpeg.
        if let Some(stdin) = self.child.stdin.take() {
            std::mem::drop(stdin);
        }

        // 2. Wacht met timeout: geef FFmpeg 3 seconden om netjes te stoppen.
        Self::wait_with_timeout(&mut self.child, Duration::from_secs(3))
    }

    /// Wacht tot het child process stopt, met een deadline.
    /// Na de deadline wordt het process ge-kill'd.
    fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return if status.success() {
                        Ok(())
                    } else {
                        Err(format!("FFmpeg exited with: {status}"))
                    };
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        log::warn!("FFmpeg did not exit within {timeout:?}, sending SIGKILL");
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err("FFmpeg did not exit in time, killed.".into());
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(format!("FFmpeg wait error: {e}")),
            }
        }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        // Best-effort cleanup: close stdin, then wait with a 2-second timeout.
        // If FFmpeg doesn't exit, kill it to prevent zombie processes.
        self.child.stdin.take();
        let _ = Self::wait_with_timeout(&mut self.child, Duration::from_secs(2));
    }
}
