pub mod encoder;
pub mod packetizer;
pub mod pipewire;
pub mod portal;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

static TOKIO_RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn get_tokio_runtime() -> Result<&'static tokio::runtime::Runtime, String> {
    Ok(TOKIO_RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    }))
}

#[derive(Clone, Debug)]
pub enum CaptureStatus {
    Idle,
    Starting(String),
    Capturing {
        width: u32,
        height: u32,
        frames: u64,
        path: String,
    },
    Error(String),
    Finished {
        path: String,
        frames: u64,
    },
}

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub data: Vec<u8>,
}

impl Frame {
    pub fn row_bytes(&self) -> usize {
        self.width as usize * 4
    }
}

pub struct CaptureSession {
    handle: Option<JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
    status: Arc<Mutex<CaptureStatus>>,
}

impl CaptureSession {
    // 1. target_addr toegevoegd aan parameters
    pub fn start(output_path: String, target_addr: String) -> Result<Self, String> {
        if output_path.trim().is_empty() {
            return Err("Output path is empty.".into());
        }

        // Voeg automatisch poort 5000 toe als enkel een IP is ingevoerd
        let full_target_addr = if target_addr.contains(':') {
            target_addr
        } else {
            format!("{}:5000", target_addr.trim())
        };

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
                run_capture_loop(stop_clone, status_clone, path_for_thread, full_target_addr);
            })
            .map_err(|e| format!("Failed to spawn capture thread: {e}"))?;

        Ok(Self {
            handle: Some(handle),
            stop_flag,
            status,
        })
    }

    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }

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

// 2. target_addr doorgegeven in de capture loop
fn run_capture_loop(
    stop_flag: Arc<AtomicBool>,
    status: Arc<Mutex<CaptureStatus>>,
    output_path: String,
    target_addr: String,
) {
    let rt = match get_tokio_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            set_status(&status, CaptureStatus::Error(e));
            return;
        }
    };

    let handle = match rt.block_on(portal::open_screencast()) {
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

    let (tx, rx) = std::sync::mpsc::channel::<Frame>();

    let status_for_feeder = Arc::clone(&status);
    let stop_for_feeder = Arc::clone(&stop_flag);
    let path_for_feeder = output_path.clone();
    let target_addr_for_feeder = target_addr.clone();

    let feeder = std::thread::Builder::new()
        .name("hyprpad-encoder".into())
        .spawn(move || {
            run_encoder(
                rx,
                stop_for_feeder,
                status_for_feeder,
                path_for_feeder,
                target_addr_for_feeder,
            )
        })
        .expect("spawn encoder feeder");

    if let Err(e) = pipewire::run_capture(pw_fd, node_id, tx, Arc::clone(&stop_flag)) {
        set_status(&status, CaptureStatus::Error(format!("PipeWire: {e}")));
    }

    let _ = feeder.join();

    log::info!("portal: closing session before drop");
    if let Err(e) = rt.block_on(handle.session.close()) {
        log::warn!("portal: Session::close failed (continuing): {e}");
    }
    log::info!("portal: session closed");

    set_status(&status, CaptureStatus::Idle);
}

// 3. target_addr ontvangen in run_encoder
fn run_encoder(
    rx: std::sync::mpsc::Receiver<Frame>,
    stop_flag: Arc<AtomicBool>,
    status: Arc<Mutex<CaptureStatus>>,
    output_path: String,
    target_addr: String,
) {
    let mut last_frame = loop {
        if stop_flag.load(Ordering::SeqCst) {
            return;
        }
        match rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(f) => break f,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                set_status(
                    &status,
                    CaptureStatus::Error("No frames received from PipeWire".into()),
                );
                return;
            }
        }
    };

    let width = last_frame.width;
    let height = last_frame.height;
    let fps = 60;
    let frame_interval = std::time::Duration::from_nanos(1_000_000_000 / fps as u64);

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

    let stdout = match enc.take_stdout() {
        Some(s) => s,
        None => {
            set_status(
                &status,
                CaptureStatus::Error("ffmpeg stdout ontbreekt (niet piped?)".into()),
            );
            return;
        }
    };

    let packetizer_stop = Arc::new(AtomicBool::new(false));
    let mut packetizer = match packetizer::Packetizer::start(
        stdout,
        &target_addr, // <-- Bestaat nu als string parameter
        Arc::clone(&packetizer_stop),
    ) {
        Ok(p) => p,
        Err(e) => {
            set_status(
                &status,
                CaptureStatus::Error(format!("Packetizer start failed: {e}")),
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

    while !stop_flag.load(Ordering::SeqCst) {
        let now = std::time::Instant::now();
        let timeout = if next_target > now {
            next_target - now
        } else {
            std::time::Duration::ZERO
        };

        match rx.recv_timeout(timeout) {
            Ok(new_frame) => {
                if new_frame.width == width && new_frame.height == height {
                    last_frame = new_frame;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }

        while let Ok(new_frame) = rx.try_recv() {
            if new_frame.width == width && new_frame.height == height {
                last_frame = new_frame;
            }
        }

        if push_frame(&mut enc, &last_frame).is_err() {
            break;
        }

        frames += 1;
        if frames % 15 == 0 {
            update_frame_count(&status, &output_path, frames);
        }

        next_target += frame_interval;
        let after_push = std::time::Instant::now();
        if next_target > after_push {
            std::thread::sleep(next_target - after_push);
        } else if after_push.duration_since(next_target) > std::time::Duration::from_millis(500) {
            next_target = after_push;
        }
    }

    update_frame_count(&status, &output_path, frames);
    finalize(&mut enc, &status, &output_path, frames, false);
    packetizer.stop();
}

fn push_frame(enc: &mut encoder::Encoder, frame: &Frame) -> Result<(), String> {
    if frame.stride as usize == frame.row_bytes() {
        enc.push_frame(&frame.data)
    } else {
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
