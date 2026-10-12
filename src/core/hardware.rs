use crate::error::{CarworxError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Information about a detected system network interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedInterface {
    pub name: String,
    pub is_can: bool,
    pub is_up: bool,
    pub operstate: String,
    pub bitrate: Option<u32>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

/// Detects all CAN and network interfaces available on the Linux host
pub fn detect_can_interfaces() -> Vec<DetectedInterface> {
    let mut interfaces = Vec::new();
    let net_dir = Path::new("/sys/class/net");

    if !net_dir.exists() || !net_dir.is_dir() {
        return interfaces;
    }

    let entries = match fs::read_dir(net_dir) {
        Ok(e) => e,
        Err(_) => return interfaces,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();

        // Check if interface is a CAN device:
        // 1. /sys/class/net/<iface>/type == 280 (ARPHRD_CAN)
        // 2. /sys/class/net/<iface>/can_state exists
        // 3. Name starts with "can" or "vcan" or "slcan"
        let is_can = if let Ok(type_str) = fs::read_to_string(path.join("type")) {
            type_str.trim() == "280"
        } else {
            path.join("can_state").exists()
                || name.starts_with("can")
                || name.starts_with("vcan")
                || name.starts_with("slcan")
        };

        // If it's not a CAN device, we skip regular eth/wlan to keep the list clean
        if !is_can {
            continue;
        }

        let operstate = fs::read_to_string(path.join("operstate"))
            .unwrap_or_else(|_| "unknown".to_string())
            .trim()
            .to_string();

        let is_up = operstate == "up" || operstate == "unknown";

        let bitrate = fs::read_to_string(path.join("can_bittiming/bitrate"))
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok());

        let rx_bytes = fs::read_to_string(path.join("statistics/rx_bytes"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

        let tx_bytes = fs::read_to_string(path.join("statistics/tx_bytes"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

        interfaces.push(DetectedInterface {
            name,
            is_can: true,
            is_up,
            operstate,
            bitrate,
            rx_bytes,
            tx_bytes,
        });
    }

    // Sort: 'can0' first, then alphabetical
    interfaces.sort_by(|a, b| {
        if a.name == "can0" {
            std::cmp::Ordering::Less
        } else if b.name == "can0" {
            std::cmp::Ordering::Greater
        } else {
            a.name.cmp(&b.name)
        }
    });

    interfaces
}

/// Verifies that a named interface exists and is brought UP on the host
#[allow(dead_code)]
pub fn verify_interface(name: &str) -> Result<DetectedInterface> {
    let interfaces = detect_can_interfaces();
    let iface = interfaces
        .into_iter()
        .find(|i| i.name == name)
        .ok_or_else(|| CarworxError::hardware_not_found(name))?;

    if !iface.is_up {
        return Err(CarworxError::hardware_down(name));
    }

    Ok(iface)
}
