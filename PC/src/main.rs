#![allow(dead_code)]
#![allow(clippy::type_complexity)]
#![allow(float_literal_f32_fallback)]
#![allow(deprecated)]
#![allow(unused)]

mod app;
mod capture;
mod gui;
mod hypr;
mod types;

fn main() -> Result<(), eframe::Error> {
    let scale: f32 = 1.5;
    let w: f32 = 857.33333;
    let h: f32 = 422.0;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("hyprland-display-streamer")
            .with_inner_size([scale * w, scale * h])
            .with_min_inner_size([w / scale, h / scale]),
        ..Default::default()
    };

    eframe::run_native(
        "Hyprland Virtual Display",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::new_scaled(_cc, scale)))),
    )
}
