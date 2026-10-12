pub mod decoder;
pub mod scanner;
pub mod schema;

pub use decoder::{decode_obd_code, decode_uds_code, get_dtc_description};
pub use scanner::{
    clear_vehicle_dtcs, perform_diagnostic_scan, resolve_vehicle_targets, ClearDtcDomainResult,
    ClearDtcReport, DiagnosticTarget, PRIMARY_DOMAIN_TARGETS,
};
pub use schema::{DiagnosticScanResult, DtcCode, DtcDomain, DtcDomainGroup};

use crate::components::Component;

/// Component for querying, decoding, and displaying Diagnostic Trouble Codes (DTCs)
pub struct DiagnosticComponent;

impl DiagnosticComponent {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DiagnosticComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for DiagnosticComponent {
    fn id(&self) -> &'static str {
        "diagnostic"
    }

    fn name(&self) -> &'static str {
        "Diagnostic Trouble Codes (DTC)"
    }

    fn description(&self) -> &'static str {
        "Query and decode confirmed Diagnostic Trouble Codes across 6 automotive domains via UDS and OBD-II"
    }
}
