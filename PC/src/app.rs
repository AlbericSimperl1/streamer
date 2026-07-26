use crate::capture;
use crate::hypr;
use crate::types::{LogEntry, LogLevel, MonitorConfig, MonitorJson};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime};

use std::collections::VecDeque;

pub struct App {
    pub config: MonitorConfig,
    pub monitors: Vec<MonitorJson>,
    pub monitor_exists: bool,
    pub log_entries: Vec<LogEntry>,
    pub auto_refresh: bool,
    pub last_refresh: Option<SystemTime>,

    pub capture: Option<capture::CaptureSession>,
    pub capture_output_path: String,
    pub stop_result_rx: Option<mpsc::Receiver<capture::CaptureStatus>>,

    pub signal_flag: Option<Arc<AtomicBool>>,
    shutdown_done: bool,

    pub fps_history: VecDeque<f32>,
    pub current_fps: f32,
    pub stream_start_time: Option<SystemTime>,
    last_frame_count: u64,
    last_fps_calc_time: Option<SystemTime>,
}

impl App {
    pub fn with_signal_flag(flag: Arc<AtomicBool>) -> Self {
        Self::with_signal_flag_opt(Some(flag))
    }

    pub(crate) fn with_signal_flag_opt(signal_flag: Option<Arc<AtomicBool>>) -> Self {
        let mut fps_history = VecDeque::with_capacity(60);
        for _ in 0..60 {
            fps_history.push_back(0.0);
        }

        let mut app = Self {
            config: MonitorConfig::default(),
            monitors: Vec::new(),
            monitor_exists: false,
            log_entries: Vec::new(),
            auto_refresh: true,
            last_refresh: None,
            capture: None,
            capture_output_path: default_capture_path(),
            stop_result_rx: None,
            signal_flag,
            shutdown_done: false,
            fps_history,
            current_fps: 0.0,
            stream_start_time: None,
            last_frame_count: 0,
            last_fps_calc_time: None,
        };
        app.log(
            "Hyprland Virtual Display controller started.",
            LogLevel::Info,
        );
        app.refresh();
        app
    }

    fn timestamp() -> String {
        chrono::Local::now().format("%H:%M:%S").to_string()
    }

    pub fn log(&mut self, msg: impl Into<String>, level: LogLevel) {
        self.log_entries.push(LogEntry {
            time: Self::timestamp(),
            message: msg.into(),
            level,
        });
        if self.log_entries.len() > 200 {
            self.log_entries.remove(0);
        }
    }

    pub fn refresh(&mut self) {
        match hypr::get_monitors() {
            Ok(monitors) => {
                let count = monitors.len();
                self.monitor_exists = monitors.iter().any(|m| m.name == self.config.name);
                self.monitors = monitors;
                self.last_refresh = Some(SystemTime::now());
                self.log(
                    format!("Refreshed — {count} monitor(s) active."),
                    LogLevel::Info,
                );
            }
            Err(e) => {
                self.log(format!("Failed to get monitors: {e}"), LogLevel::Error);
            }
        }
    }

    pub fn tick(&mut self) {
        if self.auto_refresh {
            if let Some(last) = self.last_refresh {
                if last.elapsed().unwrap_or_default() >= Duration::from_secs(2) {
                    self.refresh();
                }
            }
        }

        let now = SystemTime::now();
        let (current_frames, is_capturing) = match self.capture.as_ref().map(|c| c.status()) {
            Some(capture::CaptureStatus::Capturing { frames, .. }) => (frames, true),
            _ => (0, false),
        };

        if !is_capturing {
            self.stream_start_time = None;
            self.current_fps = 0.0;
        } else if self.stream_start_time.is_none() {
            self.stream_start_time = Some(now);
            self.last_frame_count = current_frames;
            self.last_fps_calc_time = Some(now);
        }

        let calc_due = self.last_fps_calc_time.map_or(true, |t| {
            t.elapsed().unwrap_or_default() >= Duration::from_millis(500)
        });

        if calc_due {
            if is_capturing {
                if let Some(last_time) = self.last_fps_calc_time {
                    let elapsed_secs = last_time.elapsed().unwrap_or_default().as_secs_f32();
                    if elapsed_secs > 0.0 {
                        let delta_frames =
                            current_frames.saturating_sub(self.last_frame_count) as f32;
                        let raw_fps = delta_frames / elapsed_secs;
                        // Smooth transition
                        self.current_fps = self.current_fps * 0.4 + raw_fps * 0.6;
                    }
                }
                self.last_frame_count = current_frames;
                self.last_fps_calc_time = Some(now);
            } else {
                self.current_fps = 0.0;
            }

            self.fps_history.pop_front();
            self.fps_history.push_back(self.current_fps);

            while self.fps_history.len() > 20 {
                self.fps_history.pop_front();
            }
        }
    }

    pub fn should_quit(&self) -> bool {
        self.signal_flag
            .as_ref()
            .map_or(false, |f| f.load(Ordering::SeqCst))
    }

    pub fn is_capturing(&self) -> bool {
        self.capture.as_ref().map_or(false, |c| c.is_running())
    }

    pub fn is_stopping(&self) -> bool {
        self.stop_result_rx.is_some()
    }

    pub fn apply_config(&mut self) {
        if !self.monitor_exists {
            if let Err(e) = hypr::create_headless_output(&self.config.name) {
                eprintln!("Fout bij aanmaken headless monitor: {e}");
                return;
            }
            self.monitor_exists = true;
        }

        if let Err(e) = hypr::apply_monitor_keyword(&self.config) {
            eprintln!("Fout bij updaten monitor configuratie: {e}");
        }

        self.refresh();
    }

    pub fn do_remove(&mut self) {
        let name = self.config.name.clone();

        self.log(
            format!("▶ Removing virtual monitor '{name}'..."),
            LogLevel::Info,
        );
        self.log(format!("  $ hyprctl output remove {name}"), LogLevel::Info);

        match hypr::remove_monitor(&name) {
            Ok(out) => {
                self.log(
                    format!("✓ Monitor '{name}' removed. {}", out.trim()),
                    LogLevel::Success,
                );
                self.refresh();
            }
            Err(e) => {
                self.log(format!("✗ Failed to remove: {e}"), LogLevel::Error);
            }
        }
    }

    pub fn do_start_capture(&mut self) {
        if self.is_capturing() {
            self.log("Capture already running.", LogLevel::Warning);
            return;
        }
        if !self.monitor_exists {
            self.log(
                "Create the virtual monitor first before capturing.",
                LogLevel::Warning,
            );
            return;
        }

        let path = self.capture_output_path.clone();
        self.log(format!("▶ Starting capture → {path}"), LogLevel::Info);
        self.log(
            "  A portal popup will appear — pick the virtual monitor.",
            LogLevel::Info,
        );
        let target_ip = self.config.ip.clone(); // <-- Dit ontbrak nog
        match capture::CaptureSession::start(path.clone(), target_ip) {
            Ok(session) => {
                self.capture = Some(session);
                self.log(
                    "✓ Capture session started. Select the monitor in the popup.",
                    LogLevel::Success,
                );
            }
            Err(e) => {
                self.log(format!("✗ Failed to start capture: {e}"), LogLevel::Error);
            }
        }
    }

    pub fn do_stop_capture(&mut self) {
        if let Some(mut session) = self.capture.take() {
            self.log(
                "▶ Stopping capture (finalizing MP4 in background)...",
                LogLevel::Info,
            );

            let (tx, rx) = mpsc::channel();
            self.stop_result_rx = Some(rx);

            std::thread::spawn(move || {
                session.stop();
                let final_status = session.status();
                let _ = tx.send(final_status);
            });
        }
    }

    pub fn poll_stop_result(&mut self) {
        let rx = match &self.stop_result_rx {
            Some(rx) => rx,
            None => return,
        };
        match rx.try_recv() {
            Ok(status) => {
                self.stop_result_rx = None;
                match status {
                    capture::CaptureStatus::Finished { path, frames } => {
                        self.log(
                            format!("✓ Capture saved: {path} ({frames} frames)"),
                            LogLevel::Success,
                        );
                    }
                    capture::CaptureStatus::Error(e) => {
                        self.log(format!("✗ Capture error: {e}"), LogLevel::Error);
                    }
                    _ => {
                        self.log("Capture stopped.", LogLevel::Info);
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                // Nog bezig — wachten.
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.stop_result_rx = None;
                self.log("✗ Capture stop thread ended unexpectedly.", LogLevel::Error);
            }
        }
    }

    pub fn shutdown(&mut self) {
        if self.shutdown_done {
            return;
        }
        self.shutdown_done = true;

        if let Some(mut session) = self.capture.take() {
            session.stop();
            println!("✓ Capture sessie gestopt bij afsluiten.");
        }
        if let Some(rx) = self.stop_result_rx.take() {
            let _ = rx.recv_timeout(std::time::Duration::from_secs(5));
        }
        if self.monitor_exists {
            let name = self.config.name.clone();
            match hypr::remove_monitor(&name) {
                Ok(_) => println!("✓ Monitor succesvol opgeruimd."),
                Err(e) => eprintln!("✗ Fout bij opruimen monitor bij afsluiten: {e}"),
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if self.capture.is_some() {
            self.capture.take();
        }
        if self.monitor_exists {
            let name = self.config.name.clone();
            let _ = hypr::remove_monitor(&name);
        }
    }
}

fn default_capture_path() -> String {
    let mut p = dirs_or_tmp();
    p.push("hyprpad_capture.mp4");
    p.to_string_lossy().into_owned()
}

fn dirs_or_tmp() -> std::path::PathBuf {
    if let Some(d) = std::env::var_os("XDG_VIDEOS_DIR") {
        return std::path::PathBuf::from(d);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let mut p = std::path::PathBuf::from(home);
        p.push("Videos");
        if p.exists() {
            return p;
        }
    }
    std::path::PathBuf::from("/tmp")
}
