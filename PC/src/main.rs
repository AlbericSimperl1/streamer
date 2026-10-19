#![allow(clippy::type_complexity)]

mod app;
mod display;
mod gui;
mod hypr;
mod ipc;
mod settings;
mod sunshine;
mod types;

const USAGE: &str = "\
streamer — GUI for a Hyprland virtual monitor streamed by Sunshine

USAGE:
    streamer                open the GUI
    streamer --toggle       swap the virtual monitor between A×B and B×A
                            (talks to the running GUI; bind it to a key:
                             bind = SUPER SHIFT, O, exec, streamer --toggle)
    streamer --status       print the virtual monitor state
    streamer --client-mode  Sunshine prep-cmd: size the virtual monitor to the
                            Moonlight client (reads SUNSHINE_CLIENT_WIDTH/
                            HEIGHT/FPS)
    streamer --help";

fn main() -> Result<(), eframe::Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("--toggle") | Some("--status") => {
            let cmd = args[0].trim_start_matches("--");
            match ipc::send(cmd) {
                Ok(reply) => {
                    println!("{reply}");
                    std::process::exit(if reply.starts_with("ok") { 0 } else { 1 });
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some(sunshine::CLIENT_MODE_ARG) => client_mode(),
        Some("--help") | Some("-h") => {
            println!("{USAGE}");
            return Ok(());
        }
        Some(other) => {
            eprintln!("unknown argument '{other}'\n\n{USAGE}");
            std::process::exit(2);
        }
        None => {}
    }

    if let Ok(status) = ipc::send("status") {
        eprintln!("streamer is already running ({status}).\nUse `streamer --toggle` or `--status` to talk to it.");
        std::process::exit(1);
    }

    let scale: f32 = 1.5;
    let w: f32 = 857.33333;
    let h: f32 = 444.0;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("hyprland-display-streamer")
            .with_inner_size([scale * w, scale * h])
            .with_min_inner_size([w / scale, h / scale]),
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        "Hyprland Virtual Display",
        options,
        Box::new(move |cc| {
            let app: Box<dyn eframe::App> = Box::new(app::App::new_scaled(cc, scale));
            app
        }),
    )
}

/// Runs as Sunshine prep-cmd. Always exits 0: a failing prep-cmd would make
/// Sunshine refuse the session, and streaming the monitor as it is beats that.
fn client_mode() -> ! {
    let var = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<u32>().ok());
    match (
        var("SUNSHINE_CLIENT_WIDTH"),
        var("SUNSHINE_CLIENT_HEIGHT"),
        var("SUNSHINE_CLIENT_FPS"),
    ) {
        (Some(w), Some(h), fps) => match ipc::send(&format!("client {w} {h} {}", fps.unwrap_or(0))) {
            Ok(reply) => println!("{reply}"),
            Err(e) => eprintln!("streamer GUI not reachable, monitor unchanged: {e}"),
        },
        _ => eprintln!("SUNSHINE_CLIENT_WIDTH/HEIGHT not set; run this from a Sunshine prep-cmd"),
    }
    std::process::exit(0)
}
