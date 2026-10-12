pub mod calibrations;
pub mod diagnostic;
pub mod discovery;
pub mod vehicle_selector;

pub use calibrations::CalibrationComponent;
pub use diagnostic::DiagnosticComponent;
pub use discovery::AutoDiscoveryComponent;
pub use vehicle_selector::VehicleSelectorComponent;

/// Trait defining a pluggable feature component in Carworx
pub trait Component: Send + Sync {
    /// Unique identifier for the component (e.g. "auto_discovery")
    fn id(&self) -> &'static str;

    /// Human-readable title
    fn name(&self) -> &'static str;

    /// Detailed description of what the component does
    fn description(&self) -> &'static str;
}
