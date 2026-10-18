//! mkcad application with eframe/egui UI.
//!
//! Features:
//! - Central 3D viewport (M3): wgpu renderer on the egui-wgpu device, view cube, picking
//! - Bottom diagnostic log viewer
//! - CommandBus integration for all user actions
//! - Cold-start measurement (MKCAD_STARTUP_LOG=1)
//! - Smoke test mode (--smoke flag for headless testing)
//! - Viewport load test (--stress: 1M-triangle scene)

mod viewport_panel;

use mkcad_domain::{Command, CommandBus, DiagnosticsSink};
use std::time::Instant;
use viewport_panel::{ViewportEvent, ViewportPanel};

/// Application state shared between frames.
struct AppState {
    bus: CommandBus,
    diagnostics_sink: DiagnosticsSink,
    /// Tracks startup time for cold-start measurement (M1 scaffold).
    startup_time: Instant,
    /// Exit after the first frame (`--smoke`); first-frame time is always recorded.
    smoke: bool,
    first_frame_reported: bool,
    /// Log viewer scroll position (M1 scaffold).
    #[allow(dead_code)]
    log_scroll_offset: f32,
    /// 3D viewport; None if the wgpu renderer is unavailable.
    viewport: Option<ViewportPanel>,
    /// Status-bar text for the current selection.
    selection: String,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            bus: CommandBus::new(),
            diagnostics_sink: DiagnosticsSink::new(),
            startup_time: Instant::now(),
            smoke: false,
            first_frame_reported: false,
            log_scroll_offset: 0.0,
            viewport: None,
            selection: String::new(),
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.first_frame_reported {
            self.first_frame_reported = true;
            if self.smoke || std::env::var("MKCAD_STARTUP_LOG").is_ok() {
                eprintln!(
                    "MKCAD_STARTUP_TIME_MS={}",
                    self.startup_time.elapsed().as_millis()
                );
            }
            if self.smoke {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        // Top panel: menu bar (stub)
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("mkcad");
                ui.menu_button("Debug", |ui| {
                    if let Some(vp) = self.viewport.as_mut() {
                        if ui.button("Demo scene (3 boxes)").clicked() {
                            vp.load_demo_scene();
                            ui.close_menu();
                        }
                        if ui.button("Stress scene (1M triangles)").clicked() {
                            vp.load_stress_scene();
                            ui.close_menu();
                        }
                        ui.separator();
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Emit Sample Error").clicked() {
                            let cmd = Command::EmitDiagnostic {
                                code: "SKETCH.CONSTRAINT.REDUNDANT".to_string(),
                                severity: "error".to_string(),
                            };
                            let outcome = self.bus.dispatch(cmd);
                            self.diagnostics_sink.emit("SKETCH.CONSTRAINT.REDUNDANT", "error");
                            println!("Command outcome: {:?}", outcome);
                        }

                        if ui.button("Emit Sample Warning").clicked() {
                            let cmd = Command::EmitDiagnostic {
                                code: "REF.AMBIGUOUS".to_string(),
                                severity: "warning".to_string(),
                            };
                            let outcome = self.bus.dispatch(cmd);
                            self.diagnostics_sink.emit("REF.AMBIGUOUS", "warning");
                            println!("Command outcome: {:?}", outcome);
                        }

                        if ui.button("Emit Sample Info").clicked() {
                            let cmd = Command::EmitDiagnostic {
                                code: "IO.UNITS.ASSUMED".to_string(),
                                severity: "info".to_string(),
                            };
                            let outcome = self.bus.dispatch(cmd);
                            self.diagnostics_sink.emit("IO.UNITS.ASSUMED", "info");
                            println!("Command outcome: {:?}", outcome);
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Test Undo").clicked() {
                            if let Some(label) = self.bus.undo() {
                                println!("Undid: {}", label);
                            }
                        }

                        if ui.button("Test Redo").clicked() {
                            if let Some(label) = self.bus.redo() {
                                println!("Redid: {}", label);
                            }
                        }

                        ui.label(format!(
                            "Undo: {}, Redo: {}",
                            self.bus.undo_count(),
                            self.bus.redo_count()
                        ));
                    });

                });
                if ui.button("About").clicked() {
                    // Stub for About dialog
                }
            });
        });

        // Bottom panel: log viewer
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .default_height(120.0)
            .min_height(60.0)
            .max_height(400.0)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Diagnostics: {} errors, {} warnings, {} infos",
                    self.diagnostics_sink.error_count(),
                    self.diagnostics_sink.warning_count(),
                    self.diagnostics_sink.info_count()
                ));

                // Simple diagnostic log table
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height(ui.available_height() - 30.0)
                    .show(ui, |ui| {
                        egui::Grid::new("log_grid")
                            .striped(true)
                            .spacing([20.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Code");
                                ui.label("Severity");
                                ui.label("Subject");
                                ui.end_row();

                                for entry in self.diagnostics_sink.all() {
                                    let color = match entry.severity.as_str() {
                                        "error" => egui::Color32::RED,
                                        "warning" => egui::Color32::YELLOW,
                                        _ => egui::Color32::GRAY,
                                    };
                                    ui.colored_label(color, &entry.code);
                                    ui.label(&entry.severity);
                                    ui.label(entry.subject.as_deref().unwrap_or("-"));
                                    ui.end_row();
                                }
                            });
                    });

                ui.separator();
                if ui.button("Clear Log").clicked() {
                    self.diagnostics_sink.clear();
                }
            });

        // Central panel: view toolbar + 3D viewport
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let Some(vp) = self.viewport.as_mut() else {
                    ui.label("3D viewport unavailable: no wgpu adapter");
                    return;
                };
                ui.horizontal(|ui| {
                    ui.add_space(6.0);
                    if ui.button("Home").on_hover_text("Fit all, iso view").clicked() {
                        vp.fit();
                    }
                    for (label, name) in [("Top", "top"), ("Front", "front"), ("Right", "right"), ("Iso", "iso")] {
                        if ui.button(label).clicked() {
                            vp.named_view(name);
                        }
                    }
                    let mut persp = vp.is_perspective();
                    if ui.checkbox(&mut persp, "Perspective").changed() {
                        vp.set_perspective(persp);
                    }
                    ui.separator();
                    let dt_ms = ui.input(|i| i.unstable_dt) * 1000.0;
                    ui.label(format!("{} triangles · {dt_ms:.1} ms/frame", vp.triangle_count()));
                    ui.separator();
                    ui.label(&self.selection);
                });
                if let Some(ViewportEvent::Selected(hit)) = vp.show(ui) {
                    self.selection = match hit {
                        Some(id) => format!("Selected: {}", vp.item_name(id).unwrap_or("?")),
                        None => String::new(),
                    };
                }
            });
    }
}

fn main() {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let smoke_test = args.contains(&"--smoke".to_string());
    // `--stress`: start with the 1M-triangle scene (viewport performance check).
    let stress = args.contains(&"--stress".to_string());

    // Measure startup time from process start
    let startup_instant = Instant::now();

    // Create window options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_resizable(true),
        renderer: eframe::Renderer::Wgpu,
        depth_buffer: 0,
        ..Default::default()
    };

    let _ = eframe::run_native(
        "mkcad",
        options,
        Box::new(move |cc| {
            Box::new(AppState {
                startup_time: startup_instant,
                smoke: smoke_test,
                viewport: cc.wgpu_render_state.as_ref().map(|rs| {
                    let mut vp = ViewportPanel::new(rs);
                    if stress {
                        vp.load_stress_scene();
                    }
                    vp
                }),
                ..AppState::default()
            })
        }),
    );
}
