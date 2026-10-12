use serde::{Deserialize, Serialize};

/// Universal configuration for a vehicle calibration / adaptation routine loaded from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationRoutineConfig {
    pub id: String,
    pub name: String,
    pub domain: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    pub target_ecu: String,
    pub description: String,
    #[serde(default)]
    pub warning: Option<String>,
    #[serde(default)]
    pub operator_checklist: Vec<ChecklistItem>,
    #[serde(default)]
    pub telemetry_interlocks: Vec<TelemetryInterlock>,
    pub protocol: CalibrationProtocolConfig,
    pub sequence: Vec<CalibrationSequenceStep>,
    pub verification: CalibrationVerificationConfig,
}

/// Physical manual checklist item to be verified by technician
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub id: String,
    pub prompt: String,
    #[serde(default = "default_true")]
    pub required: bool,
}

/// Live sensor telemetry interlock bounds to verify before and during calibration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryInterlock {
    pub signal_name: String,
    pub source_can_id: String,
    #[serde(default)]
    pub sub_field: Option<String>,
    pub unit: String,
    pub min: f64,
    pub max: f64,
    pub tolerance_label: String,
    #[serde(default = "default_true")]
    pub abort_on_violation: bool,
    pub error_message: String,
}

/// Diagnostic protocol parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationProtocolConfig {
    #[serde(default = "default_uds")]
    pub standard: String,
    pub request_id: String,
    pub response_id: String,
    #[serde(default)]
    pub keep_alive: Option<KeepAliveConfig>,
    #[serde(default)]
    pub session: Option<SessionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepAliveConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_service_3e")]
    pub service: String,
    #[serde(default = "default_subfunction_00")]
    pub subfunction: String,
    #[serde(default = "default_keep_alive_interval")]
    pub interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    #[serde(rename = "type", default)]
    pub session_type: Option<String>,
    #[serde(default = "default_service_10")]
    pub service: String,
    #[serde(default = "default_subfunction_03")]
    pub subfunction: String,
    #[serde(default)]
    pub expected_response: Option<String>,
}

/// Step within the execution sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationSequenceStep {
    pub step_index: usize,
    pub title: String,
    #[serde(default = "default_action_routine")]
    pub action: String, // "routine_control", "send_raw", "write_did", "pause"
    pub payload_hex: String,
    pub expected_prefix_hex: String,
    #[serde(default = "default_true")]
    pub allow_response_pending: bool, // Handles NRC 0x78
    #[serde(default = "default_step_timeout")]
    pub timeout_ms: u64,
}

/// Post-procedure verification and user instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationVerificationConfig {
    #[serde(default)]
    pub read_dids: Vec<ReadDidConfig>,
    #[serde(default)]
    pub check_dtcs: Option<CheckDtcsConfig>,
    #[serde(default)]
    pub post_instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadDidConfig {
    pub did_hex: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckDtcsConfig {
    #[serde(default = "default_service_19")]
    pub service: String,
    #[serde(default = "default_subfunction_02")]
    pub subfunction: String,
    #[serde(default)]
    pub must_not_contain: Vec<String>,
    #[serde(default)]
    pub clean_label: String,
}

fn default_true() -> bool {
    true
}

fn default_uds() -> String {
    "UDS_ON_CAN".to_string()
}

fn default_service_3e() -> String {
    "0x3E".to_string()
}

fn default_subfunction_00() -> String {
    "0x00".to_string()
}

fn default_keep_alive_interval() -> u64 {
    2000
}

fn default_service_10() -> String {
    "0x10".to_string()
}

fn default_subfunction_03() -> String {
    "0x03".to_string()
}

fn default_action_routine() -> String {
    "routine_control".to_string()
}

fn default_step_timeout() -> u64 {
    5000
}

fn default_service_19() -> String {
    "0x19".to_string()
}

fn default_subfunction_02() -> String {
    "0x02".to_string()
}

#[cfg(test)]
mod tests {
    use crate::config::VehicleManager;

    #[test]
    fn test_forester_calibrations_schema() {
        let manager = VehicleManager::new();
        let profile = manager
            .load_model_profile("Subaru", "Forester")
            .expect("Forester profile should load");

        let cals = profile.calibrations.expect("Calibrations should be present in Forester profile");
        assert!(cals.contains_key("vdc_centering"), "Should contain vdc_centering");
        assert!(cals.contains_key("epb_maintenance"), "Should contain epb_maintenance");
        assert!(cals.contains_key("cvt_oil_pressure"), "Should contain cvt_oil_pressure");

        let vdc = &cals["vdc_centering"];
        assert_eq!(vdc.target_ecu, "vdc");
        assert_eq!(vdc.protocol.request_id, "0x7B0");
        assert_eq!(vdc.protocol.response_id, "0x7B8");
        assert_eq!(vdc.telemetry_interlocks.len(), 4, "Should have 4 telemetry interlocks for VDC centering");
    }
}
