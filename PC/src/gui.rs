use crate::app::App;
use crate::sunshine::ServiceState;
use crate::types::LogLevel;
use eframe::egui;
use std::time::Duration;

// palette
// layered darks
const BG0: egui::Color32 = egui::Color32::from_rgb(24, 32, 34);
const BG1: egui::Color32 = egui::Color32::from_rgb(26, 34, 36);
const BG2: egui::Color32 = egui::Color32::from_rgb(30, 39, 41);
const BG3: egui::Color32 = egui::Color32::from_rgb(36, 45, 48);
const CNV: egui::Color32 = egui::Color32::from_rgb(14, 20, 21);

// borders
const BRD: egui::Color32 = egui::Color32::from_rgb(38, 46, 48);
const BRD_H: egui::Color32 = egui::Color32::from_rgb(58, 66, 70);

// text
const T0: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const T1: egui::Color32 = egui::Color32::from_rgb(219, 216, 210);
const T2: egui::Color32 = egui::Color32::from_rgb(130, 130, 139);
const T3: egui::Color32 = egui::Color32::from_rgb(90, 90, 96);

// accents
const ACC: egui::Color32 = egui::Color32::from_rgb(94, 193, 255);
const ACC_D: egui::Color32 = egui::Color32::from_rgb(28, 55, 75);
const GRN: egui::Color32 = egui::Color32::from_rgb(70, 190, 100);
const YEL: egui::Color32 = egui::Color32::from_rgb(220, 180, 80);
const RED: egui::Color32 = egui::Color32::from_rgb(220, 90, 90);

/// Width of the config/controls column.
const LEFT_W: f32 = 300.0;

// app
impl App {
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
        if self.pin_pending() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        // Real panels instead of nested horizontal/vertical layouts, so every
        // section always gets its full share of the window.
        let gap = egui::Frame::none().fill(BG0).inner_margin(egui::Margin::same(4.0));

        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::none().fill(BG1))
            .show_separator_line(false)
            .show(ctx, |ui| self.render_top_bar(ui));

        egui::SidePanel::left("left")
            .frame(gap)
            .resizable(false)
            .exact_width(LEFT_W)
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let controls_h = 150.0;
                let config_h = (ui.available_height() - controls_h - 4.0).max(120.0);
                self.render_config_panel(ui, config_h);
                self.render_controls_panel(ui);
            });

        egui::TopBottomPanel::bottom("sunshine")
            .frame(gap)
            .resizable(false)
            .show_separator_line(false)
            .show(ctx, |ui| self.render_sunshine_panel(ui));

        egui::CentralPanel::default().frame(gap).show(ctx, |ui| {
            let h = ui.available_height();
            self.render_canvas_panel(ui, h);
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.shutdown();
    }
}

fn sunshine_color(state: &ServiceState) -> egui::Color32 {
    match state {
        ServiceState::Active => GRN,
        ServiceState::Activating => YEL,
        ServiceState::Failed => RED,
        ServiceState::Inactive | ServiceState::Unknown(_) => T3,
    }
}

// panels

impl App {
    fn render_top_bar(&mut self, ui: &mut egui::Ui) {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());

        ui.painter().rect_filled(rect, 0.0, BG1);
        ui.painter().line_segment(
            [egui::pos2(rect.min.x, rect.max.y), rect.max],
            egui::Stroke::new(0.5_f32, BRD),
        );

        let inner = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + 8.0, rect.min.y),
            egui::pos2(rect.max.x - 8.0, rect.max.y),
        );

        let sunshine = self.sunshine_state();

        ui.allocate_ui_at_rect(inner, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                let (dot, dot_col) = if self.monitor_exists() {
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

                if self.monitor_exists() {
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

                ui.label(
                    egui::RichText::new(format!(
                        "{}×{} @ {}fps",
                        self.config.width, self.config.height, self.config.fps
                    ))
                    .color(T2)
                    .monospace()
                    .size(11.0),
                );

                ui.add_space(8.0);
                if custom_icon_button(ui, "⇄", ACC)
                    .on_hover_text("Swap orientation (A×B ⇄ B×A)")
                    .clicked()
                {
                    self.do_toggle_orientation();
                }
                if custom_icon_button(ui, "↺", ACC)
                    .on_hover_text("Reset to the main display's mode")
                    .clicked()
                {
                    self.reset_to_main();
                }

                ui.label(egui::RichText::new("│").color(BRD).monospace().size(11.0));
                let main_text = match (&self.view.main, &self.view.main_error) {
                    (Some(m), _) => (
                        format!("main {} {}×{}@{:.0}", m.name, m.width, m.height, m.refresh),
                        T3,
                    ),
                    (None, Some(_)) => ("main display unreadable".to_string(), RED),
                    _ => (String::new(), T3),
                };
                ui.label(egui::RichText::new(main_text.0).color(main_text.1).monospace().size(11.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (cap_text, cap_col) =
                        (format!("● sunshine {}", sunshine.label()), sunshine_color(&sunshine));
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

    fn render_config_panel(&mut self, ui: &mut egui::Ui, panel_h: f32) {
        panel_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(panel_h - 16.0);
            ui.vertical(|ui| {
                section_header(ui, "config");
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.render_config(ui));
            });
        });
    }

    fn render_controls_panel(&mut self, ui: &mut egui::Ui) {
        panel_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                section_header(ui, "controls");
                self.render_controls(ui);
            });
        });
    }

    fn render_config(&mut self, ui: &mut egui::Ui) {
        config_label(ui, "identifier");
        ui.horizontal(|ui| {
            tree_indent(ui);
            ui.add_enabled_ui(!self.monitor_exists(), |ui| {
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
                    egui::Stroke::new(0.5_f32, if self.monitor_exists() { GRN } else { BRD }),
                );
            });
            if self.monitor_exists() {
                ui.label(egui::RichText::new("✓").color(GRN).monospace().size(11.0));
            }
        });
        ui.add_space(6.0);

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

        config_label(ui, "frame rate");
        ui.horizontal(|ui| {
            tree_indent(ui);
            custom_drag_bar(ui, &mut self.config.fps, 1..=144, Some(1.0), 130.0);
            ui.label(
                egui::RichText::new(format!("{} Hz", self.config.fps))
                    .color(T2)
                    .monospace()
                    .size(11.0),
            );
        });
        ui.add_space(6.0);

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
        ui.add_space(6.0);

        config_label(ui, "orientation");
        ui.horizontal(|ui| {
            tree_indent(ui);
            let o = self.config.orientation();
            ui.label(
                egui::RichText::new(format!("{} {}×{}", o.label(), self.config.width, self.config.height))
                    .color(T1)
                    .monospace()
                    .size(11.0),
            );
            if custom_icon_button(ui, "⇄", ACC)
                .on_hover_text("Swap A×B ⇄ B×A (also: streamer --toggle)")
                .clicked()
            {
                self.do_toggle_orientation();
            }
        });
        ui.add_space(6.0);

        config_label(ui, "main display");
        ui.horizontal(|ui| {
            tree_indent(ui);
            let current = self.view.preferred_main.clone();
            let shown = current
                .clone()
                .unwrap_or_else(|| format!("auto ({})", self.view.main.as_ref().map(|m| m.name.as_str()).unwrap_or("?")));
            let mut choice: Option<Option<String>> = None;
            egui::ComboBox::from_id_source("main_display")
                .selected_text(egui::RichText::new(shown).monospace().size(11.0))
                .width(150.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(current.is_none(), "auto").clicked() {
                        choice = Some(None);
                    }
                    for name in self.view.physical.clone() {
                        if ui.selectable_label(current.as_deref() == Some(&name), &name).clicked() {
                            choice = Some(Some(name));
                        }
                    }
                });
            if let Some(c) = choice {
                self.set_main_monitor(c);
            }
        });
        if let Some(e) = &self.view.main_error {
            ui.label(egui::RichText::new(format!("   {e}")).color(RED).monospace().size(10.0));
        }
        ui.add_space(6.0);

        config_label(ui, "moonlight");
        ui.horizontal(|ui| {
            tree_indent(ui);
            let mut follow = self.view.follow_client;
            if ui
                .checkbox(&mut follow, egui::RichText::new("follow client resolution").monospace().size(11.0))
                .on_hover_text(
                    "At session start (Sunshine prep-cmd) resize the virtual monitor to the resolution \
                     Moonlight requests, kept in the current orientation",
                )
                .changed()
            {
                self.set_follow_client(follow);
            }
        });
    }

    fn render_controls(&mut self, ui: &mut egui::Ui) {
        let can_apply = !self.config.name.is_empty();
        let can_remove = self.monitor_exists();
        let state = self.sunshine_state();
        let running = matches!(state, ServiceState::Active | ServiceState::Activating);

        let btn_width = (ui.available_width() - 4.0) / 2.0;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            let button_text = if self.monitor_exists() {
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
            if running {
                if custom_button(ui, "stop sunshine", true, RED, btn_width).clicked() {
                    self.do_sunshine("stop");
                }
            } else if custom_button(ui, "start sunshine", true, GRN, btn_width).clicked() {
                self.do_sunshine("start");
            }
            if custom_button(ui, "restart sunshine", running, YEL, btn_width).clicked() {
                self.do_sunshine("restart");
            }
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if custom_button(ui, "⇄ orientation", true, ACC, btn_width).clicked() {
                self.do_toggle_orientation();
            }
            if custom_button(ui, "sunshine web ui", true, ACC, btn_width).clicked() {
                self.open_web_ui();
            }
        });
    }

    fn render_canvas_panel(&mut self, ui: &mut egui::Ui, panel_h: f32) {
        panel_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height((panel_h - 16.0).max(60.0));
            ui.vertical(|ui| {
                section_header(ui, "position");

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("x: {}  y: {}", self.config.x, self.config.y))
                            .color(T2)
                            .monospace()
                            .size(11.0),
                    );
                });
                ui.add_space(4.0);

                let canvas_size = ui.available_size();
                let (response, painter) =
                    ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());
                let canvas_rect = response.rect;

                // Real main display geometry in layout coordinates (Eis 2).
                let (main_name, main_x, main_y, main_w, main_h) = match &self.view.main {
                    Some(m) => (
                        m.name.clone(),
                        m.x as f32,
                        m.y as f32,
                        m.logical_w as f32,
                        m.logical_h as f32,
                    ),
                    None => ("main ?".to_string(), 0.0, 0.0, 1920.0, 1080.0),
                };

                let (lw, lh) = self.config.logical_size();
                let virt_w = lw as f32;
                let virt_h = lh as f32;
                let scale = ((canvas_size.x / (main_w + 2.0 * virt_w.max(virt_h)))
                    .min(canvas_size.y / (main_h + 2.0 * virt_w.max(virt_h))))
                    * 0.95;

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
                    if (raw_x + virt_w - main_x).abs() < snap_threshold {
                        final_x = main_x - virt_w;
                    } else if (raw_x - (main_x + main_w)).abs() < snap_threshold {
                        final_x = main_x + main_w;
                    }
                    if (raw_y + virt_h - main_y).abs() < snap_threshold {
                        final_y = main_y - virt_h;
                    } else if (raw_y - (main_y + main_h)).abs() < snap_threshold {
                        final_y = main_y + main_h;
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

                    if (raw_x + virt_w - main_x).abs() < snap_threshold {
                        raw_x = main_x - virt_w;
                        snapped_x = true;
                    } else if (raw_x - (main_x + main_w)).abs() < snap_threshold {
                        raw_x = main_x + main_w;
                        snapped_x = true;
                    }
                    if (raw_y + virt_h - main_y).abs() < snap_threshold {
                        raw_y = main_y - virt_h;
                        snapped_y = true;
                    } else if (raw_y - (main_y + main_h)).abs() < snap_threshold {
                        raw_y = main_y + main_h;
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

                painter.rect_filled(canvas_rect, 0.0, CNV);
                painter.rect_stroke(canvas_rect, 0.0, egui::Stroke::new(0.5_f32, BRD));

                let center_x = canvas_rect.center().x;
                let center_y = canvas_rect.center().y;
                let origin_x = center_x - (main_x + main_w / 2.0) * scale;
                let origin_y = center_y - (main_y + main_h / 2.0) * scale;

                let to_canvas = |sx: f32, sy: f32| -> egui::Pos2 {
                    egui::pos2(origin_x + sx * scale, origin_y + sy * scale)
                };

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
                        egui::Stroke::new(0.5_f32, grid_col),
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
                        egui::Stroke::new(0.5_f32, grid_col),
                    );
                    grid_y += grid_step_canvas;
                }

                painter.line_segment(
                    [
                        egui::pos2(origin_x - 8.0, origin_y),
                        egui::pos2(origin_x + 8.0, origin_y),
                    ],
                    egui::Stroke::new(1.0_f32, BRD_H),
                );
                painter.line_segment(
                    [
                        egui::pos2(origin_x, origin_y - 8.0),
                        egui::pos2(origin_x, origin_y + 8.0),
                    ],
                    egui::Stroke::new(1.0_f32, BRD_H),
                );

                let main_top_left = to_canvas(main_x, main_y);
                let main_rect = egui::Rect::from_min_size(
                    main_top_left,
                    egui::vec2(main_w * scale, main_h * scale),
                );
                painter.rect_filled(main_rect, 0.0, BG2);
                painter.rect_stroke(main_rect, 0.0, egui::Stroke::new(1.0_f32, BRD));
                painter.text(
                    main_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &main_name,
                    egui::FontId::monospace(9.0),
                    T2,
                );

                let virt_top_left = to_canvas(self.config.x as f32, self.config.y as f32);
                let virt_rect = egui::Rect::from_min_size(
                    virt_top_left,
                    egui::vec2(virt_w * scale, virt_h * scale),
                );

                let (fill_col, stroke_col, text_col) = if self.monitor_exists() {
                    (ACC_D, ACC, ACC)
                } else {
                    (BG3, T2, T2)
                };

                let is_grabbed = response.dragged();
                let actual_stroke = if is_grabbed {
                    egui::Stroke::new(1.5_f32, T0)
                } else {
                    egui::Stroke::new(1.0_f32, stroke_col)
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

    /// Sunshine service state, output selection, client-resolution hook and
    /// PIN pairing. Same panel/section-header/tree style as the rest.
    fn render_sunshine_panel(&mut self, ui: &mut egui::Ui) {
        let state = self.sunshine_state();
        let info = self.sunshine.clone();
        let mut action: Option<Box<dyn FnOnce(&mut App)>> = None;

        panel_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                section_header(ui, "sunshine");

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(egui::RichText::new("●").color(sunshine_color(&state)).monospace().size(10.0));
                    ui.label(small(state.label(), T1));
                    ui.label(small("│", BRD));

                    let output = info.as_ref().and_then(|i| i.output_name.clone());
                    let matches = output.as_deref() == Some(self.config.name.trim());
                    ui.label(small(
                        format!("output {}", output.as_deref().unwrap_or("(first monitor)")),
                        if matches { GRN } else { YEL },
                    ));
                    if !matches
                        && custom_icon_button(ui, "↻", ACC)
                            .on_hover_text(format!("Set Sunshine output_name = {}", self.config.name.trim()))
                            .clicked()
                    {
                        action = Some(Box::new(|app: &mut App| app.sync_output_name()));
                    }
                    let mut sync = self.settings.sync_output_name;
                    if ui
                        .checkbox(&mut sync, small("keep in sync", T2))
                        .on_hover_text("Write the virtual monitor's name to sunshine.conf on create/update")
                        .changed()
                    {
                        action = Some(Box::new(move |app: &mut App| app.set_sync_output_name(sync)));
                    }
                });

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let installed = info.as_ref().is_some_and(|i| i.prep_installed);
                    if installed {
                        ui.label(small("client-resolution hook installed", GRN));
                        if custom_icon_button(ui, "×", RED)
                            .on_hover_text("Remove the prep-cmd from Sunshine's Desktop app")
                            .clicked()
                        {
                            action = Some(Box::new(|app: &mut App| app.set_prep_cmd(false)));
                        }
                    } else {
                        ui.label(small("client-resolution hook not installed", T3));
                        if custom_icon_button(ui, "+", ACC)
                            .on_hover_text(
                                "Add `streamer --client-mode` as prep-cmd to Sunshine's Desktop app, \
                                 so the monitor takes the resolution Moonlight asks for",
                            )
                            .clicked()
                        {
                            action = Some(Box::new(|app: &mut App| app.set_prep_cmd(true)));
                        }
                    }
                });

                ui.add_space(4.0);

                let mut send = false;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(small("pair  PIN", T2));
                    let pin = underlined(ui, &mut self.pin_input, "0000", 40.0, false, 4);
                    self.pin_input.retain(|c| c.is_ascii_digit());
                    ui.label(small("device", T2));
                    underlined(ui, &mut self.pin_client_name, "ipad", 80.0, false, 32);
                    let ready = self.pin_input.len() == 4 && !self.pin_pending();
                    let label = if self.pin_pending() { "…" } else { "pair" };
                    send = small_button(ui, label, ready, GRN).clicked()
                        || (ready && pin.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                });

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(small("login", T2));
                    let user = underlined(ui, &mut self.settings.sunshine_user, "user", 80.0, false, 64);
                    let pass = underlined(ui, &mut self.sunshine_password, "password", 80.0, true, 128);
                    if user.lost_focus() || pass.lost_focus() {
                        action = Some(Box::new(|app: &mut App| app.credentials_changed()));
                    }
                    let mut remember = self.settings.sunshine_password.is_some();
                    if ui
                        .checkbox(&mut remember, small("remember", T2))
                        .on_hover_text("Store the password in ~/.config/hyprstream/settings.json (mode 600)")
                        .changed()
                    {
                        action = Some(Box::new(move |app: &mut App| app.set_remember_password(remember)));
                    }
                });

                if send {
                    action = Some(Box::new(|app: &mut App| app.do_send_pin()));
                }

                // Latest log line: the result of the last action.
                if let Some(e) = self.log_entries.last() {
                    let col = match e.level {
                        LogLevel::Error => RED,
                        LogLevel::Warning => YEL,
                        LogLevel::Success => GRN,
                        LogLevel::Info => T2,
                    };
                    ui.label(small(format!("{}  {}", e.time, e.message), col));
                }
            });
        });

        if let Some(f) = action {
            f(self);
        }
    }
}

fn small(text: impl Into<String>, color: egui::Color32) -> egui::RichText {
    egui::RichText::new(text).color(color).monospace().size(11.0)
}

/// Frameless text field with a thin underline, like the identifier field.
fn underlined(
    ui: &mut egui::Ui,
    text: &mut String,
    hint: &str,
    width: f32,
    password: bool,
    limit: usize,
) -> egui::Response {
    let response = ui.add(
        egui::TextEdit::singleline(text)
            .desired_width(width)
            .frame(false)
            .password(password)
            .char_limit(limit)
            .font(egui::FontId::monospace(11.0))
            .text_color(T1)
            .hint_text(small(hint, T3)),
    );
    let y = response.rect.bottom();
    let col = if response.has_focus() { ACC } else { BRD_H };
    ui.painter().line_segment(
        [egui::pos2(response.rect.min.x, y), egui::pos2(response.rect.max.x, y)],
        egui::Stroke::new(0.5_f32, col),
    );
    response
}

/// Inline text button, same height as a text row.
fn small_button(ui: &mut egui::Ui, text: &str, enabled: bool, color: egui::Color32) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::monospace(11.0), color);
    let size = egui::vec2(galley.size().x + 16.0, 18.0);
    let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let (bg, border, fg) = if !enabled {
        (BG1, BRD, T3)
    } else if response.hovered() {
        (darken(color, 0.15), color, T0)
    } else {
        (BG2, BRD, color)
    };
    ui.painter().rect_filled(rect, 0.0, bg);
    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(0.5_f32, border));
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::monospace(11.0), fg);
    response
}

fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.horizontal(|ui| {
        let (line1, _) = ui.allocate_exact_size(egui::vec2(8.0, 1.0), egui::Sense::hover());
        let cy = line1.center().y;
        ui.painter().line_segment(
            [egui::pos2(line1.min.x, cy), egui::pos2(line1.max.x, cy)],
            egui::Stroke::new(0.5_f32, BRD),
        );

        ui.label(
            egui::RichText::new(title)
                .color(ACC)
                .monospace()
                .size(11.0)
                .strong(),
        );

        let avail = ui.available_width();
        let (line2, _) = ui.allocate_exact_size(egui::vec2(avail, 1.0), egui::Sense::hover());
        let cy2 = line2.center().y;
        ui.painter().line_segment(
            [egui::pos2(line2.min.x, cy2), egui::pos2(line2.max.x, cy2)],
            egui::Stroke::new(0.5_f32, BRD),
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
        stroke: egui::Stroke::new(0.5_f32, BRD),
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

fn configure_style(ctx: &egui::Context, scale: f32) {
    ctx.set_pixels_per_point(scale);

    // Font
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "custom_font".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "/usr/share/fonts/Mononoki/mononoki-Regular.ttf"
        )),
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

    style.visuals.widgets.inactive.bg_fill = BG2;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, T1);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;

    style.visuals.widgets.hovered.bg_fill = BG3;
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, T0);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;

    style.visuals.widgets.active.bg_fill = BG3;
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, T0);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;

    ctx.set_style(style);
}

use egui::{emath, Align2, Color32, FontId, Response, Sense, Stroke, Ui, Vec2};

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
            .rect_stroke(rect, 0.0, Stroke::new(0.5_f32, border_col));
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

    if ui.is_rect_visible(rect) {
        let current_val = value.to_f64();
        let normalized = ((current_val - min) / (max - min)).clamp(0.0, 1.0) as f32;
        let center_y = rect.center().y;
        let handle_x = rect.min.x + rect.width() * normalized;

        let active = response.hovered() || response.dragged();

        ui.painter().line_segment(
            [
                egui::pos2(rect.min.x, center_y),
                egui::pos2(rect.max.x, center_y),
            ],
            Stroke::new(1.0_f32, BRD),
        );

        ui.painter().line_segment(
            [
                egui::pos2(rect.min.x, center_y),
                egui::pos2(handle_x, center_y),
            ],
            Stroke::new(1.0_f32, if active { ACC } else { ACC_D }),
        );

        let handle_r = if active { 4.0 } else { 3.0 };
        ui.painter().circle_filled(
            egui::pos2(handle_x, center_y),
            handle_r,
            if active { T0 } else { ACC },
        );
    }

    response
}

pub fn custom_icon_button(ui: &mut Ui, icon: &str, color: Color32) -> Response {
    let desired_size = Vec2::new(16.0, 16.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let text_col = if !response.enabled() {
            T3
        } else if response.is_pointer_button_down_on() {
            T0
        } else if response.hovered() {
            color
        } else {
            T2
        };

        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            icon,
            FontId::monospace(12.0),
            text_col,
        );
    }

    response
}
