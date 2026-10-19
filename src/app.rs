use crate::display::{Display, DisplayState};
use crate::hypr;
use crate::ipc;
use crate::settings::Settings;
use crate::sunshine::{self, ServiceState, SunshineInfo};
use crate::types::{LogEntry, LogLevel, MonitorConfig, MonitorJson};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime};

type RefreshResult = (Result<Vec<MonitorJson>, String>, SunshineInfo);

pub struct App {
    /// Editable copy of the virtual monitor config (applied with "update").
    pub config: MonitorConfig,
    pub display: Display,
    /// Last snapshot of the shared display state.
    pub view: DisplayState,
    seen_generation: u64,
    pub monitors: Vec<MonitorJson>,
    pub log_entries: Vec<LogEntry>,
    pub auto_refresh: bool,
    pub last_refresh: Option<SystemTime>,

    /// In-flight background `hyprctl monitors` + Sunshine query (never block
    /// the UI thread).
    refresh_rx: Option<mpsc::Receiver<RefreshResult>>,
    ipc_log_rx: mpsc::Receiver<String>,
    ipc_stop: Arc<AtomicBool>,
    /// In-flight PIN request to Sunshine's web API.
    pin_rx: Option<mpsc::Receiver<Result<(), String>>>,

    pub sunshine: Option<SunshineInfo>,
    pub pin_input: String,
    pub pin_client_name: String,
    pub sunshine_password: String,
    pub settings: Settings,

    shutdown_done: bool,
}

impl App {
    pub fn new() -> Self {
        let settings = Settings::load();
        let display = Display::new(
            settings.monitor.clone(),
            settings.main_monitor.clone(),
            settings.follow_client,
        );
        let view = display.snapshot();

        let (ipc_tx, ipc_log_rx) = mpsc::channel();
        let ipc_stop = Arc::new(AtomicBool::new(false));
        let ipc_result = ipc::serve(display.clone(), ipc_tx, ipc_stop.clone());

        let mut app = Self {
            config: view.config.clone(),
            seen_generation: view.generation,
            display,
            view,
            monitors: Vec::new(),
            log_entries: Vec::new(),
            auto_refresh: true,
            last_refresh: None,
            refresh_rx: None,
            ipc_log_rx,
            ipc_stop,
            pin_rx: None,
            sunshine: None,
            pin_input: String::new(),
            pin_client_name: String::new(),
            sunshine_password: settings.sunshine_password.clone().unwrap_or_default(),
            settings,
            shutdown_done: false,
        };
        app.log("Hyprland virtual display started.", LogLevel::Info);
        match (&app.view.main, &app.view.main_error) {
            (Some(m), _) => app.log(
                format!(
                    "Main display {}: {}×{} @ {:.2} Hz, scale {}",
                    m.name, m.width, m.height, m.refresh, m.scale
                ),
                LogLevel::Info,
            ),
            (None, Some(e)) => app.log(
                format!(
                    "✗ Could not read the main display ({e}); explicit fallback {}×{}@{} in use.",
                    crate::types::FALLBACK_MODE.0,
                    crate::types::FALLBACK_MODE.1,
                    crate::types::FALLBACK_MODE.2
                ),
                LogLevel::Error,
            ),
            _ => {}
        }
        if let Err(e) = ipc_result {
            app.log(format!("Hotkey socket unavailable: {e}"), LogLevel::Warning);
        }
        app.refresh();
        app
    }

    fn timestamp() -> String {
        chrono::Local::now().format("%H:%M:%S").to_string()
    }

    pub fn log(&mut self, msg: impl Into<String>, level: LogLevel) {
        let message = msg.into();
        match level {
            LogLevel::Error => log::error!("{message}"),
            LogLevel::Warning => log::warn!("{message}"),
            _ => log::info!("{message}"),
        }
        self.log_entries.push(LogEntry {
            time: Self::timestamp(),
            message,
            level,
        });
        if self.log_entries.len() > 200 {
            self.log_entries.remove(0);
        }
    }

    pub fn monitor_exists(&self) -> bool {
        self.view.exists
    }

    fn save_settings(&mut self) {
        self.settings.monitor = Some(self.display.config());
        self.settings.follow_client = self.view.follow_client;
        self.settings.main_monitor = self.view.preferred_main.clone();
        self.settings.save();
    }

    /// Starts a non-blocking refresh. The result is applied in
    /// `poll_refresh()` (called from `tick()`).
    pub fn refresh(&mut self) {
        if self.refresh_rx.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.refresh_rx = Some(rx);
        let spawned = std::thread::Builder::new()
            .name("hyprstream-refresh".into())
            .spawn(move || {
                let _ = tx.send((hypr::get_monitors(), sunshine::query()));
            });
        if spawned.is_err() {
            self.refresh_rx = None;
        }
    }

    fn poll_refresh(&mut self) {
        let Some(rx) = &self.refresh_rx else {
            return;
        };
        let (monitors, info) = match rx.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.refresh_rx = None;
                self.last_refresh = Some(SystemTime::now());
                return;
            }
        };
        self.refresh_rx = None;
        self.last_refresh = Some(SystemTime::now());

        if let Some(old) = &self.sunshine {
            if old.state != info.state {
                let level = match info.state {
                    ServiceState::Failed => LogLevel::Error,
                    ServiceState::Active => LogLevel::Success,
                    _ => LogLevel::Info,
                };
                self.log(format!("Sunshine {}", info.state.label()), level);
            }
        }
        self.sunshine = Some(info);

        match monitors {
            Ok(monitors) => {
                let old_main = self.view.main.clone();
                self.display.update_from_monitors(&monitors);
                self.monitors = monitors;
                self.sync_view();
                if self.view.main != old_main {
                    if let Some(m) = &self.view.main {
                        let msg = format!(
                            "Main display now {}: {}×{} @ {:.2} Hz at {}×{}",
                            m.name, m.width, m.height, m.refresh, m.x, m.y
                        );
                        self.log(msg, LogLevel::Info);
                    }
                }
            }
            Err(e) => {
                self.display.set_main_error(e.clone());
                self.sync_view();
                self.log(format!("Failed to read monitors: {e}"), LogLevel::Error);
            }
        }
    }

    /// Pulls the shared display state; adopts its config when something
    /// other than the GUI changed it (session start, hotkey toggle).
    fn sync_view(&mut self) {
        self.view = self.display.snapshot();
        if self.view.generation != self.seen_generation {
            self.seen_generation = self.view.generation;
            self.config = self.view.config.clone();
            self.save_settings();
        }
    }

    pub fn tick(&mut self) {
        self.poll_refresh();
        self.poll_pin();
        while let Ok(msg) = self.ipc_log_rx.try_recv() {
            self.log(msg, LogLevel::Info);
        }
        self.sync_view();

        if self.auto_refresh && self.refresh_rx.is_none() {
            let due = self.last_refresh.map_or(true, |l| {
                l.elapsed().unwrap_or_default() >= Duration::from_secs(2)
            });
            if due {
                self.refresh();
            }
        }
    }

    // virtual monitor

    pub fn apply_config(&mut self) {
        self.display.set_config(self.config.clone());
        match self.display.apply() {
            Ok(c) => {
                self.log(
                    format!("✓ Monitor '{}' {}×{}@{} at {}×{}", c.name, c.width, c.height, c.fps, c.x, c.y),
                    LogLevel::Success,
                );
            }
            Err(e) => self.log(format!("✗ Failed to apply monitor: {e}"), LogLevel::Error),
        }
        self.sync_view();
        self.save_settings();
        if self.settings.sync_output_name {
            self.sync_output_name();
        }
        self.refresh();
    }

    pub fn do_remove(&mut self) {
        let name = self.config.name.clone();
        match self.display.remove() {
            Ok(()) => self.log(format!("✓ Monitor '{name}' removed."), LogLevel::Success),
            Err(e) => self.log(format!("✗ Failed to remove: {e}"), LogLevel::Error),
        }
        self.sync_view();
        self.refresh();
    }

    /// `A×B ⇄ B×A`.
    pub fn do_toggle_orientation(&mut self) {
        // Apply pending GUI edits first so the toggle acts on what is shown.
        self.display.set_config(self.config.clone());
        match self.display.toggle_orientation() {
            Ok(c) => {
                let live = if self.view.exists { "" } else { " (applies on create)" };
                self.log(
                    format!("⇄ {}×{} ({}){live}", c.width, c.height, c.orientation().label()),
                    LogLevel::Success,
                );
                self.config = c;
            }
            Err(e) => self.log(format!("✗ Toggle failed: {e}"), LogLevel::Error),
        }
        self.sync_view();
        self.save_settings();
        self.refresh();
    }

    /// Resets the editable config to the main display's mode, left of it.
    pub fn reset_to_main(&mut self) {
        if let Some(m) = &self.view.main {
            let name = self.config.name.clone();
            self.config = MonitorConfig::for_main(m);
            self.config.name = name;
        }
    }

    pub fn set_follow_client(&mut self, on: bool) {
        self.display.set_follow_client(on);
        self.sync_view();
        self.save_settings();
    }

    pub fn set_main_monitor(&mut self, name: Option<String>) {
        self.display.set_preferred_main(name);
        self.save_settings();
        self.refresh();
    }

    // Sunshine

    pub fn sunshine_state(&self) -> ServiceState {
        self.sunshine
            .as_ref()
            .map(|s| s.state.clone())
            .unwrap_or(ServiceState::Unknown("…".into()))
    }

    pub fn do_sunshine(&mut self, action: &str) {
        let result = match action {
            "start" => sunshine::start(),
            "stop" => sunshine::stop(),
            _ => sunshine::restart(),
        };
        match result {
            Ok(()) => self.log(format!("Sunshine {action} requested."), LogLevel::Info),
            Err(e) => self.log(format!("✗ Sunshine {action}: {e}"), LogLevel::Error),
        }
        self.refresh();
    }

    pub fn open_web_ui(&mut self) {
        if let Err(e) = sunshine::open_web_ui() {
            self.log(format!("✗ {e}"), LogLevel::Error);
        }
    }

    /// Points Sunshine's `output_name` at the virtual monitor. Sunshine reads
    /// it only at startup, so a running Sunshine is restarted.
    pub fn sync_output_name(&mut self) {
        let name = self.config.name.trim().to_string();
        match sunshine::set_output_name(&name) {
            Ok(false) => {}
            Ok(true) => {
                self.log(format!("✓ Sunshine output_name = {name}"), LogLevel::Success);
                if self.sunshine_state() == ServiceState::Active {
                    self.log("Restarting Sunshine to pick up the new output.", LogLevel::Info);
                    self.do_sunshine("restart");
                }
            }
            Err(e) => self.log(format!("✗ Cannot update sunshine.conf: {e}"), LogLevel::Error),
        }
        self.refresh();
    }

    pub fn set_sync_output_name(&mut self, on: bool) {
        self.settings.sync_output_name = on;
        self.save_settings();
        if on {
            self.sync_output_name();
        }
    }

    pub fn set_prep_cmd(&mut self, install: bool) {
        match sunshine::set_prep_cmd(install) {
            Ok(()) => {
                let what = if install { "installed in" } else { "removed from" };
                self.log(
                    format!("✓ Client-resolution prep-cmd {what} Sunshine's Desktop app."),
                    LogLevel::Success,
                );
                if self.sunshine_state() == ServiceState::Active {
                    self.do_sunshine("restart");
                }
            }
            Err(e) => self.log(format!("✗ apps.json: {e}"), LogLevel::Error),
        }
        self.refresh();
    }

    pub fn pin_pending(&self) -> bool {
        self.pin_rx.is_some()
    }

    pub fn do_send_pin(&mut self) {
        if self.pin_rx.is_some() {
            return;
        }
        let pin = self.pin_input.clone();
        let name = match self.pin_client_name.trim() {
            "" => "moonlight".to_string(),
            n => n.to_string(),
        };
        let user = self.settings.sunshine_user.clone();
        let password = self.sunshine_password.clone();
        let (tx, rx) = mpsc::channel();
        self.pin_rx = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(sunshine::send_pin(&pin, &name, &user, &password));
        });
    }

    fn poll_pin(&mut self) {
        let Some(rx) = &self.pin_rx else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Err("PIN request thread ended".into()),
        };
        self.pin_rx = None;
        match result {
            Ok(()) => {
                self.log("✓ Paired with Moonlight.", LogLevel::Success);
                self.pin_input.clear();
                self.pin_client_name.clear();
            }
            Err(e) => self.log(format!("✗ Pairing: {e}"), LogLevel::Error),
        }
    }

    pub fn set_remember_password(&mut self, on: bool) {
        self.settings.sunshine_password = on.then(|| self.sunshine_password.clone());
        self.save_settings();
    }

    /// Persists the login fields (password only when "remember" is on).
    pub fn credentials_changed(&mut self) {
        if self.settings.sunshine_password.is_some() {
            self.settings.sunshine_password = Some(self.sunshine_password.clone());
        }
        self.save_settings();
    }

    pub fn shutdown(&mut self) {
        if self.shutdown_done {
            return;
        }
        self.shutdown_done = true;
        self.ipc_stop.store(true, Ordering::SeqCst);
        self.save_settings();
        let d = self.display.snapshot();
        if d.exists && d.owned {
            match self.display.remove() {
                Ok(()) => log::info!("virtual monitor removed"),
                Err(e) => log::error!("removing virtual monitor on exit: {e}"),
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.shutdown();
    }
}
