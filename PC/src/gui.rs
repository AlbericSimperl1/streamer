// use crate::app::App;
// use eframe::egui;
// use std::time::Duration;

// // ─── Music App (Omatunes) Kleurenpalet ──────────────────────────
// const BG_MAIN: egui::Color32 = egui::Color32::from_rgb(20, 20, 28); // Donkere slate achtergrond
// const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(28, 28, 38); // Paneel achtergrond
// const BG_INNER: egui::Color32 = egui::Color32::from_rgb(15, 15, 22); // Terminal & Canvas achtergrond
// const BORDER_COLOR: egui::Color32 = egui::Color32::from_rgb(42, 42, 58); // Subtiele randen

// // Accenten uit de muziek-app screenshot (Neon Lime + Soft Indigo/Blue)
// const ACCENT_LIME: egui::Color32 = egui::Color32::from_rgb(255, 246, 224);
// const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
// const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(99, 102, 241); // Indigo Blauw (#6366f1)
// const DANGER_RED: egui::Color32 = egui::Color32::from_rgb(239, 68, 68); // Soft Rood (#ef4444)
// const DANGER_HOVER: egui::Color32 = egui::Color32::from_rgb(248, 113, 113);

// const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(241, 245, 249); // Heldere tekst
// const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(148, 163, 184); // Grijze subtekst

// impl eframe::App for App {
//     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//         configure_style(ctx);

//         // — Business logic tick —
//         self.tick();
//         if self.auto_refresh {
//             ctx.request_repaint_after(Duration::from_secs(1));
//         }

//         // — Check voor signal shutdown —
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

//         // ─── Layout Container ──────────────────────────────────────────
//         egui::CentralPanel::default()
//             .frame(
//                 egui::Frame::none()
//                     .fill(BG_MAIN)
//                     .inner_margin(egui::Margin::same(16.0)),
//             )
//             .show(ctx, |ui| {
//                 ui.spacing_mut().item_spacing.y = 12.0;

//                 // 1. Header (naam app)
//                 self.render_header(ui);

//                 // 2. Midden: Config (links) + Pos (rechts)
//                 ui.columns(2, |cols| {
//                     cols[0].vertical(|ui| {
//                         self.render_config_card(ui);
//                     });
//                     cols[1].vertical(|ui| {
//                         self.render_pos_card(ui);
//                     });
//                 });

//                 // 3. Actieknoppen: [ Create ] [ Remove ] [ Start ] [ Stop ]
//                 self.render_action_bar(ui, stopping);

//                 // 4. Onder: FPS & stats
//                 self.render_graph_card(ui);
//             });
//     }

//     fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
//         self.shutdown();
//     }
// }

// // ─── UI Rendering Helpers ─────────────────────────────────────

// impl App {
//     /// Top Section: `naam app` (Header)
//     fn render_header(&mut self, ui: &mut egui::Ui) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(ui.available_width());
//             ui.horizontal(|ui| {
//                 // Neon groen muziek-icon geïnspireerd badge
//                 let (rect, _) =
//                     ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
//                 ui.painter().rect_filled(rect, 2.0, ACCENT_LIME);
//                 ui.painter().text(
//                     rect.center(),
//                     egui::Align2::CENTER_CENTER,
//                     "♬",
//                     egui::FontId::proportional(16.0),
//                     BG_MAIN,
//                 );

//                 ui.add_space(6.0);
//                 ui.label(
//                     egui::RichText::new("naam app")
//                         .color(TEXT_PRIMARY)
//                         .size(18.0)
//                         .monospace()
//                         .strong(),
//                 );

//                 ui.label(
//                     egui::RichText::new("// hyprland display streamer")
//                         .color(TEXT_MUTED)
//                         .size(12.0)
//                         .monospace(),
//                 );

//                 ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
//                     let capturing = self.is_capturing();
//                     let (status_text, color) = if capturing {
//                         ("● STREAMING", ACCENT_LIME)
//                     } else if self.is_stopping() {
//                         ("⏳ STOPPING", ACCENT_BLUE)
//                     } else if self.monitor_exists {
//                         ("● ONLINE", ACCENT_BLUE)
//                     } else {
//                         ("○ OFFLINE", TEXT_MUTED)
//                     };

//                     ui.label(
//                         egui::RichText::new(status_text)
//                             .color(color)
//                             .size(12.0)
//                             .monospace()
//                             .strong(),
//                     );

//                     ui.add_space(12.0);
//                     if ui.add(ghost_button("🔄 refresh", ACCENT_LIME)).clicked() {
//                         self.refresh();
//                     }
//                 });
//             });
//         });
//     }

//     /// Top-Left Panel: `config`
//     fn render_config_card(&mut self, ui: &mut egui::Ui) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(270.0);
//             ui.set_min_height(255.0);

//             ui.label(
//                 egui::RichText::new("configuration")
//                     .color(ACCENT_LIME)
//                     .size(16.0)
//                     .monospace()
//                     .strong(),
//             );
//             ui.add_space(8.0);

//             egui::Grid::new("config_grid")
//                 .num_columns(2)
//                 .spacing([12.0, 10.0])
//                 .show(ui, |ui| {
//                     ui.label(
//                         egui::RichText::new("identifier :")
//                             .color(TEXT_MUTED)
//                             .monospace(),
//                     );
//                     ui.horizontal(|ui| {
//                         ui.add_enabled_ui(!self.monitor_exists, |ui| {
//                             ui.add(
//                                 egui::TextEdit::singleline(&mut self.config.name)
//                                     .desired_width(120.0),
//                             );
//                         });
//                         if self.monitor_exists {
//                             ui.label(
//                                 egui::RichText::new("✓ active")
//                                     .color(ACCENT_LIME)
//                                     .monospace()
//                                     .small(),
//                             );
//                         }
//                     });
//                     ui.end_row();

//                     // res :
//                     ui.label(egui::RichText::new("width :").color(TEXT_MUTED).monospace());
//                     ui.horizontal(|ui| {
//                         ui.add(
//                             egui::DragValue::new(&mut self.config.width)
//                                 .range(320..=7680)
//                                 .suffix(" px"),
//                         );
//                     });
//                     ui.end_row();
//                     ui.label(
//                         egui::RichText::new("height :")
//                             .color(TEXT_MUTED)
//                             .monospace(),
//                     );
//                     ui.horizontal(|ui| {
//                         ui.add(
//                             egui::DragValue::new(&mut self.config.height)
//                                 .range(320..=7680)
//                                 .suffix(" px"),
//                         );
//                     });
//                     ui.end_row();

//                     // frame rate :
//                     ui.label(
//                         egui::RichText::new("frame rate :")
//                             .color(TEXT_MUTED)
//                             .monospace(),
//                     );
//                     ui.add(
//                         egui::DragValue::new(&mut self.config.fps)
//                             .range(1..=240)
//                             .suffix(" Hz"),
//                     );
//                     ui.end_row();

//                     // scale :
//                     ui.label(egui::RichText::new("scale :").color(TEXT_MUTED).monospace());
//                     ui.add(
//                         egui::DragValue::new(&mut self.config.scale)
//                             .range(0.5f32..=3.0f32)
//                             .speed(0.1),
//                     );
//                     ui.end_row();
//                 });
//             ui.add_space(5.0);
//             inner_frame().show(ui, |ui| {
//                 ui.set_width(ui.available_width());
//                 ui.monospace(
//                     egui::RichText::new(format!("$ {}", self.config.to_keyword()))
//                         .color(ACCENT_LIME)
//                         .size(11.0),
//                 );
//             });
//         });
//     }

//     /// Top-Right Panel: `pos` (Monitor 2D visualizer with interactive dragging)
//     fn render_pos_card(&mut self, ui: &mut egui::Ui) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(ui.available_width());
//             ui.set_min_height(255.0);

//             ui.horizontal(|ui| {
//                 ui.label(
//                     egui::RichText::new("position")
//                         .color(ACCENT_LIME)
//                         .size(16.0)
//                         .monospace()
//                         .strong(),
//                 );
//                 ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
//                     ui.label(
//                         (egui::RichText::new(format!(
//                             "x: {}  y: {}",
//                             self.config.x, self.config.y
//                         ))
//                         .color(ACCENT_LIME))
//                         .monospace()
//                         .size(16.0),
//                     );
//                 });
//             });
//             ui.add_space(4.0);

//             inner_frame().show(ui, |ui| {
//                 let (response, painter) = ui.allocate_painter(
//                     egui::vec2(ui.available_width(), 180.0),
//                     egui::Sense::click_and_drag(),
//                 );

//                 let canvas_rect = response.rect;

//                 // --- nwg-displays logica ---

//                 let scale = 0.065; // 1 screen pixel = 0.065 canvas pixels (schaal)

//                 // Hoofdmonitor (we gaan uit van een 1920x1080 scherm op 0,0)
//                 let main_w = 1920.0;
//                 let main_h = 1080.0;
//                 let main_x = 0.0;
//                 let main_y = 0.0;

//                 // Virtuele monitor eigenschappen uit de config
//                 let virt_w = self.config.width as f32;
//                 let virt_h = self.config.height as f32;

//                 // --- Sleep & Raster Logica (1:1 beweging, 90px raster bij loslaten) ---

//                 if response.drag_started() {
//                     // Sla de startpositie op als float om precisie te behouden tijdens het slepen
//                     ui.memory_mut(|m| {
//                         m.data
//                             .insert_temp(egui::Id::new("virt_raw_x"), self.config.x as f32);
//                         m.data
//                             .insert_temp(egui::Id::new("virt_raw_y"), self.config.y as f32);
//                     });
//                 }

//                 if response.dragged() {
//                     let delta = response.drag_delta();

//                     // Haal de raw float posities op en update ze met de cursor delta
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

//                     // Update de monitor positie 1:1 met de cursor (zonder afronding)
//                     self.config.x = raw_x as i32;
//                     self.config.y = raw_y as i32;
//                 }

//                 if response.drag_stopped() {
//                     // Discretiseer naar 90px raster bij het loslaten
//                     let grid_step = 90;
//                     self.config.x = ((self.config.x as f32 / grid_step as f32).round()
//                         * grid_step as f32) as i32;
//                     self.config.y = ((self.config.y as f32 / grid_step as f32).round()
//                         * grid_step as f32) as i32;

//                     // Reset memory naar de afgeronde waarde
//                     ui.memory_mut(|m| {
//                         m.data
//                             .insert_temp(egui::Id::new("virt_raw_x"), self.config.x as f32);
//                         m.data
//                             .insert_temp(egui::Id::new("virt_raw_y"), self.config.y as f32);
//                     });
//                 }

//                 // --- Teken Logica ---

//                 painter.rect_filled(canvas_rect, 2.0, BG_INNER);
//                 painter.rect_stroke(canvas_rect, 2.0, egui::Stroke::new(1.0, BORDER_COLOR));

//                 let center_x = canvas_rect.center().x;
//                 let center_y = canvas_rect.center().y;

//                 // Bepaal het nulpunt op het canvas (zodat het hoofd-scherm min of meer in het midden ligt)
//                 let origin_x = center_x - (main_w / 2.0) * scale;
//                 let origin_y = center_y - (main_h / 2.0) * scale;

//                 // Hulpfunctie: Vertaal scherm-coördinaten naar canvas-punten
//                 let to_canvas = |sx: f32, sy: f32| -> egui::Pos2 {
//                     egui::pos2(origin_x + sx * scale, origin_y + sy * scale)
//                 };

//                 // --- Teken het 90px Raster ---
//                 let grid_step_canvas = 90.0 * scale;

//                 // Verticale lijnen
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
//                         egui::Stroke::new(0.5, egui::Color32::from_rgb(25, 25, 35)), // Zeer subtiel
//                     );
//                     grid_x += grid_step_canvas;
//                 }

//                 // Horizontale lijnen
//                 let mut grid_y = origin_y;
//                 while grid_y > canvas_rect.min.y {
//                     grid_y -= grid_step_canvas;
//                 }
//                 while grid_y < canvas_rect.max.y {
//                     painter.line_segment(
//                         [
//                             egui::pos2(canvas_rect.min.x, grid_y),
//                             egui::pos2(canvas_rect.max.x, grid_y),
//                         ],
//                         egui::Stroke::new(0.5, egui::Color32::from_rgb(25, 25, 35)), // Zeer subtiel
//                     );
//                     grid_y += grid_step_canvas;
//                 }

//                 // 1. Teken hoofdmonitor (vanaf zijn linkerbovenhoek)
//                 let main_top_left = to_canvas(main_x, main_y);
//                 let main_rect = egui::Rect::from_min_size(
//                     main_top_left,
//                     egui::vec2(main_w * scale, main_h * scale),
//                 );

//                 painter.rect_filled(main_rect, 2.0, egui::Color32::from_rgb(35, 42, 60));
//                 painter.rect_stroke(main_rect, 2.0, egui::Stroke::new(1.5, ACCENT_BLUE));
//                 painter.text(
//                     main_rect.center(),
//                     egui::Align2::CENTER_CENTER,
//                     "main\n(DP-1)",
//                     egui::FontId::monospace(10.0),
//                     TEXT_PRIMARY,
//                 );

//                 // 2. Teken virtuele monitor (vanaf zijn linkerbovenhoek)
//                 let virt_top_left =
//                     to_canvas((self.config.x as f32 - 1440.0), self.config.y as f32);

//                 let virt_rect = egui::Rect::from_min_size(
//                     virt_top_left,
//                     egui::vec2(virt_w * scale, virt_h * scale),
//                 );

//                 let (fill_col, stroke_col) = if self.monitor_exists {
//                     (
//                         egui::Color32::from_rgba_premultiplied(132, 204, 22, 50),
//                         ACCENT_LIME,
//                     )
//                 } else {
//                     (
//                         egui::Color32::from_rgba_premultiplied(148, 163, 184, 25),
//                         TEXT_MUTED,
//                     )
//                 };

//                 let is_grabbed = response.dragged();

//                 let actual_stroke = if is_grabbed {
//                     egui::Stroke::new(2.5, ACCENT_HOVER)
//                 } else {
//                     egui::Stroke::new(1.5, stroke_col)
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
//                     egui::FontId::monospace(9.5),
//                     if self.monitor_exists {
//                         ACCENT_LIME
//                     } else {
//                         TEXT_MUTED
//                     },
//                 );

//                 // Instructie label
//                 painter.text(
//                     egui::pos2(canvas_rect.min.x + 6.0, canvas_rect.max.y - 6.0),
//                     egui::Align2::LEFT_BOTTOM,
//                     "↔ drag to position (snaps to 90px grid)",
//                     egui::FontId::monospace(9.0),
//                     TEXT_MUTED,
//                 );
//             });
//         });
//     }

//     /// Center Action Bar: `create` | `remove` | `start` | `stop`
//     fn render_action_bar(&mut self, ui: &mut egui::Ui, stopping: bool) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(ui.available_width());
//             ui.set_height(15.0);

//             let can_create = !self.monitor_exists && !self.config.name.is_empty();
//             let can_remove = self.monitor_exists && !self.is_capturing();
//             let can_start = self.monitor_exists && !self.is_capturing() && !stopping;
//             let can_stop = self.is_capturing() && !stopping;

//             ui.columns(4, |cols| {
//                 // Button 1: create
//                 cols[0].scope(|ui| {
//                     set_button_style(ui, ACCENT_LIME, ACCENT_HOVER, BG_MAIN);
//                     let button_text = if self.monitor_exists {
//                         "Update Config"
//                     } else {
//                         "Create Monitor"
//                     };

//                     if ui.button(button_text).clicked() {
//                         self.apply_config();
//                     }
//                 });

//                 // Button 2: remove
//                 cols[1].scope(|ui| {
//                     set_button_style(ui, DANGER_RED, DANGER_HOVER, TEXT_PRIMARY);
//                     if ui
//                         .add_enabled(
//                             can_remove,
//                             egui::Button::new("remove")
//                                 .min_size(egui::vec2(ui.available_width(), 32.0)),
//                         )
//                         .clicked()
//                     {
//                         self.do_remove();
//                     }
//                 });

//                 // Button 3: start
//                 cols[2].scope(|ui| {
//                     set_button_style(
//                         ui,
//                         ACCENT_BLUE,
//                         egui::Color32::from_rgb(129, 140, 248),
//                         TEXT_PRIMARY,
//                     );
//                     if ui
//                         .add_enabled(
//                             can_start,
//                             egui::Button::new("start")
//                                 .min_size(egui::vec2(ui.available_width(), 32.0)),
//                         )
//                         .clicked()
//                     {
//                         self.do_start_capture();
//                     }
//                 });

//                 // Button 4: stop
//                 cols[3].scope(|ui| {
//                     set_button_style(ui, DANGER_RED, DANGER_HOVER, TEXT_PRIMARY);
//                     if ui
//                         .add_enabled(
//                             can_stop,
//                             egui::Button::new("stop")
//                                 .min_size(egui::vec2(ui.available_width(), 32.0)),
//                         )
//                         .clicked()
//                     {
//                         self.do_stop_capture();
//                     }
//                 });
//             });
//         });
//     }

//     /// Bottom-Right Panel: `fps` (Performance Graph & Metrics)
//     fn render_graph_card(&mut self, ui: &mut egui::Ui) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(ui.available_width());
//             ui.set_min_height(200.0);

//             ui.horizontal(|ui| {
//                 ui.label(
//                     egui::RichText::new("fps")
//                         .color(ACCENT_LIME)
//                         .size(16.0)
//                         .monospace()
//                         .strong(),
//                 );
//                 ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
//                     ui.label(
//                         egui::RichText::new(format!("{:.1} FPS", self.current_fps))
//                             .color(ACCENT_LIME)
//                             .monospace()
//                             .strong(),
//                     );
//                 });
//             });
//             ui.add_space(4.0);

//             // FPS Live Graph Canvas
//             inner_frame().show(ui, |ui| {
//                 let (response, painter) = ui.allocate_painter(
//                     egui::vec2(ui.available_width(), 100.0),
//                     egui::Sense::hover(),
//                 );
//                 let rect = response.rect;

//                 // Graph Background
//                 painter.rect_filled(rect, 2.0, BG_INNER);
//                 painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, BORDER_COLOR));

//                 // 60 FPS reference target line
//                 let target_y = rect.max.y - (60.0 / 75.0) * rect.height();
//                 painter.line_segment(
//                     [
//                         egui::pos2(rect.min.x, target_y),
//                         egui::pos2(rect.max.x, target_y),
//                     ],
//                     egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 70)),
//                 );

//                 // Build line points from self.fps_history
//                 let history_len = self.fps_history.len();
//                 if history_len > 1 {
//                     let mut points = Vec::with_capacity(history_len);
//                     let step_x = rect.width() / (history_len - 1) as f32;

//                     for (i, &fps_val) in self.fps_history.iter().enumerate() {
//                         let clamped_fps = fps_val.clamp(0.0, 75.0);
//                         let x = rect.min.x + i as f32 * step_x;
//                         let y = rect.max.y - (clamped_fps / 75.0) * (rect.height() - 8.0) - 4.0;
//                         points.push(egui::pos2(x, y));
//                     }

//                     // ─── Alleen de trace lijn (geen vulkleur meer) ───
//                     painter.add(egui::Shape::line(
//                         points,
//                         egui::Stroke::new(1.5, ACCENT_LIME), // Strakke, iets dunnere lijn
//                     ));
//                 }

//                 // X-axis time indicators (t-10s -> t)
//                 painter.text(
//                     egui::pos2(rect.min.x + 6.0, rect.max.y - 12.0),
//                     egui::Align2::LEFT_BOTTOM,
//                     "t - 5",
//                     egui::FontId::monospace(9.0),
//                     TEXT_MUTED,
//                 );
//                 painter.text(
//                     egui::pos2(rect.max.x - 6.0, rect.max.y - 12.0),
//                     egui::Align2::RIGHT_BOTTOM,
//                     "t",
//                     egui::FontId::monospace(9.0),
//                     TEXT_MUTED,
//                 );
//             });

//             ui.add_space(6.0);

//             // Performance metrics row below graph
//             ui.horizontal(|ui| {
//                 ui.spacing_mut().item_spacing.x = 16.0;

//                 // Packet loss
//                 ui.label(
//                     egui::RichText::new("packet loss :")
//                         .color(TEXT_MUTED)
//                         .monospace()
//                         .small(),
//                 );
//                 ui.label(
//                     egui::RichText::new("0.0%")
//                         .color(ACCENT_LIME)
//                         .monospace()
//                         .small(),
//                 );

//                 // Tijd (elapsed stream duration)
//                 ui.label(
//                     egui::RichText::new("tijd :")
//                         .color(TEXT_MUTED)
//                         .monospace()
//                         .small(),
//                 );
//                 let elapsed_str = match self.stream_start_time {
//                     Some(start) => {
//                         let secs = start.elapsed().unwrap_or_default().as_secs();
//                         format!("{:02}:{:02}", secs / 60, secs % 60)
//                     }
//                     None => "00:00".to_string(),
//                 };
//                 ui.label(
//                     egui::RichText::new(elapsed_str)
//                         .color(TEXT_PRIMARY)
//                         .monospace()
//                         .small(),
//                 );
//             });
//         });
//     }
// }

// // ─── Styling Helper Frames & Functions ─────────────────────────

// fn configure_style(ctx: &egui::Context) {
//     let mut style = (*ctx.style()).clone();

//     style.spacing.item_spacing = egui::vec2(8.0, 8.0);
//     style.spacing.button_padding = egui::vec2(12.0, 6.0);

//     // ─── Scherpere randen (Rounding) ───
//     // let sharp_rounding = egui::Rounding::same(2.0); // Strakke, moderne hoeken
//     // style.visuals.window_rounding = sharp_rounding;
//     // style.visuals.widgets.noninteractive.rounding = sharp_rounding;
//     // style.visuals.widgets.inactive.rounding = sharp_rounding;
//     // style.visuals.widgets.hovered.rounding = sharp_rounding;
//     // style.visuals.widgets.active.rounding = sharp_rounding;
//     // style.visuals.widgets.open.rounding = sharp_rounding;

//     style.visuals.dark_mode = true;
//     style.visuals.code_bg_color = BG_INNER;
//     style.visuals.override_text_color = Some(TEXT_PRIMARY);

//     // Inputs & DragValues
//     style.visuals.widgets.inactive.bg_fill = BG_INNER;
//     style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
//     style.visuals.widgets.hovered.bg_fill = BG_INNER;
//     style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ACCENT_LIME);
//     style.visuals.widgets.active.bg_fill = BG_INNER;
//     style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, ACCENT_LIME);

//     ctx.set_style(style);
// }

// fn panel_frame() -> egui::Frame {
//     egui::Frame {
//         fill: BG_PANEL,
//         // rounding: egui::Rounding::same(3.0), // Iets minder scherp voor de hoofdpanelen
//         inner_margin: egui::Margin::same(14.0),
//         stroke: egui::Stroke::new(1.0, BORDER_COLOR),
//         ..Default::default()
//     }
// }

// fn inner_frame() -> egui::Frame {
//     egui::Frame {
//         fill: BG_INNER,
//         // rounding: egui::Rounding::same(0.0), // Strakke binnenkaders
//         inner_margin: egui::Margin::same(10.0),
//         stroke: egui::Stroke::new(1.0, BORDER_COLOR),
//         ..Default::default()
//     }
// }

// fn set_button_style(ui: &mut egui::Ui, bg: egui::Color32, hover: egui::Color32, fg: egui::Color32) {
//     let style = ui.style_mut();
//     style.visuals.widgets.inactive.bg_fill = bg;
//     style.visuals.widgets.hovered.bg_fill = hover;
//     style.visuals.widgets.active.bg_fill = hover;
//     style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, fg);
//     style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, fg);
// }

// fn ghost_button(text: impl Into<String>, color: egui::Color32) -> impl egui::Widget {
//     egui::Button::new(
//         egui::RichText::new(text.into())
//             .color(color)
//             .monospace()
//             .small(),
//     )
//     .fill(egui::Color32::TRANSPARENT)
//     .stroke(egui::Stroke::NONE)
// }

// use crate::app::App;
// use eframe::egui;
// use std::time::Duration;

// // ─── Music App (Omatunes) Kleurenpalet ──────────────────────────
// const BG_MAIN: egui::Color32 = egui::Color32::from_rgb(20, 20, 28); // Donkere slate achtergrond
// const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(28, 28, 38); // Paneel achtergrond
// const BG_INNER: egui::Color32 = egui::Color32::from_rgb(15, 15, 22); // Terminal & Canvas achtergrond
// const BORDER_COLOR: egui::Color32 = egui::Color32::from_rgb(42, 42, 58); // Subtiele randen

// // Accenten uit de muziek-app screenshot (Neon Lime + Soft Indigo/Blue)
// const ACCENT_LIME: egui::Color32 = egui::Color32::from_rgb(255, 246, 224);
// const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
// const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(99, 102, 241); // Indigo Blauw (#6366f1)
// const DANGER_RED: egui::Color32 = egui::Color32::from_rgb(239, 68, 68); // Soft Rood (#ef4444)
// const DANGER_HOVER: egui::Color32 = egui::Color32::from_rgb(248, 113, 113);

// const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(241, 245, 249); // Heldere tekst
// const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(148, 163, 184); // Grijze subtekst

// impl eframe::App for App {
//     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//         configure_style(ctx);

//         // — Zorg dat de applicatie altijd bovenop zweegt (Pop-up modus) —
//         ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
//             egui::WindowLevel::AlwaysOnTop,
//         ));

//         // — Business logic tick —
//         self.tick();
//         if self.auto_refresh {
//             ctx.request_repaint_after(Duration::from_secs(1));
//         }

//         // — Check voor signal shutdown —
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

//         // ─── Layout Container ──────────────────────────────────────────
//         egui::CentralPanel::default()
//             .frame(
//                 egui::Frame::none()
//                     .fill(BG_MAIN)
//                     .inner_margin(egui::Margin::same(0.0)), // Geen marges rondom
//             )
//             .show(ctx, |ui| {
//                 ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0); // Geen ruimte tussen de cards

//                 // 1. Header (naam app)
//                 self.render_header(ui);

//                 // 2. Midden: Config (links vast) + Pos (rechts uitvullend)
//                 ui.horizontal(|ui| {
//                     self.render_config_card(ui);
//                     self.render_pos_card(ui);
//                 });

//                 // 3. Actieknoppen: [ Create ] [ Remove ] [ Start ] [ Stop ]
//                 self.render_action_bar(ui, stopping);
//             });
//     }

//     fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
//         self.shutdown();
//     }
// }

// // ─── UI Rendering Helpers ─────────────────────────────────────

// impl App {
//     /// Top Section: `naam app` (Header)
//     fn render_header(&mut self, ui: &mut egui::Ui) {
//         panel_frame().show(ui, |ui| {
//             ui.spacing_mut().item_spacing.x = 8.0; // Behoud some horizontal spacing inside
//             ui.set_width(ui.available_width());
//             ui.horizontal(|ui| {
//                 // Neon groen muziek-icon geïnspireerd badge
//                 let (rect, _) =
//                     ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
//                 ui.painter().rect_filled(rect, 2.0, ACCENT_LIME);
//                 ui.painter().text(
//                     rect.center(),
//                     egui::Align2::CENTER_CENTER,
//                     "♬",
//                     egui::FontId::proportional(16.0),
//                     BG_MAIN,
//                 );

//                 ui.add_space(6.0);
//                 ui.label(
//                     egui::RichText::new("naam app")
//                         .color(TEXT_PRIMARY)
//                         .size(18.0)
//                         .monospace()
//                         .strong(),
//                 );

//                 ui.label(
//                     egui::RichText::new("// hyprland display streamer")
//                         .color(TEXT_MUTED)
//                         .size(14.0)
//                         .monospace(),
//                 );

//                 ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
//                     let capturing = self.is_capturing();
//                     let (status_text, color) = if capturing {
//                         ("● STREAMING", ACCENT_LIME)
//                     } else if self.is_stopping() {
//                         ("⏳ STOPPING", ACCENT_BLUE)
//                     } else if self.monitor_exists {
//                         ("● ONLINE", ACCENT_BLUE)
//                     } else {
//                         ("○ OFFLINE", TEXT_MUTED)
//                     };

//                     ui.label(
//                         egui::RichText::new(status_text)
//                             .color(color)
//                             .size(14.0)
//                             .monospace()
//                             .strong(),
//                     );

//                     ui.add_space(12.0);
//                     if ui.add(ghost_button("🔄 refresh", ACCENT_LIME)).clicked() {
//                         self.refresh();
//                     }
//                 });
//             });
//         });
//     }

//     /// Top-Left Panel: `config`
//     // fn render_config_card(&mut self, ui: &mut egui::Ui) {
//     //     panel_frame().show(ui, |ui| {
//     //         ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0); // Spacing inside the card
//     //         ui.set_width(330.0); // Vaste logische breedte voor configuratie
//     //         ui.set_min_height(255.0);

//     //         ui.label(
//     //             egui::RichText::new("configuration")
//     //                 .color(ACCENT_LIME)
//     //                 .size(16.0)
//     //                 .monospace()
//     //                 .strong(),
//     //         );
//     //         ui.add_space(8.0);

//     //         egui::Grid::new("config_grid")
//     //             .num_columns(2)
//     //             .spacing([12.0, 10.0])
//     //             .show(ui, |ui| {
//     //                 ui.label(
//     //                     egui::RichText::new("identifier :")
//     //                         .color(TEXT_MUTED)
//     //                         .monospace()
//     //                         .size(14.0),
//     //                 );
//     //                 ui.horizontal(|ui| {
//     //                     ui.add_enabled_ui(!self.monitor_exists, |ui| {
//     //                         ui.add(
//     //                             egui::TextEdit::singleline(&mut self.config.name)
//     //                                 .desired_width(140.0)
//     //                                 .text_color(TEXT_PRIMARY),
//     //                         );
//     //                     });
//     //                     if self.monitor_exists {
//     //                         ui.label(
//     //                             egui::RichText::new("✓ active")
//     //                                 .color(ACCENT_LIME)
//     //                                 .monospace()
//     //                                 .size(14.0),
//     //                         );
//     //                     }
//     //                 });
//     //                 ui.end_row();

//     //                 // res :
//     //                 ui.label(
//     //                     egui::RichText::new("width :")
//     //                         .color(TEXT_MUTED)
//     //                         .monospace()
//     //                         .size(14.0),
//     //                 );
//     //                 ui.horizontal(|ui| {
//     //                     ui.add(
//     //                         egui::DragValue::new(&mut self.config.width)
//     //                             .range(320..=7680)
//     //                             .suffix(" px"),
//     //                     );
//     //                 });
//     //                 ui.end_row();
//     //                 ui.label(
//     //                     egui::RichText::new("height :")
//     //                         .color(TEXT_MUTED)
//     //                         .monospace()
//     //                         .size(14.0),
//     //                 );
//     //                 ui.horizontal(|ui| {
//     //                     ui.add(
//     //                         egui::DragValue::new(&mut self.config.height)
//     //                             .range(320..=7680)
//     //                             .suffix(" px"),
//     //                     );
//     //                 });
//     //                 ui.end_row();

//     //                 // frame rate :
//     //                 ui.label(
//     //                     egui::RichText::new("frame rate :")
//     //                         .color(TEXT_MUTED)
//     //                         .monospace()
//     //                         .size(14.0),
//     //                 );
//     //                 ui.add(
//     //                     egui::DragValue::new(&mut self.config.fps)
//     //                         .range(1..=240)
//     //                         .suffix(" Hz"),
//     //                 );
//     //                 ui.end_row();

//     //                 // scale :
//     //                 ui.label(
//     //                     egui::RichText::new("scale :")
//     //                         .color(TEXT_MUTED)
//     //                         .monospace()
//     //                         .size(14.0),
//     //                 );
//     //                 ui.add(
//     //                     egui::DragValue::new(&mut self.config.scale)
//     //                         .range(0.5f32..=3.0f32)
//     //                         .speed(0.1),
//     //                 );
//     //                 ui.end_row();
//     //             });
//     // });
//     // }

//     fn render_config_card(&mut self, ui: &mut egui::Ui) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(270.0);
//             ui.set_min_height(255.0);

//             ui.label(
//                 egui::RichText::new("configuration")
//                     .color(ACCENT_LIME)
//                     .size(16.0)
//                     .monospace()
//                     .strong(),
//             );
//             ui.end_row();

//             egui::Grid::new("config_grid")
//                 .num_columns(2)
//                 .spacing([12.0, 10.0])
//                 .show(ui, |ui| {
//                     ui.label(
//                         egui::RichText::new("identifier :")
//                             .color(TEXT_MUTED)
//                             .monospace(),
//                     );
//                     ui.horizontal(|ui| {
//                         ui.add_enabled_ui(!self.monitor_exists, |ui| {
//                             ui.add(
//                                 egui::TextEdit::singleline(&mut self.config.name)
//                                     .desired_width(120.0),
//                             );
//                         });
//                         if self.monitor_exists {
//                             ui.label(
//                                 egui::RichText::new("✓ active")
//                                     .color(ACCENT_LIME)
//                                     .monospace()
//                                     .small(),
//                             );
//                         }
//                     });
//                     ui.end_row();

//                     // res :
//                     ui.label(egui::RichText::new("width :").color(TEXT_MUTED).monospace());
//                     ui.horizontal(|ui| {
//                         ui.add(
//                             egui::DragValue::new(&mut self.config.width)
//                                 .range(320..=7680)
//                                 .suffix(" px"),
//                         );
//                     });
//                     ui.end_row();
//                     ui.label(
//                         egui::RichText::new("height :")
//                             .color(TEXT_MUTED)
//                             .monospace(),
//                     );
//                     ui.horizontal(|ui| {
//                         ui.add(
//                             egui::DragValue::new(&mut self.config.height)
//                                 .range(320..=7680)
//                                 .suffix(" px"),
//                         );
//                     });
//                     ui.end_row();

//                     // frame rate :
//                     ui.label(
//                         egui::RichText::new("frame rate :")
//                             .color(TEXT_MUTED)
//                             .monospace(),
//                     );
//                     ui.add(
//                         egui::DragValue::new(&mut self.config.fps)
//                             .range(1..=240)
//                             .suffix(" Hz"),
//                     );
//                     ui.end_row();

//                     // scale :
//                     ui.label(egui::RichText::new("scale :").color(TEXT_MUTED).monospace());
//                     ui.add(
//                         egui::DragValue::new(&mut self.config.scale)
//                             .range(0.5f32..=3.0f32)
//                             .speed(0.1),
//                     );
//                     ui.end_row();
//                 });
//             ui.add_space(5.0);
//             inner_frame().show(ui, |ui| {
//                 ui.set_width(ui.available_width());
//                 ui.monospace(
//                     egui::RichText::new(format!("$ {}", self.config.to_keyword()))
//                         .color(ACCENT_LIME)
//                         .size(11.0),
//                 );
//             });
//         });
//     }

//     /// Top-Right Panel: `pos` (Vult de volledige resterende breedte)
//     fn render_pos_card(&mut self, ui: &mut egui::Ui) {
//         let available_width = ui.available_width();

//         ui.allocate_ui_with_layout(
//             egui::vec2(available_width, 255.0),
//             egui::Layout::top_down(egui::Align::LEFT),
//             |ui| {
//                 panel_frame().show(ui, |ui| {
//                     ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0); // Spacing inside
//                     ui.set_min_height(255.0);
//                     ui.set_min_width(ui.available_width());

//                     ui.horizontal(|ui| {
//                         ui.label(
//                             egui::RichText::new("position")
//                                 .color(ACCENT_LIME)
//                                 .size(16.0)
//                                 .monospace()
//                                 .strong(),
//                         );
//                         ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
//                             ui.label(
//                                 (egui::RichText::new(format!(
//                                     "x: {}  y: {}",
//                                     self.config.x, self.config.y
//                                 ))
//                                 .color(ACCENT_LIME))
//                                 .monospace()
//                                 .size(16.0),
//                             );
//                         });
//                     });
//                     ui.add_space(4.0);

//                     inner_frame().show(ui, |ui| {
//                         ui.set_min_height(ui.available_height());
//                         ui.set_min_width(ui.available_width());

//                         let canvas_size = egui::vec2(ui.available_width(), ui.available_height());
//                         let (response, painter) =
//                             ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());

//                         let canvas_rect = response.rect;

//                         // --- nwg-displays logica ---

//                         let scale = 0.065; // 1 screen pixel = 0.065 canvas pixels (schaal)

//                         // Hoofdmonitor (we gaan uit van een 1920x1080 scherm op 0,0)
//                         let main_w = 1920.0;
//                         let main_h = 1080.0;
//                         let main_x = 0.0;
//                         let main_y = 0.0;

//                         // Virtuele monitor eigenschappen uit de config
//                         let virt_w = self.config.width as f32;
//                         let virt_h = self.config.height as f32;

//                         // --- Sleep & Raster Logica (1:1 beweging, 90px raster bij loslaten) ---

//                         if response.drag_started() {
//                             // Sla de startpositie op als float om precisie te behouden tijdens het slepen
//                             ui.memory_mut(|m| {
//                                 m.data
//                                     .insert_temp(egui::Id::new("virt_raw_x"), self.config.x as f32);
//                                 m.data
//                                     .insert_temp(egui::Id::new("virt_raw_y"), self.config.y as f32);
//                             });
//                         }

//                         if response.dragged() {
//                             let delta = response.drag_delta();

//                             // Haal de raw float posities op en update ze met de cursor delta
//                             let raw_x = ui.memory_mut(|m| {
//                                 let val = m.data.get_temp_mut_or(
//                                     egui::Id::new("virt_raw_x"),
//                                     self.config.x as f32,
//                                 );
//                                 *val += delta.x / scale;
//                                 *val
//                             });

//                             let raw_y = ui.memory_mut(|m| {
//                                 let val = m.data.get_temp_mut_or(
//                                     egui::Id::new("virt_raw_y"),
//                                     self.config.y as f32,
//                                 );
//                                 *val += delta.y / scale;
//                                 *val
//                             });

//                             // Update de monitor positie 1:1 met de cursor (zonder afronding)
//                             self.config.x = raw_x as i32;
//                             self.config.y = raw_y as i32;
//                         }

//                         if response.drag_stopped() {
//                             // Discretiseer naar 90px raster bij het loslaten
//                             let grid_step = 90;
//                             self.config.x = ((self.config.x as f32 / grid_step as f32).round()
//                                 * grid_step as f32)
//                                 as i32;
//                             self.config.y = ((self.config.y as f32 / grid_step as f32).round()
//                                 * grid_step as f32)
//                                 as i32;

//                             // Reset memory naar de afgeronde waarde
//                             ui.memory_mut(|m| {
//                                 m.data
//                                     .insert_temp(egui::Id::new("virt_raw_x"), self.config.x as f32);
//                                 m.data
//                                     .insert_temp(egui::Id::new("virt_raw_y"), self.config.y as f32);
//                             });
//                         }

//                         // --- Teken Logica ---

//                         painter.rect_filled(canvas_rect, 2.0, BG_INNER);
//                         painter.rect_stroke(canvas_rect, 2.0, egui::Stroke::new(1.0, BORDER_COLOR));

//                         let center_x = canvas_rect.center().x;
//                         let center_y = canvas_rect.center().y;

//                         // Bepaal het nulpunt op het canvas (zodat het hoofd-scherm min of meer in het midden ligt)
//                         let origin_x = center_x - (main_w / 2.0) * scale;
//                         let origin_y = center_y - (main_h / 2.0) * scale;

//                         // Hulpfunctie: Vertaal scherm-coördinaten naar canvas-punten
//                         let to_canvas = |sx: f32, sy: f32| -> egui::Pos2 {
//                             egui::pos2(origin_x + sx * scale, origin_y + sy * scale)
//                         };

//                         // --- Teken het 90px Raster ---
//                         let grid_step_canvas = 90.0 * scale;

//                         // Verticale lijnen
//                         let mut grid_x = origin_x;
//                         while grid_x > canvas_rect.min.x {
//                             grid_x -= grid_step_canvas;
//                         }
//                         while grid_x < canvas_rect.max.x {
//                             painter.line_segment(
//                                 [
//                                     egui::pos2(grid_x, canvas_rect.min.y),
//                                     egui::pos2(grid_x, canvas_rect.max.y),
//                                 ],
//                                 egui::Stroke::new(0.5, egui::Color32::from_rgb(25, 25, 35)), // Zeer subtiel
//                             );
//                             grid_x += grid_step_canvas;
//                         }

//                         // Horizontale lijnen
//                         let mut grid_y = origin_y;
//                         while grid_y > canvas_rect.min.y {
//                             grid_y -= grid_step_canvas;
//                         }
//                         while grid_y < canvas_rect.max.y {
//                             painter.line_segment(
//                                 [
//                                     egui::pos2(canvas_rect.min.x, grid_y),
//                                     egui::pos2(canvas_rect.max.x, grid_y),
//                                 ],
//                                 egui::Stroke::new(0.5, egui::Color32::from_rgb(25, 25, 35)), // Zeer subtiel
//                             );
//                             grid_y += grid_step_canvas;
//                         }

//                         // 1. Teken hoofdmonitor (vanaf zijn linkerbovenhoek)
//                         let main_top_left = to_canvas(main_x, main_y);
//                         let main_rect = egui::Rect::from_min_size(
//                             main_top_left,
//                             egui::vec2(main_w * scale, main_h * scale),
//                         );

//                         painter.rect_filled(main_rect, 2.0, egui::Color32::from_rgb(35, 42, 60));
//                         painter.rect_stroke(main_rect, 2.0, egui::Stroke::new(1.5, ACCENT_BLUE));
//                         painter.text(
//                             main_rect.center(),
//                             egui::Align2::CENTER_CENTER,
//                             "main\n(DP-1)",
//                             egui::FontId::monospace(12.0),
//                             TEXT_PRIMARY,
//                         );

//                         // 2. Teken virtuele monitor (vanaf zijn linkerbovenhoek)
//                         // Let op: x en y zijn hier absolute schermcoördinaten
//                         let virt_top_left = to_canvas(
//                             self.config.x as f32 - self.config.width as f32,
//                             self.config.y as f32,
//                         );

//                         let virt_rect = egui::Rect::from_min_size(
//                             virt_top_left,
//                             egui::vec2(virt_w * scale, virt_h * scale),
//                         );

//                         let (fill_col, stroke_col) = if self.monitor_exists {
//                             (
//                                 egui::Color32::from_rgba_premultiplied(132, 204, 22, 50),
//                                 ACCENT_LIME,
//                             )
//                         } else {
//                             (
//                                 egui::Color32::from_rgba_premultiplied(148, 163, 184, 25),
//                                 TEXT_MUTED,
//                             )
//                         };

//                         let is_grabbed = response.dragged();

//                         let actual_stroke = if is_grabbed {
//                             egui::Stroke::new(2.5, ACCENT_HOVER)
//                         } else {
//                             egui::Stroke::new(1.5, stroke_col)
//                         };

//                         painter.rect_filled(virt_rect, 2.0, fill_col);
//                         painter.rect_stroke(virt_rect, 2.0, actual_stroke);
//                         painter.text(
//                             virt_rect.center(),
//                             egui::Align2::CENTER_CENTER,
//                             &format!(
//                                 "{}\n{}x{}",
//                                 self.config.name, self.config.width, self.config.height
//                             ),
//                             egui::FontId::monospace(12.0),
//                             if self.monitor_exists {
//                                 ACCENT_LIME
//                             } else {
//                                 TEXT_MUTED
//                             },
//                         );

//                         // Instructie label
//                         painter.text(
//                             egui::pos2(canvas_rect.min.x + 6.0, canvas_rect.max.y - 6.0),
//                             egui::Align2::LEFT_BOTTOM,
//                             "↔ drag to position (snaps to 90px grid)",
//                             egui::FontId::monospace(12.0),
//                             TEXT_MUTED,
//                         );
//                     });
//                 });
//             },
//         );
//     }

//     /// Center Action Bar: `create` | `remove` | `start` | `stop`
//     fn render_action_bar(&mut self, ui: &mut egui::Ui, stopping: bool) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(ui.available_width());
//             ui.set_min_height(50.0);

//             let can_create = !self.monitor_exists && !self.config.name.is_empty();
//             let can_remove = self.monitor_exists && !self.is_capturing();
//             let can_start = self.monitor_exists && !self.is_capturing() && !stopping;
//             let can_stop = self.is_capturing() && !stopping;

//             ui.columns(4, |cols| {
//                 // Button 1: create
//                 cols[0].scope(|ui| {
//                     set_button_style(ui, ACCENT_LIME, ACCENT_HOVER, BG_MAIN);
//                     let button_text = if self.monitor_exists {
//                         "Update Config"
//                     } else {
//                         "Create Monitor"
//                     };

//                     if ui
//                         .add_enabled(
//                             can_create,
//                             egui::Button::new(button_text)
//                                 .min_size(egui::vec2(ui.available_width(), 36.0)),
//                         )
//                         .clicked()
//                     {
//                         self.apply_config();
//                     }
//                 });

//                 // Button 2: remove
//                 cols[1].scope(|ui| {
//                     set_button_style(ui, DANGER_RED, DANGER_HOVER, TEXT_PRIMARY);
//                     if ui
//                         .add_enabled(
//                             can_remove,
//                             egui::Button::new("remove")
//                                 .min_size(egui::vec2(ui.available_width(), 36.0)),
//                         )
//                         .clicked()
//                     {
//                         self.do_remove();
//                     }
//                 });

//                 // Button 3: start
//                 cols[2].scope(|ui| {
//                     set_button_style(
//                         ui,
//                         ACCENT_BLUE,
//                         egui::Color32::from_rgb(129, 140, 248),
//                         TEXT_PRIMARY,
//                     );
//                     if ui
//                         .add_enabled(
//                             can_start,
//                             egui::Button::new("start")
//                                 .min_size(egui::vec2(ui.available_width(), 36.0)),
//                         )
//                         .clicked()
//                     {
//                         self.do_start_capture();
//                     }
//                 });

//                 // Button 4: stop
//                 cols[3].scope(|ui| {
//                     set_button_style(ui, DANGER_RED, DANGER_HOVER, TEXT_PRIMARY);
//                     if ui
//                         .add_enabled(
//                             can_stop,
//                             egui::Button::new("stop")
//                                 .min_size(egui::vec2(ui.available_width(), 36.0)),
//                         )
//                         .clicked()
//                     {
//                         self.do_stop_capture();
//                     }
//                 });
//             });
//         });
//     }
// }

// // ─── Styling Helper Frames & Functions ─────────────────────────

// fn configure_style(ctx: &egui::Context) {
//     let mut style = (*ctx.style()).clone();

//     style.spacing.item_spacing = egui::vec2(8.0, 8.0);
//     style.spacing.button_padding = egui::vec2(12.0, 8.0);

//     // ─── Scherpere randen (Rounding) ───
//     let sharp_rounding = egui::Rounding::same(2.0);
//     style.visuals.window_rounding = sharp_rounding;
//     style.visuals.widgets.noninteractive.rounding = sharp_rounding;
//     style.visuals.widgets.inactive.rounding = sharp_rounding;
//     style.visuals.widgets.hovered.rounding = sharp_rounding;
//     style.visuals.widgets.active.rounding = sharp_rounding;
//     style.visuals.widgets.open.rounding = sharp_rounding;

//     style.visuals.dark_mode = true;
//     style.visuals.code_bg_color = BG_INNER;
//     style.visuals.override_text_color = Some(TEXT_PRIMARY);

//     // Globale tekstgroottes
//     style.text_styles = [
//         (egui::TextStyle::Body, egui::FontId::monospace(14.0)),
//         (egui::TextStyle::Button, egui::FontId::monospace(14.0)),
//         (egui::TextStyle::Small, egui::FontId::monospace(13.0)),
//         (egui::TextStyle::Monospace, egui::FontId::monospace(14.0)),
//     ]
//     .into();

//     // Inputs & DragValues
//     style.visuals.widgets.inactive.bg_fill = BG_INNER;
//     style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
//     style.visuals.widgets.hovered.bg_fill = BG_INNER;
//     style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ACCENT_LIME);
//     style.visuals.widgets.active.bg_fill = BG_INNER;
//     style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, ACCENT_LIME);

//     ctx.set_style(style);
// }

// fn panel_frame() -> egui::Frame {
//     egui::Frame {
//         fill: BG_PANEL,
//         inner_margin: egui::Margin::same(14.0),
//         stroke: egui::Stroke::new(1.0, BORDER_COLOR), // Behoud randen
//         ..Default::default()
//     }
// }

// fn inner_frame() -> egui::Frame {
//     egui::Frame {
//         fill: BG_INNER,
//         inner_margin: egui::Margin::same(10.0),
//         stroke: egui::Stroke::new(1.0, BORDER_COLOR),
//         ..Default::default()
//     }
// }

// fn set_button_style(ui: &mut egui::Ui, bg: egui::Color32, hover: egui::Color32, fg: egui::Color32) {
//     let style = ui.style_mut();
//     style.visuals.widgets.inactive.bg_fill = bg;
//     style.visuals.widgets.hovered.bg_fill = hover;
//     style.visuals.widgets.active.bg_fill = hover;
//     style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, fg);
//     style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, fg);
// }

// fn ghost_button(text: impl Into<String>, color: egui::Color32) -> impl egui::Widget {
//     egui::Button::new(
//         egui::RichText::new(text.into())
//             .color(color)
//             .monospace()
//             .size(14.0),
//     )
//     .fill(egui::Color32::TRANSPARENT)
//     .stroke(egui::Stroke::NONE)
// }

// use crate::app::App;
// use eframe::egui;
// use std::time::Duration;

// // ─── Kleurenpalet ──────────────────────────────────────────────
// const BG_MAIN: egui::Color32 = egui::Color32::from_rgb(18, 18, 24);
// const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(26, 26, 36);
// const BG_INNER: egui::Color32 = egui::Color32::from_rgb(14, 14, 20);
// const BORDER_COLOR: egui::Color32 = egui::Color32::from_rgb(40, 40, 56);

// const ACCENT_LIME: egui::Color32 = egui::Color32::from_rgb(255, 246, 224);
// const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
// const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(99, 102, 241);
// const DANGER_RED: egui::Color32 = egui::Color32::from_rgb(239, 68, 68);
// const DANGER_HOVER: egui::Color32 = egui::Color32::from_rgb(248, 113, 113);

// const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(241, 245, 249);
// const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(148, 163, 184);

// impl eframe::App for App {
//     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//         configure_style(ctx);

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

//         // 1. Fixed Header Boven
//         egui::TopBottomPanel::top("header_panel")
//             .frame(egui::Frame::none().fill(BG_MAIN).inner_margin(8.0))
//             .show(ctx, |ui| {
//                 self.render_header(ui);
//             });

//         // 2. Fixed Action Bar Onder
//         egui::TopBottomPanel::bottom("action_bar_panel")
//             .frame(egui::Frame::none().fill(BG_MAIN).inner_margin(8.0))
//             .show(ctx, |ui| {
//                 self.render_action_bar(ui, stopping);
//             });

//         // 3. Flexibele Middenzone (Config + Pos)
//         egui::CentralPanel::default()
//             .frame(
//                 egui::Frame::none()
//                     .fill(BG_MAIN)
//                     .inner_margin(egui::Margin::symmetric(8.0, 0.0)),
//             )
//             .show(ctx, |ui| {
//                 ui.horizontal(|ui| {
//                     // Config blijft vaste breedte
//                     self.render_config_card(ui);

//                     ui.add_space(8.0);

//                     // Position neemt ALLE overgebleven hoogte & breedte in
//                     self.render_pos_card(ui);
//                 });
//             });
//     }

//     fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
//         self.shutdown();
//     }
// }

// impl App {
//     /// Header
//     fn render_header(&mut self, ui: &mut egui::Ui) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(ui.available_width());
//             ui.horizontal(|ui| {
//                 let (rect, _) =
//                     ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
//                 ui.painter().rect_filled(rect, 2.0, ACCENT_LIME);
//                 ui.painter().text(
//                     rect.center(),
//                     egui::Align2::CENTER_CENTER,
//                     "♬",
//                     egui::FontId::proportional(14.0),
//                     BG_MAIN,
//                 );

//                 ui.add_space(4.0);
//                 ui.label(
//                     egui::RichText::new("naam app")
//                         .color(TEXT_PRIMARY)
//                         .size(15.0)
//                         .monospace()
//                         .strong(),
//                 );

//                 ui.label(
//                     egui::RichText::new("// hyprland display streamer")
//                         .color(TEXT_MUTED)
//                         .size(12.0)
//                         .monospace(),
//                 );

//                 ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
//                     let capturing = self.is_capturing();
//                     let (status_text, color) = if capturing {
//                         ("● STREAMING", ACCENT_LIME)
//                     } else if self.is_stopping() {
//                         ("⏳ STOPPING", ACCENT_BLUE)
//                     } else if self.monitor_exists {
//                         ("● ONLINE", ACCENT_BLUE)
//                     } else {
//                         ("○ OFFLINE", TEXT_MUTED)
//                     };

//                     ui.label(
//                         egui::RichText::new(status_text)
//                             .color(color)
//                             .size(12.0)
//                             .monospace()
//                             .strong(),
//                     );

//                     ui.add_space(10.0);
//                     if ui.add(ghost_button("🔄 refresh", ACCENT_LIME)).clicked() {
//                         self.refresh();
//                     }
//                 });
//             });
//         });
//     }

//     /// Config Card (Schoon verticaal, zonder $ {} commando)
//     fn render_config_card(&mut self, ui: &mut egui::Ui) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(260.0);
//             ui.set_height(ui.available_height());

//             ui.vertical(|ui| {
//                 ui.label(
//                     egui::RichText::new("configuration")
//                         .color(ACCENT_LIME)
//                         .size(15.0)
//                         .monospace()
//                         .strong(),
//                 );
//                 ui.add_space(10.0);

//                 egui::Grid::new("config_grid")
//                     .num_columns(2)
//                     .spacing([12.0, 12.0])
//                     .show(ui, |ui| {
//                         // Identifier
//                         ui.label(
//                             egui::RichText::new("identifier :")
//                                 .color(TEXT_MUTED)
//                                 .monospace(),
//                         );
//                         ui.horizontal(|ui| {
//                             ui.add_enabled_ui(!self.monitor_exists, |ui| {
//                                 ui.add(
//                                     egui::TextEdit::singleline(&mut self.config.name)
//                                         .desired_width(100.0),
//                                 );
//                             });
//                             if self.monitor_exists {
//                                 ui.label(egui::RichText::new("✓").color(ACCENT_LIME).monospace());
//                             }
//                         });
//                         ui.end_row();

//                         // Width
//                         ui.label(egui::RichText::new("width :").color(TEXT_MUTED).monospace());
//                         ui.add(
//                             egui::DragValue::new(&mut self.config.width)
//                                 .range(320..=7680)
//                                 .suffix(" px"),
//                         );
//                         ui.end_row();

//                         // Height
//                         ui.label(
//                             egui::RichText::new("height :")
//                                 .color(TEXT_MUTED)
//                                 .monospace(),
//                         );
//                         ui.add(
//                             egui::DragValue::new(&mut self.config.height)
//                                 .range(320..=7680)
//                                 .suffix(" px"),
//                         );
//                         ui.end_row();

//                         // Frame Rate
//                         ui.label(
//                             egui::RichText::new("frame rate :")
//                                 .color(TEXT_MUTED)
//                                 .monospace(),
//                         );
//                         ui.add(
//                             egui::DragValue::new(&mut self.config.fps)
//                                 .range(1..=240)
//                                 .suffix(" Hz"),
//                         );
//                         ui.end_row();

//                         // Scale
//                         ui.label(egui::RichText::new("scale :").color(TEXT_MUTED).monospace());
//                         ui.add(
//                             egui::DragValue::new(&mut self.config.scale)
//                                 .range(0.5f32..=3.0f32)
//                                 .speed(0.1),
//                         );
//                         ui.end_row();
//                     });
//             });
//         });
//     }

//     /// Position Card (Vult alle resterende hoogte & breedte volledig uit)
//     fn render_pos_card(&mut self, ui: &mut egui::Ui) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(ui.available_width());
//             ui.set_height(ui.available_height());

//             ui.vertical(|ui| {
//                 ui.horizontal(|ui| {
//                     ui.label(
//                         egui::RichText::new("position")
//                             .color(ACCENT_LIME)
//                             .size(15.0)
//                             .monospace()
//                             .strong(),
//                     );
//                     ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
//                         ui.label(
//                             egui::RichText::new(format!(
//                                 "x: {}  y: {}",
//                                 self.config.x, self.config.y
//                             ))
//                             .color(ACCENT_LIME)
//                             .monospace()
//                             .size(14.0),
//                         );
//                     });
//                 });
//                 ui.add_space(6.0);

//                 // Dynamic Canvas Frame
//                 inner_frame().show(ui, |ui| {
//                     let canvas_size = ui.available_size();
//                     let (response, painter) =
//                         ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());
//                     let canvas_rect = response.rect;

//                     let scale = 0.065;
//                     let main_w = 1920.0;
//                     let main_h = 1080.0;
//                     let main_x = 0.0;
//                     let main_y = 0.0;

//                     let virt_w = self.config.width as f32;
//                     let virt_h = self.config.height as f32;

//                     // Dragging Logic
//                     if response.drag_started() {
//                         ui.memory_mut(|m| {
//                             m.data
//                                 .insert_temp(egui::Id::new("virt_raw_x"), self.config.x as f32);
//                             m.data
//                                 .insert_temp(egui::Id::new("virt_raw_y"), self.config.y as f32);
//                         });
//                     }

//                     if response.dragged() {
//                         let delta = response.drag_delta();
//                         let raw_x = ui.memory_mut(|m| {
//                             let val = m
//                                 .data
//                                 .get_temp_mut_or(egui::Id::new("virt_raw_x"), self.config.x as f32);
//                             *val += delta.x / scale;
//                             *val
//                         });

//                         let raw_y = ui.memory_mut(|m| {
//                             let val = m
//                                 .data
//                                 .get_temp_mut_or(egui::Id::new("virt_raw_y"), self.config.y as f32);
//                             *val += delta.y / scale;
//                             *val
//                         });

//                         self.config.x = raw_x as i32;
//                         self.config.y = raw_y as i32;
//                     }

//                     if response.drag_stopped() {
//                         let grid_step = 90;
//                         self.config.x = ((self.config.x as f32 / grid_step as f32).round()
//                             * grid_step as f32) as i32;
//                         self.config.y = ((self.config.y as f32 / grid_step as f32).round()
//                             * grid_step as f32) as i32;

//                         ui.memory_mut(|m| {
//                             m.data
//                                 .insert_temp(egui::Id::new("virt_raw_x"), self.config.x as f32);
//                             m.data
//                                 .insert_temp(egui::Id::new("virt_raw_y"), self.config.y as f32);
//                         });
//                     }

//                     // Drawing
//                     painter.rect_filled(canvas_rect, 2.0, BG_INNER);
//                     painter.rect_stroke(canvas_rect, 2.0, egui::Stroke::new(1.0, BORDER_COLOR));

//                     let center_x = canvas_rect.center().x;
//                     let center_y = canvas_rect.center().y;

//                     let origin_x = center_x - (main_w / 2.0) * scale;
//                     let origin_y = center_y - (main_h / 2.0) * scale;

//                     let to_canvas = |sx: f32, sy: f32| -> egui::Pos2 {
//                         egui::pos2(origin_x + sx * scale, origin_y + sy * scale)
//                     };

//                     // Grid
//                     let grid_step_canvas = 90.0 * scale;
//                     let mut grid_x = origin_x;
//                     while grid_x > canvas_rect.min.x {
//                         grid_x -= grid_step_canvas;
//                     }
//                     while grid_x < canvas_rect.max.x {
//                         painter.line_segment(
//                             [
//                                 egui::pos2(grid_x, canvas_rect.min.y),
//                                 egui::pos2(grid_x, canvas_rect.max.y),
//                             ],
//                             egui::Stroke::new(0.5, egui::Color32::from_rgb(25, 25, 35)),
//                         );
//                         grid_x += grid_step_canvas;
//                     }

//                     let mut grid_y = origin_y;
//                     while grid_y > canvas_rect.min.y {
//                         grid_y -= grid_step_canvas;
//                     }
//                     while grid_y < canvas_rect.max.y {
//                         painter.line_segment(
//                             [
//                                 egui::pos2(canvas_rect.min.x, grid_y),
//                                 egui::pos2(canvas_rect.max.x, grid_y),
//                             ],
//                             egui::Stroke::new(0.5, egui::Color32::from_rgb(25, 25, 35)),
//                         );
//                         grid_y += grid_step_canvas;
//                     }

//                     // Main Monitor
//                     let main_top_left = to_canvas(main_x, main_y);
//                     let main_rect = egui::Rect::from_min_size(
//                         main_top_left,
//                         egui::vec2(main_w * scale, main_h * scale),
//                     );

//                     painter.rect_filled(main_rect, 2.0, egui::Color32::from_rgb(32, 38, 54));
//                     painter.rect_stroke(main_rect, 2.0, egui::Stroke::new(1.2, ACCENT_BLUE));
//                     painter.text(
//                         main_rect.center(),
//                         egui::Align2::CENTER_CENTER,
//                         "main\n(DP-1)",
//                         egui::FontId::monospace(10.0),
//                         TEXT_PRIMARY,
//                     );

//                     // Virtual Monitor
//                     let virt_top_left = to_canvas(
//                         self.config.x as f32 - self.config.width as f32,
//                         self.config.y as f32,
//                     );

//                     let virt_rect = egui::Rect::from_min_size(
//                         virt_top_left,
//                         egui::vec2(virt_w * scale, virt_h * scale),
//                     );

//                     let (fill_col, stroke_col) = if self.monitor_exists {
//                         (
//                             egui::Color32::from_rgba_premultiplied(132, 204, 22, 40),
//                             ACCENT_LIME,
//                         )
//                     } else {
//                         (
//                             egui::Color32::from_rgba_premultiplied(148, 163, 184, 20),
//                             TEXT_MUTED,
//                         )
//                     };

//                     let is_grabbed = response.dragged();
//                     let actual_stroke = if is_grabbed {
//                         egui::Stroke::new(2.0, ACCENT_HOVER)
//                     } else {
//                         egui::Stroke::new(1.2, stroke_col)
//                     };

//                     painter.rect_filled(virt_rect, 2.0, fill_col);
//                     painter.rect_stroke(virt_rect, 2.0, actual_stroke);
//                     painter.text(
//                         virt_rect.center(),
//                         egui::Align2::CENTER_CENTER,
//                         &format!(
//                             "{}\n{}x{}",
//                             self.config.name, self.config.width, self.config.height
//                         ),
//                         egui::FontId::monospace(10.0),
//                         if self.monitor_exists {
//                             ACCENT_LIME
//                         } else {
//                             TEXT_MUTED
//                         },
//                     );

//                     // Help text
//                     painter.text(
//                         egui::pos2(canvas_rect.min.x + 8.0, canvas_rect.max.y - 8.0),
//                         egui::Align2::LEFT_BOTTOM,
//                         "↔ drag to position (snaps to 90px grid)",
//                         egui::FontId::monospace(10.0),
//                         TEXT_MUTED,
//                     );
//                 });
//             });
//         });
//     }

//     /// Action Bar Bottom
//     fn render_action_bar(&mut self, ui: &mut egui::Ui, stopping: bool) {
//         panel_frame().show(ui, |ui| {
//             ui.set_width(ui.available_width());

//             let can_create = !self.monitor_exists && !self.config.name.is_empty();
//             let can_remove = self.monitor_exists && !self.is_capturing();
//             let can_start = self.monitor_exists && !self.is_capturing() && !stopping;
//             let can_stop = self.is_capturing() && !stopping;

//             ui.columns(4, |cols| {
//                 cols[0].scope(|ui| {
//                     set_button_style(ui, ACCENT_LIME, ACCENT_HOVER, BG_MAIN);
//                     let button_text = if self.monitor_exists {
//                         "Update Config"
//                     } else {
//                         "Create Monitor"
//                     };
//                     if ui
//                         .add_enabled(
//                             can_create,
//                             egui::Button::new(button_text)
//                                 .min_size(egui::vec2(ui.available_width(), 32.0)),
//                         )
//                         .clicked()
//                     {
//                         self.apply_config();
//                     }
//                 });

//                 cols[1].scope(|ui| {
//                     set_button_style(ui, DANGER_RED, DANGER_HOVER, TEXT_PRIMARY);
//                     if ui
//                         .add_enabled(
//                             can_remove,
//                             egui::Button::new("remove")
//                                 .min_size(egui::vec2(ui.available_width(), 32.0)),
//                         )
//                         .clicked()
//                     {
//                         self.do_remove();
//                     }
//                 });

//                 cols[2].scope(|ui| {
//                     set_button_style(
//                         ui,
//                         ACCENT_BLUE,
//                         egui::Color32::from_rgb(129, 140, 248),
//                         TEXT_PRIMARY,
//                     );
//                     if ui
//                         .add_enabled(
//                             can_start,
//                             egui::Button::new("start")
//                                 .min_size(egui::vec2(ui.available_width(), 32.0)),
//                         )
//                         .clicked()
//                     {
//                         self.do_start_capture();
//                     }
//                 });

//                 cols[3].scope(|ui| {
//                     set_button_style(ui, DANGER_RED, DANGER_HOVER, TEXT_PRIMARY);
//                     if ui
//                         .add_enabled(
//                             can_stop,
//                             egui::Button::new("stop")
//                                 .min_size(egui::vec2(ui.available_width(), 32.0)),
//                         )
//                         .clicked()
//                     {
//                         self.do_stop_capture();
//                     }
//                 });
//             });
//         });
//     }
// }

// // ─── Styling Helpers ───────────────────────────────────────────

// fn configure_style(ctx: &egui::Context) {
//     let mut style = (*ctx.style()).clone();

//     style.spacing.item_spacing = egui::vec2(8.0, 8.0);
//     style.spacing.button_padding = egui::vec2(10.0, 5.0);

//     let rounding = egui::Rounding::same(2.0);
//     style.visuals.window_rounding = rounding;
//     style.visuals.widgets.noninteractive.rounding = rounding;
//     style.visuals.widgets.inactive.rounding = rounding;
//     style.visuals.widgets.hovered.rounding = rounding;
//     style.visuals.widgets.active.rounding = rounding;

//     style.visuals.dark_mode = true;
//     style.visuals.code_bg_color = BG_INNER;
//     style.visuals.override_text_color = Some(TEXT_PRIMARY);

//     style.visuals.widgets.inactive.bg_fill = BG_INNER;
//     style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
//     style.visuals.widgets.hovered.bg_fill = BG_INNER;
//     style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ACCENT_LIME);
//     style.visuals.widgets.active.bg_fill = BG_INNER;
//     style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, ACCENT_LIME);

//     ctx.set_style(style);
// }

// fn panel_frame() -> egui::Frame {
//     egui::Frame {
//         fill: BG_PANEL,
//         inner_margin: egui::Margin::same(10.0),
//         stroke: egui::Stroke::new(1.0, BORDER_COLOR),
//         ..Default::default()
//     }
// }

// fn inner_frame() -> egui::Frame {
//     egui::Frame {
//         fill: BG_INNER,
//         inner_margin: egui::Margin::same(8.0),
//         stroke: egui::Stroke::new(1.0, BORDER_COLOR),
//         ..Default::default()
//     }
// }

// fn set_button_style(ui: &mut egui::Ui, bg: egui::Color32, hover: egui::Color32, fg: egui::Color32) {
//     let style = ui.style_mut();
//     style.visuals.widgets.inactive.bg_fill = bg;
//     style.visuals.widgets.hovered.bg_fill = hover;
//     style.visuals.widgets.active.bg_fill = hover;
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

use crate::app::App;
use eframe::egui;
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
            .frame(egui::Frame::none().fill(BG_MAIN).inner_margin(8.0))
            .show(ctx, |ui| {
                self.render_header(ui);
            });

        // 2. Middengebied: Left Panel (Config + Buttons) & Right Panel (Position canvas)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(BG_MAIN)
                    .inner_margin(egui::Margin::symmetric(8.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Links: Config + Geclusterde knoppen
                    self.render_config_card(ui, stopping);

                    ui.add_space(8.0);

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

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);

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

                ui.add_space(6.0);

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
                        egui::RichText::new("position")
                            .color(ACCENT_LIME)
                            .size(15.0)
                            .monospace()
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "x: {}  y: {}",
                                self.config.x, self.config.y
                            ))
                            .color(ACCENT_LIME)
                            .monospace()
                            .size(14.0),
                        );
                    });
                });
                ui.add_space(6.0);

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
        inner_margin: egui::Margin::same(10.0),
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
