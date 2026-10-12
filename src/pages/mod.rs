pub mod auto_discovery_page;
pub mod calibration_page;
pub mod diagnostic_page;
pub mod landing_page;

pub use auto_discovery_page::{AutoDiscoveryPage, AutoDiscoveryPageAction};
pub use calibration_page::{CalibrationPage, CalibrationPageAction};
pub use diagnostic_page::{DiagnosticPage, DiagnosticPageAction};
pub use landing_page::{LandingPage, LandingPageAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Landing,
    VehicleSelector,
    AutoDiscovery,
    Diagnostics,
    Calibrations,
}

