#![allow(dead_code)]
#![allow(clippy::type_complexity)]
#![allow(float_literal_f32_fallback)]
mod app;
mod capture;
mod gui;
mod hypr;
mod types;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() -> Result<(), eframe::Error> {
    // Globale signal flag — wordt gezet door Ctrl+C / SIGTERM.
    // De GUI-loop checkt dit en triggert een graceful shutdown.
    let signal_flag = Arc::new(AtomicBool::new(false));
    let flag_for_handler = Arc::clone(&signal_flag);

    ctrlc::set_handler(move || {
        eprintln!("\n⚠ Signal ontvangen — bezig met opruimen...");
        flag_for_handler.store(true, Ordering::SeqCst);
    })
    .expect("Failed to install signal handler");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 740.0])
            .with_min_inner_size([440.0, 520.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Hyprland Virtual Display",
        options,
        Box::new(move |_cc| Ok(Box::new(app::App::with_signal_flag(signal_flag)))),
    )
}

