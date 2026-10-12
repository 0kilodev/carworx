use serde::{Deserialize, Serialize};

/// Representation of a registered vehicle profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Car {
    pub id: i64,
    pub name: String,
    pub make: String,
    pub model: String,
    pub year: i32,
    pub trim: Option<String>,
    pub vin: Option<String>,
    pub engine: Option<String>,
    pub transmission: Option<String>,
    pub protocol: String,
    pub is_active: bool,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Helper struct for creating a new vehicle profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCar {
    pub name: String,
    pub make: String,
    pub model: String,
    pub year: i32,
    pub trim: Option<String>,
    pub vin: Option<String>,
    pub engine: Option<String>,
    pub transmission: Option<String>,
    pub protocol: Option<String>,
    pub notes: Option<String>,
}

/// Representation of a hardware interface (SocketCAN, slcan, virtual)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInterface {
    pub id: i64,
    pub name: String,
    pub interface_type: String,
    pub bitrate: u32,
    pub is_active: bool,
    pub notes: Option<String>,
    pub last_seen_at: Option<String>,
}

/// Representation of a universal vehicle configuration profile (e.g. CAN map, DTC catalog)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarConfig {
    pub id: i64,
    pub car_id: Option<i64>,
    pub name: String,
    pub config_type: String,
    pub file_path: String,
    pub description: Option<String>,
    pub is_enabled: bool,
    pub created_at: String,
}

/// Helper struct for adding a new configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCarConfig {
    pub car_id: Option<i64>,
    pub name: String,
    pub config_type: String,
    pub file_path: String,
    pub description: Option<String>,
}
