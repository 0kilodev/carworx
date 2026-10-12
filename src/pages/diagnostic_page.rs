use eframe::egui::{Align, Layout, RichText, Ui};

use crate::components::diagnostic::{
    clear_vehicle_dtcs, perform_diagnostic_scan, resolve_vehicle_targets, DiagnosticScanResult,
    DtcCode, DtcDomainGroup,
};
use crate::core::hardware::detect_can_interfaces;
use crate::core::Database;
use crate::schema::models::Car;
use crate::ui::{
    card_frame, danger_button, elevated_card_frame, primary_button, secondary_button,
    COLOR_ACCENT_AMBER, COLOR_ACCENT_GREEN, COLOR_ACCENT_RED, COLOR_BORDER, COLOR_PRIMARY_CYAN,
    COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
};

#[derive(Debug, Clone)]
pub enum DiagnosticPageAction {
    NavigateToSelectVehicle,
    NavigateToAutoDiscovery,
}

pub struct DiagnosticPage {
    pub is_scanning: bool,
    pub scan_result: Option<Result<DiagnosticScanResult, String>>,
    pub selected_interface: String,
    pub show_clear_confirm: bool,
    pub clear_status_message: Option<(String, bool)>,
}

impl Default for DiagnosticPage {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticPage {
    pub fn new() -> Self {
        Self {
            is_scanning: false,
            scan_result: None,
            selected_interface: "can0".to_string(),
            show_clear_confirm: false,
            clear_status_message: None,
        }
    }

    pub fn render(&mut self, ui: &mut Ui, db: &Database) -> Option<DiagnosticPageAction> {
        let mut action = None;

        // 1. PAGE HEADER
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔧").size(28.0));
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Automotive Diagnostics (DTC)")
                            .color(COLOR_PRIMARY_CYAN)
                            .size(22.0)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(
                            "Multi-domain Diagnostic Trouble Code scanner via UDS & OBD-II. Query and decode confirmed fault codes across Engine, Transmission, ABS, EPS, BIU, and SRS domains.",
                        )
                        .color(COLOR_TEXT_SECONDARY)
                        .size(13.0),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let interfaces = detect_can_interfaces();
                    let can_up = interfaces.iter().any(|i| i.is_up);
                    let iface_text = if can_up { "● CAN BUS READY" } else { "○ NO ACTIVE CAN" };
                    let iface_color = if can_up { COLOR_ACCENT_GREEN } else { COLOR_ACCENT_AMBER };
                    ui.label(RichText::new(iface_text).color(iface_color).strong().size(12.0));
                });
            });
        });

        ui.add_space(14.0);

        // 2. ACTIVE VEHICLE CHECK
        let active_car = db.get_active_car().ok().flatten();

        match active_car {
            None => {
                // EMPTY STATE: User has not selected any vehicle
                elevated_card_frame(ui, COLOR_ACCENT_AMBER, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new("⚠️").size(48.0));
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("No Vehicle Selected")
                                .color(COLOR_TEXT_PRIMARY)
                                .size(20.0)
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(
                                "A vehicle must be selected in order to locate protocol definitions, ECU target addresses, and CAN bus identifiers.\n\nPlease select an existing vehicle from your garage or run auto-discovery to identify your connected vehicle.",
                            )
                            .color(COLOR_TEXT_SECONDARY)
                            .size(14.0),
                        );
                        ui.add_space(20.0);

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 16.0;
                            if primary_button(ui, "🚙 Select Vehicle from Garage").clicked() {
                                action = Some(DiagnosticPageAction::NavigateToSelectVehicle);
                            }

                            if secondary_button(ui, "🔍 Run Auto-Discovery").clicked() {
                                action = Some(DiagnosticPageAction::NavigateToAutoDiscovery);
                            }
                        });
                        ui.add_space(20.0);
                    });
                });
            }
            Some(car) => {
                // VEHICLE SELECTED STATE: Display Vehicle Metadata, Controls & Domain DTC Results
                self.render_vehicle_selected(ui, &car);
            }
        }

        action
    }

    fn render_vehicle_selected(&mut self, ui: &mut Ui, car: &Car) {
        // Vehicle Metadata Banner
        elevated_card_frame(ui, COLOR_BORDER, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("ACTIVE VEHICLE FOR DIAGNOSTIC SCAN")
                            .color(COLOR_TEXT_MUTED)
                            .size(11.0)
                            .strong(),
                    );
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{} {} {}", car.year, car.make, car.model))
                                .color(COLOR_TEXT_PRIMARY)
                                .size(16.0)
                                .strong(),
                        );
                        if let Some(ref vin) = car.vin {
                            ui.label(
                                RichText::new(format!("(VIN: {})", vin))
                                    .color(COLOR_PRIMARY_CYAN)
                                    .size(13.0),
                            );
                        }
                    });

                    // Target ECUs preview
                    let targets = resolve_vehicle_targets(car);
                    let target_summary = targets
                        .iter()
                        .map(|t| format!("{}: 0x{:03X}", t.domain.code(), t.request_id))
                        .collect::<Vec<_>>()
                        .join(" | ");

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("Config Target ECUs: {}", target_summary))
                            .color(COLOR_TEXT_SECONDARY)
                            .size(11.0),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("6 Domains Registered")
                            .color(COLOR_ACCENT_GREEN)
                            .size(12.0)
                            .strong(),
                    );
                });
            });
        });

        ui.add_space(14.0);

        // Control & Action Bar
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                // Interface selector
                ui.label(RichText::new("CAN Adapter:").color(COLOR_TEXT_SECONDARY).size(13.0));
                let interfaces = detect_can_interfaces();
                if interfaces.is_empty() {
                    ui.label(RichText::new("can0 (fallback)").color(COLOR_TEXT_MUTED));
                } else {
                    for iface in &interfaces {
                        let is_selected = self.selected_interface == iface.name;
                        let text = format!("{} ({})", iface.name, iface.operstate);
                        if ui.selectable_label(is_selected, text).clicked() {
                            self.selected_interface = iface.name.clone();
                        }
                    }
                }

                ui.add_space(16.0);

                // RUN DIAGNOSTIC SCAN BUTTON
                if primary_button(ui, "⚡ Run Diagnostic (DTC) Scan").clicked() {
                    self.clear_status_message = None;
                    match perform_diagnostic_scan(car, &self.selected_interface) {
                        Ok(res) => {
                            self.scan_result = Some(Ok(res));
                        }
                        Err(e) => {
                            self.scan_result = Some(Err(e.to_string()));
                        }
                    }
                }

                ui.add_space(8.0);

                // CLEAR VEHICLE DTCS BUTTON (Requires user confirmation)
                if danger_button(ui, "🗑️ Clear DTCs on Vehicle").clicked() {
                    self.show_clear_confirm = true;
                }

                if self.scan_result.is_some() {
                    ui.add_space(8.0);
                    if secondary_button(ui, "Reset Scan View").clicked() {
                        self.scan_result = None;
                        self.clear_status_message = None;
                    }
                }
            });
        });

        ui.add_space(14.0);

        // CLEAR CONFIRMATION DIALOG MODAL / CARD
        if self.show_clear_confirm {
            elevated_card_frame(ui, COLOR_ACCENT_RED, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚠️").size(28.0));
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("CONFIRM CLEARING DTC FAULT CODES ON VEHICLE")
                                    .color(COLOR_ACCENT_RED)
                                    .size(16.0)
                                    .strong(),
                            );
                            ui.label(
                                RichText::new(
                                    "Are you sure you want to clear Diagnostic Trouble Codes directly on the car?\n\n\
                                    • This sends UDS Service 0x14 (0x14 FF FF FF) & OBD-II Mode $04 clear commands to all 6 vehicle domains.\n\
                                    • Stored fault codes and emissions readiness monitor states will be reset on vehicle ECUs.\n\
                                    • Any physical hardware faults that remain unresolved will cause DTCs to return upon ignition cycle.",
                                )
                                .color(COLOR_TEXT_PRIMARY)
                                .size(13.0),
                            );
                        });
                    });

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if danger_button(ui, "⚠️ YES, CLEAR CODES ON VEHICLE").clicked() {
                            self.show_clear_confirm = false;
                            match clear_vehicle_dtcs(car, &self.selected_interface) {
                                Ok(report) => {
                                    self.clear_status_message = Some((
                                        format!("DTC clear command dispatched to vehicle ECUs: {}. Refreshing scan...", report.summary),
                                        false,
                                    ));
                                    // Automatically re-scan to show current vehicle state
                                    if let Ok(new_scan) = perform_diagnostic_scan(car, &self.selected_interface) {
                                        self.scan_result = Some(Ok(new_scan));
                                    }
                                }
                                Err(e) => {
                                    self.clear_status_message = Some((format!("Failed to clear DTCs: {}", e), true));
                                }
                            }
                        }

                        ui.add_space(12.0);
                        if secondary_button(ui, "Cancel").clicked() {
                            self.show_clear_confirm = false;
                        }
                    });
                });
            });

            ui.add_space(14.0);
        }

        // CLEAR STATUS MESSAGE BANNER
        if let Some((ref msg, is_err)) = self.clear_status_message {
            let banner_color = if is_err { COLOR_ACCENT_RED } else { COLOR_ACCENT_GREEN };
            let icon = if is_err { "❌" } else { "✓" };
            elevated_card_frame(ui, banner_color, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(icon).size(18.0));
                    ui.add_space(6.0);
                    ui.label(RichText::new(msg).color(banner_color).strong().size(13.0));
                });
            });
            ui.add_space(14.0);
        }

        // Scan Results Display
        if let Some(ref result) = self.scan_result {
            match result {
                Ok(scan) => {
                    self.render_scan_results(ui, scan);
                }
                Err(err) => {
                    elevated_card_frame(ui, COLOR_ACCENT_RED, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("❌").size(24.0));
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("Diagnostic Scan Failed")
                                        .color(COLOR_ACCENT_RED)
                                        .size(15.0)
                                        .strong(),
                                );
                                ui.label(RichText::new(err).color(COLOR_TEXT_PRIMARY).size(13.0));
                            });
                        });
                    });
                }
            }
        }
    }

    fn render_scan_results(&self, ui: &mut Ui, scan: &DiagnosticScanResult) {
        // Summary Header Card
        let (status_text, status_color) = if scan.total_confirmed_dtcs == 0 {
            ("✓ ALL SYSTEMS CLEAN (0 CONFIRMED FAULTS)", COLOR_ACCENT_GREEN)
        } else {
            (
                "⚠️ CONFIRMED FAULTS DETECTED",
                COLOR_ACCENT_RED,
            )
        };

        elevated_card_frame(ui, status_color, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(status_text).color(status_color).size(16.0).strong());
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Total Confirmed DTCs: {} | Domains Scanned: {} | Interface: {}",
                                scan.total_confirmed_dtcs,
                                scan.domains.len(),
                                scan.interface
                            ))
                            .color(COLOR_TEXT_PRIMARY)
                            .size(13.0),
                        );
                    });
                    ui.label(RichText::new(&scan.notes).color(COLOR_TEXT_MUTED).size(11.0));
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let source_label = if scan.is_live { "● LIVE CAN SCAN" } else { "● BASELINE SNAPSHOT" };
                    let source_color = if scan.is_live { COLOR_ACCENT_GREEN } else { COLOR_PRIMARY_CYAN };
                    ui.label(RichText::new(source_label).color(source_color).strong().size(12.0));
                });
            });
        });

        ui.add_space(14.0);

        // DOMAIN GROUPS (Strictly 6 domain groups, showing ONLY confirmed DTCs)
        ui.label(
            RichText::new("SYSTEM DOMAINS & CONFIRMED DIAGNOSTIC TROUBLE CODES")
                .color(COLOR_TEXT_MUTED)
                .size(12.0)
                .strong(),
        );
        ui.add_space(6.0);

        for group in &scan.domains {
            self.render_domain_group(ui, group);
            ui.add_space(10.0);
        }
    }

    fn render_domain_group(&self, ui: &mut Ui, group: &DtcDomainGroup) {
        let has_faults = group.has_faults();
        let border_color = if has_faults {
            COLOR_ACCENT_RED
        } else {
            COLOR_BORDER
        };

        elevated_card_frame(ui, border_color, |ui| {
            ui.vertical(|ui| {
                // Group Header
                ui.horizontal(|ui| {
                    ui.label(RichText::new(group.domain.icon()).size(22.0));
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(group.domain.name())
                                    .color(COLOR_TEXT_PRIMARY)
                                    .size(15.0)
                                    .strong(),
                            );
                            ui.label(
                                RichText::new(format!("({})", group.ecu_name))
                                    .color(COLOR_TEXT_SECONDARY)
                                    .size(12.0),
                            );
                        });
                        ui.label(
                            RichText::new(format!(
                                "CAN Req: 0x{:03X} -> Resp: 0x{:03X}",
                                group.request_id, group.response_id
                            ))
                            .color(COLOR_TEXT_MUTED)
                            .size(11.0),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if has_faults {
                            let badge_text = format!("⚠️ {} Confirmed Fault(s)", group.fault_count());
                            ui.label(RichText::new(badge_text).color(COLOR_ACCENT_RED).strong().size(13.0));
                        } else {
                            ui.label(
                                RichText::new("✓ No Confirmed Faults (Normal)")
                                    .color(COLOR_ACCENT_GREEN)
                                    .strong()
                                    .size(13.0),
                            );
                        }
                    });
                });

                // Confirmed DTC List for this domain
                if has_faults {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    for dtc in &group.confirmed_dtcs {
                        self.render_dtc_row(ui, dtc);
                        ui.add_space(6.0);
                    }
                }
            });
        });
    }

    fn render_dtc_row(&self, ui: &mut Ui, dtc: &DtcCode) {
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                // DTC Code in bold monospace
                ui.label(
                    RichText::new(&dtc.code)
                        .color(COLOR_ACCENT_RED)
                        .size(16.0)
                        .monospace()
                        .strong(),
                );

                ui.add_space(8.0);

                // Confirmed badge
                ui.label(
                    RichText::new("[CONFIRMED]")
                        .color(COLOR_ACCENT_RED)
                        .size(11.0)
                        .strong(),
                );

                ui.add_space(8.0);

                // DTC Description
                ui.label(
                    RichText::new(&dtc.description)
                        .color(COLOR_TEXT_PRIMARY)
                        .size(13.0)
                        .strong(),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let status_flags_str = dtc.status_labels.join(", ");
                    ui.label(
                        RichText::new(format!("Raw: 0x{} | Mask: 0x{:02X} ({})", dtc.raw_hex, dtc.status_byte, status_flags_str))
                            .color(COLOR_TEXT_MUTED)
                            .size(11.0)
                            .monospace(),
                    );
                });
            });
        });
    }
}
