use eframe::egui::{self, CentralPanel, RichText, TopBottomPanel};

use crate::components::vehicle_selector::{VehicleSelectorAction, VehicleSelectorComponent};
use crate::core::Database;
use crate::pages::{
    AutoDiscoveryPage, AutoDiscoveryPageAction, CalibrationPage, CalibrationPageAction,
    DiagnosticPage, DiagnosticPageAction, LandingPage, LandingPageAction, Page,
};
use crate::ui::{
    apply_automotive_theme, COLOR_ACCENT_GREEN, COLOR_PRIMARY_CYAN, COLOR_SURFACE,
    COLOR_TEXT_MUTED,
};

pub struct CarworxApp {
    pub db: Database,
    pub current_page: Page,
    pub landing_page: LandingPage,
    pub vehicle_selector: VehicleSelectorComponent,
    pub auto_discovery_page: AutoDiscoveryPage,
    pub diagnostic_page: DiagnosticPage,
    pub calibration_page: CalibrationPage,
}

use crate::components::diagnostic::perform_diagnostic_scan;

impl CarworxApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        db: Database,
        initial_page: Option<Page>,
        auto_scan: bool,
        routine: Option<String>,
        confirm: bool,
    ) -> Self {
        apply_automotive_theme(&cc.egui_ctx);

        // Start background CAN Telemetry Service on the active CAN interface immediately
        let ifaces = crate::core::hardware::detect_can_interfaces();
        let target_iface = ifaces.iter().find(|i| i.is_up).map(|i| i.name.as_str()).unwrap_or("can0");
        crate::components::calibrations::telemetry::get_telemetry_service().start(target_iface);

        let mut diagnostic_page = DiagnosticPage::new();
        if auto_scan {
            if let Ok(Some(ref car)) = db.get_active_car() {
                let iface = "can0";
                if let Ok(res) = perform_diagnostic_scan(car, iface) {
                    diagnostic_page.scan_result = Some(Ok(res));
                }
            }
        }

        let mut calibration_page = CalibrationPage::new();
        if let Some(ref routine_query) = routine {
            if let Ok(Some(ref car)) = db.get_active_car() {
                let routines = crate::components::calibrations::CalibrationComponent::load_routines_for_car(car);
                if let Some(target) = routines.iter().find(|r| {
                    r.id.eq_ignore_ascii_case(routine_query)
                        || r.name.to_lowercase().contains(&routine_query.to_lowercase())
                }) {
                    calibration_page.select_routine(target.clone());
                    if confirm {
                        calibration_page.show_confirm_dialog = true;
                    }
                }
            }
        }

        Self {
            db,
            current_page: initial_page.unwrap_or(Page::Landing),
            landing_page: LandingPage::new(),
            vehicle_selector: VehicleSelectorComponent::new(),
            auto_discovery_page: AutoDiscoveryPage::new(),
            diagnostic_page,
            calibration_page,
        }
    }
}

impl eframe::App for CarworxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request periodic repaint every 500ms for live CAN hardware & bus telemetry monitoring
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // TOP NAVIGATION BAR
        TopBottomPanel::top("top_nav_bar")
            .frame(egui::Frame::none().fill(COLOR_SURFACE).inner_margin(egui::Margin::symmetric(16.0_f32, 10.0_f32)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🚗").size(20.0));
                    ui.label(
                        RichText::new("CARWORX")
                            .color(COLOR_PRIMARY_CYAN)
                            .size(16.0)
                            .strong(),
                    );

                    ui.add_space(20.0);

                    // Navigation buttons
                    if ui.selectable_label(self.current_page == Page::Landing, "🏠 Landing Page").clicked() {
                        self.current_page = Page::Landing;
                    }

                    if ui.selectable_label(self.current_page == Page::VehicleSelector, "🚙 Select Vehicle").clicked() {
                        self.current_page = Page::VehicleSelector;
                    }

                    if ui.selectable_label(self.current_page == Page::AutoDiscovery, "🔍 Auto Discovery").clicked() {
                        self.current_page = Page::AutoDiscovery;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Ok(Some(car)) = self.db.get_active_car() {
                            ui.label(
                                RichText::new(format!("● {} {} {}", car.year, car.make, car.model))
                                    .color(COLOR_ACCENT_GREEN)
                                    .size(12.0)
                                    .strong(),
                            );
                        } else {
                            ui.label(RichText::new("○ No Active Vehicle").color(COLOR_TEXT_MUTED).size(12.0));
                        }
                    });
                });
            });

        // BOTTOM STATUS BAR
        TopBottomPanel::bottom("bottom_status_bar")
            .frame(egui::Frame::none().fill(COLOR_SURFACE).inner_margin(egui::Margin::symmetric(16.0_f32, 6.0_f32)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Database: {}", self.db.path().display()))
                            .color(COLOR_TEXT_MUTED)
                            .size(11.0),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("CLI Mode: carworx --cli | Headless: carworx --headless")
                                .color(COLOR_TEXT_MUTED)
                                .size(11.0),
                        );
                    });
                });
            });

        // CENTRAL CONTENT PANEL
        CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.current_page {
                    Page::Landing => {
                        if let Some(action) = self.landing_page.render(ui, &self.db) {
                            match action {
                                LandingPageAction::NavigateToSelectVehicle => {
                                    self.current_page = Page::VehicleSelector;
                                }
                                LandingPageAction::NavigateToAutoDiscovery => {
                                    self.current_page = Page::AutoDiscovery;
                                }
                                LandingPageAction::NavigateToDiagnostics => {
                                    self.current_page = Page::Diagnostics;
                                }
                                LandingPageAction::NavigateToCalibrations => {
                                    self.current_page = Page::Calibrations;
                                }
                                LandingPageAction::CarSwitched(_) => {
                                    // Refresh active vehicle state
                                }
                            }
                        }
                    }
                    Page::VehicleSelector => {
                        if let Some(action) = self.vehicle_selector.render(ui, &self.db) {
                            match action {
                                VehicleSelectorAction::VehicleSaved(_) => {
                                    self.current_page = Page::Landing;
                                }
                                VehicleSelectorAction::VehicleSelected(_) => {
                                    self.current_page = Page::Landing;
                                }
                                VehicleSelectorAction::Cancelled => {
                                    self.current_page = Page::Landing;
                                }
                            }
                        }
                    }
                    Page::AutoDiscovery => {
                        if let Some(action) = self.auto_discovery_page.render(ui, &self.db) {
                            match action {
                                AutoDiscoveryPageAction::BackToLanding => {
                                    self.current_page = Page::Landing;
                                }
                                AutoDiscoveryPageAction::VehicleDiscovered(_) => {
                                    self.current_page = Page::Landing;
                                }
                            }
                        }
                    }
                    Page::Diagnostics => {
                        if let Some(action) = self.diagnostic_page.render(ui, &self.db) {
                            match action {
                                DiagnosticPageAction::NavigateToSelectVehicle => {
                                    self.current_page = Page::VehicleSelector;
                                }
                                DiagnosticPageAction::NavigateToAutoDiscovery => {
                                    self.current_page = Page::AutoDiscovery;
                                }
                            }
                        }
                    }
                    Page::Calibrations => {
                        // Request periodic UI repaint (20 FPS) so live sensor telemetry animates smoothly
                        ctx.request_repaint_after(std::time::Duration::from_millis(50));

                        if let Some(action) = self.calibration_page.render(ui, &self.db) {
                            match action {
                                CalibrationPageAction::NavigateToSelectVehicle => {
                                    self.current_page = Page::VehicleSelector;
                                }
                                CalibrationPageAction::NavigateToAutoDiscovery => {
                                    self.current_page = Page::AutoDiscovery;
                                }
                            }
                        }
                    }
                }
            });
        });
    }
}
