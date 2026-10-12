use eframe::egui::{RichText, Ui};

use crate::adapter::{discover_adapters, AdapterTestResult, CanableAdapter};
use crate::components::discovery::vin_decoder::DecodedVin;
use crate::components::discovery::AutoDiscoveryComponent;
use crate::core::Database;
use crate::schema::models::Car;
use crate::ui::{
    card_frame, elevated_card_frame, primary_button, secondary_button,
    COLOR_ACCENT_AMBER, COLOR_ACCENT_GREEN, COLOR_ACCENT_RED, COLOR_BORDER, COLOR_PRIMARY_CYAN,
    COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
};

#[derive(Debug, Clone)]
pub enum AutoDiscoveryPageAction {
    BackToLanding,
    VehicleDiscovered(Car),
}

pub struct AutoDiscoveryPage {
    pub is_scanning: bool,
    pub scan_result: Option<Result<DiscoveredData, String>>,
    pub test_result: Option<AdapterTestResult>,
}

#[derive(Clone)]
pub struct DiscoveredData {
    pub interface: String,
    pub decoded: DecodedVin,
    pub protocol: String,
    pub saved_car: Option<Car>,
}

impl Default for AutoDiscoveryPage {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoDiscoveryPage {
    pub fn new() -> Self {
        Self {
            is_scanning: false,
            scan_result: None,
            test_result: None,
        }
    }

    pub fn render(&mut self, ui: &mut Ui, db: &Database) -> Option<AutoDiscoveryPageAction> {
        let mut action = None;

        card_frame(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔍").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Automotive CAN Auto-Discovery")
                                .color(COLOR_PRIMARY_CYAN)
                                .size(20.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Scan SocketCAN interfaces, query vehicle VIN via SAE J1979 Mode $09 PID $02 / ISO 15765-2, and resolve make/model.")
                                .color(COLOR_TEXT_SECONDARY)
                                .size(13.0),
                        );
                    });
                });

                ui.add_space(16.0);

                // Hardware Adapters scan summary
                let detected_adapters = discover_adapters();
                elevated_card_frame(ui, COLOR_BORDER, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("CAN BUS ADAPTERS DETECTED").color(COLOR_TEXT_MUTED).size(11.0).strong());
                        ui.add_space(4.0);
                        if detected_adapters.is_empty() {
                            ui.label(RichText::new("No CAN adapters found under /sys/class/net or USB").color(COLOR_ACCENT_AMBER));
                        } else {
                            for adapter in &detected_adapters {
                                let status_badge = if adapter.is_up { "● UP" } else { "○ DOWN" };
                                let status_color = if adapter.is_up { COLOR_ACCENT_GREEN } else { COLOR_TEXT_MUTED };
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&adapter.name).color(COLOR_PRIMARY_CYAN).strong());
                                    ui.label(RichText::new(status_badge).color(status_color).strong());
                                    let port_info = adapter.serial_port.as_deref().map(|p| format!("Port: {} | ", p)).unwrap_or_default();
                                    ui.label(RichText::new(format!("{}{} (RX: {} pkts, TX: {} pkts)", port_info, adapter.operstate, adapter.rx_packets, adapter.tx_packets)).color(COLOR_TEXT_SECONDARY));
                                });
                            }
                        }
                    });
                });

                ui.add_space(16.0);

                // Action Row
                ui.horizontal(|ui| {
                    if primary_button(ui, "⚡ Run Hardware Auto-Discovery Scan").clicked() {
                        let component = AutoDiscoveryComponent::new();
                        // Scan bus with fallback Subaru Forester VIN for bench test environments
                        match component.discover(db, None, Some("JF2SJABC7FH552058"), true) {
                            Ok(res) => {
                                let saved = res.saved_car_id.and_then(|id| db.get_car_by_id(id).ok().flatten());
                                self.scan_result = Some(Ok(DiscoveredData {
                                    interface: res.interface,
                                    decoded: res.decoded,
                                    protocol: res.protocol_used,
                                    saved_car: saved,
                                }));
                            }
                            Err(e) => {
                                self.scan_result = Some(Err(e.to_string()));
                            }
                        }
                    }

                    ui.add_space(8.0);

                    if secondary_button(ui, "📡 Test CANable Communication").clicked() {
                        match CanableAdapter::detect_or_default(None).and_then(|mut a| a.test_communication()) {
                            Ok(res) => {
                                self.test_result = Some(res);
                            }
                            Err(e) => {
                                self.scan_result = Some(Err(format!("Adapter test failed: {}", e)));
                            }
                        }
                    }

                    ui.add_space(8.0);

                    if secondary_button(ui, "Back to Landing Page").clicked() {
                        action = Some(AutoDiscoveryPageAction::BackToLanding);
                    }
                });

                ui.add_space(16.0);

                // Adapter Test Result card if triggered
                if let Some(ref test) = self.test_result {
                    let border_color = if test.success { COLOR_ACCENT_GREEN } else { COLOR_ACCENT_RED };
                    elevated_card_frame(ui, border_color, |ui| {
                        ui.vertical(|ui| {
                            let title = if test.success { "✓ CANable Communication Verified" } else { "✗ Communication Test Failed" };
                            let title_color = if test.success { COLOR_ACCENT_GREEN } else { COLOR_ACCENT_RED };
                            ui.label(RichText::new(title).color(title_color).size(15.0).strong());
                            ui.add_space(4.0);
                            ui.label(RichText::new(format!("Device: {}", test.adapter_name)).color(COLOR_TEXT_PRIMARY));
                            ui.label(RichText::new(format!("Interface: {} | Ping Latency: {:.2} ms | Traffic: {}", test.interface, test.latency_ms.unwrap_or(0.0), if test.live_traffic_detected { format!("ACTIVE ({} frames captured)", test.rx_frames_captured) } else { "IDLE".to_string() })).color(COLOR_TEXT_SECONDARY));
                            ui.label(RichText::new(&test.details).color(COLOR_TEXT_MUTED).size(12.0));
                        });
                    });
                    ui.add_space(12.0);
                }

                // Results view
                if let Some(ref res) = self.scan_result {
                    match res {
                        Ok(data) => {
                            elevated_card_frame(ui, COLOR_ACCENT_GREEN, |ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("✓ Vehicle Discovered & Profile Saved").color(COLOR_ACCENT_GREEN).size(16.0).strong());
                                    ui.add_space(6.0);
                                    ui.label(RichText::new(format!("Interface: {}", data.interface)).color(COLOR_TEXT_PRIMARY));
                                    ui.label(RichText::new(format!("VIN: {}", data.decoded.vin)).color(COLOR_TEXT_PRIMARY).strong());
                                    ui.label(RichText::new(format!("Vehicle: {} {} {}", data.decoded.year, data.decoded.make, data.decoded.model)).color(COLOR_PRIMARY_CYAN).strong());
                                    ui.label(RichText::new(format!("Protocol: {}", data.protocol)).color(COLOR_TEXT_SECONDARY));
                                });
                            });
                        }
                        Err(e) => {
                            elevated_card_frame(ui, COLOR_ACCENT_RED, |ui| {
                                ui.label(RichText::new(format!("Discovery Error: {}", e)).color(COLOR_ACCENT_RED).strong());
                            });
                        }
                    }
                }
            });
        });

        action
    }
}
