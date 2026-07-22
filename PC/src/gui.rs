use crate::app::App;
use eframe::egui;
use std::time::Duration;

// ─── Music App (Omatunes) Kleurenpalet ──────────────────────────
const BG_MAIN: egui::Color32 = egui::Color32::from_rgb(20, 20, 28); // Donkere slate achtergrond
const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(28, 28, 38); // Paneel achtergrond
const BG_INNER: egui::Color32 = egui::Color32::from_rgb(15, 15, 22); // Terminal & Canvas achtergrond
const BORDER_COLOR: egui::Color32 = egui::Color32::from_rgb(42, 42, 58); // Subtiele randen

// Accenten uit de muziek-app screenshot (Neon Lime + Soft Indigo/Blue)
const ACCENT_LIME: egui::Color32 = egui::Color32::from_rgb(132, 204, 22); // Neon Lime Groen (#84cc16)
const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(163, 230, 53); // Lichter Neon Groen
const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(99, 102, 241); // Indigo Blauw (#6366f1)
const DANGER_RED: egui::Color32 = egui::Color32::from_rgb(239, 68, 68); // Soft Rood (#ef4444)
const DANGER_HOVER: egui::Color32 = egui::Color32::from_rgb(248, 113, 113);

const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(241, 245, 249); // Heldere tekst
const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(148, 163, 184); // Grijze subtekst

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        configure_style(ctx);

        // — Business logic tick —
        self.tick();
        if self.auto_refresh {
            ctx.request_repaint_after(Duration::from_secs(1));
        }

        // — Check voor signal shutdown —
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

        // ─── Layout Container ──────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(BG_MAIN)
                    .inner_margin(egui::Margin::same(16.0)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;

                // 1. Header (naam app)
                self.render_header(ui);

                // 2. Midden: Config (links) + Pos (rechts)
                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        self.render_config_card(ui);
                    });
                    cols[1].vertical(|ui| {
                        self.render_pos_card(ui);
                    });
                });

                // 3. Actieknoppen: [ Create ] [ Remove ] [ Start ] [ Stop ]
                self.render_action_bar(ui, stopping);

                // 4. Onder: FPS & stats
                self.render_graph_card(ui);
            });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.shutdown();
    }
}

// ─── UI Rendering Helpers ─────────────────────────────────────

impl App {
    /// Top Section: `naam app` (Header)
    fn render_header(&mut self, ui: &mut egui::Ui) {
        panel_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // Neon groen muziek-icon geïnspireerd badge
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, ACCENT_LIME);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "♬",
                    egui::FontId::proportional(16.0),
                    BG_MAIN,
                );

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("naam app")
                        .color(TEXT_PRIMARY)
                        .size(18.0)
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

                    ui.add_space(12.0);
                    if ui.add(ghost_button("🔄 refresh", ACCENT_LIME)).clicked() {
                        self.refresh();
                    }
                });
            });
        });
    }

    /// Top-Left Panel: `config`
    fn render_config_card(&mut self, ui: &mut egui::Ui) {
        panel_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(200.0);

            ui.label(
                egui::RichText::new("config")
                    .color(ACCENT_LIME)
                    .size(16.0)
                    .monospace()
                    .strong(),
            );
            ui.add_space(8.0);

            egui::Grid::new("config_grid")
                .num_columns(2)
                .spacing([12.0, 10.0])
                .show(ui, |ui| {
                    // identifier :
                    ui.label(
                        egui::RichText::new("identifier :")
                            .color(TEXT_MUTED)
                            .monospace(),
                    );
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(!self.monitor_exists, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.config.name)
                                    .desired_width(120.0),
                            );
                        });
                        if self.monitor_exists {
                            ui.label(
                                egui::RichText::new("✓ active")
                                    .color(ACCENT_LIME)
                                    .monospace()
                                    .small(),
                            );
                        }
                    });
                    ui.end_row();

                    // res :
                    ui.label(egui::RichText::new("res :").color(TEXT_MUTED).monospace());
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.config.width)
                                .range(320..=7680)
                                .suffix(" px"),
                        );
                        ui.label(egui::RichText::new("×").color(TEXT_MUTED));
                        ui.add(
                            egui::DragValue::new(&mut self.config.height)
                                .range(240..=4320)
                                .suffix(" px"),
                        );
                    });
                    ui.end_row();

                    // frame rate :
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

                    // scale :
                    ui.label(egui::RichText::new("scale :").color(TEXT_MUTED).monospace());
                    ui.add(
                        egui::DragValue::new(&mut self.config.scale)
                            .range(0.5f32..=3.0f32)
                            .speed(0.1),
                    );
                    ui.end_row();
                });

            ui.add_space(10.0);
            inner_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.monospace(
                    egui::RichText::new(format!("$ {}", self.config.to_keyword()))
                        .color(ACCENT_LIME)
                        .size(11.0),
                );
            });
        });
    }

    /// Top-Right Panel: `pos` (Monitor 2D visualizer with interactive dragging)
    fn render_pos_card(&mut self, ui: &mut egui::Ui) {
        panel_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            // ui.set_min_height(200.0);
            // ui.set_height(ui.available_height());

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("pos")
                        .color(ACCENT_LIME)
                        .size(16.0)
                        .monospace()
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("x: {}  y: {}", self.config.x, self.config.y))
                            .color(ACCENT_LIME)
                            .monospace()
                            .small(),
                    );
                });
            });
            ui.add_space(4.0);

            inner_frame().show(ui, |ui| {
                let (response, painter) = ui.allocate_painter(
                    egui::vec2(ui.available_width(), 180.0), // Iets groter gemaakt voor betere verhoudingen
                    egui::Sense::click_and_drag(),
                );

                let canvas_rect = response.rect;

                // --- nwg-displays logica ---

                let scale = 0.1; // 1 screen pixel = 0.1 canvas pixels (schaal)

                let snap_dist = 50.0; // Afstand in screen-pixels waarop hij magnetisch vastklikt

                // Hoofdmonitor (we gaan uit van een 1920x1080 scherm op 0,0)

                let main_w = 1920.0;

                let main_h = 1080.0;

                let main_x = 0.0;

                let main_y = 0.0;

                // Virtuele monitor eigenschappen uit de config

                let virt_w = self.config.width as f32;

                let virt_h = self.config.height as f32;

                // --- Sleep & Snap Logica ---

                if response.dragged() {
                    let delta = response.drag_delta();

                    // Reken canvas sleepbeweging om naar scherm-pixels

                    let dx = (delta.x / scale).round() as i32;

                    let dy = (delta.y / scale).round() as i32;

                    self.config.x += dx;

                    self.config.y += dy;

                    // Randen berekenen voor snapping

                    let v_left = self.config.x as f32;

                    let v_right = self.config.x as f32 + virt_w;

                    let v_top = self.config.y as f32;

                    let v_bottom = self.config.y as f32 + virt_h;

                    let m_right = main_x + main_w;

                    let m_left = main_x;

                    let m_bottom = main_y + main_h;

                    let m_top = main_y;

                    // Horizontaal snappen (X-as)

                    if (v_left - m_right).abs() < snap_dist {
                        self.config.x = m_right as i32;
                    } else if (v_right - m_left).abs() < snap_dist {
                        self.config.x = (m_left - virt_w) as i32;
                    }

                    // Verticaal snappen (Y-as)

                    if (v_top - m_bottom).abs() < snap_dist {
                        self.config.y = m_bottom as i32;
                    } else if (v_bottom - m_top).abs() < snap_dist {
                        self.config.y = (m_top - virt_h) as i32;
                    }
                }

                // Cursor hints

                if response.hovered() {
                    ui.ctx().set_cursor_icon(if response.dragged() {
                        egui::CursorIcon::Grabbing
                    } else {
                        egui::CursorIcon::Grab
                    });
                }

                // --- Teken Logica ---

                painter.rect_filled(canvas_rect, 2.0, BG_INNER);

                painter.rect_stroke(canvas_rect, 2.0, egui::Stroke::new(1.0, BORDER_COLOR));

                let center_x = canvas_rect.center().x;

                let center_y = canvas_rect.center().y;

                // Bepaal het nulpunt op het canvas (zodat het hoofd-scherm min of meer in het midden ligt)

                let origin_x = center_x - (main_w / 2.0) * scale;

                let origin_y = center_y - (main_h / 2.0) * scale;

                // Hulpfunctie: Vertaal scherm-coördinaten naar canvas-punten

                let to_canvas = |sx: f32, sy: f32| -> egui::Pos2 {
                    egui::pos2(origin_x + sx * scale, origin_y + sy * scale)
                };

                // 1. Teken hoofdmonitor (vanaf zijn linkerbovenhoek)

                let main_top_left = to_canvas(main_x, main_y);

                let main_rect = egui::Rect::from_min_size(
                    main_top_left,
                    egui::vec2(main_w * scale, main_h * scale),
                );

                painter.rect_filled(main_rect, 2.0, egui::Color32::from_rgb(35, 42, 60));

                painter.rect_stroke(main_rect, 2.0, egui::Stroke::new(1.5, ACCENT_BLUE));

                painter.text(
                    main_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "main\n(DP-1)",
                    egui::FontId::monospace(10.0),
                    TEXT_PRIMARY,
                );

                // 2. Teken virtuele monitor (vanaf zijn linkerbovenhoek)

                let virt_top_left = to_canvas(self.config.x as f32, self.config.y as f32);

                let virt_rect = egui::Rect::from_min_size(
                    virt_top_left,
                    egui::vec2(virt_w * scale, virt_h * scale),
                );

                let (fill_col, stroke_col) = if self.monitor_exists {
                    (
                        egui::Color32::from_rgba_premultiplied(132, 204, 22, 50),
                        ACCENT_LIME,
                    )
                } else {
                    (
                        egui::Color32::from_rgba_premultiplied(148, 163, 184, 25),
                        TEXT_MUTED,
                    )
                };

                let is_grabbed = response.dragged();

                let actual_stroke = if is_grabbed {
                    egui::Stroke::new(2.5, ACCENT_HOVER)
                } else {
                    egui::Stroke::new(1.5, stroke_col)
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
                    egui::FontId::monospace(9.5),
                    if self.monitor_exists {
                        ACCENT_LIME
                    } else {
                        TEXT_MUTED
                    },
                );

                // Instructie label

                painter.text(
                    egui::pos2(canvas_rect.min.x + 6.0, canvas_rect.max.y - 6.0),
                    egui::Align2::LEFT_BOTTOM,
                    "↔ drag to position (snaps to edges)",
                    egui::FontId::monospace(9.0),
                    TEXT_MUTED,
                );
            });
        });
    }

    /// Center Action Bar: `create` | `remove` | `start` | `stop`
    fn render_action_bar(&mut self, ui: &mut egui::Ui, stopping: bool) {
        panel_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());

            let can_create = !self.monitor_exists && !self.config.name.is_empty();
            let can_remove = self.monitor_exists && !self.is_capturing();
            let can_start = self.monitor_exists && !self.is_capturing() && !stopping;
            let can_stop = self.is_capturing() && !stopping;

            ui.columns(4, |cols| {
                // Button 1: create
                cols[0].scope(|ui| {
                    set_button_style(ui, ACCENT_LIME, ACCENT_HOVER, BG_MAIN);
                    if ui
                        .add_enabled(
                            can_create,
                            egui::Button::new("create")
                                .min_size(egui::vec2(ui.available_width(), 32.0)),
                        )
                        .clicked()
                    {
                        self.do_create();
                    }
                });

                // Button 2: remove
                cols[1].scope(|ui| {
                    set_button_style(ui, DANGER_RED, DANGER_HOVER, TEXT_PRIMARY);
                    if ui
                        .add_enabled(
                            can_remove,
                            egui::Button::new("remove")
                                .min_size(egui::vec2(ui.available_width(), 32.0)),
                        )
                        .clicked()
                    {
                        self.do_remove();
                    }
                });

                // Button 3: start
                cols[2].scope(|ui| {
                    set_button_style(
                        ui,
                        ACCENT_BLUE,
                        egui::Color32::from_rgb(129, 140, 248),
                        TEXT_PRIMARY,
                    );
                    if ui
                        .add_enabled(
                            can_start,
                            egui::Button::new("start")
                                .min_size(egui::vec2(ui.available_width(), 32.0)),
                        )
                        .clicked()
                    {
                        self.do_start_capture();
                    }
                });

                // Button 4: stop
                cols[3].scope(|ui| {
                    set_button_style(ui, DANGER_RED, DANGER_HOVER, TEXT_PRIMARY);
                    if ui
                        .add_enabled(
                            can_stop,
                            egui::Button::new("stop")
                                .min_size(egui::vec2(ui.available_width(), 32.0)),
                        )
                        .clicked()
                    {
                        self.do_stop_capture();
                    }
                });
            });
        });
    }

    /// Bottom-Right Panel: `fps` (Performance Graph & Metrics)
    fn render_graph_card(&mut self, ui: &mut egui::Ui) {
        panel_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(200.0);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("fps")
                        .color(ACCENT_LIME)
                        .size(16.0)
                        .monospace()
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("{:.1} FPS", self.current_fps))
                            .color(ACCENT_LIME)
                            .monospace()
                            .strong(),
                    );
                });
            });
            ui.add_space(4.0);

            // FPS Live Graph Canvas
            inner_frame().show(ui, |ui| {
                let (response, painter) = ui.allocate_painter(
                    egui::vec2(ui.available_width(), 100.0),
                    egui::Sense::hover(),
                );
                let rect = response.rect;

                // Graph Background
                painter.rect_filled(rect, 2.0, BG_INNER);
                painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, BORDER_COLOR));

                // 60 FPS reference target line
                let target_y = rect.max.y - (60.0 / 75.0) * rect.height();
                painter.line_segment(
                    [
                        egui::pos2(rect.min.x, target_y),
                        egui::pos2(rect.max.x, target_y),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 70)),
                );

                // Build line points from self.fps_history
                let history_len = self.fps_history.len();
                if history_len > 1 {
                    let mut points = Vec::with_capacity(history_len);
                    let step_x = rect.width() / (history_len - 1) as f32;

                    for (i, &fps_val) in self.fps_history.iter().enumerate() {
                        let clamped_fps = fps_val.clamp(0.0, 75.0);
                        let x = rect.min.x + i as f32 * step_x;
                        let y = rect.max.y - (clamped_fps / 75.0) * (rect.height() - 8.0) - 4.0;
                        points.push(egui::pos2(x, y));
                    }

                    // ─── Alleen de trace lijn (geen vulkleur meer) ───
                    painter.add(egui::Shape::line(
                        points,
                        egui::Stroke::new(1.5, ACCENT_LIME), // Strakke, iets dunnere lijn
                    ));
                }

                // X-axis time indicators (t-10s -> t)
                painter.text(
                    egui::pos2(rect.min.x + 6.0, rect.max.y - 12.0),
                    egui::Align2::LEFT_BOTTOM,
                    "t - 5",
                    egui::FontId::monospace(9.0),
                    TEXT_MUTED,
                );
                painter.text(
                    egui::pos2(rect.max.x - 6.0, rect.max.y - 12.0),
                    egui::Align2::RIGHT_BOTTOM,
                    "t",
                    egui::FontId::monospace(9.0),
                    TEXT_MUTED,
                );
            });

            ui.add_space(6.0);

            // Performance metrics row below graph
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;

                // Packet loss
                ui.label(
                    egui::RichText::new("packet loss :")
                        .color(TEXT_MUTED)
                        .monospace()
                        .small(),
                );
                ui.label(
                    egui::RichText::new("0.0%")
                        .color(ACCENT_LIME)
                        .monospace()
                        .small(),
                );

                // Tijd (elapsed stream duration)
                ui.label(
                    egui::RichText::new("tijd :")
                        .color(TEXT_MUTED)
                        .monospace()
                        .small(),
                );
                let elapsed_str = match self.stream_start_time {
                    Some(start) => {
                        let secs = start.elapsed().unwrap_or_default().as_secs();
                        format!("{:02}:{:02}", secs / 60, secs % 60)
                    }
                    None => "00:00".to_string(),
                };
                ui.label(
                    egui::RichText::new(elapsed_str)
                        .color(TEXT_PRIMARY)
                        .monospace()
                        .small(),
                );
            });
        });
    }
}

// ─── Styling Helper Frames & Functions ─────────────────────────

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);

    // ─── Scherpere randen (Rounding) ───
    // let sharp_rounding = egui::Rounding::same(2.0); // Strakke, moderne hoeken
    // style.visuals.window_rounding = sharp_rounding;
    // style.visuals.widgets.noninteractive.rounding = sharp_rounding;
    // style.visuals.widgets.inactive.rounding = sharp_rounding;
    // style.visuals.widgets.hovered.rounding = sharp_rounding;
    // style.visuals.widgets.active.rounding = sharp_rounding;
    // style.visuals.widgets.open.rounding = sharp_rounding;

    style.visuals.dark_mode = true;
    style.visuals.code_bg_color = BG_INNER;
    style.visuals.override_text_color = Some(TEXT_PRIMARY);

    // Inputs & DragValues
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
        // rounding: egui::Rounding::same(3.0), // Iets minder scherp voor de hoofdpanelen
        inner_margin: egui::Margin::same(14.0),
        stroke: egui::Stroke::new(1.0, BORDER_COLOR),
        ..Default::default()
    }
}

fn inner_frame() -> egui::Frame {
    egui::Frame {
        fill: BG_INNER,
        // rounding: egui::Rounding::same(0.0), // Strakke binnenkaders
        inner_margin: egui::Margin::same(10.0),
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
            .small(),
    )
    .fill(egui::Color32::TRANSPARENT)
    .stroke(egui::Stroke::NONE)
}
