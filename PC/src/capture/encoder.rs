use std::io::Write;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

/// Owns a running `ffmpeg -i - …` process.
///
/// **Wijziging t.o.v. de vorige versie:** ffmpeg verstuurt niet meer zelf
/// over UDP (`-f mpegts udp://...`). MPEG-TS-containeroverhead liet de
/// Annex-B-scanner aan de iPad-kant struikelen over toevallige
/// `00 00 01`-patronen in TS-sync-bytes/PAT/PMT — dat was de bron van eerst
/// de crash en daarna de "absurd veel frames"-bug. In plaats daarvan zet
/// ffmpeg een kale Annex-B H.264 elementary stream op stdout
/// (`-f h264 -`); `Packetizer` (zie `packetizer.rs`) leest die stream,
/// herkent de NAL-unit-grenzen zelf, en verstuurt ze gechunkt met een
/// expliciete header over UDP.
pub struct Encoder {
    child: Child,
    width: u32,
    height: u32,
}

impl Encoder {
    /// Spawn ffmpeg reading raw BGR0 frames from stdin and emitting a raw
    /// Annex-B H.264 elementary stream on stdout — mirrors the standalone
    /// command:
    ///
    /// ```text
    /// ffmpeg -re -f lavfi -i testsrc=size=1920x1080:rate=60 \
    ///   -c:v libx264 -preset ultrafast -tune zerolatency \
    ///   -g 30 -keyint_min 30 \
    ///   -f h264 -
    /// ```
    ///
    /// `output_path` is only kept for API compatibility with the caller —
    /// there's no file output, the stream goes to stdout for the
    /// `Packetizer` to consume.
    pub fn start(width: u32, height: u32, fps: u32, _output_path: &str) -> Result<Self, String> {
        let size = format!("{width}x{height}");
        let rate = format!("{fps}");
        let gop = format!("{fps}");

        let child = Command::new("ffmpeg")
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
                "-pix_fmt",
                "yuv420p",
                "-g",
                &gop,
                "-keyint_min",
                &gop,
                // `-tune zerolatency` zet sliced-threads=1 aan (meerdere
                // slice-NAL's per frame, voor lagere latency bij meerdere
                // CPU-threads). We laten dat nu gewoon toe, maar vragen x264
                // om vóór elk frame een Access Unit Delimiter (NAL type 9)
                // te zetten — dat is de marker waarmee de ontvanger slices
                // weer tot hele frames groepeert (zie H264Decoder.swift).
                "-x264-params",
                "aud=1",
                // --- OUTPUT: kale Annex-B H.264 elementary stream naar
                // stdout. Geen MPEG-TS, geen UDP hier — de Packetizer aan
                // de Rust-kant doet nu zelf de framing/verzending.
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

    /// Neem de stdout-handle over zodat de `Packetizer` er NAL-units uit kan
    /// lezen. Mag maar één keer aangeroepen worden (direct na `start`).
    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
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
    /// seconds to exit gracefully; after that it gets SIGKILL'd. Closing
    /// stdin also causes ffmpeg to close stdout (EOF), which lets the
    /// `Packetizer`'s read loop stop on its own.
    pub fn finish(&mut self) -> Result<(), String> {
        if let Some(stdin) = self.child.stdin.take() {
            std::mem::drop(stdin);
        }
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
        self.child.stdin.take();
        let _ = Self::wait_with_timeout(&mut self.child, Duration::from_secs(2));
    }
}
