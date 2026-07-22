#![allow(dead_code)]
#![allow(clippy::type_complexity)]
// #![allow(float_literal_f32_fallback)]
mod app;
mod capture;
mod gui;
mod hypr;
mod types;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("hyprland-display-streamer")
            .with_inner_size([840.0, 400.0])
            .with_min_inner_size([720.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Hyprland Virtual Display",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::new()))),
    )
}
