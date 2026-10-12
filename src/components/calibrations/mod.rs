pub mod executor;
pub mod schema;
pub mod telemetry;


pub use executor::{
    evaluate_interlocks, execute_calibration_sequence, CalibrationExecutionResult,
    ExecutionLogEntry, InterlockStatus,
};
pub use schema::{
    CalibrationProtocolConfig, CalibrationRoutineConfig, CalibrationSequenceStep,
    CalibrationVerificationConfig, ChecklistItem, TelemetryInterlock,
};

use crate::components::Component;
use crate::config::vehicle::VehicleManager;
use crate::schema::models::Car;

pub struct CalibrationComponent;

impl Default for CalibrationComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl CalibrationComponent {
    pub fn new() -> Self {
        Self
    }

    /// Load all calibration routines defined for a specific car
    pub fn load_routines_for_car(car: &Car) -> Vec<CalibrationRoutineConfig> {
        let manager = VehicleManager::new();
        if let Ok(profile) = manager.load_model_profile(&car.make, &car.model) {
            if let Some(calibrations_val) = profile.calibrations {
                let mut routines: Vec<CalibrationRoutineConfig> = calibrations_val.into_values().collect();
                routines.sort_by(|a, b| a.name.cmp(&b.name));
                return routines;
            }
        }
        Vec::new()
    }
}

impl Component for CalibrationComponent {
    fn id(&self) -> &'static str {
        "calibrations"
    }

    fn name(&self) -> &'static str {
        "Vehicle Calibrations & Adaptations"
    }

    fn description(&self) -> &'static str {
        "Universal schema-driven vehicle calibration engine for zero-points, sensor learning, and actuator maintenance."
    }
}
