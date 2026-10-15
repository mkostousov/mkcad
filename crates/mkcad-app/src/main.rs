//! mkcad application with eframe/egui UI.
//!
//! Features:
//! - Empty central panel for 3D viewport (M3 will add wgpu rendering)
//! - Bottom diagnostic log viewer
//! - CommandBus integration for all user actions
//! - Cold-start measurement (MKCAD_STARTUP_LOG=1)
//! - Smoke test mode (--smoke flag for headless testing)

use mkcad_domain::{Command, CommandBus, DiagnosticsSink};
use std::time::Instant;

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
                if ui.button("About").clicked() {
                    // Stub for About dialog
                }
            });
        });

        // Bottom panel: log viewer
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .min_height(100.0)
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
                    .auto_shrink([false; 2])
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

        // Central panel: empty for now (M3 will add 3D viewport)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("3D Viewport (M3)");
                ui.label("Central rendering area — wgpu integration deferred to M3");

                ui.separator();

                // Debug controls
                ui.heading("Debug Controls");
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

                ui.separator();

                // Diagnostics summary
                ui.label(format!(
                    "Commands dispatched, total diagnostics collected: {}",
                    self.diagnostics_sink.count()
                ));
            });
        });
    }
}

fn main() {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let smoke_test = args.contains(&"--smoke".to_string());

    // Measure startup time from process start
    let startup_instant = Instant::now();

    // Create window options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_resizable(true),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "mkcad",
        options,
        Box::new(move |_cc| {
            Box::new(AppState {
                startup_time: startup_instant,
                smoke: smoke_test,
                ..AppState::default()
            })
        }),
    );
}
