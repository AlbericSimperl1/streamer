use crate::app::App;
use eframe::egui;
use std::time::Duration;

// ─── system24-inspired Palette ──────────────────────────────────
// Layered darks
const BG: egui::Color32 = egui::Color32::from_rgb(19, 25, 26);
const BG0: egui::Color32 = egui::Color32::from_rgb(24, 32, 34);
const BG1: egui::Color32 = egui::Color32::from_rgb(26, 34, 36);
const BG2: egui::Color32 = egui::Color32::from_rgb(30, 39, 41);
const BG3: egui::Color32 = egui::Color32::from_rgb(36, 45, 48);
const CNV: egui::Color32 = egui::Color32::from_rgb(14, 20, 21);

// Borders
const BRD: egui::Color32 = egui::Color32::from_rgb(38, 46, 48);
const BRD_H: egui::Color32 = egui::Color32::from_rgb(58, 66, 70);

// Text
const T0: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const T1: egui::Color32 = egui::Color32::from_rgb(219, 216, 210);
const T2: egui::Color32 = egui::Color32::from_rgb(130, 130, 139);
const T3: egui::Color32 = egui::Color32::from_rgb(90, 90, 96);

// Accents
const ACC: egui::Color32 = egui::Color32::from_rgb(94, 193, 255);
const ACC_D: egui::Color32 = egui::Color32::from_rgb(28, 55, 75);
const GRN: egui::Color32 = egui::Color32::from_rgb(70, 190, 100);
const YEL: egui::Color32 = egui::Color32::from_rgb(220, 180, 80);
const RED: egui::Color32 = egui::Color32::from_rgb(220, 90, 90);
const RED_D: egui::Color32 = egui::Color32::from_rgb(65, 28, 28);

// ─── App ────────────────────────────────────────────────────────

impl App {
    pub fn new() -> Self {
        Self::with_signal_flag_opt(None)
    }

    pub fn new_scaled(cc: &eframe::CreationContext, scale: f32) -> Self {
        configure_style(&cc.egui_ctx, scale);
        Self::new()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;

                    // ── Top status bar ──
                    self.render_top_bar(ui);

                    // ── Main content area ──
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.add_space(4.0);

                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            self.render_config_panel(ui);
                            self.render_controls_panel(ui, stopping);
                        });

                        self.render_canvas_panel(ui);

                        ui.add_space(4.0);
                    });

                    // ── Bottom status bar ──
                    self.render_bottom_bar(ui);
                    ui.add_space(4.0);
                });
            });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.shutdown();
    }
}

// ─── Panels ─────────────────────────────────────────────────────

impl App {
    // ── Top status bar ──
    fn render_top_bar(&self, ui: &mut egui::Ui) {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());

        ui.painter().rect_filled(rect, 0.0, BG1);
        ui.painter().line_segment(
            [egui::pos2(rect.min.x, rect.max.y), rect.max],
            egui::Stroke::new(0.5, BRD),
        );

        let inner = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + 8.0, rect.min.y),
            egui::pos2(rect.max.x - 8.0, rect.max.y),
        );

        ui.allocate_ui_at_rect(inner, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                // Status dot + app name
                let (dot, dot_col) = if self.is_capturing() {
                    ("●", ACC)
                } else if self.monitor_exists {
                    ("●", GRN)
                } else {
                    ("○", T3)
                };
                ui.label(
                    egui::RichText::new(dot)
                        .color(dot_col)
                        .monospace()
                        .size(10.0),
                );
                ui.label(
                    egui::RichText::new("virtual-display")
                        .color(T1)
                        .monospace()
                        .size(12.0),
                );

                ui.label(egui::RichText::new("│").color(BRD).monospace().size(11.0));

                // Monitor status
                if self.monitor_exists {
                    ui.label(
                        egui::RichText::new(format!("● {}", self.config.name))
                            .color(GRN)
                            .monospace()
                            .size(11.0),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("○ no monitor")
                            .color(T3)
                            .monospace()
                            .size(11.0),
                    );
                }

                ui.label(egui::RichText::new("│").color(BRD).monospace().size(11.0));

                // Config summary
                ui.label(
                    egui::RichText::new(format!(
                        "{}×{} @ {}fps",
                        self.config.width, self.config.height, self.config.fps
                    ))
                    .color(T2)
                    .monospace()
                    .size(11.0),
                );

                // Right-aligned capture status
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (cap_text, cap_col) = if self.is_capturing() {
                        ("● capturing", ACC)
                    } else if self.is_stopping() {
                        ("● stopping", YEL)
                    } else if self.monitor_exists {
                        ("● ready", GRN)
                    } else {
                        ("idle", T3)
                    };
                    ui.label(
                        egui::RichText::new(cap_text)
                            .color(cap_col)
                            .monospace()
                            .size(11.0),
                    );
                });
            });
        });
    }

    // ── Bottom status bar ──
    fn render_bottom_bar(&self, ui: &mut egui::Ui) {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), egui::Sense::hover());

        ui.painter().rect_filled(rect, 0.0, BG1);
        ui.painter().line_segment(
            [
                egui::pos2(rect.min.x, rect.min.y),
                egui::pos2(rect.max.x, rect.min.y),
            ],
            egui::Stroke::new(0.5, BRD),
        );

        let inner = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + 8.0, rect.min.y),
            egui::pos2(rect.max.x - 8.0, rect.max.y),
        );

        ui.allocate_ui_at_rect(inner, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                ui.label(
                    egui::RichText::new(format!("pos: ({}, {})", self.config.x, self.config.y))
                        .color(T2)
                        .monospace()
                        .size(11.0),
                );
                ui.label(egui::RichText::new("│").color(BRD).monospace().size(11.0));
                ui.label(
                    egui::RichText::new(format!("scale: {:.2}", self.config.scale))
                        .color(T2)
                        .monospace()
                        .size(11.0),
                );
                ui.label(egui::RichText::new("│").color(BRD).monospace().size(11.0));
                ui.label(
                    egui::RichText::new("grid: 90px")
                        .color(T2)
                        .monospace()
                        .size(11.0),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("drag canvas to reposition")
                            .color(T3)
                            .monospace()
                            .size(10.0),
                    );
                });
            });
        });
    }

    // ── Config panel ──
    fn render_config_panel(&mut self, ui: &mut egui::Ui) {
        panel_frame().show(ui, |ui| {
            ui.set_width(280.0);
            ui.vertical(|ui| {
                section_header(ui, "config");
                self.render_config(ui);
            });
        });
    }

    // ── Controls panel ──
    fn render_controls_panel(&mut self, ui: &mut egui::Ui, stopping: bool) {
        panel_frame().show(ui, |ui| {
            ui.set_width(280.0);
            ui.vertical(|ui| {
                section_header(ui, "controls");
                self.render_controls(ui, stopping);
            });
        });
    }

    // ── Config items (tree-style) ──
    fn render_config(&mut self, ui: &mut egui::Ui) {
        // ── identifier ──
        config_label(ui, "identifier");
        ui.horizontal(|ui| {
            tree_indent(ui);
            ui.add_enabled_ui(!self.monitor_exists, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.config.name)
                        .desired_width(130.0)
                        .frame(false)
                        .text_color(T1)
                        .hint_text(
                            egui::RichText::new("name...")
                                .color(T3)
                                .monospace()
                                .size(11.0),
                        ),
                );
                let line_y = response.rect.bottom();
                ui.painter().line_segment(
                    [
                        egui::pos2(response.rect.min.x, line_y),
                        egui::pos2(response.rect.max.x, line_y),
                    ],
                    egui::Stroke::new(0.5, if self.monitor_exists { GRN } else { BRD }),
                );
            });
            if self.monitor_exists {
                ui.label(egui::RichText::new("✓").color(GRN).monospace().size(11.0));
            }
        });
        ui.add_space(6.0);

        // ── width ──
        config_label(ui, "width");
        ui.horizontal(|ui| {
            tree_indent(ui);
            custom_drag_bar(ui, &mut self.config.width, 300..=3840, Some(30.0), 130.0);
            ui.label(
                egui::RichText::new(format!("{} px", self.config.width))
                    .color(T2)
                    .monospace()
                    .size(11.0),
            );
        });
        ui.add_space(6.0);

        // ── height ──
        config_label(ui, "height");
        ui.horizontal(|ui| {
            tree_indent(ui);
            custom_drag_bar(ui, &mut self.config.height, 300..=3840, Some(30.0), 130.0);
            ui.label(
                egui::RichText::new(format!("{} px", self.config.height))
                    .color(T2)
                    .monospace()
                    .size(11.0),
            );
        });
        ui.add_space(6.0);

        // ── frame rate ──
        config_label(ui, "frame rate");
        ui.horizontal(|ui| {
            tree_indent(ui);
            custom_drag_bar(ui, &mut self.config.fps, 1..=90, Some(1.0), 130.0);
            ui.label(
                egui::RichText::new(format!("{} Hz", self.config.fps))
                    .color(T2)
                    .monospace()
                    .size(11.0),
            );
        });
        ui.add_space(6.0);

        // ── scale ──
        config_label(ui, "scale");
        ui.horizontal(|ui| {
            tree_indent(ui);
            custom_drag_bar(
                ui,
                &mut self.config.scale,
                0.5f32..=4.0f32,
                Some(0.05),
                130.0,
            );
            ui.label(
                egui::RichText::new(format!("{:.2}", self.config.scale))
                    .color(T2)
                    .monospace()
                    .size(11.0),
            );
        });
    }

    // ── Action buttons ──
    fn render_controls(&mut self, ui: &mut egui::Ui, stopping: bool) {
        let can_apply = !self.config.name.is_empty() && !stopping;
        let can_remove = self.monitor_exists && !self.is_capturing() && !stopping;
        let can_start = self.monitor_exists && !self.is_capturing() && !stopping;
        let can_stop = self.is_capturing() && !stopping;

        let btn_width = (272.0 - 4.0) / 2.0;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            let button_text = if self.monitor_exists {
                "update"
            } else {
                "create"
            };
            if custom_button(ui, button_text, can_apply, ACC, btn_width).clicked() {
                self.apply_config();
            }
            if custom_button(ui, "remove", can_remove, RED, btn_width).clicked() {
                self.do_remove();
            }
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            if custom_button(ui, "start", can_start, GRN, btn_width).clicked() {
                self.do_start_capture();
            }
            if custom_button(ui, "stop", can_stop, RED, btn_width).clicked() {
                self.do_stop_capture();
            }
        });
    }

    // ── Canvas / position panel ──
    fn render_canvas_panel(&mut self, ui: &mut egui::Ui) {
        panel_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(ui.available_height());
            ui.vertical(|ui| {
                section_header(ui, "position");

                // Position info line
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("x: {}  y: {}", self.config.x, self.config.y))
                            .color(T2)
                            .monospace()
                            .size(11.0),
                    );
                });
                ui.add_space(4.0);

                // ── Canvas ──
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

                // ── Drag logic ──
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
                    let snap_threshold = 250.0;
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
                    let snap_threshold = 350.0;
                    let mut raw_x = self.config.x as f32;
                    let mut raw_y = self.config.y as f32;
                    let mut snapped_x = false;
                    let mut snapped_y = false;

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

                // ── Drawing ──
                painter.rect_filled(canvas_rect, 0.0, CNV);
                painter.rect_stroke(canvas_rect, 0.0, egui::Stroke::new(0.5, BRD));

                let center_x = canvas_rect.center().x;
                let center_y = canvas_rect.center().y;
                let origin_x = center_x - (main_w / 2.0) * scale;
                let origin_y = center_y - (main_h / 2.0) * scale;

                let to_canvas = |sx: f32, sy: f32| -> egui::Pos2 {
                    egui::pos2(origin_x + sx * scale, origin_y + sy * scale)
                };

                // Grid lines
                let grid_step_canvas = 90.0 * scale;
                let grid_col = egui::Color32::from_rgb(26, 30, 32);
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
                        egui::Stroke::new(0.5, grid_col),
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
                        egui::Stroke::new(0.5, grid_col),
                    );
                    grid_y += grid_step_canvas;
                }

                // Origin crosshair
                painter.line_segment(
                    [
                        egui::pos2(origin_x - 8.0, origin_y),
                        egui::pos2(origin_x + 8.0, origin_y),
                    ],
                    egui::Stroke::new(1.0, BRD_H),
                );
                painter.line_segment(
                    [
                        egui::pos2(origin_x, origin_y - 8.0),
                        egui::pos2(origin_x, origin_y + 8.0),
                    ],
                    egui::Stroke::new(1.0, BRD_H),
                );

                // Main monitor
                let main_top_left = to_canvas(main_x, main_y);
                let main_rect = egui::Rect::from_min_size(
                    main_top_left,
                    egui::vec2(main_w * scale, main_h * scale),
                );
                painter.rect_filled(main_rect, 0.0, BG2);
                painter.rect_stroke(main_rect, 0.0, egui::Stroke::new(1.0, BRD));
                painter.text(
                    main_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "main (DP-1)",
                    egui::FontId::monospace(9.0),
                    T2,
                );

                // Virtual monitor
                let virt_top_left = to_canvas(self.config.x as f32, self.config.y as f32);
                let virt_rect = egui::Rect::from_min_size(
                    virt_top_left,
                    egui::vec2(virt_w * scale, virt_h * scale),
                );

                let (fill_col, stroke_col, text_col) = if self.monitor_exists {
                    (ACC_D, ACC, ACC)
                } else {
                    (BG3, T2, T2)
                };

                let is_grabbed = response.dragged();
                let actual_stroke = if is_grabbed {
                    egui::Stroke::new(1.5, T0)
                } else {
                    egui::Stroke::new(1.0, stroke_col)
                };

                painter.rect_filled(virt_rect, 0.0, fill_col);
                painter.rect_stroke(virt_rect, 0.0, actual_stroke);
                painter.text(
                    virt_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &format!(
                        "{}\n{}×{}",
                        self.config.name, self.config.width, self.config.height
                    ),
                    egui::FontId::monospace(9.0),
                    text_col,
                );
            });
        });
    }
}

// ─── Helpers ────────────────────────────────────────────────────

fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.horizontal(|ui| {
        // Short line before title
        let (line1, _) = ui.allocate_exact_size(egui::vec2(8.0, 1.0), egui::Sense::hover());
        let cy = line1.center().y;
        ui.painter().line_segment(
            [egui::pos2(line1.min.x, cy), egui::pos2(line1.max.x, cy)],
            egui::Stroke::new(0.5, BRD),
        );

        // Title text
        ui.label(
            egui::RichText::new(title)
                .color(ACC)
                .monospace()
                .size(11.0)
                .strong(),
        );

        // Line after title (fills remaining width)
        let avail = ui.available_width();
        let (line2, _) = ui.allocate_exact_size(egui::vec2(avail, 1.0), egui::Sense::hover());
        let cy2 = line2.center().y;
        ui.painter().line_segment(
            [egui::pos2(line2.min.x, cy2), egui::pos2(line2.max.x, cy2)],
            egui::Stroke::new(0.5, BRD),
        );
    });
    ui.add_space(4.0);
}

fn config_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(T2).monospace().size(11.0));
}

fn tree_indent(ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("└─").color(BRD).monospace().size(11.0));
}

fn panel_frame() -> egui::Frame {
    egui::Frame {
        fill: BG1,
        inner_margin: egui::Margin::same(8.0),
        stroke: egui::Stroke::new(0.5, BRD),
        rounding: egui::Rounding::same(0.0),
        ..Default::default()
    }
}

fn darken(c: egui::Color32, factor: f32) -> egui::Color32 {
    egui::Color32::from_rgb(
        (c.r() as f32 * factor) as u8,
        (c.g() as f32 * factor) as u8,
        (c.b() as f32 * factor) as u8,
    )
}

// ─── Style ──────────────────────────────────────────────────────

// fn configure_style(ctx: &egui::Context, scale: f32) {
//     ctx.set_pixels_per_point(scale);

//     // Font
//     let mut fonts = egui::FontDefinitions::default();
//     fonts.font_data.insert(
//         "custom_font".to_owned(),
//         egui::FontData::from_static(include_bytes!("/usr/share/fonts/mononoki-Regular.ttf")),
//     );
//     fonts
//         .families
//         .entry(egui::FontFamily::Proportional)
//         .or_default()
//         .insert(0, "custom_font".to_owned());
//     fonts
//         .families
//         .entry(egui::FontFamily::Monospace)
//         .or_default()
//         .insert(0, "custom_font".to_owned());
//     ctx.set_fonts(fonts);

//     // Style
//     let mut style = (*ctx.style()).clone();
//     style.spacing.item_spacing = egui::vec2(6.0, 4.0);
//     style.spacing.button_padding = egui::vec2(8.0, 4.0);

//     let rounding = egui::Rounding::same(0.0);
//     style.visuals.window_rounding = rounding;
//     style.visuals.widgets.noninteractive.rounding = rounding;
//     style.visuals.widgets.inactive.rounding = rounding;
//     style.visuals.widgets.hovered.rounding = rounding;
//     style.visuals.widgets.active.rounding = rounding;

//     style.visuals.dark_mode = true;
//     style.visuals.code_bg_color = CNV;
//     style.visuals.override_text_color = Some(T1);

//     style.visuals.widgets.inactive.bg_fill = BG2;
//     style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, T1);
//     style.visuals.widgets.inactive.border_color = egui::Color32::TRANSPARENT;
//     style.visuals.widgets.hovered.bg_fill = BG3;
//     style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, T0);
//     style.visuals.widgets.hovered.border_color = egui::Color32::TRANSPARENT;
//     style.visuals.widgets.active.bg_fill = BG3;
//     style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, T0);
//     style.visuals.widgets.active.border_color = egui::Color32::TRANSPARENT;

//     ctx.set_style(style);
// }

fn configure_style(ctx: &egui::Context, scale: f32) {
    ctx.set_pixels_per_point(scale);

    // Font
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "custom_font".to_owned(),
        egui::FontData::from_static(include_bytes!("/usr/share/fonts/mononoki-Regular.ttf")),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "custom_font".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "custom_font".to_owned());
    ctx.set_fonts(fonts);

    // Style
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);

    let rounding = egui::Rounding::same(0.0);
    style.visuals.window_rounding = rounding;
    style.visuals.widgets.noninteractive.rounding = rounding;
    style.visuals.widgets.inactive.rounding = rounding;
    style.visuals.widgets.hovered.rounding = rounding;
    style.visuals.widgets.active.rounding = rounding;

    style.visuals.dark_mode = true;
    style.visuals.code_bg_color = CNV;
    style.visuals.override_text_color = Some(T1);

    // ── FIX: bg_stroke in plaats van border_color ──
    style.visuals.widgets.inactive.bg_fill = BG2;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, T1);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;

    style.visuals.widgets.hovered.bg_fill = BG3;
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, T0);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;

    style.visuals.widgets.active.bg_fill = BG3;
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, T0);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;

    ctx.set_style(style);
}

// ─── Custom Widgets ─────────────────────────────────────────────

use egui::{emath, Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui, Vec2};

/// Flat TUI-style button. `color` determines the accent (ACC, GRN, RED).
pub fn custom_button(
    ui: &mut Ui,
    text: &str,
    enabled: bool,
    color: Color32,
    width: f32,
) -> Response {
    let desired_size = Vec2::new(width, 28.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(desired_size, sense);

    if ui.is_rect_visible(rect) {
        let dark = darken(color, 0.15);

        let (bg_col, border_col, text_col) = if !enabled {
            (BG1, BRD, T3)
        } else if response.is_pointer_button_down_on() {
            (dark, color, T0)
        } else if response.hovered() {
            (dark, color, T0)
        } else {
            (BG2, BRD, color)
        };

        ui.painter().rect_filled(rect, 0.0, bg_col);
        ui.painter()
            .rect_stroke(rect, 0.0, Stroke::new(0.5, border_col));
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::monospace(12.0),
            text_col,
        );
    }

    response
}

/// Minimal drag bar — thin line with a dot handle.
pub fn custom_drag_bar<T: emath::Numeric>(
    ui: &mut Ui,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
    step: Option<f64>,
    width: f32,
) -> Response {
    let height = 16.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, height), Sense::click_and_drag());

    let min = range.start().to_f64();
    let max = range.end().to_f64();

    // ── Drag logic ──
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

    // ── Rendering ──
    if ui.is_rect_visible(rect) {
        let current_val = value.to_f64();
        let normalized = ((current_val - min) / (max - min)).clamp(0.0, 1.0) as f32;
        let center_y = rect.center().y;
        let handle_x = rect.min.x + rect.width() * normalized;

        let active = response.hovered() || response.dragged();

        // Track (full width, dim)
        ui.painter().line_segment(
            [
                egui::pos2(rect.min.x, center_y),
                egui::pos2(rect.max.x, center_y),
            ],
            Stroke::new(1.0, BRD),
        );

        // Fill (left of handle, accent)
        ui.painter().line_segment(
            [
                egui::pos2(rect.min.x, center_y),
                egui::pos2(handle_x, center_y),
            ],
            Stroke::new(1.0, if active { ACC } else { ACC_D }),
        );

        // Handle (dot)
        let handle_r = if active { 4.0 } else { 3.0 };
        ui.painter().circle_filled(
            egui::pos2(handle_x, center_y),
            handle_r,
            if active { T0 } else { ACC },
        );
    }

    response
}
