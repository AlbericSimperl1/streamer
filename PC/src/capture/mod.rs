pub mod encoder;
pub mod pipewire;
pub mod portal;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Live state of a capture session, readable from the GUI thread
#[derive(Clone, Debug)]
pub enum CaptureStatus {
    /// No session created yet.
    Idle,
    /// Portal popup shown or PipeWire stream being set up.
    Starting(String),
    /// Actively capturing. `frames` counts received frames.
    Capturing {
        width: u32,
        height: u32,
        frames: u64,
        path: String,
    },
    /// Unrecoverable error.
    Error(String),
    /// Session was stopped and the file is finalized.
    Finished { path: String, frames: u64 },
}

/// One raw frame coming from PipeWire. Pixel layout is BGR0 (XRGB8888 in
/// little-endian memory).
pub struct Frame {
    pub width: u32,
    pub height: u32,
    /// Bytes per row (may be larger than `width * 4`).
    pub stride: u32,
    /// Packed BGR0 pixel data, `height * stride` bytes.
    pub data: Vec<u8>,
}

impl Frame {
    /// Total byte length the encoder expects for a tightly packed row:
    /// `width * 4`. If `stride` differs, the caller must repack.
    pub fn row_bytes(&self) -> usize {
        self.width as usize * 4
    }
}

/// Owns a running capture. Drop or [`CaptureSession::stop`] tears it down.
pub struct CaptureSession {
    handle: Option<JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
    status: Arc<Mutex<CaptureStatus>>,
}

impl CaptureSession {
    /// Spawn a capture session writing H.264 to `output_path` (e.g. an `.mp4`).
    /// Returns immediately; the actual work happens on a background thread.
    pub fn start(output_path: String) -> Result<Self, String> {
        if output_path.trim().is_empty() {
            return Err("Output path is empty.".into());
        }

        let stop_flag = Arc::new(AtomicBool::new(false));
        let status = Arc::new(Mutex::new(CaptureStatus::Starting(
            "Requesting screencast portal…".into(),
        )));

        let stop_clone = Arc::clone(&stop_flag);
        let status_clone = Arc::clone(&status);
        let path_for_thread = output_path.clone();

        let handle = std::thread::Builder::new()
            .name("hyprpad-capture".into())
            .spawn(move || {
                run_capture_loop(stop_clone, status_clone, path_for_thread);
            })
            .map_err(|e| format!("Failed to spawn capture thread: {e}"))?;

        Ok(Self {
            handle: Some(handle),
            stop_flag,
            status,
        })
    }

    /// Ask the session to stop and block until it has torn down (FFmpeg
    /// finalizes the MP4 moov atom, PipeWire stream is released).
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            // Don't block forever — FFmpeg can take a moment to finalize.
            let _ = h.join();
        }
    }

    /// Snapshot the current status (cheap — clones a small enum).
    pub fn status(&self) -> CaptureStatus {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(CaptureStatus::Error("status mutex poisoned".into()))
    }

    pub fn is_running(&self) -> bool {
        self.handle.is_some()
    }
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        // Best-effort cleanup if the caller forgot to stop().
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn set_status(status: &Arc<Mutex<CaptureStatus>>, s: CaptureStatus) {
    if let Ok(mut guard) = status.lock() {
        *guard = s;
    }
}

/// Runs entirely on the capture thread.
fn run_capture_loop(
    stop_flag: Arc<AtomicBool>,
    status: Arc<Mutex<CaptureStatus>>,
    output_path: String,
) {
    // 1. Portal handshake (async). Use a one-off tokio runtime.
    let portal_result = {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                set_status(
                    &status,
                    CaptureStatus::Error(format!("Tokio runtime failed: {e}")),
                );
                return;
            }
        };
        rt.block_on(portal::open_screencast())
    };

    let handle = match portal_result {
        Ok(v) => v,
        Err(e) => {
            set_status(&status, CaptureStatus::Error(format!("Portal: {e}")));
            return;
        }
    };
    let pw_fd = handle.fd;
    let node_id = handle.node_id;

    set_status(
        &status,
        CaptureStatus::Starting("Connecting PipeWire stream…".into()),
    );

    // 2. PipeWire → frame channel → FFmpeg.
    let (tx, rx) = std::sync::mpsc::channel::<Frame>();

    let status_for_feeder = Arc::clone(&status);
    let stop_for_feeder = Arc::clone(&stop_flag);
    let path_for_feeder = output_path.clone();
    let feeder = std::thread::Builder::new()
        .name("hyprpad-encoder".into())
        .spawn(move || run_encoder(rx, stop_for_feeder, status_for_feeder, path_for_feeder))
        .expect("spawn encoder feeder");

    // Blocks until stop_flag is set or PipeWire dies.
    if let Err(e) = pipewire::run_capture(pw_fd, node_id, tx, Arc::clone(&stop_flag)) {
        set_status(&status, CaptureStatus::Error(format!("PipeWire: {e}")));
    }

    // Dropping rx naturally ends the feeder when the last queued frame is read.
    let _ = feeder.join();
}

/// Runs on the `hyprpad-encoder` thread. Receives frames and pumps them into
/// FFmpeg at a constant framerate.
fn run_encoder(
    rx: std::sync::mpsc::Receiver<Frame>,
    stop_flag: Arc<AtomicBool>,
    status: Arc<Mutex<CaptureStatus>>,
    output_path: String,
) {
    // 1. Wacht op het allereerste frame (met 100ms timeout om stop_flag te respecteren).
    let mut last_frame = loop {
        if stop_flag.load(Ordering::SeqCst) {
            return;
        }
        match rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(f) => break f,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                set_status(&status, CaptureStatus::Error("No frames received from PipeWire".into()));
                return;
            }
        }
    };

    let width = last_frame.width;
    let height = last_frame.height;
    let fps = 30;
    let frame_interval = std::time::Duration::from_nanos(1_000_000_000 / fps as u64); // ~33.3ms

    let mut enc = match encoder::Encoder::start(width, height, fps, &output_path) {
        Ok(e) => e,
        Err(e) => {
            set_status(
                &status,
                CaptureStatus::Error(format!("FFmpeg start failed: {e}")),
            );
            return;
        }
    };

    set_status(
        &status,
        CaptureStatus::Capturing {
            width,
            height,
            frames: 0,
            path: output_path.clone(),
        },
    );

    let mut frames: u64 = 0;
    let mut next_target = std::time::Instant::now();

    // Loop tot stop_flag gezet is
    while !stop_flag.load(Ordering::SeqCst) {
        let now = std::time::Instant::now();
        let timeout = if next_target > now {
            next_target - now
        } else {
            std::time::Duration::ZERO
        };

        // Probeer een nieuw frame van PipeWire te halen.
        // Als de virtuele monitor niet veranderd is (statisch scherm), geeft PipeWire GEEN nieuwe frames.
        // In dat geval valt recv_timeout droog, en hergebruiken we `last_frame`.
        match rx.recv_timeout(timeout) {
            Ok(new_frame) => {
                if new_frame.width == width && new_frame.height == height {
                    last_frame = new_frame;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Niks ontvangen binnen timeout window (scherm is statisch) -> we herhalen last_frame
            }
        }

        // Stuur ook eventuele overtollige gebufferde frames door naar het nieuwste frame
        while let Ok(new_frame) = rx.try_recv() {
            if new_frame.width == width && new_frame.height == height {
                last_frame = new_frame;
            }
        }

        // Push frame naar FFmpeg (stdin)
        if push_frame(&mut enc, &last_frame).is_err() {
            break;
        }

        frames += 1;
        if frames % 15 == 0 {
            update_frame_count(&status, &output_path, frames);
        }

        // Pacing to keep exact 30 FPS feeding to FFmpeg
        next_target += frame_interval;
        let after_push = std::time::Instant::now();
        if next_target > after_push {
            std::thread::sleep(next_target - after_push);
        } else if after_push.duration_since(next_target) > std::time::Duration::from_millis(500) {
            // Als we ver achter lopen, reset timing target naar nu
            next_target = after_push;
        }
    }

    update_frame_count(&status, &output_path, frames);
    finalize(&mut enc, &status, &output_path, frames, false);
}

fn push_frame(enc: &mut encoder::Encoder, frame: &Frame) -> Result<(), String> {
    if frame.stride as usize == frame.row_bytes() {
        enc.push_frame(&frame.data)
    } else {
        // Repack rows to drop stride padding.
        let row = frame.row_bytes();
        let mut packed = Vec::with_capacity(frame.height as usize * row);
        for y in 0..frame.height as usize {
            let start = y * frame.stride as usize;
            packed.extend_from_slice(&frame.data[start..start + row]);
        }
        enc.push_frame(&packed)
    }
}

fn update_frame_count(status: &Arc<Mutex<CaptureStatus>>, path: &str, frames: u64) {
    if let Ok(mut g) = status.lock() {
        if let CaptureStatus::Capturing {
            width,
            height,
            path: p,
            ..
        } = &*g
        {
            *g = CaptureStatus::Capturing {
                width: *width,
                height: *height,
                frames,
                path: p.clone(),
            };
            let _ = path;
        }
    }
}

fn finalize(
    enc: &mut encoder::Encoder,
    status: &Arc<Mutex<CaptureStatus>>,
    path: &str,
    frames: u64,
    errored: bool,
) {
    let res = enc.finish();
    let msg = match (res, errored) {
        (Ok(_), false) => CaptureStatus::Finished {
            path: path.to_string(),
            frames,
        },
        (Ok(_), true) => CaptureStatus::Error(format!(
            "Encoder stopped early; {frames} frames written to {path}"
        )),
        (Err(e), _) => CaptureStatus::Error(format!("FFmpeg finalize failed: {e}")),
    };
    set_status(status, msg);
}
