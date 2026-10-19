use serde::{Deserialize, Serialize};

/// Hyprland: `hyprctl monitors all -j`
#[derive(Debug, Clone, Deserialize)]
pub struct MonitorJson {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub make: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub serial: String,
    pub width: u32,
    pub height: u32,
    #[serde(rename = "refreshRate")]
    pub fps: f64,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
    #[serde(default)]
    pub transform: u32,
    #[serde(default)]
    pub focused: bool,
    #[serde(default)]
    pub disabled: bool,
}

impl MonitorJson {
    /// Headless outputs (`hyprctl output create headless`) report no EDID data.
    pub fn is_headless(&self) -> bool {
        self.make.is_empty() && self.model.is_empty() && self.serial.is_empty()
    }

    /// Size in layout (logical) coordinates, i.e. after transform and scale.
    pub fn logical_size(&self) -> (i32, i32) {
        let (w, h) = if self.transform % 2 == 1 {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        };
        let s = if self.scale > 0.0 { self.scale } else { 1.0 };
        ((w as f32 / s).round() as i32, (h as f32 / s).round() as i32)
    }
}

/// The physical screen the virtual monitor is positioned against.
#[derive(Debug, Clone, PartialEq)]
pub struct MainDisplay {
    pub name: String,
    pub description: String,
    pub width: u32,
    pub height: u32,
    pub refresh: f64,
    pub scale: f32,
    pub x: i32,
    pub y: i32,
    /// Layout size (after transform/scale).
    pub logical_w: i32,
    pub logical_h: i32,
}

impl MainDisplay {
    pub fn from_json(m: &MonitorJson) -> Self {
        let (logical_w, logical_h) = m.logical_size();
        Self {
            name: m.name.clone(),
            description: m.description.clone(),
            width: m.width,
            height: m.height,
            refresh: m.fps,
            scale: m.scale,
            x: m.x,
            y: m.y,
            logical_w,
            logical_h,
        }
    }

    pub fn fps(&self) -> u32 {
        (self.refresh.round() as u32).max(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    Landscape,
    Portrait,
}

impl Orientation {
    pub fn of(width: u32, height: u32) -> Self {
        if height > width {
            Orientation::Portrait
        } else {
            Orientation::Landscape
        }
    }

    pub fn flipped(self) -> Self {
        match self {
            Orientation::Landscape => Orientation::Portrait,
            Orientation::Portrait => Orientation::Landscape,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Orientation::Landscape => "landscape",
            Orientation::Portrait => "portrait",
        }
    }

    /// Orders a resolution so it matches this orientation.
    pub fn apply(self, width: u32, height: u32) -> (u32, u32) {
        let (long, short) = (width.max(height), width.min(height));
        match self {
            Orientation::Landscape => (long, short),
            Orientation::Portrait => (short, long),
        }
    }
}

/// Virtual monitor parameters. `width`/`height` are the current mode, already
/// in the current orientation; `x`/`y` are layout coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
}

/// Explicit default used only when the main display cannot be read; the GUI
/// and log say so when it is in effect.
pub const FALLBACK_MODE: (u32, u32, u32) = (1920, 1080, 60);

impl MonitorConfig {
    /// Defaults derived from the real main display: same mode and scale,
    /// placed directly to its left.
    pub fn for_main(main: &MainDisplay) -> Self {
        let mut c = Self {
            name: "virtual".into(),
            width: main.width,
            height: main.height,
            fps: main.fps(),
            x: 0,
            y: main.y,
            scale: main.scale,
        };
        c.x = main.x - c.logical_size().0;
        c
    }

    pub fn fallback() -> Self {
        let (width, height, fps) = FALLBACK_MODE;
        Self {
            name: "virtual".into(),
            width,
            height,
            fps,
            x: -(width as i32),
            y: 0,
            scale: 1.0,
        }
    }

    pub fn orientation(&self) -> Orientation {
        Orientation::of(self.width, self.height)
    }

    pub fn logical_size(&self) -> (i32, i32) {
        let s = if self.scale > 0.0 { self.scale } else { 1.0 };
        (
            (self.width as f32 / s).round() as i32,
            (self.height as f32 / s).round() as i32,
        )
    }

    /// `name,WxH@fps,XxY,scale`.
    pub fn to_keyword(&self) -> String {
        format!(
            "{},{}x{}@{},{}x{},{}",
            self.name, self.width, self.height, self.fps, self.x, self.y, self.scale
        )
    }

    /// Lua form for Hyprland's `hyprctl eval`.
    pub fn to_lua(&self) -> String {
        let name = self.name.replace('\\', "\\\\").replace('"', "\\\"");
        format!(
            "hl.monitor({{output = \"{name}\", mode = \"{}x{}@{}\", position = \"{}x{}\", scale = {}}})",
            self.width, self.height, self.fps, self.x, self.y, self.scale
        )
    }

    /// Changes the mode and keeps the monitor glued to the same edge of the
    /// main display it was attached to (left/above edges move with the size).
    pub fn resize_keeping_edge(&mut self, width: u32, height: u32, main: Option<&MainDisplay>) {
        let (old_w, old_h) = self.logical_size();
        self.width = width;
        self.height = height;
        let (new_w, new_h) = self.logical_size();
        let Some(main) = main else {
            return;
        };
        if self.x + old_w <= main.x {
            // left of main: right edge stays on main's left edge
            self.x = main.x - new_w;
        }
        if self.y + old_h <= main.y {
            // above main: bottom edge stays on main's top edge
            self.y = main.y - new_h;
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

pub struct LogEntry {
    pub time: String,
    pub message: String,
    pub level: LogLevel,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn main_display() -> MainDisplay {
        MainDisplay {
            name: "DP-2".into(),
            description: String::new(),
            width: 3840,
            height: 2160,
            refresh: 59.997,
            scale: 1.0,
            x: 2304,
            y: 0,
            logical_w: 3840,
            logical_h: 2160,
        }
    }

    #[test]
    fn defaults_follow_main_display() {
        let c = MonitorConfig::for_main(&main_display());
        assert_eq!((c.width, c.height, c.fps), (3840, 2160, 60));
        assert_eq!(c.x, 2304 - 3840);
    }

    #[test]
    fn orientation_toggle_keeps_left_edge() {
        let main = main_display();
        let mut c = MonitorConfig {
            name: "v".into(),
            width: 2000,
            height: 1500,
            fps: 60,
            x: 2304 - 1000,
            y: 0,
            scale: 2.0,
        };
        let (w, h) = c.orientation().flipped().apply(c.width, c.height);
        c.resize_keeping_edge(w, h, Some(&main));
        assert_eq!((c.width, c.height), (1500, 2000));
        assert_eq!(c.x + c.logical_size().0, 2304);
    }

    #[test]
    fn headless_detection_and_logical_size() {
        let m = MonitorJson {
            id: 1,
            name: "HEADLESS-1".into(),
            description: String::new(),
            make: String::new(),
            model: String::new(),
            serial: String::new(),
            width: 2880,
            height: 2160,
            fps: 60.0,
            x: 0,
            y: 0,
            scale: 1.25,
            transform: 1,
            focused: false,
            disabled: false,
        };
        assert!(m.is_headless());
        assert_eq!(m.logical_size(), (1728, 2304));
    }
}
