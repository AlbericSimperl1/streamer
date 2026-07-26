use std::io::Write;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};
pub struct Encoder {
    child: Child,
    width: u32,
    height: u32,
}

impl Encoder {
    pub fn start(width: u32, height: u32, fps: u32, _output_path: &str) -> Result<Self, String> {
        let size = format!("{width}x{height}");
        let rate = format!("{fps}");
        let gop = format!("{fps}");

        let child = Command::new("ffmpeg")
            .args([
                // input
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
                // enucoder
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-tune",
                "zerolatency",
                // bgr0 -> yuv420p
                "-pix_fmt",
                "yuv420p",
                "-g",
                &gop,
                "-keyint_min",
                &gop,
                "-x264-params",
                "aud=1",
                "-f",
                "h264",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| e.to_string())?;

        Ok(Self {
            child,
            width,
            height,
        })
    }

    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
    }

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

    pub fn finish(&mut self) -> Result<(), String> {
        if let Some(stdin) = self.child.stdin.take() {
            std::mem::drop(stdin);
        }
        Self::wait_with_timeout(&mut self.child, Duration::from_secs(3))
    }

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
        self.child.stdin.take();
        let _ = Self::wait_with_timeout(&mut self.child, Duration::from_secs(2));
    }
}
