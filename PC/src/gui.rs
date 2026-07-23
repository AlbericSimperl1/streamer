use crate::app::App;
use eframe::egui;
use egui::accesskit::TextAlign::Center;
use std::time::Duration;

// ─── Kleurenpalet ──────────────────────────────────────────────
const BG_MAIN: egui::Color32 = egui::Color32::from_rgb(18, 18, 24);
const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(26, 26, 36);
const BG_INNER: egui::Color32 = egui::Color32::from_rgb(14, 14, 20);
const BORDER_COLOR: egui::Color32 = egui::Color32::from_rgb(40, 40, 56);

const ACCENT_LIME: egui::Color32 = egui::Color32::from_rgb(255, 246, 224);
const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(99, 102, 241);
const DANGER_RED: egui::Color32 = egui::Color32::from_rgb(239, 68, 68);
const DANGER_HOVER: egui::Color32 = egui::Color32::from_rgb(248, 113, 113);

const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(241, 245, 249);
const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(148, 163, 184);

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        configure_style(ctx);

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

        // 1. Header Boven
        egui::TopBottomPanel::top("header_panel")
            .frame(egui::Frame::none().fill(BG_MAIN).inner_margin(1.0))
            .show(ctx, |ui| {
                self.render_header(ui);
            });

        // 2. Middengebied: Left Panel (Config + Buttons) & Right Panel (Position canvas)
        // egui::CentralPanel::default()
        //     .frame(
        //         egui::Frame::none()
        //             .fill(BG_MAIN)
        //             .inner_margin(egui::Margin::symmetric(1.5, 1.5)),
        //     )
        //     .show(ctx, |ui| {
        //         ui.horizontal(|ui| {
        //             // Links: Config + Geclusterde knoppen
        //             self.render_config_card(ui, stopping);

        //             ui.add_space(1.0);

        //             // Rechts: Position Canvas (vult alle overgebleven ruimte op!)
        //             self.render_pos_card(ui);
        //         });
        //     });
        //
        // 2. Middengebied: Left Panel (Config + Buttons) & Right Panel (Position canvas)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(BG_MAIN)
                    .inner_margin(egui::Margin::symmetric(1.5, 1.5)),
            )
            .show(ctx, |ui| {
                // Stel de horizontale ruimte tussen de kaarten in (bijv. 2.0 px):
                ui.spacing_mut().item_spacing.x = 2.0;

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
    /// Header
    fn render_header(&mut self, ui: &mut egui::Ui) {
        panel_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, ACCENT_LIME);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "♬",
                    egui::FontId::proportional(14.0),
                    BG_MAIN,
                );

                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("naam app")
                        .color(TEXT_PRIMARY)
                        .size(15.0)
                        .monospace()
                        .strong(),
                );

                ui.label(
                    egui::RichText::new("// hyprland display streamer")
                        .color(TEXT_MUTED)
                        .size(12.0)
                        .monospace(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let capturing = self.is_capturing();
                    let (status_text, color) = if capturing {
                        ("● STREAMING", ACCENT_LIME)
                    } else if self.is_stopping() {
                        ("⏳ STOPPING", ACCENT_BLUE)
                    } else if self.monitor_exists {
                        ("● ONLINE", ACCENT_BLUE)
                    } else {
                        ("○ OFFLINE", TEXT_MUTED)
                    };

                    ui.label(
                        egui::RichText::new(status_text)
                            .color(color)
                            .size(12.0)
                            .monospace()
                            .strong(),
                    );

                    ui.add_space(10.0);
                    if ui.add(ghost_button("🔄 refresh", ACCENT_LIME)).clicked() {
                        self.refresh();
                    }
                });
            });
        });
    }

    /// Linker Paneel: Configuraties & Geclusterde Knoppen
    fn render_config_card(&mut self, ui: &mut egui::Ui, stopping: bool) {
        panel_frame().show(ui, |ui| {
            ui.set_width(280.0);
            ui.set_height(ui.available_height());

            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("configuration")
                        .color(ACCENT_LIME)
                        .size(15.0)
                        .monospace()
                        .strong(),
                );
                ui.add_space(8.0);

                // Config velden
                egui::Grid::new("config_grid")
                    .num_columns(2)
                    .spacing([12.0, 10.0])
                    .show(ui, |ui| {
                        // Identifier
                        ui.label(
                            egui::RichText::new("identifier :")
                                .color(TEXT_MUTED)
                                .monospace(),
                        );
                        ui.horizontal(|ui| {
                            ui.add_enabled_ui(!self.monitor_exists, |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.config.name)
                                        .desired_width(110.0),
                                );
                            });
                            if self.monitor_exists {
                                ui.label(egui::RichText::new("✓").color(ACCENT_LIME).monospace());
                            }
                        });
                        ui.end_row();

                        // Width
                        ui.label(egui::RichText::new("width :").color(TEXT_MUTED).monospace());
                        ui.add(
                            egui::DragValue::new(&mut self.config.width)
                                .range(320..=7680)
                                .suffix(" px"),
                        );
                        ui.end_row();

                        // Height
                        ui.label(
                            egui::RichText::new("height :")
                                .color(TEXT_MUTED)
                                .monospace(),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.config.height)
                                .range(320..=7680)
                                .suffix(" px"),
                        );
                        ui.end_row();

                        // Frame Rate
                        ui.label(
                            egui::RichText::new("frame rate :")
                                .color(TEXT_MUTED)
                                .monospace(),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.config.fps)
                                .range(1..=240)
                                .suffix(" Hz"),
                        );
                        ui.end_row();

                        // Scale
                        ui.label(egui::RichText::new("scale :").color(TEXT_MUTED).monospace());
                        ui.add(
                            egui::DragValue::new(&mut self.config.scale)
                                .range(0.5f32..=3.0f32)
                                .speed(0.1),
                        );
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // ─── Geclusterde Actieknoppen (2x2 Grid) ───
                let can_create = !self.monitor_exists && !self.config.name.is_empty();
                let can_remove = self.monitor_exists && !self.is_capturing();
                let can_start = self.monitor_exists && !self.is_capturing() && !stopping;
                let can_stop = self.is_capturing() && !stopping;

                let btn_width = (ui.available_width() - 8.0) / 2.0;

                ui.horizontal(|ui| {
                    // Button 1: Create / Update
                    set_button_style(ui, ACCENT_LIME, ACCENT_HOVER, BG_MAIN);
                    let button_text = if self.monitor_exists {
                        "Update"
                    } else {
                        "Create"
                    };
                    if ui
                        .add_enabled(
                            can_create,
                            egui::Button::new(button_text).min_size(egui::vec2(btn_width, 32.0)),
                        )
                        .clicked()
                    {
                        self.apply_config();
                    }

                    // Button 2: Remove
                    set_button_style(ui, DANGER_RED, DANGER_HOVER, TEXT_PRIMARY);
                    if ui
                        .add_enabled(
                            can_remove,
                            egui::Button::new("Remove").min_size(egui::vec2(btn_width, 32.0)),
                        )
                        .clicked()
                    {
                        self.do_remove();
                    }
                });

                ui.add_space(3.0);

                ui.horizontal(|ui| {
                    // Button 3: Start
                    set_button_style(
                        ui,
                        ACCENT_BLUE,
                        egui::Color32::from_rgb(129, 140, 248),
                        TEXT_PRIMARY,
                    );
                    if ui
                        .add_enabled(
                            can_start,
                            egui::Button::new("Start").min_size(egui::vec2(btn_width, 32.0)),
                        )
                        .clicked()
                    {
                        self.do_start_capture();
                    }

                    // Button 4: Stop
                    set_button_style(ui, DANGER_RED, DANGER_HOVER, TEXT_PRIMARY);
                    if ui
                        .add_enabled(
                            can_stop,
                            egui::Button::new("Stop").min_size(egui::vec2(btn_width, 32.0)),
                        )
                        .clicked()
                    {
                        self.do_stop_capture();
                    }
                });
            });
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
                        .color(ACCENT_LIME)
                        .monospace()
                        .size(14.0),
                    );
                });
                ui.add_space(3.0);

                // Dynamic Inner Canvas
                inner_frame().show(ui, |ui| {
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

                        self.config.x = raw_x as i32;
                        self.config.y = raw_y as i32;
                    }

                    if response.drag_stopped() {
                        let grid_step = 90;
                        self.config.x = ((self.config.x as f32 / grid_step as f32).round()
                            * grid_step as f32) as i32;
                        self.config.y = ((self.config.y as f32 / grid_step as f32).round()
                            * grid_step as f32) as i32;

                        ui.memory_mut(|m| {
                            m.data
                                .insert_temp(egui::Id::new("virt_raw_x"), self.config.x as f32);
                            m.data
                                .insert_temp(egui::Id::new("virt_raw_y"), self.config.y as f32);
                        });
                    }

                    // Teken logica
                    painter.rect_filled(canvas_rect, 2.0, BG_INNER);
                    painter.rect_stroke(canvas_rect, 2.0, egui::Stroke::new(1.0, BORDER_COLOR));

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
                            egui::Stroke::new(0.5, egui::Color32::from_rgb(25, 25, 35)),
                        );
                        grid_x += grid_step_canvas;
                    }

                    let mut grid_y = origin_y;
                    while grid_y > canvas_rect.min.y {
                        grid_y -= grid_step_canvas;
                    }
                    while grid_y < canvas_rect.max.y {
                        painter.line_segment(
                            [
                                egui::pos2(canvas_rect.min.x, grid_y),
                                egui::pos2(canvas_rect.max.x, grid_y),
                            ],
                            egui::Stroke::new(0.5, egui::Color32::from_rgb(25, 25, 35)),
                        );
                        grid_y += grid_step_canvas;
                    }

                    // Main Monitor
                    let main_top_left = to_canvas(main_x, main_y);
                    let main_rect = egui::Rect::from_min_size(
                        main_top_left,
                        egui::vec2(main_w * scale, main_h * scale),
                    );

                    painter.rect_filled(main_rect, 2.0, egui::Color32::from_rgb(32, 38, 54));
                    painter.rect_stroke(main_rect, 2.0, egui::Stroke::new(1.2, ACCENT_BLUE));
                    painter.text(
                        main_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "main\n(DP-1)",
                        egui::FontId::monospace(10.0),
                        TEXT_PRIMARY,
                    );

                    // Virtual Monitor
                    let virt_top_left = to_canvas(self.config.x as f32, self.config.y as f32);

                    let virt_rect = egui::Rect::from_min_size(
                        virt_top_left,
                        egui::vec2(virt_w * scale, virt_h * scale),
                    );

                    let (fill_col, stroke_col) = if self.monitor_exists {
                        (
                            egui::Color32::from_rgba_premultiplied(132, 204, 22, 40),
                            ACCENT_LIME,
                        )
                    } else {
                        (
                            egui::Color32::from_rgba_premultiplied(148, 163, 184, 20),
                            TEXT_MUTED,
                        )
                    };

                    let is_grabbed = response.dragged();
                    let actual_stroke = if is_grabbed {
                        egui::Stroke::new(2.0, ACCENT_HOVER)
                    } else {
                        egui::Stroke::new(1.2, stroke_col)
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
                        if self.monitor_exists {
                            ACCENT_LIME
                        } else {
                            TEXT_MUTED
                        },
                    );

                    // Instructielabel
                    painter.text(
                        egui::pos2(canvas_rect.min.x + 8.0, canvas_rect.max.y - 8.0),
                        egui::Align2::LEFT_BOTTOM,
                        "↔ drag to position (snaps to 90px grid)",
                        egui::FontId::monospace(10.0),
                        TEXT_MUTED,
                    );
                });
            });
        });
    }
}

// ─── Styling Helpers ───────────────────────────────────────────

fn configure_style(ctx: &egui::Context) {
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
    style.visuals.code_bg_color = BG_INNER;
    style.visuals.override_text_color = Some(TEXT_PRIMARY);

    style.visuals.widgets.inactive.bg_fill = BG_INNER;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
    style.visuals.widgets.hovered.bg_fill = BG_INNER;
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ACCENT_LIME);
    style.visuals.widgets.active.bg_fill = BG_INNER;
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, ACCENT_LIME);

    ctx.set_style(style);
}

fn panel_frame() -> egui::Frame {
    egui::Frame {
        fill: BG_PANEL,
        inner_margin: egui::Margin::same(3.0),
        stroke: egui::Stroke::new(1.0, BORDER_COLOR),
        ..Default::default()
    }
}

fn inner_frame() -> egui::Frame {
    egui::Frame {
        fill: BG_INNER,
        inner_margin: egui::Margin::same(8.0),
        stroke: egui::Stroke::new(1.0, BORDER_COLOR),
        ..Default::default()
    }
}

fn set_button_style(ui: &mut egui::Ui, bg: egui::Color32, hover: egui::Color32, fg: egui::Color32) {
    let style = ui.style_mut();
    style.visuals.widgets.inactive.bg_fill = bg;
    style.visuals.widgets.hovered.bg_fill = hover;
    style.visuals.widgets.active.bg_fill = hover;
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
