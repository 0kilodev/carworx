use eframe::egui::{Align, Layout, RichText, Ui};

use crate::core::hardware::detect_can_interfaces;
use crate::core::Database;
use crate::schema::models::Car;
use crate::ui::{
    card_frame, elevated_card_frame, primary_button, secondary_button, COLOR_ACCENT_AMBER,
    COLOR_ACCENT_GREEN, COLOR_BORDER, COLOR_PRIMARY_CYAN, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
    COLOR_TEXT_SECONDARY,
};

#[derive(Debug, Clone)]
pub enum LandingPageAction {
    #[allow(dead_code)]
    NavigateToSelectVehicle,
    #[allow(dead_code)]
    NavigateToAutoDiscovery,
    NavigateToDiagnostics,
    NavigateToCalibrations,
    CarSwitched(Car),
}

pub struct LandingPage {
    pub discovery_in_progress: bool,
    pub last_discovery_summary: Option<(String, bool)>, // (summary, is_error)
}

impl Default for LandingPage {
    fn default() -> Self {
        Self::new()
    }
}

impl LandingPage {
    pub fn new() -> Self {
        Self {
            discovery_in_progress: false,
            last_discovery_summary: None,
        }
    }

    pub fn render(&mut self, ui: &mut Ui, db: &Database) -> Option<LandingPageAction> {
        let mut action = None;

        // Header / Welcome Banner
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🚗").size(36.0));
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("CARWORX")
                            .color(COLOR_PRIMARY_CYAN)
                            .size(24.0)
                            .extra_letter_spacing(2.0)
                            .strong(),
                    );
                    ui.label(
                        RichText::new("Universal Automotive Diagnostics & CAN Bus Platform")
                            .color(COLOR_TEXT_SECONDARY)
                            .size(13.0),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // System Status Chips
                    let interfaces = detect_can_interfaces();
                    let can_up = interfaces.iter().any(|i| i.is_up);
                    let iface_text = if can_up { "● CAN BUS READY" } else { "○ NO ACTIVE CAN" };
                    let iface_color = if can_up { COLOR_ACCENT_GREEN } else { COLOR_ACCENT_AMBER };

                    ui.label(RichText::new(iface_text).color(iface_color).strong().size(12.0));
                });
            });
        });

        ui.add_space(14.0);

        // Active Status Bar
        let active_car = db.get_active_car().ok().flatten();
        let active_iface = db.get_active_interface().ok().flatten();

        elevated_card_frame(ui, COLOR_BORDER, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("ACTIVE VEHICLE").color(COLOR_TEXT_MUTED).size(11.0).strong());
                    ui.add_space(2.0);
                    match active_car {
                        Some(ref car) => {
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
                        }
                        None => {
                            ui.label(
                                RichText::new("No active vehicle selected")
                                    .color(COLOR_ACCENT_AMBER)
                                    .size(14.0),
                            );
                        }
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("HARDWARE INTERFACE").color(COLOR_TEXT_MUTED).size(11.0).strong());
                        ui.add_space(2.0);
                        match active_iface {
                            Some(ref iface) => {
                                ui.label(
                                    RichText::new(format!("{} ({} bps, {})", iface.name, iface.bitrate, iface.interface_type))
                                        .color(COLOR_ACCENT_GREEN)
                                        .size(14.0)
                                        .strong(),
                                );
                            }
                            None => {
                                ui.label(
                                    RichText::new("can0 (System Default)")
                                        .color(COLOR_TEXT_SECONDARY)
                                        .size(14.0),
                                );
                            }
                        }
                    });
                });
            });
        });

        ui.add_space(18.0);

        // DIAGNOSTIC (DTC) SYSTEM CARD
        ui.label(
            RichText::new("VEHICLE DIAGNOSTICS & FAULT SCANNER")
                .color(COLOR_TEXT_SECONDARY)
                .size(12.0)
                .strong(),
        );
        ui.add_space(6.0);

        elevated_card_frame(ui, COLOR_PRIMARY_CYAN, |ui| {
            let button_width = 180.0;
            let card_width = ui.available_width();
            let left_width = (card_width - button_width - 20.0).max(100.0);

            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    eframe::egui::vec2(left_width, 0.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.label(RichText::new("🔧").size(32.0));
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.set_max_width(left_width - 45.0);
                            ui.label(
                                RichText::new("Vehicle Diagnostics (DTC)")
                                    .color(COLOR_PRIMARY_CYAN)
                                    .size(16.0)
                                    .strong(),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(
                                    "Multi-domain Diagnostic Trouble Code scanner. Query confirmed fault codes across Engine, Transmission, ABS/VDC, EPS, BIU, and Safety domains via UDS & OBD-II.",
                                )
                                .color(COLOR_TEXT_PRIMARY)
                                .size(13.0),
                            );

                            ui.add_space(6.0);
                            match active_car {
                                Some(ref car) => {
                                    ui.label(
                                        RichText::new(format!(
                                            "● Active Vehicle: {} {} {} (Target ECUs: ECM, TCM, VDC, EPS, BIU, SRS)",
                                            car.year, car.make, car.model
                                        ))
                                        .color(COLOR_ACCENT_GREEN)
                                        .size(12.0),
                                    );
                                }
                                None => {
                                    ui.label(
                                        RichText::new("○ No active vehicle selected — choose a vehicle from your garage or run auto-discovery")
                                            .color(COLOR_TEXT_MUTED)
                                            .size(12.0),
                                    );
                                }
                            }
                        });
                    },
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary_button(ui, "⚡ Open Diagnostics").clicked() {
                        action = Some(LandingPageAction::NavigateToDiagnostics);
                    }
                });
            });
        });

        ui.add_space(14.0);

        // VEHICLE CALIBRATIONS (WORK SUPPORT) SYSTEM CARD
        ui.label(
            RichText::new("VEHICLE CALIBRATIONS & ADAPTATIONS")
                .color(COLOR_TEXT_SECONDARY)
                .size(12.0)
                .strong(),
        );
        ui.add_space(6.0);

        elevated_card_frame(ui, COLOR_PRIMARY_CYAN, |ui| {
            let button_width = 180.0;
            let card_width = ui.available_width();
            let left_width = (card_width - button_width - 20.0).max(100.0);

            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    eframe::egui::vec2(left_width, 0.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.label(RichText::new("🎯").size(32.0));
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.set_max_width(left_width - 45.0);
                            ui.label(
                                RichText::new("Vehicle Calibrations (Work Support)")
                                    .color(COLOR_PRIMARY_CYAN)
                                    .size(16.0)
                                    .strong(),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(
                                    "Universal schema-driven adaptation engine. Perform VDC sensor centering, EPB brake maintenance caliper retraction, CVT oil pressure calibration, and sensor adaptations.",
                                )
                                .color(COLOR_TEXT_SECONDARY)
                                .size(12.0),
                            );

                            ui.add_space(6.0);
                            match active_car {
                                Some(ref car) => {
                                    let routines = crate::components::calibrations::CalibrationComponent::load_routines_for_car(car);
                                    ui.label(
                                        RichText::new(format!("● {} calibration routines configured for {} {}", routines.len(), car.make, car.model))
                                            .color(COLOR_ACCENT_GREEN)
                                            .size(12.0),
                                    );
                                }
                                None => {
                                    ui.label(
                                        RichText::new("○ No active vehicle selected — choose a vehicle to view available calibrations")
                                            .color(COLOR_TEXT_MUTED)
                                            .size(12.0),
                                    );
                                }
                            }
                        });
                    },
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary_button(ui, "🎯 Open Calibrations").clicked() {
                        action = Some(LandingPageAction::NavigateToCalibrations);
                    }
                });
            });
        });

        ui.add_space(18.0);

        // Saved Vehicles in Garage
        match db.list_cars() {
            Ok(cars) if !cars.is_empty() => {
                ui.label(
                    RichText::new("SAVED VEHICLES IN GARAGE")
                        .color(COLOR_TEXT_SECONDARY)
                        .size(12.0)
                        .strong(),
                );
                ui.add_space(6.0);

                card_frame(ui, |ui| {
                    ui.vertical(|ui| {
                        for car in cars {
                            let is_active = active_car.as_ref().map(|c| c.id == car.id).unwrap_or(false);
                            ui.horizontal(|ui| {
                                let badge = if is_active { "★ ACTIVE" } else { "☆" };
                                let badge_color = if is_active { COLOR_ACCENT_GREEN } else { COLOR_TEXT_MUTED };
                                ui.label(RichText::new(badge).color(badge_color).strong());

                                ui.label(
                                    RichText::new(format!("{} {} {}", car.year, car.make, car.model))
                                        .color(COLOR_TEXT_PRIMARY)
                                        .strong(),
                                );

                                if let Some(ref eng) = car.engine {
                                    ui.label(RichText::new(format!("({})", eng)).color(COLOR_TEXT_SECONDARY).size(12.0));
                                }

                                if let Some(ref vin) = car.vin {
                                    ui.label(RichText::new(format!("VIN: {}", vin)).color(COLOR_TEXT_MUTED).size(12.0));
                                }

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if !is_active {
                                        if secondary_button(ui, "Activate").clicked() {
                                            if db.set_active_car(car.id).is_ok() {
                                                action = Some(LandingPageAction::CarSwitched(car));
                                            }
                                        }
                                    }
                                });
                            });
                            ui.separator();
                        }
                    });
                });
            }
            _ => {
                ui.label(
                    RichText::new("SAVED VEHICLES IN GARAGE")
                        .color(COLOR_TEXT_SECONDARY)
                        .size(12.0)
                        .strong(),
                );
                ui.add_space(6.0);

                card_frame(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("No vehicles currently saved in your garage.")
                                .color(COLOR_TEXT_MUTED)
                                .size(13.0),
                        );
                        ui.label(
                            RichText::new("Use the top navigation bar to select or discover a vehicle.")
                                .color(COLOR_TEXT_SECONDARY)
                                .size(12.0),
                        );
                    });
                });
            }
        }

        action
    }
}
