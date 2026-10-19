//! Tiny control socket so a hotkey or Sunshine's prep-cmd can drive the
//! running GUI: `streamer --toggle` (bind it in Hyprland, e.g.
//! `bind = SUPER SHIFT, O, exec, streamer --toggle`), `streamer --status`,
//! `streamer --client-mode`.

use crate::display::Display;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

pub fn socket_path() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("hyprstream.sock")
}

/// Handles commands on the GUI's behalf; `log` receives messages for the
/// GUI log. Commands act on `Display` directly so they work even while the
/// window is minimized and not repainting.
pub fn serve(display: Display, log: Sender<String>, stop: Arc<AtomicBool>) -> Result<(), String> {
    let path = socket_path();
    if UnixStream::connect(&path).is_ok() {
        return Err(format!("another instance is listening on {path:?}"));
    }
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).map_err(|e| format!("bind {path:?}: {e}"))?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    std::thread::Builder::new()
        .name("hyprstream-ipc".into())
        .spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((conn, _)) => handle(conn, &display, &log),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(200)),
                }
            }
            let _ = std::fs::remove_file(&path);
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn handle(mut conn: UnixStream, display: &Display, log: &Sender<String>) {
    let _ = conn.set_nonblocking(false);
    let _ = conn.set_read_timeout(Some(Duration::from_secs(2)));
    let mut line = String::new();
    if BufReader::new(&conn).read_line(&mut line).is_err() {
        return;
    }
    let line = line.trim();
    let reply = match line {
        "toggle" => match display.toggle_orientation() {
            Ok(c) => {
                let msg = format!("⇄ orientation → {}×{} ({})", c.width, c.height, c.orientation().label());
                let _ = log.send(msg.clone());
                format!("ok {msg}")
            }
            Err(e) => {
                let _ = log.send(format!("✗ toggle failed: {e}"));
                format!("error {e}")
            }
        },
        "status" => {
            let d = display.snapshot();
            let c = d.config;
            format!(
                "ok monitor {} {}x{}@{} at {}x{} ({})",
                c.name,
                c.width,
                c.height,
                c.fps,
                c.x,
                c.y,
                if d.exists { "active" } else { "not created" }
            )
        }
        cmd if cmd.starts_with("client ") => client_mode(cmd, display, log),
        other => format!("error unknown command '{other}'"),
    };
    let _ = writeln!(conn, "{reply}");
}

/// `client W H FPS`: a Moonlight session is starting.
fn client_mode(cmd: &str, display: &Display, log: &Sender<String>) -> String {
    let nums: Vec<u32> = cmd
        .split_whitespace()
        .skip(1)
        .filter_map(|n| n.parse().ok())
        .collect();
    let [w, h, fps] = nums[..] else {
        return "error usage: client W H FPS".into();
    };
    match display.prepare_session(w, h, fps) {
        Ok(c) => {
            let msg = format!(
                "Moonlight session {w}×{h}@{fps} → monitor {}×{}@{}",
                c.width, c.height, c.fps
            );
            let _ = log.send(msg.clone());
            format!("ok {msg}")
        }
        Err(e) => {
            let _ = log.send(format!("✗ session setup failed: {e}"));
            format!("error {e}")
        }
    }
}

/// Client side for `streamer --<command>`. Returns the instance's reply.
pub fn send(command: &str) -> Result<String, String> {
    let path = socket_path();
    let mut conn = UnixStream::connect(&path).map_err(|e| format!("no running instance ({path:?}): {e}"))?;
    let _ = conn.set_read_timeout(Some(Duration::from_secs(5)));
    writeln!(conn, "{command}").map_err(|e| e.to_string())?;
    let mut reply = String::new();
    BufReader::new(&conn)
        .read_line(&mut reply)
        .map_err(|e| e.to_string())?;
    Ok(reply.trim().to_string())
}
