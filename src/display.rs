//! The virtual (headless) monitor, shared between the GUI and the hotkey
//! socket: main-display detection, create/update/remove, orientation toggle
//! and per-session mode selection.

use crate::hypr;
use crate::types::{MainDisplay, MonitorConfig, MonitorJson};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone, Debug)]
pub struct DisplayState {
    pub config: MonitorConfig,
    pub exists: bool,
    pub main: Option<MainDisplay>,
    /// Why the main display could not be read (GUI shows this).
    pub main_error: Option<String>,
    /// Other physical monitors that could serve as main display.
    pub physical: Vec<String>,
    pub preferred_main: Option<String>,
    /// Size the virtual monitor to what Moonlight asks for at session start
    /// (Sunshine prep-cmd `streamer --client-mode`).
    pub follow_client: bool,
    /// Bumped whenever the monitor geometry is applied.
    pub generation: u64,
    /// This process created or configured the output (only then it is
    /// removed on exit).
    pub owned: bool,
}

#[derive(Clone)]
pub struct Display {
    inner: Arc<Mutex<DisplayState>>,
}

impl Display {
    /// `saved` is the persisted monitor config, if any. Without one the
    /// defaults come from the real main display, or an explicit fallback.
    pub fn new(saved: Option<MonitorConfig>, preferred_main: Option<String>, follow_client: bool) -> Self {
        let monitors = hypr::get_monitors();
        let virtual_name = saved.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "virtual".into());
        let (main, main_error, physical, exists) = match &monitors {
            Ok(ms) => {
                let main = hypr::pick_main(ms, &virtual_name, preferred_main.as_deref());
                let err = main.is_none().then(|| "no physical monitor found".to_string());
                (main, err, physical_names(ms, &virtual_name), ms.iter().any(|m| m.name == virtual_name))
            }
            Err(e) => (None, Some(e.clone()), Vec::new(), false),
        };

        let config = match (saved, &main) {
            (Some(c), _) => c,
            (None, Some(m)) => MonitorConfig::for_main(m),
            (None, None) => {
                log::error!(
                    "display: could not read the main display ({}); using explicit fallback {}x{}@{}",
                    main_error.as_deref().unwrap_or("?"),
                    crate::types::FALLBACK_MODE.0,
                    crate::types::FALLBACK_MODE.1,
                    crate::types::FALLBACK_MODE.2
                );
                MonitorConfig::fallback()
            }
        };

        Self {
            inner: Arc::new(Mutex::new(DisplayState {
                config,
                exists,
                main,
                main_error,
                physical,
                preferred_main,
                follow_client,
                generation: 0,
                owned: false,
            })),
        }
    }

    fn lock(&self) -> MutexGuard<'_, DisplayState> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn snapshot(&self) -> DisplayState {
        self.lock().clone()
    }

    pub fn config(&self) -> MonitorConfig {
        self.lock().config.clone()
    }

    /// GUI edits; takes effect on the next `apply`.
    pub fn set_config(&self, c: MonitorConfig) {
        self.lock().config = c;
    }

    pub fn set_follow_client(&self, on: bool) {
        self.lock().follow_client = on;
    }

    pub fn set_preferred_main(&self, name: Option<String>) {
        self.lock().preferred_main = name;
    }

    /// Updates main display / existence from a fresh `hyprctl monitors`.
    pub fn update_from_monitors(&self, monitors: &[MonitorJson]) {
        let mut g = self.lock();
        let name = g.config.name.clone();
        g.exists = monitors.iter().any(|m| m.name == name);
        g.physical = physical_names(monitors, &name);
        let main = hypr::pick_main(monitors, &name, g.preferred_main.as_deref());
        g.main_error = main.is_none().then(|| "no physical monitor found".to_string());
        g.main = main;
    }

    pub fn set_main_error(&self, e: String) {
        self.lock().main_error = Some(e);
    }

    /// Creates the output if needed and applies the current config.
    pub fn apply(&self) -> Result<MonitorConfig, String> {
        let mut g = self.lock();
        apply_locked(&mut g)?;
        Ok(g.config.clone())
    }

    pub fn remove(&self) -> Result<(), String> {
        let mut g = self.lock();
        let name = g.config.name.clone();
        hypr::remove_monitor(&name)?;
        g.exists = false;
        g.owned = false;
        g.generation += 1;
        Ok(())
    }

    /// `A×B ⇄ B×A`. Applied immediately when the monitor exists; Sunshine
    /// re-initializes its capture when the output size changes.
    pub fn toggle_orientation(&self) -> Result<MonitorConfig, String> {
        let mut g = self.lock();
        let (w, h) = g.config.orientation().flipped().apply(g.config.width, g.config.height);
        let main = g.main.clone();
        g.config.resize_keeping_edge(w, h, main.as_ref());
        if g.exists {
            apply_locked(&mut g)?;
        }
        Ok(g.config.clone())
    }

    /// Sets the monitor up for a Moonlight session asking for `w×h@fps`
    /// (called from Sunshine's prep-cmd before capture starts).
    /// With `follow_client` the mode becomes the client's resolution in the
    /// monitor's current orientation; otherwise the configured mode is kept.
    pub fn prepare_session(&self, w: u32, h: u32, fps: u32) -> Result<MonitorConfig, String> {
        let mut g = self.lock();
        if g.follow_client && w > 0 && h > 0 {
            let orientation = g.config.orientation();
            let (nw, nh) = orientation.apply(w, h);
            let main = g.main.clone();
            g.config.resize_keeping_edge(nw, nh, main.as_ref());
            if fps > 0 {
                g.config.fps = fps;
            }
        }
        apply_locked(&mut g)?;
        Ok(g.config.clone())
    }
}

fn physical_names(monitors: &[MonitorJson], virtual_name: &str) -> Vec<String> {
    monitors
        .iter()
        .filter(|m| !m.disabled && !m.is_headless() && m.name != virtual_name)
        .map(|m| m.name.clone())
        .collect()
}

fn apply_locked(g: &mut DisplayState) -> Result<(), String> {
    if g.config.name.trim().is_empty() {
        return Err("monitor name is empty".into());
    }
    if !g.exists {
        hypr::create_headless_output(&g.config.name)?;
        g.exists = true;
    }
    hypr::apply_monitor(&g.config)?;
    g.owned = true;
    g.generation += 1;
    log::info!("display: applied {}", g.config.to_keyword());
    Ok(())
}
