use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// Status bitfield constants for ISO 14229 UDS DTC status byte
pub const UDS_STATUS_TEST_FAILED: u8 = 0x01;
pub const UDS_STATUS_TEST_FAILED_THIS_CYCLE: u8 = 0x02;
pub const UDS_STATUS_PENDING: u8 = 0x04;
pub const UDS_STATUS_CONFIRMED: u8 = 0x08;
pub const UDS_STATUS_NOT_COMPLETED_SINCE_CLEAR: u8 = 0x10;
pub const UDS_STATUS_FAILED_SINCE_CLEAR: u8 = 0x20;
pub const UDS_STATUS_NOT_COMPLETED_THIS_CYCLE: u8 = 0x40;
pub const UDS_STATUS_WARNING_INDICATOR_REQ: u8 = 0x80;

/// Decode a 2-byte raw SAE J2012 diagnostic trouble code
pub fn decode_obd_code(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 2 {
        return None;
    }
    let first = bytes[0];
    let second = bytes[1];

    let family_char = match (first >> 6) & 0x03 {
        0 => 'P', // Powertrain
        1 => 'C', // Chassis
        2 => 'B', // Body
        3 => 'U', // Network
        _ => 'P',
    };

    let digit1 = (first >> 4) & 0x03;
    let digit2 = first & 0x0F;
    Some(format!("{}{:X}{:X}{:02X}", family_char, digit1, digit2, second))
}

/// Decode a 2-byte or 3-byte UDS diagnostic trouble code
///
/// On Subaru ECUs, a leading 0x00 byte often precedes the standard 2-byte J2012 code.
pub fn decode_uds_code(bytes: &[u8]) -> String {
    if bytes.len() == 3 {
        if bytes[0] == 0x00 {
            // Subaru leading 0x00 padding
            if let Some(code) = decode_obd_code(&bytes[1..3]) {
                return code;
            }
        } else if let Some(base) = decode_obd_code(&bytes[0..2]) {
            // Standard UDS: 2-byte base code + 1-byte failure symptom/subtype
            return format!("{}-{:02X}", base, bytes[2]);
        }
    } else if bytes.len() == 2 {
        if let Some(code) = decode_obd_code(bytes) {
            return code;
        }
    }

    // Fallback hex
    format!("0x{}", bytes.iter().map(|b| format!("{:02X}", b)).collect::<String>())
}

/// Decompose UDS status mask byte into human-readable flags
pub fn decode_uds_status_flags(status_byte: u8) -> Vec<String> {
    let mut flags = Vec::new();
    if status_byte & UDS_STATUS_TEST_FAILED != 0 {
        flags.push("testFailed".to_string());
    }
    if status_byte & UDS_STATUS_TEST_FAILED_THIS_CYCLE != 0 {
        flags.push("testFailedThisCycle".to_string());
    }
    if status_byte & UDS_STATUS_PENDING != 0 {
        flags.push("pendingDTC".to_string());
    }
    if status_byte & UDS_STATUS_CONFIRMED != 0 {
        flags.push("confirmedDTC".to_string());
    }
    if status_byte & UDS_STATUS_NOT_COMPLETED_SINCE_CLEAR != 0 {
        flags.push("notCompletedSinceClear".to_string());
    }
    if status_byte & UDS_STATUS_FAILED_SINCE_CLEAR != 0 {
        flags.push("testFailedSinceClear".to_string());
    }
    if status_byte & UDS_STATUS_NOT_COMPLETED_THIS_CYCLE != 0 {
        flags.push("notCompletedThisCycle".to_string());
    }
    if status_byte & UDS_STATUS_WARNING_INDICATOR_REQ != 0 {
        flags.push("warningIndicatorRequested".to_string());
    }
    flags
}

/// Global cached DTC descriptions dictionary
static DTC_CATALOG: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Loads and returns the DTC description dictionary
pub fn get_dtc_description(code: &str) -> String {
    let catalog = DTC_CATALOG.get_or_init(|| {
        let mut map = HashMap::new();

        // 1. Preload common verified Subaru Forester DTCs
        map.insert("P2404".to_string(), "Evaporative Emission System Leak Detection Pump Sense Circuit Range/Performance".to_string());
        map.insert("P2419".to_string(), "Evaporative Emission System Switching Valve Control Circuit Low".to_string());
        map.insert("U0073".to_string(), "Control Module Communication Bus Off".to_string());
        map.insert("U0100".to_string(), "Lost Communication with Engine Control Module (ECM)".to_string());
        map.insert("U0101".to_string(), "Lost Communication with Transmission Control Module (TCM)".to_string());
        map.insert("U0122".to_string(), "Lost Communication with Vehicle Dynamics Control (VDC) Module".to_string());
        map.insert("U0126".to_string(), "Lost Communication with Steering Angle Sensor Module".to_string());
        map.insert("U0131".to_string(), "Lost Communication with Power Steering Control Module (EPS)".to_string());
        map.insert("U0140".to_string(), "Lost Communication with Body Integrated Unit (BIU)".to_string());
        map.insert("U0155".to_string(), "Lost Communication with Instrument Panel Cluster (IPC)".to_string());
        map.insert("U1201".to_string(), "CAN Communication Failure / Verification Error".to_string());
        map.insert("C1422".to_string(), "VDC Interruption by Engine Fail / Signal Check".to_string());
        map.insert("C2543".to_string(), "Electric Power Steering Control Unit / Interruption Code".to_string());
        map.insert("B1407".to_string(), "Keyless / Immobilizer Communication Circuit Malfunction".to_string());
        map.insert("P0300".to_string(), "Random/Multiple Cylinder Misfire Detected".to_string());
        map.insert("P0420".to_string(), "Catalyst System Efficiency Below Threshold (Bank 1)".to_string());

        // 2. Load from disk if catalog JSON exists
        let paths = [
            "config/vehicles/subaru/forester_2015_dtc_catalog.json",
            "../config/vehicles/subaru/forester_2015_dtc_catalog.json",
            "raw/dtc/forester_2015_dtc_catalog.json",
        ];

        for p in paths {
            if Path::new(p).exists() {
                if let Ok(content) = fs::read_to_string(p) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(list) = val.get("dtcs").and_then(|d| d.as_array()) {
                            for item in list {
                                if let (Some(c), Some(desc)) = (
                                    item.get("code").and_then(|s| s.as_str()),
                                    item.get("description").or_else(|| item.get("item")).and_then(|s| s.as_str()),
                                ) {
                                    map.insert(c.to_string(), desc.to_string());
                                }
                            }
                        }
                    }
                }
                break;
            }
        }

        map
    });

    // Check exact code or base code (e.g. without subtype)
    if let Some(desc) = catalog.get(code) {
        return desc.clone();
    }

    if let Some((base, _)) = code.split_once('-') {
        if let Some(desc) = catalog.get(base) {
            return desc.clone();
        }
    }

    // Default description based on family prefix
    let prefix = code.chars().next().unwrap_or(' ');
    match prefix {
        'P' => "Powertrain System Trouble Code".to_string(),
        'C' => "Chassis / Braking System Trouble Code".to_string(),
        'B' => "Body Electrical System Trouble Code".to_string(),
        'U' => "Network / Inter-ECU Bus Communication Trouble Code".to_string(),
        _ => "Diagnostic Trouble Code".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_obd_codes() {
        // P2404: 0x24 0x04 -> P2404
        // first: 0x24 -> (0x24 >> 6) = 0 ('P'), (0x24 >> 4) & 0x03 = 2, 0x24 & 0x0F = 4 -> P2404
        let code = decode_obd_code(&[0x24, 0x04]).unwrap();
        assert_eq!(code, "P2404");

        // U0073: 'U' is (3 << 6) | 0x00 = 0xC0
        let code_u = decode_obd_code(&[0xC0, 0x73]).unwrap();
        assert_eq!(code_u, "U0073");
    }

    #[test]
    fn test_decode_subaru_uds_padding() {
        let code = decode_uds_code(&[0x00, 0x24, 0x04]);
        assert_eq!(code, "P2404");
    }

    #[test]
    fn test_status_flags() {
        let flags = decode_uds_status_flags(0x2F);
        assert!(flags.contains(&"confirmedDTC".to_string()));
        assert!(flags.contains(&"pendingDTC".to_string()));
        assert!(flags.contains(&"testFailed".to_string()));
    }
}
