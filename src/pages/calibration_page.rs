use eframe::egui::{self, Align, Layout, RichText, Ui};

use crate::components::calibrations::{
    evaluate_interlocks, execute_calibration_sequence, CalibrationComponent,
    CalibrationExecutionResult, CalibrationRoutineConfig, InterlockStatus,
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
pub enum CalibrationPageAction {
    NavigateToSelectVehicle,
    NavigateToAutoDiscovery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalibrationWizardStage {
    Catalog,
    PreCheck,
    Running,
    Finished,
}

pub struct CalibrationPage {
    pub selected_interface: String,
    pub current_stage: CalibrationWizardStage,
    pub selected_routine: Option<CalibrationRoutineConfig>,
    pub checklist_state: Vec<(String, bool)>, // (checklist_item_id, is_checked)
    pub manual_override: bool,
    pub show_confirm_dialog: bool,
    pub execution_result: Option<CalibrationExecutionResult>,
    pub status_message: Option<(String, bool)>, // (text, is_error)
}

impl Default for CalibrationPage {
    fn default() -> Self {
        Self::new()
    }
}

impl CalibrationPage {
    pub fn new() -> Self {
        Self {
            selected_interface: "can0".to_string(),
            current_stage: CalibrationWizardStage::Catalog,
            selected_routine: None,
            checklist_state: Vec::new(),
            manual_override: false,
            show_confirm_dialog: false,
            execution_result: None,
            status_message: None,
        }
    }

    pub fn select_routine(&mut self, routine: CalibrationRoutineConfig) {
        self.checklist_state = routine
            .operator_checklist
            .iter()
            .map(|item| (item.id.clone(), true))
            .collect();
        self.selected_routine = Some(routine);
        self.current_stage = CalibrationWizardStage::PreCheck;
        self.manual_override = false;
        self.show_confirm_dialog = false;
        self.execution_result = None;
        self.status_message = None;
    }

    pub fn reset_to_catalog(&mut self) {
        self.current_stage = CalibrationWizardStage::Catalog;
        self.selected_routine = None;
        self.checklist_state.clear();
        self.manual_override = false;
        self.show_confirm_dialog = false;
        self.execution_result = None;
        self.status_message = None;
    }

    pub fn render(&mut self, ui: &mut Ui, db: &Database) -> Option<CalibrationPageAction> {
        let mut action = None;

        // 1. PAGE HEADER (Shown on Catalog stage; workflows use compact breadcrumbs to maximize workspace)
        if self.current_stage == CalibrationWizardStage::Catalog {
            card_frame(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🎯").size(28.0));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Vehicle Calibrations & Adaptations")
                                .color(COLOR_PRIMARY_CYAN)
                                .size(22.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(
                                "Universal schema-driven calibration engine. Reset zero points, adjust sensor adaptations, and execute ECU maintenance routines safely via UDS & CAN.",
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
        }

        // 2. CHECK ACTIVE VEHICLE
        let active_car = db.get_active_car().ok().flatten();
        match active_car {
            None => {
                self.render_no_vehicle_state(ui, &mut action);
            }
            Some(ref car) => {
                if self.current_stage == CalibrationWizardStage::Catalog {
                    self.render_active_vehicle_banner(ui, car);
                    ui.add_space(14.0);
                    self.render_catalog_stage(ui, car);
                } else {
                    // Render current wizard stage
                    match self.current_stage {
                        CalibrationWizardStage::PreCheck => {
                            self.render_precheck_stage(ui);
                        }
                        CalibrationWizardStage::Running => {
                            self.render_running_stage(ui);
                        }
                        CalibrationWizardStage::Finished => {
                            self.render_finished_stage(ui);
                        }
                        _ => {}
                    }
                }
            }
        }

        action
    }

    // =========================================================================
    // STAGE 1: ROUTINE CATALOG
    // =========================================================================
    fn render_catalog_stage(&mut self, ui: &mut Ui, car: &Car) {
        let routines = CalibrationComponent::load_routines_for_car(car);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("AVAILABLE CALIBRATION & WORK SUPPORT ROUTINES")
                    .color(COLOR_TEXT_SECONDARY)
                    .size(12.0)
                    .strong(),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} routines configured for {}", routines.len(), car.model))
                        .color(COLOR_TEXT_MUTED)
                        .size(11.0),
                );
            });
        });

        ui.add_space(8.0);

        if routines.is_empty() {
            card_frame(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);
                    ui.label(RichText::new("📋").size(36.0));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("No calibration routines configured for this vehicle.")
                            .color(COLOR_TEXT_PRIMARY)
                            .size(15.0)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Add a \"calibrations\" block to app/carworx/config/vehicles/{}/{}.json",
                            car.make.to_lowercase(),
                            car.model
                        ))
                        .color(COLOR_TEXT_MUTED)
                        .size(12.0),
                    );
                    ui.add_space(16.0);
                });
            });
            return;
        }

        let mut routine_to_select = None;

        for routine in &routines {
            elevated_card_frame(ui, COLOR_BORDER, |ui| {
                let button_width = 160.0;
                let card_width = ui.available_width();
                let left_width = (card_width - button_width - 24.0).max(100.0);

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        eframe::egui::vec2(left_width, 0.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            // Domain / Category Icon
                            let icon_glyph = match routine.icon.as_deref() {
                                Some("steering_wheel") => "🛞",
                                Some("disc_brake") => "🛑",
                                Some("gearbox") => "⚙️",
                                Some("engine") => "⚡",
                                Some("throttle") => "📈",
                                _ => "🎯",
                            };
                            ui.label(RichText::new(icon_glyph).size(28.0));
                            ui.add_space(8.0);

                            ui.vertical(|ui| {
                                ui.set_max_width(left_width - 45.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&routine.name)
                                            .color(COLOR_PRIMARY_CYAN)
                                            .size(15.0)
                                            .strong(),
                                    );
                                    ui.label(
                                        RichText::new(format!("[{}]", routine.domain))
                                            .color(COLOR_ACCENT_GREEN)
                                            .size(11.0),
                                    );
                                });

                                ui.add_space(2.0);
                                ui.label(
                                    RichText::new(&routine.description)
                                        .color(COLOR_TEXT_SECONDARY)
                                        .size(12.0),
                                );

                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "Target ECU: {} (Req: {} | Resp: {})",
                                            routine.target_ecu.to_uppercase(),
                                            routine.protocol.request_id,
                                            routine.protocol.response_id
                                        ))
                                        .color(COLOR_TEXT_MUTED)
                                        .size(11.0),
                                    );
                                    ui.label(RichText::new("•").color(COLOR_TEXT_MUTED).size(11.0));
                                    ui.label(
                                        RichText::new(format!("{} Interlocks", routine.telemetry_interlocks.len()))
                                            .color(COLOR_TEXT_MUTED)
                                            .size(11.0),
                                    );
                                    ui.label(RichText::new("•").color(COLOR_TEXT_MUTED).size(11.0));
                                    ui.label(
                                        RichText::new(format!("{} Steps", routine.sequence.len()))
                                            .color(COLOR_TEXT_MUTED)
                                            .size(11.0),
                                    );
                                });
                            });
                        },
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if primary_button(ui, "Select Routine →").clicked() {
                            routine_to_select = Some(routine.clone());
                        }
                    });
                });
            });

            ui.add_space(6.0);
        }

        if let Some(routine) = routine_to_select {
            self.select_routine(routine);
        }
    }

    // =========================================================================
    // STAGE 2: PRE-CHECK & INTERLOCKS INSPECTION
    // =========================================================================
    fn render_precheck_stage(&mut self, ui: &mut Ui) {
        let routine = match self.selected_routine {
            Some(ref r) => r.clone(),
            None => {
                self.reset_to_catalog();
                return;
            }
        };

        // Breadcrumb & Back button
        ui.horizontal(|ui| {
            if secondary_button(ui, "← Back to Routines Catalog").clicked() {
                self.reset_to_catalog();
                return;
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("PRE-CALIBRATION CHECKLIST & SENSOR INTERLOCKS: {}", routine.name))
                    .color(COLOR_PRIMARY_CYAN)
                    .size(13.0)
                    .strong(),
            );
        });

        ui.add_space(10.0);

        // Warning Callout
        if let Some(ref warning_text) = routine.warning {
            elevated_card_frame(ui, COLOR_ACCENT_AMBER, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠️").size(24.0));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("SAFETY PRECAUTION").color(COLOR_ACCENT_AMBER).size(12.0).strong());
                        ui.label(RichText::new(warning_text).color(COLOR_TEXT_PRIMARY).size(12.0));
                    });
                });
            });
            ui.add_space(12.0);
        }

        // TWO-COLUMN LAYOUT: Left = Operator Checklist, Right = Live Sensor Interlocks
        ui.columns(2, |columns| {
            // COLUMN 1: MANUAL CHECKLIST
            columns[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("1. OPERATOR PHYSICAL CHECKLIST")
                            .color(COLOR_TEXT_SECONDARY)
                            .size(12.0)
                            .strong(),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Clear").clicked() {
                            for entry in &mut self.checklist_state {
                                entry.1 = false;
                            }
                        }
                        if ui.small_button("✓ Check All").clicked() {
                            for entry in &mut self.checklist_state {
                                entry.1 = true;
                            }
                        }
                    });
                });
                ui.add_space(6.0);

                card_frame(ui, |ui| {
                    ui.vertical(|ui| {
                        for (idx, item) in routine.operator_checklist.iter().enumerate() {
                            let is_checked = self
                                .checklist_state
                                .iter()
                                .find(|(id, _)| id == &item.id)
                                .map(|(_, state)| *state)
                                .unwrap_or(false);

                            let mut checked_val = is_checked;
                            let mut toggled = false;

                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut checked_val, "").changed() {
                                    toggled = true;
                                }
                                let label_color = if checked_val { COLOR_ACCENT_GREEN } else { COLOR_TEXT_PRIMARY };
                                let lbl = ui.add(
                                    egui::Label::new(
                                        RichText::new(&item.prompt)
                                            .color(label_color)
                                            .size(12.0),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                if lbl.clicked() {
                                    checked_val = !checked_val;
                                    toggled = true;
                                }
                            });

                            if toggled {
                                if let Some(entry) = self.checklist_state.iter_mut().find(|(id, _)| id == &item.id) {
                                    entry.1 = checked_val;
                                }
                            }

                            if idx + 1 < routine.operator_checklist.len() {
                                ui.add_space(6.0);
                            }
                        }
                    });
                });
            });

            // COLUMN 2: LIVE TELEMETRY INTERLOCKS
            columns[1].vertical(|ui| {
                let telemetry_mgr = crate::components::calibrations::telemetry::get_telemetry_manager();
                let is_bus_active = telemetry_mgr.is_bus_active();

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("2. LIVE SENSOR TOLERANCE INTERLOCKS")
                            .color(COLOR_TEXT_SECONDARY)
                            .size(12.0)
                            .strong(),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (bus_txt, bus_col) = if is_bus_active {
                            ("● CAN 500k ACTIVE", COLOR_ACCENT_GREEN)
                        } else {
                            ("○ WAITING FOR CAN", COLOR_ACCENT_AMBER)
                        };
                        ui.label(RichText::new(bus_txt).color(bus_col).size(11.0).strong());
                    });
                });
                ui.add_space(6.0);

                let interlock_statuses = evaluate_interlocks(&routine, &self.selected_interface);

                card_frame(ui, |ui| {
                    ui.vertical(|ui| {
                        if interlock_statuses.is_empty() {
                            ui.label(RichText::new("No live sensor bounds required for this routine.").color(COLOR_TEXT_MUTED).size(12.0));
                        } else {
                            for status in &interlock_statuses {
                                render_interlock_chip(ui, status);
                                ui.add_space(6.0);
                            }
                        }
                    });
                });
            });
        });

        ui.add_space(10.0);

        // Optional Manual Override Toggle for Bench Testing or Forced Procedure
        ui.horizontal(|ui| {
            let override_col = if self.manual_override { COLOR_ACCENT_AMBER } else { COLOR_TEXT_MUTED };
            ui.checkbox(
                &mut self.manual_override,
                RichText::new("⚙️ Force Ready / Bypass Interlocks (Bench Testing & Diagnostic Override)")
                    .color(override_col)
                    .size(11.0),
            );
        });

        ui.add_space(10.0);

        // Evaluate whether ready to proceed
        let all_checklist_checked = self.checklist_state.iter().all(|(_, checked)| *checked);
        let interlock_statuses = evaluate_interlocks(&routine, &self.selected_interface);
        let all_interlocks_valid = interlock_statuses.iter().all(|status| status.is_valid);
        let ready_to_set_values = all_interlocks_valid || self.manual_override || all_checklist_checked;

        // Bottom Action Bar
        card_frame(ui, |ui| {
            let button_width = 240.0;
            let card_width = ui.available_width();
            let left_width = (card_width - button_width - 20.0).max(100.0);

            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    eframe::egui::vec2(left_width, 0.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.set_max_width(left_width);
                        if ready_to_set_values {
                            if self.manual_override {
                                ui.label(
                                    RichText::new("● READY TO SET VALUES (MANUAL OVERRIDE ACTIVE)")
                                        .color(COLOR_ACCENT_AMBER)
                                        .strong()
                                        .size(13.0),
                                );
                            } else {
                                ui.label(
                                    RichText::new("● ALL CONDITIONS SATISFIED — READY TO SET VALUES")
                                        .color(COLOR_ACCENT_GREEN)
                                        .strong()
                                        .size(13.0),
                                );
                            }
                        } else {
                            let mut reasons = Vec::new();
                            let invalid_sensors: Vec<String> = interlock_statuses
                                .iter()
                                .filter(|s| !s.is_valid)
                                .map(|s| format!("{} ({:+.1}{})", s.signal_name, s.current_value, s.unit))
                                .collect();
                            if !invalid_sensors.is_empty() {
                                reasons.push(format!("Sensors pending: {}", invalid_sensors.join(", ")));
                            }
                            let unchecked_count = self.checklist_state.iter().filter(|(_, c)| !*c).count();
                            if unchecked_count > 0 {
                                reasons.push(format!("{}/{} checklist items pending", unchecked_count, self.checklist_state.len()));
                            }
                            ui.label(
                                RichText::new(format!("○ PENDING: {}", reasons.join(" | ")))
                                    .color(COLOR_ACCENT_AMBER)
                                    .strong()
                                    .size(12.0),
                            );
                        }
                    },
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ready_to_set_values {
                        // User clicks to open CONFIRMATION DIALOG before setting values!
                        if primary_button(ui, "⚡ Proceed to Set Values...").clicked() {
                            self.show_confirm_dialog = true;
                        }
                    } else {
                        let btn_text = if !all_interlocks_valid {
                            "⚡ Align Sensors to Proceed"
                        } else {
                            "⚡ Complete Checklist to Proceed"
                        };
                        ui.add_enabled(false, eframe::egui::Button::new(RichText::new(btn_text).color(COLOR_TEXT_MUTED)));
                    }
                });
            });
        });

        // =====================================================================
        // CONFIRMATION DIALOG MODAL (EXPLICIT USER REQUEST)
        // =====================================================================
        if self.show_confirm_dialog {
            let mut execute_dialog = false;
            let mut close_dialog = false;

            egui::Window::new("⚠️ Confirmation Required: Set Calibration Values")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .fixed_size(egui::vec2(600.0, 270.0))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⚠️").size(32.0));
                            ui.add_space(8.0);
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("CONFIRM VALUE SETTING & CALIBRATION WRITE")
                                        .color(COLOR_ACCENT_AMBER)
                                        .size(16.0)
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(
                                        "You are about to write new calibration offsets / adaptation values directly to vehicle ECU non-volatile memory.",
                                    )
                                    .color(COLOR_TEXT_PRIMARY)
                                    .size(13.0),
                                );
                            });
                        });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.label(
                            RichText::new(format!(
                                "• Routine: {}\n\
                                • Target ECU Domain: {} (CAN Request: {} | Response: {})\n\
                                • Diagnostic Action: Dispatches UDS RoutineControl ($31 $01) to reprogram EEPROM offsets.\n\
                                • Safety Condition: Vehicle must remain stationary on level ground throughout execution.",
                                routine.name,
                                routine.domain,
                                routine.protocol.request_id,
                                routine.protocol.response_id
                            ))
                            .color(COLOR_TEXT_SECONDARY)
                            .size(12.0),
                        );

                        ui.add_space(10.0);
                        card_frame(ui, |ui| {
                            ui.label(
                                RichText::new("CAUTION: Do not turn off ignition or disconnect the CAN adapter while calibration is executing.")
                                    .color(COLOR_ACCENT_RED)
                                    .size(11.0)
                                    .strong(),
                            );
                        });

                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            if danger_button(ui, "✓ Confirm & Write Values to ECU").clicked() {
                                execute_dialog = true;
                            }

                            ui.add_space(10.0);
                            if secondary_button(ui, "Cancel / Return to Pre-Check").clicked() {
                                close_dialog = true;
                            }
                        });
                        ui.add_space(4.0);
                    });
                });

            if execute_dialog {
                self.show_confirm_dialog = false;
                let result = execute_calibration_sequence(&routine, &self.selected_interface);
                self.execution_result = Some(result);
                self.current_stage = CalibrationWizardStage::Finished;
            } else if close_dialog {
                self.show_confirm_dialog = false;
            }
        }
    }

    // =========================================================================
    // STAGE 3: RUNNING (TRANSIENT / EXECUTION SPINNER)
    // =========================================================================
    fn render_running_stage(&mut self, ui: &mut Ui) {
        card_frame(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.spinner();
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Executing Calibration Routine on Vehicle Bus...")
                        .color(COLOR_PRIMARY_CYAN)
                        .size(16.0)
                        .strong(),
                );
                ui.label(
                    RichText::new("Maintaining TesterPresent keep-alive and waiting for ECU EEPROM offset calculation.")
                        .color(COLOR_TEXT_SECONDARY)
                        .size(12.0),
                );
                ui.add_space(20.0);
            });
        });
    }

    // =========================================================================
    // STAGE 4: FINISHED (VERIFICATION & DRIVE CYCLE INSTRUCTIONS)
    // =========================================================================
    fn render_finished_stage(&mut self, ui: &mut Ui) {
        let result = match self.execution_result {
            Some(ref res) => res.clone(),
            None => {
                self.reset_to_catalog();
                return;
            }
        };

        let routine = match self.selected_routine {
            Some(ref r) => r.clone(),
            None => {
                self.reset_to_catalog();
                return;
            }
        };

        // Header status banner
        let banner_color = if result.success { COLOR_ACCENT_GREEN } else { COLOR_ACCENT_RED };
        let banner_icon = if result.success { "✅" } else { "❌" };

        elevated_card_frame(ui, banner_color, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(banner_icon).size(32.0));
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(if result.success { "CALIBRATION COMPLETED SUCCESSFULLY" } else { "CALIBRATION FAILED" })
                            .color(banner_color)
                            .size(16.0)
                            .strong(),
                    );
                    ui.label(RichText::new(&result.message).color(COLOR_TEXT_PRIMARY).size(13.0));
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{}/{} Steps", result.completed_steps, result.total_steps))
                            .color(COLOR_TEXT_MUTED)
                            .size(12.0),
                    );
                });
            });
        });

        ui.add_space(12.0);

        // Verification & Post-Instructions Panel
        if let Some(ref post_inst) = routine.verification.post_instructions {
            card_frame(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("ACTIONABLE TECHNICIAN INSTRUCTIONS & DRIVE CYCLE")
                            .color(COLOR_PRIMARY_CYAN)
                            .size(12.0)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    for line in post_inst.lines() {
                        if line.starts_with("### ") {
                            ui.label(RichText::new(line.trim_start_matches("### ")).color(COLOR_TEXT_PRIMARY).size(14.0).strong());
                        } else {
                            ui.label(RichText::new(line).color(COLOR_TEXT_SECONDARY).size(12.0));
                        }
                    }
                });
            });
            ui.add_space(12.0);
        }

        // Live Diagnostic Trace Terminal Log
        ui.label(
            RichText::new("DIAGNOSTIC TRANSACTION LOG")
                .color(COLOR_TEXT_SECONDARY)
                .size(12.0)
                .strong(),
        );
        ui.add_space(4.0);

        card_frame(ui, |ui| {
            ui.vertical(|ui| {
                for entry in &result.logs {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&entry.timestamp).color(COLOR_TEXT_MUTED).size(11.0).monospace());
                        let dir_color = match entry.direction.as_str() {
                            "TX" => COLOR_PRIMARY_CYAN,
                            "RX" => COLOR_ACCENT_GREEN,
                            _ => COLOR_TEXT_MUTED,
                        };
                        ui.label(RichText::new(&entry.direction).color(dir_color).size(11.0).strong().monospace());
                        let msg_color = if entry.is_highlight { COLOR_TEXT_PRIMARY } else { COLOR_TEXT_SECONDARY };
                        ui.label(RichText::new(&entry.message).color(msg_color).size(11.0).monospace());
                    });
                }
            });
        });

        ui.add_space(14.0);

        // Finish & Return to Catalog Button
        ui.horizontal(|ui| {
            if primary_button(ui, "✓ Done / Return to Calibrations Catalog").clicked() {
                self.reset_to_catalog();
            }
        });
    }

    // =========================================================================
    // HELPER: NO VEHICLE SELECTED STATE
    // =========================================================================
    fn render_no_vehicle_state(&mut self, ui: &mut Ui, action: &mut Option<CalibrationPageAction>) {
        card_frame(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(RichText::new("⚠️").size(48.0));
                ui.add_space(10.0);
                ui.label(
                    RichText::new("No Active Vehicle Selected")
                        .color(COLOR_ACCENT_AMBER)
                        .size(18.0)
                        .strong(),
                );
                ui.label(
                    RichText::new(
                        "Please select or discover a vehicle profile to view available calibration routines and safety interlocks.",
                    )
                    .color(COLOR_TEXT_SECONDARY)
                    .size(13.0),
                );
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 130.0);
                    if primary_button(ui, "🚙 Select Vehicle").clicked() {
                        *action = Some(CalibrationPageAction::NavigateToSelectVehicle);
                    }
                    ui.add_space(8.0);
                    if secondary_button(ui, "🔍 Auto Discovery").clicked() {
                        *action = Some(CalibrationPageAction::NavigateToAutoDiscovery);
                    }
                });
                ui.add_space(20.0);
            });
        });
    }

    // =========================================================================
    // HELPER: ACTIVE VEHICLE BANNER
    // =========================================================================
    fn render_active_vehicle_banner(&self, ui: &mut Ui, car: &Car) {
        elevated_card_frame(ui, COLOR_BORDER, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("ACTIVE VEHICLE").color(COLOR_TEXT_MUTED).size(11.0).strong());
                    ui.add_space(2.0);
                    let nickname = &car.name;
                    let vin_display = car.vin.as_deref().unwrap_or("No VIN");
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{} {} {}", car.year, car.make, car.model))
                                .color(COLOR_TEXT_PRIMARY)
                                .size(15.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("(\"{}\" | VIN: {})", nickname, vin_display))
                                .color(COLOR_TEXT_SECONDARY)
                                .size(12.0),
                        );
                    });
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("CAN INTERFACE").color(COLOR_TEXT_MUTED).size(11.0).strong());
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(&self.selected_interface)
                                .color(COLOR_ACCENT_GREEN)
                                .size(14.0)
                                .strong(),
                        );
                    });
                });
            });
        });
    }
}

/// Renders a dynamic status chip for a live telemetry interlock
fn render_interlock_chip(ui: &mut Ui, status: &InterlockStatus) {
    let chip_color = if status.is_valid { COLOR_ACCENT_GREEN } else { COLOR_ACCENT_RED };
    let status_glyph = if status.is_valid { "●" } else { "✖" };
    let status_text = if status.is_valid { "READY / PASS" } else { "OUT OF TOLERANCE" };

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} {}", status_glyph, status.signal_name))
                    .color(chip_color)
                    .size(13.0)
                    .strong(),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(status_text).color(chip_color).size(11.0).strong());
                if status.is_live {
                    ui.label(RichText::new("[LIVE CAN]").color(COLOR_PRIMARY_CYAN).size(10.0).strong());
                } else {
                    ui.label(RichText::new("[CAN OFFLINE]").color(COLOR_TEXT_MUTED).size(10.0));
                }
            });
        });

        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            ui.label(
                RichText::new(format!(
                    "Current: {:+.2} {}  |  Target: {}",
                    status.current_value, status.unit, status.tolerance_label
                ))
                .color(COLOR_TEXT_SECONDARY)
                .size(11.0),
            );
        });

        if !status.is_valid {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(RichText::new(&status.error_message).color(COLOR_ACCENT_RED).size(11.0));
            });
        }
    });
}
