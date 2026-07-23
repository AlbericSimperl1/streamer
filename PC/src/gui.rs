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
const C2_HOVER: egui::Color32 = egui::Color32::from_rgb(48, 51, 57); // Net iets lichter dan C2
const C2_CLICKED: egui::Color32 = egui::Color32::from_rgb(68, 72, 80); // Net iets lichter dan C2_HOVER

/// text
const T0: egui::Color32 = egui::Color32::from_rgb(255, 255, 255); // "titels"
const T1: egui::Color32 = egui::Color32::from_rgb(255, 246, 226); // primary
const T2: egui::Color32 = egui::Color32::from_rgb(146, 138, 132); // inactive

/// accent rgb(20, 170, 118)
const A1: egui::Color32 = egui::Color32::from_rgb(20, 170, 118); // primary
const A2: egui::Color32 = egui::Color32::from_rgb(28, 57, 47); // secondary

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
                    // Links: Verticale stapeling van Config kaart en Controls kaart
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        self.render_config_card(ui);
                        self.render_controls_card(ui, stopping);
                    });

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
    /// Linker Paneel Boven: Configuratie Kaart
    fn render_config_card(&mut self, ui: &mut egui::Ui) {
        panel_frame().show(ui, |ui| {
            ui.set_width(280.0);
            // Geen set_height, zodat de kaart zich aanpast aan de inhoud

            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("configuration")
                        .color(T0)
                        .size(15.0)
                        .monospace()
                        .strong(),
                );
                ui.add_space(6.0);

                // De configuratie velden (Grid)
                self.render_config(ui);
            });
        });
    }

    /// Linker Paneel Onder: Actieknoppen Kaart
    fn render_controls_card(&mut self, ui: &mut egui::Ui, stopping: bool) {
        panel_frame().show(ui, |ui| {
            ui.set_width(280.0);

            ui.vertical(|ui| {
                // De actieknoppen
                self.render_controls(ui, stopping);
            });
        });
    }

    /// Configuratie Grid (Identifier, Width, Height, FPS, Scale, Status)
    fn render_config(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("config_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                // ─── 1. Identifier (Achtergrond C2 met dunne onderlijn) ───
                ui.label(egui::RichText::new("identifier :").color(T1).monospace());
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!self.monitor_exists, |ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.config.name)
                                .desired_width(110.0)
                                .frame(false),
                        );

                        let line_y = response.rect.bottom();
                        ui.painter().line_segment(
                            [
                                egui::pos2(response.rect.min.x, line_y),
                                egui::pos2(response.rect.max.x, line_y),
                            ],
                            egui::Stroke::new(1.0, C4),
                        );
                    });
                    if self.monitor_exists {
                        ui.label(egui::RichText::new("✓").color(T0).monospace());
                    }
                });
                ui.end_row();

                // ─── 2. Width (Discreet per 30px, max 3840) ───
                ui.label(egui::RichText::new("width :").color(T1).monospace());
                ui.horizontal(|ui| {
                    custom_drag_bar(ui, &mut self.config.width, 300..=3840, Some(30.0), 100.0);
                    ui.label(
                        egui::RichText::new(format!("{} px", self.config.width))
                            .color(T1)
                            .monospace()
                            .size(12.0),
                    );
                });
                ui.end_row();

                // ─── 3. Height (Discreet per 30px, max 3840) ───
                ui.label(egui::RichText::new("height :").color(T1).monospace());
                ui.horizontal(|ui| {
                    custom_drag_bar(ui, &mut self.config.height, 300..=3840, Some(30.0), 100.0);
                    ui.label(
                        egui::RichText::new(format!("{} px", self.config.height))
                            .color(T1)
                            .monospace()
                            .size(12.0),
                    );
                });
                ui.end_row();

                // ─── 4. Frame Rate (Discreet per integer, max 90 Hz) ───
                ui.label(egui::RichText::new("frame rate :").color(T1).monospace());
                ui.horizontal(|ui| {
                    custom_drag_bar(ui, &mut self.config.fps, 1..=90, Some(1.0), 100.0);
                    ui.label(
                        egui::RichText::new(format!("{} Hz", self.config.fps))
                            .color(T1)
                            .monospace()
                            .size(12.0),
                    );
                });
                ui.end_row();

                // ─── 5. Scale (Discreet per 0.05, max 4.0) ───
                ui.label(egui::RichText::new("scale :").color(T1).monospace());
                ui.horizontal(|ui| {
                    custom_drag_bar(
                        ui,
                        &mut self.config.scale,
                        0.5f32..=4.0f32,
                        Some(0.05),
                        100.0,
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.2}", self.config.scale))
                            .color(T1)
                            .monospace()
                            .size(12.0),
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

        let btn_width = (272.0 - 4.0) / 2.0;

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
                    (egui::Color32::from_rgb(22, 81, 60), A1)
                } else {
                    (A2, A1)
                };
                // rgb(22, 81, 60)

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
    // 1. Schaal instellen
    ctx.set_pixels_per_point(scale);

    // 2. Globaal Font Laden en Configureren
    let mut fonts = egui::FontDefinitions::default();

    // Laad het .ttf of .otf bestand (pas het pad aan naar jouw font-bestand)
    fonts.font_data.insert(
        "custom_font".to_owned(),
        egui::FontData::from_static(include_bytes!("/usr/share/fonts/mononoki-Regular.ttf")),
    );

    // Zet het font als hoogste prioriteit voor Proportional (normale tekst)
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "custom_font".to_owned());

    // Zet het font OOK als hoogste prioriteit voor Monospace (zodat .monospace() in je UI hetzelfde font gebruikt)
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "custom_font".to_owned());

    // Pas de fontdefinities toe op de context
    ctx.set_fonts(fonts);

    // 3. Overige Styling & Visuals
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
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };

    let (rect, response) = ui.allocate_exact_size(desired_size, sense);

    if ui.is_rect_visible(rect) {
        // Bepaal de achtergrond-, rand- en tekstkleur op basis van de staat
        let (bg_col, border_col, text_col) = if !enabled {
            (C2, C4, T2)
        } else if response.is_pointer_button_down_on() {
            // 1. Ingedrukt / Clicked staat (Lichtst)
            (C2_CLICKED, C4, T0)
        } else if response.hovered() {
            // 2. Muis zweeft erboven / Hover staat (Middel)
            (C2_HOVER, C4, T0)
        } else {
            // 3. Ruststand / Normal staat
            (C2, C4, T1)
        };

        // Teken achtergrond en rand
        ui.painter().rect_filled(rect, 2.0, bg_col);
        ui.painter()
            .rect_stroke(rect, 2.0, Stroke::new(0.5, border_col));

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

use egui::{emath, pos2, vec2, Rect};

/// Tekent een minimalistische custom drag bar / slider in OmaTunes-stijl.
pub fn custom_drag_bar<T: emath::Numeric>(
    ui: &mut Ui,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
    step: Option<f64>,
    width: f32,
) -> Response {
    let height = 22.0; // Totale interactieve hoogte
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click_and_drag());

    let min = range.start().to_f64();
    let max = range.end().to_f64();

    // ─── 1. Muis & Drag Logica ─────────────────────────────────────
    if response.dragged() || response.clicked() {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let normalized = ((pointer_pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
            let raw_val = min + (normalized as f64) * (max - min);

            let new_val = if let Some(s) = step {
                ((raw_val - min) / s).round() * s + min
            } else {
                raw_val
            };

            *value = T::from_f64(new_val.clamp(min, max));
        }
    }

    // ─── 2. Rendering met Painter ──────────────────────────────────
    if ui.is_rect_visible(rect) {
        let current_val = value.to_f64();
        let normalized = ((current_val - min) / (max - min)).clamp(0.0, 1.0) as f32;
        let center_y = rect.center().y;

        // Background Track (donker spoor met dunne rand)
        let track_h = 4.0;
        let track_rect = Rect::from_min_max(
            pos2(rect.min.x, center_y - track_h / 2.0),
            pos2(rect.max.x, center_y + track_h / 2.0),
        );
        ui.painter().rect_filled(track_rect, 2.0, C3);
        ui.painter()
            .rect_stroke(track_rect, 2.0, Stroke::new(0.5, C4));

        // Active Progress Fill (Accent-kleur voortgang)
        let fill_w = (rect.width() * normalized).max(0.0);
        let filled_rect = Rect::from_min_max(
            pos2(rect.min.x, center_y - track_h / 2.0),
            pos2(rect.min.x + fill_w, center_y + track_h / 2.0),
        );

        let fill_color = if response.dragged() {
            A1
        } else if response.hovered() {
            A1
        } else {
            A2
        };
        ui.painter().rect_filled(filled_rect, 2.0, fill_color);

        // Handle / Thumb (Dunne verticale pill die opschaalt en oplicht bij hover/drag)
        let handle_x = rect.min.x + fill_w;
        let handle_size = if response.hovered() || response.dragged() {
            vec2(6.0, 14.0)
        } else {
            vec2(4.0, 10.0)
        };
        let handle_rect = Rect::from_center_size(pos2(handle_x, center_y), handle_size);
        let handle_color = if response.hovered() || response.dragged() {
            T0
        } else {
            A1
        };
        ui.painter().rect_filled(handle_rect, 2.0, handle_color);
    }

    response
}
