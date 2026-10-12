use serde::{Deserialize, Serialize};

/// The 6 automotive system domains
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DtcDomain {
    Engine,
    Transmission,
    Abs,
    Steering,
    Body,
    Safety,
}

impl DtcDomain {
    /// Human-readable title of the domain
    pub fn name(&self) -> &'static str {
        match self {
            DtcDomain::Engine => "Engine / Powertrain (ECM)",
            DtcDomain::Transmission => "Transmission Control (TCM)",
            DtcDomain::Abs => "ABS / VDC & Braking",
            DtcDomain::Steering => "Electric Power Steering (EPS)",
            DtcDomain::Body => "Body & Electronics (BIU)",
            DtcDomain::Safety => "Safety & Restraints (Airbag/SRS)",
        }
    }

    /// Display emoji icon for UI rendering
    pub fn icon(&self) -> &'static str {
        match self {
            DtcDomain::Engine => "🏎️",
            DtcDomain::Transmission => "⚙️",
            DtcDomain::Abs => "🛑",
            DtcDomain::Steering => "🔄",
            DtcDomain::Body => "🚘",
            DtcDomain::Safety => "🛡️",
        }
    }

    /// Short identifier string
    pub fn code(&self) -> &'static str {
        match self {
            DtcDomain::Engine => "engine",
            DtcDomain::Transmission => "transmission",
            DtcDomain::Abs => "abs",
            DtcDomain::Steering => "steering",
            DtcDomain::Body => "body",
            DtcDomain::Safety => "safety",
        }
    }
}

impl std::fmt::Display for DtcDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// An individual Diagnostic Trouble Code (DTC)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DtcCode {
    /// Standardized alphanumeric DTC (e.g. "P2404", "C1422", "U0073")
    pub code: String,
    /// Raw hexadecimal payload bytes from the ECU response
    pub raw_hex: String,
    /// Detailed description of the fault condition
    pub description: String,
    /// UDS status byte mask (0x00 - 0xFF)
    pub status_byte: u8,
    /// Whether this is a confirmed fault (bit 0x08)
    pub is_confirmed: bool,
    /// Whether this fault is pending (bit 0x04)
    pub is_pending: bool,
    /// Whether this fault failed during the current or recent test (bit 0x01)
    pub is_test_failed: bool,
    /// Decomposed UDS status flags
    pub status_labels: Vec<String>,
    /// Associated vehicle domain
    pub domain: String,
    /// Specific ECU name
    pub ecu_name: String,
}

/// Grouping of confirmed DTCs within a specific vehicle domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DtcDomainGroup {
    pub domain: DtcDomain,
    pub domain_name: String,
    pub ecu_name: String,
    pub request_id: u32,
    pub response_id: u32,
    pub is_online: bool,
    /// Filtered list of ONLY confirmed DTCs for this domain
    pub confirmed_dtcs: Vec<DtcCode>,
}

impl DtcDomainGroup {
    pub fn new(
        domain: DtcDomain,
        ecu_name: impl Into<String>,
        request_id: u32,
        response_id: u32,
        is_online: bool,
        confirmed_dtcs: Vec<DtcCode>,
    ) -> Self {
        Self {
            domain_name: domain.name().to_string(),
            domain,
            ecu_name: ecu_name.into(),
            request_id,
            response_id,
            is_online,
            confirmed_dtcs,
        }
    }

    /// Whether this domain has any confirmed DTCs
    pub fn has_faults(&self) -> bool {
        !self.confirmed_dtcs.is_empty()
    }

    /// Number of confirmed DTCs in this domain
    pub fn fault_count(&self) -> usize {
        self.confirmed_dtcs.len()
    }
}

/// Full diagnostic scan report across all domains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticScanResult {
    pub timestamp: String,
    pub car_id: i64,
    pub car_name: String,
    pub interface: String,
    pub total_confirmed_dtcs: usize,
    pub domains: Vec<DtcDomainGroup>,
    pub is_live: bool,
    pub notes: String,
}

impl DiagnosticScanResult {
    pub fn total_faults(&self) -> usize {
        self.total_confirmed_dtcs
    }

    pub fn is_clean(&self) -> bool {
        self.total_confirmed_dtcs == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_six_automotive_domains() {
        let domains = [
            DtcDomain::Engine,
            DtcDomain::Transmission,
            DtcDomain::Abs,
            DtcDomain::Steering,
            DtcDomain::Body,
            DtcDomain::Safety,
        ];

        assert_eq!(domains.len(), 6);
        assert_eq!(domains[0].code(), "engine");
        assert_eq!(domains[1].code(), "transmission");
        assert_eq!(domains[2].code(), "abs");
        assert_eq!(domains[3].code(), "steering");
        assert_eq!(domains[4].code(), "body");
        assert_eq!(domains[5].code(), "safety");

        for d in &domains {
            assert!(!d.name().is_empty());
            assert!(!d.icon().is_empty());
        }
    }

    #[test]
    fn test_domain_group_confirmed_fault_filtering() {
        let group_empty = DtcDomainGroup::new(
            DtcDomain::Engine,
            "ECM",
            0x7E0,
            0x7E8,
            true,
            vec![],
        );
        assert!(!group_empty.has_faults());
        assert_eq!(group_empty.fault_count(), 0);

        let dtc = DtcCode {
            code: "P2404".to_string(),
            raw_hex: "002404".to_string(),
            description: "Evaporative Emission System Leak Detection".to_string(),
            status_byte: 0x28,
            is_confirmed: true,
            is_pending: false,
            is_test_failed: false,
            status_labels: vec!["confirmedDTC".to_string()],
            domain: DtcDomain::Engine.name().to_string(),
            ecu_name: "ECM".to_string(),
        };

        let group_with_fault = DtcDomainGroup::new(
            DtcDomain::Engine,
            "ECM",
            0x7E0,
            0x7E8,
            true,
            vec![dtc],
        );
        assert!(group_with_fault.has_faults());
        assert_eq!(group_with_fault.fault_count(), 1);
        assert!(group_with_fault.confirmed_dtcs[0].is_confirmed);
    }
}

