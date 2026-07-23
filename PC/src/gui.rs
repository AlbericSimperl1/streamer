// use crate::app::App;
// use eframe::egui;
// use egui::accesskit::TextAlign::Center;
// use std::time::Duration;

// // ─── Kleurenpalet ──────────────────────────────────────────────

// const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(255, 232, 106);
// const DANGER_RED: egui::Color32 = egui::Color32::from_rgb(239, 68, 68);
// const DANGER_HOVER: egui::Color32 = egui::Color32::from_rgb(248, 113, 113);

// // mijn kleuren
// /// panels enzo
// const C1: egui::Color32 = egui::Color32::from_rgba_premultiplied(4, 6, 10, 5);
// const C2: egui::Color32 = egui::Color32::from_rgb(33, 35, 39); // panels
// const C3: egui::Color32 = egui::Color32::from_rgb(23, 25, 29); // input fields
// const C4: egui::Color32 = egui::Color32::from_rgb(84, 86, 90); // borders

// /// text
// const T0: egui::Color32 = egui::Color32::from_rgb(255, 255, 255); // "titels"
// const T1: egui::Color32 = egui::Color32::from_rgb(255, 246, 226); // primary
// const T2: egui::Color32 = egui::Color32::from_rgb(186, 178, 162); // inactive

// /// accent
// const A1: egui::Color32 = egui::Color32::from_rgb(255, 238, 143); // primary
// const A2: egui::Color32 = egui::Color32::from_rgb(97, 83, 21); // secondary

// impl App {
//     // Bestaande new() blijft zoals hij was:
//     pub fn new() -> Self {
//         Self::with_signal_flag_opt(None)
//     }

//     pub fn new_scaled(cc: &eframe::CreationContext, scale: f32) -> Self {
//         // 1. Stel de schaal EENMALIG in bij het opstarten
//         configure_style(&cc.egui_ctx, scale);

//         // 2. Maak de app gewoon aan via new()
//         Self::new()
//     }
// }

// impl eframe::App for App {
//     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//         // configure_style(ctx, self.scale);

//         self.tick();
//         if self.auto_refresh {
//             ctx.request_repaint_after(Duration::from_secs(1));
//         }

//         if self.should_quit() {
//             self.shutdown();
//             ctx.send_viewport_cmd(egui::ViewportCommand::Close);
//             return;
//         }

//         let capturing = self.is_capturing();
//         let stopping = self.is_stopping();
//         if capturing || stopping {
//             ctx.request_repaint_after(Duration::from_millis(100));
//         }

//         self.poll_stop_result();

//         // Middengebied: Left Panel (Config + Buttons) & Right Panel (Position canvas)
//         egui::CentralPanel::default()
//             .frame(
//                 egui::Frame::none()
//                     .fill(C1)
//                     .inner_margin(egui::Margin::symmetric(1.5, 1.5)),
//             )
//             .show(ctx, |ui| {
//                 // Stel de horizontale ruimte tussen de kaarten in (bijv. 8.0 px):
//                 ui.spacing_mut().item_spacing.x = 8.0;

//                 ui.horizontal(|ui| {
//                     // Links: Config + Geclusterde knoppen
//                     self.render_config_card(ui, stopping);

//                     // Rechts: Position Canvas (vult alle overgebleven ruimte op!)
//                     self.render_pos_card(ui);
//                 });
//             });
//     }

//     fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
//         self.shutdown();
//     }
// }

// impl App {
//     /// Linker Paneel: Wrapper voor Configuraties & Geclusterde Knoppen
//     fn render_config_card(&mut self, ui: &mut egui::Ui, stopping: bool) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(280.0);
//             ui.set_height(ui.available_height());

//             ui.vertical(|ui| {
//                 ui.label(
//                     egui::RichText::new("configuration")
//                         .color(T0)
//                         .size(15.0)
//                         .monospace()
//                         .strong(),
//                 );
//                 ui.add_space(8.0);

//                 // 1. De configuratie velden (Grid)
//                 self.render_config(ui);

//                 ui.add_space(8.0);
//                 ui.separator();
//                 ui.add_space(8.0);

//                 // 2. De actieknoppen
//                 self.render_controls(ui, stopping);
//             });
//         });
//     }

//     /// Configuratie Grid (Identifier, Width, Height, FPS, Scale, Status)
//     fn render_config(&mut self, ui: &mut egui::Ui) {
//         egui::Grid::new("config_grid")
//             .num_columns(2)
//             .spacing([12.0, 10.0])
//             .show(ui, |ui| {
//                 // Identifier
//                 ui.label(egui::RichText::new("identifier :").color(T1).monospace());
//                 ui.horizontal(|ui| {
//                     ui.add_enabled_ui(!self.monitor_exists, |ui| {
//                         ui.add(
//                             egui::TextEdit::singleline(&mut self.config.name).desired_width(110.0),
//                         );
//                     });
//                     if self.monitor_exists {
//                         ui.label(egui::RichText::new("✓").color(T0).monospace());
//                     }
//                 });
//                 ui.end_row();

//                 // Width
//                 ui.label(egui::RichText::new("width :").color(T1).monospace());
//                 ui.add(
//                     egui::DragValue::new(&mut self.config.width)
//                         .range(320..=7680)
//                         .suffix(" px"),
//                 );
//                 ui.end_row();

//                 // Height
//                 ui.label(egui::RichText::new("height :").color(T1).monospace());
//                 ui.add(
//                     egui::DragValue::new(&mut self.config.height)
//                         .range(320..=7680)
//                         .suffix(" px"),
//                 );
//                 ui.end_row();

//                 // Frame Rate
//                 ui.label(egui::RichText::new("frame rate :").color(T1).monospace());
//                 ui.add(
//                     egui::DragValue::new(&mut self.config.fps)
//                         .range(1..=240)
//                         .suffix(" Hz"),
//                 );
//                 ui.end_row();

//                 // Scale
//                 ui.label(egui::RichText::new("scale :").color(T1).monospace());
//                 ui.add(
//                     egui::DragValue::new(&mut self.config.scale)
//                         .range(0.5f32..=3.0f32)
//                         .speed(0.1),
//                 );

//                 ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
//                     let capturing = self.is_capturing();
//                     let (status_text, color) = if capturing {
//                         ("● STREAMING", A1)
//                     } else if self.is_stopping() {
//                         ("⏳ STOPPING", DANGER_RED)
//                     } else if self.monitor_exists {
//                         ("● ONLINE", T1)
//                     } else {
//                         ("○ OFFLINE", T2)
//                     };

//                     ui.label(
//                         egui::RichText::new(status_text)
//                             .color(color)
//                             .size(12.0)
//                             .monospace()
//                             .strong(),
//                     );
//                 });
//                 ui.end_row();
//             });
//     }

//     /// Actieknoppen (Create/Update, Remove, Start, Stop)
//     fn render_controls(&mut self, ui: &mut egui::Ui, stopping: bool) {
//         // Bugfix: can_apply checkt nu alleen of de naam niet leeg is en we niet aan het capturen zijn.
//         // Hierdoor mag je zowel Create doen als Update uitvoeren.
//         let can_apply = !self.config.name.is_empty() && !self.is_capturing() && !stopping;
//         let can_remove = self.monitor_exists && !self.is_capturing() && !stopping;
//         let can_start = self.monitor_exists && !self.is_capturing() && !stopping;
//         let can_stop = self.is_capturing() && !stopping;

//         let btn_width = (ui.available_width() - 8.0) / 2.0;

//         ui.horizontal(|ui| {
//             // Button 1: Create / Update
//             let button_text = if self.monitor_exists {
//                 "Update"
//             } else {
//                 "Create"
//             };
//             if custom_button(ui, button_text, can_apply, A1, ACCENT_HOVER, btn_width).clicked() {
//                 self.apply_config();
//             }

//             // Button 2: Remove
//             if custom_button(
//                 ui,
//                 "Remove",
//                 can_remove,
//                 DANGER_RED,
//                 DANGER_HOVER,
//                 btn_width,
//             )
//             .clicked()
//             {
//                 self.do_remove();
//             }
//         });

//         ui.add_space(3.0);

//         ui.horizontal(|ui| {
//             // Button 3: Start
//             let start_color = egui::Color32::from_rgb(129, 140, 248);
//             let start_hover = egui::Color32::from_rgb(165, 180, 252);
//             if custom_button(ui, "Start", can_start, start_color, start_hover, btn_width).clicked()
//             {
//                 self.do_start_capture();
//             }

//             // Button 4: Stop
//             if custom_button(ui, "Stop", can_stop, DANGER_RED, DANGER_HOVER, btn_width).clicked() {
//                 self.do_stop_capture();
//             }
//         });
//     }

//     /// Rechter Paneel: Position Canvas (vult volledige ruimte op)
//     fn render_pos_card(&mut self, ui: &mut egui::Ui) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(ui.available_width());
//             ui.set_height(ui.available_height());

//             ui.vertical(|ui| {
//                 ui.horizontal(|ui| {
//                     ui.label(
//                         egui::RichText::new(format!(
//                             "position (x: {}  y: {})",
//                             self.config.x, self.config.y
//                         ))
//                         .color(T0)
//                         .monospace()
//                         .size(14.0),
//                     );
//                 });
//                 ui.add_space(3.0);

//                 // Dynamic Inner Canvas
//                 let canvas_size = ui.available_size();
//                 let (response, painter) =
//                     ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());
//                 let canvas_rect = response.rect;

//                 let scale = 0.065;
//                 let main_w = 1920.0;
//                 let main_h = 1080.0;
//                 let main_x = 0.0;
//                 let main_y = 0.0;

//                 let virt_w = self.config.width as f32;
//                 let virt_h = self.config.height as f32;

//                 // Dragging Logic
//                 if response.drag_started() {
//                     ui.memory_mut(|m| {
//                         m.data
//                             .insert_temp(egui::Id::new("virt_raw_x"), self.config.x as f32);
//                         m.data
//                             .insert_temp(egui::Id::new("virt_raw_y"), self.config.y as f32);
//                     });
//                 }

//                 if response.dragged() {
//                     let delta = response.drag_delta();
//                     let raw_x = ui.memory_mut(|m| {
//                         let val = m
//                             .data
//                             .get_temp_mut_or(egui::Id::new("virt_raw_x"), self.config.x as f32);
//                         *val += delta.x / scale;
//                         *val
//                     });

//                     let raw_y = ui.memory_mut(|m| {
//                         let val = m
//                             .data
//                             .get_temp_mut_or(egui::Id::new("virt_raw_y"), self.config.y as f32);
//                         *val += delta.y / scale;
//                         *val
//                     });

//                     self.config.x = raw_x as i32;
//                     self.config.y = raw_y as i32;
//                 }

//                 if response.drag_stopped() {
//                     let grid_step = 90;
//                     self.config.x = ((self.config.x as f32 / grid_step as f32).round()
//                         * grid_step as f32) as i32;
//                     self.config.y = ((self.config.y as f32 / grid_step as f32).round()
//                         * grid_step as f32) as i32;

//                     ui.memory_mut(|m| {
//                         m.data
//                             .insert_temp(egui::Id::new("virt_raw_x"), self.config.x as f32);
//                         m.data
//                             .insert_temp(egui::Id::new("virt_raw_y"), self.config.y as f32);
//                     });
//                 }

//                 // Teken logica
//                 painter.rect_filled(canvas_rect, 2.0, C3);
//                 painter.rect_stroke(canvas_rect, 2.0, egui::Stroke::new(0.0, C4));

//                 let center_x = canvas_rect.center().x;
//                 let center_y = canvas_rect.center().y;

//                 let origin_x = center_x - (main_w / 2.0) * scale;
//                 let origin_y = center_y - (main_h / 2.0) * scale;

//                 let to_canvas = |sx: f32, sy: f32| -> egui::Pos2 {
//                     egui::pos2(origin_x + sx * scale, origin_y + sy * scale)
//                 };

//                 // Grid
//                 let grid_step_canvas = 90.0 * scale;
//                 let mut grid_x = origin_x;
//                 while grid_x > canvas_rect.min.x {
//                     grid_x -= grid_step_canvas;
//                 }
//                 while grid_x < canvas_rect.max.x {
//                     painter.line_segment(
//                         [
//                             egui::pos2(grid_x, canvas_rect.min.y),
//                             egui::pos2(grid_x, canvas_rect.max.y),
//                         ],
//                         egui::Stroke::new(0.5, egui::Color32::from_rgb(45, 47, 51)),
//                     );
//                     grid_x += grid_step_canvas;
//                 }

//                 let mut grid_y = origin_y;
//                 while grid_y > canvas_rect.min.y + 2.0 {
//                     grid_y -= grid_step_canvas;
//                 }
//                 while grid_y < canvas_rect.max.y + 1.0 {
//                     painter.line_segment(
//                         [
//                             egui::pos2(canvas_rect.min.x, grid_y),
//                             egui::pos2(canvas_rect.max.x, grid_y),
//                         ],
//                         egui::Stroke::new(0.5, egui::Color32::from_rgb(45, 47, 51)),
//                     );
//                     grid_y += grid_step_canvas;
//                 }

//                 // Main Monitor
//                 let main_top_left = to_canvas(main_x, main_y);
//                 let main_rect = egui::Rect::from_min_size(
//                     main_top_left,
//                     egui::vec2(main_w * scale, main_h * scale),
//                 );

//                 painter.rect_filled(main_rect, 2.0, egui::Color32::from_rgb(40, 40, 38));
//                 painter.rect_stroke(main_rect, 2.0, egui::Stroke::new(3.0, C4));
//                 painter.text(
//                     main_rect.center(),
//                     egui::Align2::CENTER_CENTER,
//                     "main\n(DP-1)",
//                     egui::FontId::monospace(10.0),
//                     T1,
//                 );

//                 // Virtual Monitor
//                 let virt_top_left = to_canvas(self.config.x as f32, self.config.y as f32);

//                 let virt_rect = egui::Rect::from_min_size(
//                     virt_top_left,
//                     egui::vec2(virt_w * scale, virt_h * scale),
//                 );

//                 let (fill_col, stroke_col) = if self.monitor_exists {
//                     (egui::Color32::from_rgb(142, 122, 31), A1)
//                 } else {
//                     (A2, A1)
//                 };

//                 let is_grabbed = response.dragged();
//                 let actual_stroke = if is_grabbed {
//                     egui::Stroke::new(3.0, ACCENT_HOVER)
//                 } else {
//                     egui::Stroke::new(3.0, stroke_col)
//                 };

//                 painter.rect_filled(virt_rect, 2.0, fill_col);
//                 painter.rect_stroke(virt_rect, 2.0, actual_stroke);
//                 painter.text(
//                     virt_rect.center(),
//                     egui::Align2::CENTER_CENTER,
//                     &format!(
//                         "{}\n{}x{}",
//                         self.config.name, self.config.width, self.config.height
//                     ),
//                     egui::FontId::monospace(10.0),
//                     A1,
//                 );
//             });
//         });
//     }
// }

// // ─── Styling Helpers ───────────────────────────────────────────

// fn configure_style(ctx: &egui::Context, scale: f32) {
//     // Stel de schaalfactor globally in (1.25 = 125% van normale grootte)
//     // let scale = 1.25;
//     ctx.set_pixels_per_point(scale);

//     let mut style = (*ctx.style()).clone();

//     style.spacing.item_spacing = egui::vec2(8.0, 8.0);
//     style.spacing.button_padding = egui::vec2(8.0, 4.0);

//     let rounding = egui::Rounding::same(2.0);
//     style.visuals.window_rounding = rounding;
//     style.visuals.widgets.noninteractive.rounding = rounding;
//     style.visuals.widgets.inactive.rounding = rounding;
//     style.visuals.widgets.hovered.rounding = rounding;
//     style.visuals.widgets.active.rounding = rounding;

//     style.visuals.dark_mode = true;
//     style.visuals.code_bg_color = C3;
//     style.visuals.override_text_color = Some(T1);

//     style.visuals.widgets.inactive.bg_fill = C3;
//     style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, T1);
//     style.visuals.widgets.hovered.bg_fill = C3;
//     style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, T0);
//     style.visuals.widgets.active.bg_fill = C3;
//     style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, T0);

//     ctx.set_style(style);
// }

// fn panel_frame() -> egui::Frame {
//     egui::Frame {
//         fill: C2,
//         inner_margin: egui::Margin::same(3.0),
//         stroke: egui::Stroke::new(1.0, C4),
//         ..Default::default()
//     }
// }

// fn inner_frame() -> egui::Frame {
//     egui::Frame {
//         fill: C3,
//         inner_margin: egui::Margin::same(8.0),
//         stroke: egui::Stroke::new(1.0, C4),
//         ..Default::default()
//     }
// }

// fn set_button_style(ui: &mut egui::Ui, bg: egui::Color32, hover: egui::Color32, fg: egui::Color32) {
//     let style = ui.style_mut();
//     style.visuals.widgets.inactive.bg_fill = bg;
//     style.visuals.widgets.hovered.bg_fill = bg;
//     style.visuals.widgets.active.bg_fill = bg;
//     style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, fg);
//     style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, fg);
// }

// fn ghost_button(text: impl Into<String>, color: egui::Color32) -> impl egui::Widget {
//     egui::Button::new(
//         egui::RichText::new(text.into())
//             .color(color)
//             .monospace()
//             .size(12.0),
//     )
//     .fill(egui::Color32::TRANSPARENT)
//     .stroke(egui::Stroke::NONE)
// }

// use egui::{Align2, Color32, FontId, Response, Sense, Stroke, Ui, Vec2};

// /// Tekent een aangepaste knop met harde disable-beveiliging.
// pub fn custom_button(
//     ui: &mut Ui,
//     text: &str,
//     enabled: bool,
//     accent_color: Color32,
//     hover_color: Color32,
//     width: f32,
// ) -> Response {
//     let desired_size = Vec2::new(width, 32.0);

//     // HARD SECURITY: Als 'enabled' false is, luisteren we NIET naar kliks.
//     // Dit voorkomt dat de knop reageert als hij grijs getekend staat.
//     let sense = if enabled {
//         Sense::click()
//     } else {
//         Sense::hover()
//     };

//     let (rect, response) = ui.allocate_exact_size(desired_size, sense);

//     // Render-berekeningen alleen uitvoeren als de widget zichtbaar is op het scherm
//     if ui.is_rect_visible(rect) {
//         // Bepaal visuele staat
//         let (border_col, text_col) = if !enabled {
//             (Color32::from_rgb(48, 50, 56), T2)
//         } else if response.hovered() {
//             (hover_color, T0)
//         } else {
//             (accent_color, accent_color)
//         };

//         // Teken de achtergrond en rand
//         ui.painter().rect_filled(rect, 2.0, C1);
//         ui.painter()
//             .rect_stroke(rect, 2.0, Stroke::new(1.0, border_col));

//         // Teken de tekst in het midden
//         ui.painter().text(
//             rect.center(),
//             Align2::CENTER_CENTER,
//             text,
//             FontId::monospace(13.0),
//             text_col,
//         );
//     }

//     response
// }

use crate::app::App;
use eframe::egui;
use egui::accesskit::TextAlign::Center;
use std::time::Duration;

// ─── Kleurenpalet ──────────────────────────────────────────────

const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(255, 232, 106);
const DANGER_RED: egui::Color32 = egui::Color32::from_rgb(239, 68, 68);
const DANGER_HOVER: egui::Color32 = egui::Color32::from_rgb(248, 113, 113);

// mijn kleuren
/// panels enzo
const C1: egui::Color32 = egui::Color32::from_rgba_premultiplied(4, 6, 10, 5);
const C2: egui::Color32 = egui::Color32::from_rgb(33, 35, 39); // panels
const C3: egui::Color32 = egui::Color32::from_rgb(23, 25, 29); // input fields
const C4: egui::Color32 = egui::Color32::from_rgb(84, 86, 90); // borders

/// text
const T0: egui::Color32 = egui::Color32::from_rgb(255, 255, 255); // "titels"
const T1: egui::Color32 = egui::Color32::from_rgb(255, 246, 226); // primary
const T2: egui::Color32 = egui::Color32::from_rgb(186, 178, 162); // inactive

/// accent
const A1: egui::Color32 = egui::Color32::from_rgb(255, 238, 143); // primary
const A2: egui::Color32 = egui::Color32::from_rgb(97, 83, 21); // secondary

impl App {
    // Bestaande new() blijft zoals hij was:
    pub fn new() -> Self {
        Self::with_signal_flag_opt(None)
    }

    pub fn new_scaled(cc: &eframe::CreationContext, scale: f32) -> Self {
        // 1. Stel de schaal EENMALIG in bij het opstarten
        configure_style(&cc.egui_ctx, scale);

        // 2. Maak de app gewoon aan via new()
        Self::new()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // configure_style(ctx, self.scale);

        self.tick();
        if self.auto_refresh {
            ctx.request_repaint_after(Duration::from_secs(1));
        }

        if self.should_quit() {
            self.shutdown();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let capturing = self.is_capturing();
        let stopping = self.is_stopping();
        if capturing || stopping {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        self.poll_stop_result();

        // Middengebied: Left Panel (Config + Buttons) & Right Panel (Position canvas)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(C1)
                    .inner_margin(egui::Margin::symmetric(1.5, 1.5)),
            )
            .show(ctx, |ui| {
                // Stel de horizontale ruimte tussen de kaarten in (bijv. 8.0 px):
                ui.spacing_mut().item_spacing.x = 8.0;

                ui.horizontal(|ui| {
                    // Links: Config + Geclusterde knoppen
                    self.render_config_card(ui, stopping);

                    // Rechts: Position Canvas (vult alle overgebleven ruimte op!)
                    self.render_pos_card(ui);
                });
            });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.shutdown();
    }
}

impl App {
    /// Linker Paneel: Wrapper voor Configuraties & Geclusterde Knoppen
    fn render_config_card(&mut self, ui: &mut egui::Ui, stopping: bool) {
        panel_frame().show(ui, |ui| {
            ui.set_width(280.0);
            ui.set_height(ui.available_height());

            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("configuration")
                        .color(T0)
                        .size(15.0)
                        .monospace()
                        .strong(),
                );
                ui.add_space(8.0);

                // 1. De configuratie velden (Grid)
                self.render_config(ui);

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // 2. De actieknoppen
                self.render_controls(ui, stopping);
            });
        });
    }

    /// Configuratie Grid (Identifier, Width, Height, FPS, Scale, Status)
    fn render_config(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("config_grid")
            .num_columns(2)
            .spacing([12.0, 10.0])
            .show(ui, |ui| {
                // Identifier
                ui.label(egui::RichText::new("identifier :").color(T1).monospace());
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!self.monitor_exists, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.name).desired_width(110.0),
                        );
                    });
                    if self.monitor_exists {
                        ui.label(egui::RichText::new("✓").color(T0).monospace());
                    }
                });
                ui.end_row();

                // Width
                ui.label(egui::RichText::new("width :").color(T1).monospace());
                ui.add(
                    egui::DragValue::new(&mut self.config.width)
                        .range(320..=7680)
                        .suffix(" px"),
                );
                ui.end_row();

                // Height
                ui.label(egui::RichText::new("height :").color(T1).monospace());
                ui.add(
                    egui::DragValue::new(&mut self.config.height)
                        .range(320..=7680)
                        .suffix(" px"),
                );
                ui.end_row();

                // Frame Rate
                ui.label(egui::RichText::new("frame rate :").color(T1).monospace());
                ui.add(
                    egui::DragValue::new(&mut self.config.fps)
                        .range(1..=240)
                        .suffix(" Hz"),
                );
                ui.end_row();

                // Scale
                ui.label(egui::RichText::new("scale :").color(T1).monospace());
                ui.add(
                    egui::DragValue::new(&mut self.config.scale)
                        .range(0.5f32..=3.0f32)
                        .speed(0.1),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let capturing = self.is_capturing();
                    let (status_text, color) = if capturing {
                        ("● STREAMING", A1)
                    } else if self.is_stopping() {
                        ("⏳ STOPPING", DANGER_RED)
                    } else if self.monitor_exists {
                        ("● ONLINE", T1)
                    } else {
                        ("○ OFFLINE", T2)
                    };

                    ui.label(
                        egui::RichText::new(status_text)
                            .color(color)
                            .size(12.0)
                            .monospace()
                            .strong(),
                    );
                });
                ui.end_row();
            });
    }

    /// Actieknoppen (Create/Update, Remove, Start, Stop)
    fn render_controls(&mut self, ui: &mut egui::Ui, stopping: bool) {
        // Bugfix: can_apply checkt nu alleen of de naam niet leeg is en we niet aan het capturen zijn.
        let can_apply = !self.config.name.is_empty() && !self.is_capturing() && !stopping;
        let can_remove = self.monitor_exists && !self.is_capturing() && !stopping;
        let can_start = self.monitor_exists && !self.is_capturing() && !stopping;
        let can_stop = self.is_capturing() && !stopping;

        let btn_width = (ui.available_width() - 8.0) / 2.0;

        ui.horizontal(|ui| {
            // Button 1: Create / Update
            let button_text = if self.monitor_exists {
                "Update"
            } else {
                "Create"
            };
            if custom_button(ui, button_text, can_apply, A1, ACCENT_HOVER, btn_width).clicked() {
                self.apply_config();
            }

            // Button 2: Remove
            if custom_button(
                ui,
                "Remove",
                can_remove,
                DANGER_RED,
                DANGER_HOVER,
                btn_width,
            )
            .clicked()
            {
                self.do_remove();
            }
        });

        ui.add_space(3.0);

        ui.horizontal(|ui| {
            // Button 3: Start
            let start_color = egui::Color32::from_rgb(129, 140, 248);
            let start_hover = egui::Color32::from_rgb(165, 180, 252);
            if custom_button(ui, "Start", can_start, start_color, start_hover, btn_width).clicked()
            {
                self.do_start_capture();
            }

            // Button 4: Stop
            if custom_button(ui, "Stop", can_stop, DANGER_RED, DANGER_HOVER, btn_width).clicked() {
                self.do_stop_capture();
            }
        });
    }

    /// Rechter Paneel: Position Canvas (vult volledige ruimte op)
    fn render_pos_card(&mut self, ui: &mut egui::Ui) {
        panel_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(ui.available_height());

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "position (x: {}  y: {})",
                            self.config.x, self.config.y
                        ))
                        .color(T0)
                        .monospace()
                        .size(14.0),
                    );
                });
                ui.add_space(3.0);

                // Dynamic Inner Canvas
                let canvas_size = ui.available_size();
                let (response, painter) =
                    ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());
                let canvas_rect = response.rect;

                let scale = 0.065;
                let main_w = 1920.0;
                let main_h = 1080.0;
                let main_x = 0.0;
                let main_y = 0.0;

                let virt_w = self.config.width as f32;
                let virt_h = self.config.height as f32;

                // Dragging Logic
                if response.drag_started() {
                    ui.memory_mut(|m| {
                        m.data
                            .insert_temp(egui::Id::new("virt_raw_x"), self.config.x as f32);
                        m.data
                            .insert_temp(egui::Id::new("virt_raw_y"), self.config.y as f32);
                    });
                }

                if response.dragged() {
                    let delta = response.drag_delta();
                    let raw_x = ui.memory_mut(|m| {
                        let val = m
                            .data
                            .get_temp_mut_or(egui::Id::new("virt_raw_x"), self.config.x as f32);
                        *val += delta.x / scale;
                        *val
                    });

                    let raw_y = ui.memory_mut(|m| {
                        let val = m
                            .data
                            .get_temp_mut_or(egui::Id::new("virt_raw_y"), self.config.y as f32);
                        *val += delta.y / scale;
                        *val
                    });

                    let mut final_x = raw_x;
                    let mut final_y = raw_y;

                    // Snapping tijdens het slepen (plakt vast aan de randen)
                    let snap_threshold = 250.0; // virtuele pixels
                    if (raw_x + virt_w).abs() < snap_threshold {
                        final_x = -virt_w;
                    } else if (raw_x - main_w).abs() < snap_threshold {
                        final_x = main_w;
                    }

                    if (raw_y + virt_h).abs() < snap_threshold {
                        final_y = -virt_h;
                    } else if (raw_y - main_h).abs() < snap_threshold {
                        final_y = main_h;
                    }

                    self.config.x = final_x as i32;
                    self.config.y = final_y as i32;
                }

                if response.drag_stopped() {
                    let grid_step = 90;
                    let snap_threshold = 350.0; // Iets ruimer bij het loslaten

                    let mut raw_x = self.config.x as f32;
                    let mut raw_y = self.config.y as f32;
                    let mut snapped_x = false;
                    let mut snapped_y = false;

                    // 1. Check of we tegen het main scherm aan liggen
                    if (raw_x + virt_w).abs() < snap_threshold {
                        raw_x = -virt_w;
                        snapped_x = true;
                    } else if (raw_x - main_w).abs() < snap_threshold {
                        raw_x = main_w;
                        snapped_x = true;
                    }

                    if (raw_y + virt_h).abs() < snap_threshold {
                        raw_y = -virt_h;
                        snapped_y = true;
                    } else if (raw_y - main_h).abs() < snap_threshold {
                        raw_y = main_h;
                        snapped_y = true;
                    }

                    // 2. Voor de assen die NIET tegen het scherm aanliggen, gebruik het grid
                    if !snapped_x {
                        raw_x = (raw_x / grid_step as f32).round() * grid_step as f32;
                    }
                    if !snapped_y {
                        raw_y = (raw_y / grid_step as f32).round() * grid_step as f32;
                    }

                    self.config.x = raw_x as i32;
                    self.config.y = raw_y as i32;

                    ui.memory_mut(|m| {
                        m.data
                            .insert_temp(egui::Id::new("virt_raw_x"), self.config.x as f32);
                        m.data
                            .insert_temp(egui::Id::new("virt_raw_y"), self.config.y as f32);
                    });
                }

                // Teken logica
                painter.rect_filled(canvas_rect, 2.0, C3);
                painter.rect_stroke(canvas_rect, 2.0, egui::Stroke::new(0.0, C4));

                let center_x = canvas_rect.center().x;
                let center_y = canvas_rect.center().y;

                let origin_x = center_x - (main_w / 2.0) * scale;
                let origin_y = center_y - (main_h / 2.0) * scale;

                let to_canvas = |sx: f32, sy: f32| -> egui::Pos2 {
                    egui::pos2(origin_x + sx * scale, origin_y + sy * scale)
                };

                // Grid
                let grid_step_canvas = 90.0 * scale;
                let mut grid_x = origin_x;
                while grid_x > canvas_rect.min.x {
                    grid_x -= grid_step_canvas;
                }
                while grid_x < canvas_rect.max.x {
                    painter.line_segment(
                        [
                            egui::pos2(grid_x, canvas_rect.min.y),
                            egui::pos2(grid_x, canvas_rect.max.y),
                        ],
                        egui::Stroke::new(0.5, egui::Color32::from_rgb(45, 47, 51)),
                    );
                    grid_x += grid_step_canvas;
                }

                let mut grid_y = origin_y;
                while grid_y > canvas_rect.min.y + 2.0 {
                    grid_y -= grid_step_canvas;
                }
                while grid_y < canvas_rect.max.y + 1.0 {
                    painter.line_segment(
                        [
                            egui::pos2(canvas_rect.min.x, grid_y),
                            egui::pos2(canvas_rect.max.x, grid_y),
                        ],
                        egui::Stroke::new(0.5, egui::Color32::from_rgb(45, 47, 51)),
                    );
                    grid_y += grid_step_canvas;
                }

                // Main Monitor
                let main_top_left = to_canvas(main_x, main_y);
                let main_rect = egui::Rect::from_min_size(
                    main_top_left,
                    egui::vec2(main_w * scale, main_h * scale),
                );

                painter.rect_filled(main_rect, 2.0, egui::Color32::from_rgb(40, 40, 38));
                painter.rect_stroke(main_rect, 2.0, egui::Stroke::new(3.0, C4));
                painter.text(
                    main_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "main\n(DP-1)",
                    egui::FontId::monospace(10.0),
                    T1,
                );

                // Virtual Monitor
                let virt_top_left = to_canvas(self.config.x as f32, self.config.y as f32);

                let virt_rect = egui::Rect::from_min_size(
                    virt_top_left,
                    egui::vec2(virt_w * scale, virt_h * scale),
                );

                let (fill_col, stroke_col) = if self.monitor_exists {
                    (egui::Color32::from_rgb(142, 122, 31), A1)
                } else {
                    (A2, A1)
                };

                let is_grabbed = response.dragged();
                let actual_stroke = if is_grabbed {
                    egui::Stroke::new(3.0, ACCENT_HOVER)
                } else {
                    egui::Stroke::new(3.0, stroke_col)
                };

                painter.rect_filled(virt_rect, 2.0, fill_col);
                painter.rect_stroke(virt_rect, 2.0, actual_stroke);
                painter.text(
                    virt_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &format!(
                        "{}\n{}x{}",
                        self.config.name, self.config.width, self.config.height
                    ),
                    egui::FontId::monospace(10.0),
                    A1,
                );
            });
        });
    }
}

// ─── Styling Helpers ───────────────────────────────────────────

fn configure_style(ctx: &egui::Context, scale: f32) {
    // Stel de schaalfactor globally in (1.25 = 125% van normale grootte)
    // let scale = 1.25;
    ctx.set_pixels_per_point(scale);

    let mut style = (*ctx.style()).clone();

    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);

    let rounding = egui::Rounding::same(2.0);
    style.visuals.window_rounding = rounding;
    style.visuals.widgets.noninteractive.rounding = rounding;
    style.visuals.widgets.inactive.rounding = rounding;
    style.visuals.widgets.hovered.rounding = rounding;
    style.visuals.widgets.active.rounding = rounding;

    style.visuals.dark_mode = true;
    style.visuals.code_bg_color = C3;
    style.visuals.override_text_color = Some(T1);

    style.visuals.widgets.inactive.bg_fill = C3;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, T1);
    style.visuals.widgets.hovered.bg_fill = C3;
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, T0);
    style.visuals.widgets.active.bg_fill = C3;
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, T0);

    ctx.set_style(style);
}

fn panel_frame() -> egui::Frame {
    egui::Frame {
        fill: C2,
        inner_margin: egui::Margin::same(3.0),
        stroke: egui::Stroke::new(1.0, C4),
        ..Default::default()
    }
}

fn inner_frame() -> egui::Frame {
    egui::Frame {
        fill: C3,
        inner_margin: egui::Margin::same(8.0),
        stroke: egui::Stroke::new(1.0, C4),
        ..Default::default()
    }
}

fn set_button_style(ui: &mut egui::Ui, bg: egui::Color32, hover: egui::Color32, fg: egui::Color32) {
    let style = ui.style_mut();
    style.visuals.widgets.inactive.bg_fill = bg;
    style.visuals.widgets.hovered.bg_fill = bg;
    style.visuals.widgets.active.bg_fill = bg;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, fg);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, fg);
}

fn ghost_button(text: impl Into<String>, color: egui::Color32) -> impl egui::Widget {
    egui::Button::new(
        egui::RichText::new(text.into())
            .color(color)
            .monospace()
            .size(12.0),
    )
    .fill(egui::Color32::TRANSPARENT)
    .stroke(egui::Stroke::NONE)
}

use egui::{Align2, Color32, FontId, Response, Sense, Stroke, Ui, Vec2};

/// Tekent een aangepaste knop met harde disable-beveiliging.
pub fn custom_button(
    ui: &mut Ui,
    text: &str,
    enabled: bool,
    accent_color: Color32,
    hover_color: Color32,
    width: f32,
) -> Response {
    let desired_size = Vec2::new(width, 32.0);

    // HARD SECURITY: Als 'enabled' false is, luisteren we NIET naar kliks.
    // Dit voorkomt dat de knop reageert als hij grijs getekend staat.
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };

    let (rect, response) = ui.allocate_exact_size(desired_size, sense);

    // Render-berekeningen alleen uitvoeren als de widget zichtbaar is op het scherm
    if ui.is_rect_visible(rect) {
        // Bepaal visuele staat
        let (border_col, text_col) = if !enabled {
            (Color32::from_rgb(48, 50, 56), T2)
        } else if response.hovered() {
            (hover_color, T0)
        } else {
            (accent_color, accent_color)
        };

        // Teken de achtergrond en rand
        ui.painter().rect_filled(rect, 2.0, C1);
        ui.painter()
            .rect_stroke(rect, 2.0, Stroke::new(1.0, border_col));

        // Teken de tekst in het midden
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::monospace(13.0),
            text_col,
        );
    }

    response
}
