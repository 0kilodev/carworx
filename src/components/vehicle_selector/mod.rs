use eframe::egui::{RichText, Ui};
use std::collections::HashSet;

use crate::components::Component;
use crate::config::vehicle::{MakeInfo, ModelInfo, VehicleManager};
use crate::core::Database;
use crate::error::{CarworxError, Result};
use crate::schema::models::{Car, NewCar};
use crate::ui::{
    card_frame, dropdown_select, labeled_text_box, primary_button, secondary_button,
    COLOR_ACCENT_GREEN, COLOR_ACCENT_RED, COLOR_PRIMARY_CYAN, COLOR_TEXT_MUTED,
    COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY,
};

/// Hardcoded list of supported years from 1991 to 2026 (descending)
pub fn hardcoded_years() -> Vec<u32> {
    (1991..=2026).rev().collect()
}

#[derive(Debug, Clone)]
pub enum VehicleSelectorAction {
    VehicleSaved(Car),
    VehicleSelected(Car),
    Cancelled,
}

pub struct VehicleSelectorComponent {
    // Data sources
    pub years: Vec<u32>,
    pub year_strings: Vec<String>,
    pub makes: Vec<MakeInfo>,
    pub make_names: Vec<String>,

    // Current selections
    pub selected_year_idx: Option<usize>,
    pub selected_make_idx: Option<usize>,
    pub selected_model_idx: Option<usize>,
    pub selected_engine_idx: Option<usize>,

    // Cached dependent options
    pub current_model_names: Vec<String>,
    pub current_engine_names: Vec<String>,

    // Optional inputs
    pub nickname: String,
    pub vin: String,

    // Feedback
    pub status_message: Option<(String, bool)>, // (message, is_error)
}

impl Default for VehicleSelectorComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for VehicleSelectorComponent {
    fn id(&self) -> &'static str {
        "vehicle_selector"
    }

    fn name(&self) -> &'static str {
        "Vehicle Selector"
    }

    fn description(&self) -> &'static str {
        "Configure and select vehicle profile (Year, Make, Model, Engine) from catalog or custom input"
    }
}

impl VehicleSelectorComponent {
    pub fn new() -> Self {
        let years = hardcoded_years();
        let year_strings = years.iter().map(|y| y.to_string()).collect();

        // Load makes from vehicles.yaml / catalog via VehicleManager
        let manager = VehicleManager::new();
        let makes = manager.list_makes().unwrap_or_default();
        let make_names = makes.iter().map(|m| m.name.clone()).collect();

        // Default selection: Subaru Forester 2015 if available
        let mut selected_year_idx = None;
        for (i, y) in years.iter().enumerate() {
            if *y == 2015 {
                selected_year_idx = Some(i);
                break;
            }
        }

        let mut selected_make_idx = None;
        for (i, m) in makes.iter().enumerate() {
            if m.id.eq_ignore_ascii_case("subaru") {
                selected_make_idx = Some(i);
                break;
            }
        }

        let mut component = Self {
            years,
            year_strings,
            makes,
            make_names,
            selected_year_idx,
            selected_make_idx,
            selected_model_idx: None,
            selected_engine_idx: None,
            current_model_names: Vec::new(),
            current_engine_names: Vec::new(),
            nickname: String::new(),
            vin: "JF2SJABC7FH552058".to_string(),
            status_message: None,
        };

        // Initialize dependent models and engines
        component.update_models();
        if let Some(make) = component.selected_make() {
            if let Some(forester_idx) = make.models.iter().position(|m| m.id.eq_ignore_ascii_case("forester")) {
                component.selected_model_idx = Some(forester_idx);
                component.update_engines();
                if !component.current_engine_names.is_empty() {
                    component.selected_engine_idx = Some(0);
                }
            }
        }
        component.update_default_nickname();

        component
    }

    pub fn selected_year(&self) -> Option<u32> {
        self.selected_year_idx.and_then(|i| self.years.get(i).copied())
    }

    pub fn selected_make(&self) -> Option<&MakeInfo> {
        self.selected_make_idx.and_then(|i| self.makes.get(i))
    }

    pub fn selected_model(&self) -> Option<&ModelInfo> {
        let make = self.selected_make()?;
        self.selected_model_idx.and_then(|i| make.models.get(i))
    }

    pub fn selected_engine(&self) -> Option<&String> {
        self.selected_engine_idx.and_then(|i| self.current_engine_names.get(i))
    }

    pub fn update_models(&mut self) {
        if let Some(make) = self.selected_make() {
            self.current_model_names = make.models.iter().map(|m| m.name.clone()).collect();
        } else {
            self.current_model_names.clear();
        }
        self.selected_model_idx = None;
        self.update_engines();
    }

    pub fn update_engines(&mut self) {
        self.current_engine_names.clear();
        self.selected_engine_idx = None;

        if let Some(model) = self.selected_model() {
            let year_opt = self.selected_year();

            // 1. Try to find engines matching the selected generation
            let mut matched_engines = Vec::new();
            if let Some(year) = year_opt {
                for gen in &model.generations {
                    if is_year_in_generation_range(year, &gen.years) {
                        for eng in &gen.engines {
                            if !matched_engines.contains(eng) {
                                matched_engines.push(eng.clone());
                            }
                        }
                    }
                }
            }

            // 2. If no exact year generation matched or none found, collect all engines for this model
            if matched_engines.is_empty() {
                let mut seen = HashSet::new();
                for gen in &model.generations {
                    for eng in &gen.engines {
                        if seen.insert(eng.clone()) {
                            matched_engines.push(eng.clone());
                        }
                    }
                }
            }

            if matched_engines.is_empty() {
                matched_engines.push("Standard Factory Engine".to_string());
            }

            self.current_engine_names = matched_engines;
        }
    }

    pub fn update_default_nickname(&mut self) {
        let year_str = self.selected_year().map(|y| y.to_string()).unwrap_or_default();
        let make_str = self.selected_make().map(|m| m.name.as_str()).unwrap_or_default();
        let model_str = self.selected_model().map(|m| m.name.as_str()).unwrap_or_default();

        if !make_str.is_empty() && !model_str.is_empty() {
            self.nickname = format!("{} {} {}", year_str, make_str, model_str).trim().to_string();
        }
    }

    /// Save the selected vehicle into the SQLite database and activate it
    pub fn save_to_database(&mut self, db: &Database) -> Result<Car> {
        let year = self.selected_year().ok_or_else(|| {
            CarworxError::validation("year", "Year is required. Please select a year from 1991 to 2026.")
        })?;

        let make = self.selected_make().ok_or_else(|| {
            CarworxError::validation("make", "Make is required. Please select a make.")
        })?;

        let model = self.selected_model().ok_or_else(|| {
            CarworxError::validation("model", "Model is required. Please select a model.")
        })?;

        let engine = self.selected_engine().cloned().unwrap_or_else(|| "Standard".to_string());

        let vin = if self.vin.trim().is_empty() {
            None
        } else {
            Some(self.vin.trim().to_string())
        };

        let nickname = if self.nickname.trim().is_empty() {
            format!("{} {} {}", year, make.name, model.name)
        } else {
            self.nickname.trim().to_string()
        };

        let new_car = NewCar {
            name: nickname,
            make: make.name.clone(),
            model: model.name.clone(),
            year: year as i32,
            trim: None,
            vin,
            engine: Some(engine),
            transmission: None,
            protocol: Some(make.default_protocol.clone()),
            notes: Some(format!("Configured via GUI Vehicle Selector ({})", make.default_protocol)),
        };

        let car = db.add_car(&new_car)?;
        db.set_active_car(car.id)?;

        Ok(car)
    }

    /// Render the interactive Vehicle Selector UI
    pub fn render(&mut self, ui: &mut Ui, db: &Database) -> Option<VehicleSelectorAction> {
        let mut action = None;

        card_frame(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🚙").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Vehicle Specification Selector")
                                .color(COLOR_PRIMARY_CYAN)
                                .size(20.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Select vehicle Year (1991-2026), Make, Model, and Engine from catalog.")
                                .color(COLOR_TEXT_SECONDARY)
                                .size(13.0),
                        );
                    });
                });

                ui.add_space(16.0);

                // Row 1: Year & Make Dropdowns
                ui.horizontal(|ui| {
                    // Year Dropdown (1991 to 2026)
                    let year_changed = dropdown_select(
                        ui,
                        "dropdown_year",
                        "Model Year (1991 - 2026)",
                        &mut self.selected_year_idx,
                        &self.year_strings,
                        "Select Year...",
                        180.0,
                    );
                    if year_changed {
                        self.update_engines();
                        self.update_default_nickname();
                    }

                    ui.add_space(20.0);

                    // Make Dropdown (loaded from vehicles.yaml)
                    let make_changed = dropdown_select(
                        ui,
                        "dropdown_make",
                        "Vehicle Make (Catalog)",
                        &mut self.selected_make_idx,
                        &self.make_names,
                        "Select Make...",
                        260.0,
                    );
                    if make_changed {
                        self.update_models();
                        self.update_default_nickname();
                    }
                });

                ui.add_space(12.0);

                // Row 2: Model & Engine Dropdowns
                ui.horizontal(|ui| {
                    // Model Dropdown (dependent on Make)
                    let model_changed = dropdown_select(
                        ui,
                        "dropdown_model",
                        "Vehicle Model",
                        &mut self.selected_model_idx,
                        &self.current_model_names,
                        if self.selected_make_idx.is_some() { "Select Model..." } else { "Select Make First" },
                        240.0,
                    );
                    if model_changed {
                        self.update_engines();
                        if !self.current_engine_names.is_empty() {
                            self.selected_engine_idx = Some(0);
                        }
                        self.update_default_nickname();
                    }

                    ui.add_space(20.0);

                    // Engine Dropdown (dependent on Model & Year)
                    dropdown_select(
                        ui,
                        "dropdown_engine",
                        "Engine Powertrain",
                        &mut self.selected_engine_idx,
                        &self.current_engine_names,
                        if self.selected_model_idx.is_some() { "Select Engine..." } else { "Select Model First" },
                        340.0,
                    );
                });

                ui.add_space(16.0);

                // Row 3: Nickname & VIN text boxes
                ui.horizontal(|ui| {
                    labeled_text_box(ui, "Vehicle Nickname", &mut self.nickname, "e.g. 2015 Subaru Forester");
                    ui.add_space(20.0);
                    labeled_text_box(ui, "VIN (Optional 17-digit)", &mut self.vin, "e.g. JF2SJABC7FH552058");
                });

                ui.add_space(16.0);

                // Summary Pill / Specs Preview
                if let (Some(year), Some(make), Some(model)) = (self.selected_year(), self.selected_make(), self.selected_model()) {
                    let engine_desc = self.selected_engine().map(|e| e.as_str()).unwrap_or("Standard Engine");
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✓ Configured Profile:").color(COLOR_ACCENT_GREEN).strong());
                            ui.label(
                                RichText::new(format!("{} {} {} - {}", year, make.name, model.name, engine_desc))
                                    .color(COLOR_TEXT_PRIMARY)
                                    .strong(),
                            );
                            ui.label(
                                RichText::new(format!("(Protocol: {}, {} bps)", make.default_protocol, make.default_bitrate))
                                    .color(COLOR_TEXT_MUTED)
                                    .size(12.0),
                            );
                        });
                    });
                    ui.add_space(10.0);
                }

                // Status / Error message banner
                if let Some((ref msg, is_err)) = self.status_message {
                    let color = if is_err { COLOR_ACCENT_RED } else { COLOR_ACCENT_GREEN };
                    ui.label(RichText::new(msg).color(color).strong());
                    ui.add_space(8.0);
                }

                // Action Buttons Row
                ui.horizontal(|ui| {
                    if primary_button(ui, "💾 Save & Activate Vehicle").clicked() {
                        match self.save_to_database(db) {
                            Ok(car) => {
                                self.status_message = Some((
                                    format!("Successfully saved and activated {} (ID: {})", car.make, car.id),
                                    false,
                                ));
                                action = Some(VehicleSelectorAction::VehicleSaved(car));
                            }
                            Err(e) => {
                                self.status_message = Some((e.to_string(), true));
                            }
                        }
                    }

                    ui.add_space(12.0);

                    if secondary_button(ui, "Back to Landing Page").clicked() {
                        action = Some(VehicleSelectorAction::Cancelled);
                    }
                });
            });
        });

        action
    }
}

/// Helper function to parse year range strings like "2014-2018", "2011+", or "2019-2024"
fn is_year_in_generation_range(year: u32, range_str: &str) -> bool {
    let clean = range_str.trim();
    if clean.ends_with('+') {
        let start_str = clean.trim_end_matches('+');
        if let Ok(start) = start_str.parse::<u32>() {
            return year >= start;
        }
    } else if clean.contains('-') {
        let parts: Vec<&str> = clean.split('-').collect();
        if parts.len() == 2 {
            if let (Ok(start), Ok(end)) = (parts[0].trim().parse::<u32>(), parts[1].trim().parse::<u32>()) {
                return year >= start && year <= end;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardcoded_years() {
        let years = hardcoded_years();
        assert_eq!(*years.first().unwrap(), 2026);
        assert_eq!(*years.last().unwrap(), 1991);
        assert_eq!(years.len(), 36);
    }

    #[test]
    fn test_is_year_in_generation_range() {
        assert!(is_year_in_generation_range(2015, "2014-2018"));
        assert!(is_year_in_generation_range(2014, "2014-2018"));
        assert!(is_year_in_generation_range(2018, "2014-2018"));
        assert!(!is_year_in_generation_range(2019, "2014-2018"));
        assert!(is_year_in_generation_range(2022, "2019-2024"));
        assert!(is_year_in_generation_range(2025, "2020+"));
    }

    #[test]
    fn test_vehicle_selector_init() {
        let selector = VehicleSelectorComponent::new();
        assert!(!selector.makes.is_empty(), "Should load makes from catalog");
        assert!(selector.selected_year().is_some());
    }
}
