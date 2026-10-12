use serde::{Deserialize, Serialize};

use crate::components::discovery::vin_decoder::{DecodedVin, VinDecoder};
use crate::components::Component;
use crate::core::hardware::detect_can_interfaces;
use crate::core::isotp::query_live_vin;
use crate::core::Database;
use crate::error::{CarworxError, Result};
use crate::schema::models::NewCar;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDiscoveryResult {
    pub interface: String,
    pub interface_state: String,
    #[serde(default)]
    pub adapter_name: Option<String>,
    pub decoded: DecodedVin,
    pub protocol_used: String,
    pub saved_car_id: Option<i64>,
}

pub struct AutoDiscoveryComponent;

impl Default for AutoDiscoveryComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoDiscoveryComponent {
    pub fn new() -> Self {
        Self
    }

    /// Automatically discovers CAN hardware interface, queries vehicle metadata via SAE J1979 / UDS,
    /// and optionally saves the discovered car and active interface into the database.
    pub fn discover(
        &self,
        db: &Database,
        specified_iface: Option<&str>,
        fallback_vin: Option<&str>,
        save_to_db: bool,
    ) -> Result<AutoDiscoveryResult> {
        let detected = detect_can_interfaces();

        // 1. Resolve target interface
        let iface = if let Some(name) = specified_iface {
            detected
                .into_iter()
                .find(|d| d.name == name)
                .ok_or_else(|| CarworxError::hardware_not_found(name))?
        } else {
            // Find first interface that is UP, with preference for 'can0'
            let mut candidates: Vec<_> = detected.into_iter().filter(|d| d.is_up).collect();
            if candidates.is_empty() {
                return Err(CarworxError::hardware_not_found(
                    "No active CAN interfaces found under /sys/class/net. Connect adapter or run 'ip link set can0 up'.",
                ));
            }
            // Move can0 to front if present
            candidates.sort_by(|a, b| {
                if a.name == "can0" {
                    std::cmp::Ordering::Less
                } else if b.name == "can0" {
                    std::cmp::Ordering::Greater
                } else {
                    a.name.cmp(&b.name)
                }
            });
            candidates.remove(0)
        };

        // 2. Query vehicle VIN over the live CAN bus
        let (vin_str, protocol_used) = match query_live_vin(&iface.name, 1000) {
            Ok(vin) => (vin, "SAE J1979 Mode $09 PID $02 (ISO 15765-2 / UDS)".to_string()),
            Err(e) => {
                // If bus is not actively responding (e.g. key in off position, or offline bench),
                // check if fallback VIN was provided
                if let Some(fvin) = fallback_vin {
                    (
                        fvin.to_string(),
                        format!("Target Fallback Specification ({})", e),
                    )
                } else {
                    return Err(e);
                }
            }
        };

        // 3. Decode VIN using universal ISO 3779 / NHTSA decoder
        let decoded = VinDecoder::decode(&vin_str)?;

        // 4. Save to database if requested
        let mut saved_car_id = None;
        if save_to_db {
            // Register hardware interface
            db.register_interface(&iface.name, "socketcan", 500000, true, None)?;

            let profile_name = format!("{} {} ({})", decoded.make, decoded.model, decoded.year);
            let new_car = NewCar {
                name: profile_name,
                make: decoded.make.clone(),
                model: decoded.model.clone(),
                year: decoded.year,
                trim: decoded.body_type.clone(),
                vin: Some(decoded.vin.clone()),
                engine: Some("Auto-Detected Powertrain".to_string()),
                transmission: None,
                protocol: Some("ISO 15765-2 / CAN 500k (SAE J1979 / UDS)".to_string()),
                notes: Some(format!(
                    "Auto-discovered via {} using protocol: {}",
                    iface.name, protocol_used
                )),
            };

            let car = db.add_car(&new_car)?;
            db.set_active_car(car.id)?;
            saved_car_id = Some(car.id);
        }

        // Discover adapter details
        let adapters = crate::adapter::discover_adapters();
        let adapter_name = adapters
            .iter()
            .find(|a| a.interface == iface.name)
            .map(|a| a.name.clone());

        Ok(AutoDiscoveryResult {
            interface: iface.name,
            interface_state: iface.operstate,
            adapter_name,
            decoded,
            protocol_used,
            saved_car_id,
        })
    }
}

impl Component for AutoDiscoveryComponent {
    fn id(&self) -> &'static str {
        "auto_discovery"
    }

    fn name(&self) -> &'static str {
        "Vehicle & Hardware Auto-Discovery"
    }

    fn description(&self) -> &'static str {
        "Scans for active CAN hardware interfaces and retrieves vehicle metadata (VIN, Make, Model, Year) over CAN"
    }
}
