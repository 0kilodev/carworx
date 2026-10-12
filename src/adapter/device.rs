use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::adapter::frame::CanFrame;

/// Types of supported CAN bus adapters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdapterType {
    /// CANable / CANable 2.0 USB hardware adapter (Candlelight gs_usb or SLCAN)
    Canable,
    /// Generic Linux SocketCAN network interface (e.g. vcan, mcp251x, peak)
    SocketCan,
    /// Simulated mock adapter for testing without physical CAN hardware
    Mock,
}

impl std::fmt::Display for AdapterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterType::Canable => write!(f, "CANable (USB)"),
            AdapterType::SocketCan => write!(f, "SocketCAN"),
            AdapterType::Mock => write!(f, "Mock Adapter"),
        }
    }
}

/// Discovered USB hardware device info for CANable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanableUsbInfo {
    pub sysfs_path: PathBuf,
    pub vendor_id: String,
    pub product_id: String,
    pub manufacturer: Option<String>,
    pub product_name: Option<String>,
    pub serial_number: Option<String>,
    pub tty_port: Option<String>,
}

/// Information about a detected CAN adapter device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterDeviceInfo {
    pub name: String,
    pub adapter_type: AdapterType,
    pub interface: String,
    pub serial_port: Option<String>,
    pub usb_vendor_id: Option<String>,
    pub usb_product_id: Option<String>,
    pub manufacturer: Option<String>,
    pub product_name: Option<String>,
    pub serial_number: Option<String>,
    pub is_up: bool,
    pub operstate: String,
    pub bitrate: Option<u32>,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

/// Diagnostic test report after verifying adapter communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterTestResult {
    pub success: bool,
    pub adapter_name: String,
    pub interface: String,
    pub tx_success: bool,
    pub rx_success: bool,
    pub latency_ms: Option<f64>,
    pub test_frame: CanFrame,
    pub rx_frames_captured: usize,
    pub sample_frames: Vec<CanFrame>,
    pub live_traffic_detected: bool,
    pub details: String,
}

/// Scans the Linux sysfs tree (`/sys/bus/usb/devices`) for connected CANable adapters
pub fn scan_canable_usb_devices() -> Vec<CanableUsbInfo> {
    let mut devices = Vec::new();
    let usb_dir = Path::new("/sys/bus/usb/devices");

    if !usb_dir.exists() || !usb_dir.is_dir() {
        return devices;
    }

    let entries = match fs::read_dir(usb_dir) {
        Ok(e) => e,
        Err(_) => return devices,
    };

    for entry in entries.flatten() {
        let dev_path = entry.path();
        let vendor_id_path = dev_path.join("idVendor");
        let product_id_path = dev_path.join("idProduct");

        if !vendor_id_path.exists() || !product_id_path.exists() {
            continue;
        }

        let vendor_id = fs::read_to_string(&vendor_id_path)
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        let product_id = fs::read_to_string(&product_id_path)
            .unwrap_or_default()
            .trim()
            .to_lowercase();

        let manufacturer = fs::read_to_string(dev_path.join("manufacturer"))
            .ok()
            .map(|s| s.trim().to_string());
        let product_name = fs::read_to_string(dev_path.join("product"))
            .ok()
            .map(|s| s.trim().to_string());
        let serial_number = fs::read_to_string(dev_path.join("serial"))
            .ok()
            .map(|s| s.trim().to_string());

        // Identification criteria for CANable / CANable 2.0:
        // 1. 0x16d0:0x117e -> CANable 2.0 (MCS / Openlight Labs)
        // 2. 0x1d50:0x606f -> CANable / candleLight (gs_usb)
        // 3. Product name or manufacturer contains "canable"
        let is_canable_vid_pid = (vendor_id == "16d0" && product_id == "117e")
            || (vendor_id == "1d50" && product_id == "606f");

        let is_canable_name = product_name
            .as_deref()
            .map(|p| p.to_lowercase().contains("canable"))
            .unwrap_or(false);

        if is_canable_vid_pid || is_canable_name {
            // Find associated tty port (e.g. ttyACM0) if exposed
            let mut tty_port = None;
            if let Ok(sub_entries) = fs::read_dir(&dev_path) {
                for sub in sub_entries.flatten() {
                    let sub_name = sub.file_name().to_string_lossy().to_string();
                    if sub_name.contains(":1.") {
                        let tty_dir = sub.path().join("tty");
                        if tty_dir.exists() {
                            if let Ok(ttys) = fs::read_dir(&tty_dir) {
                                for tty in ttys.flatten() {
                                    let tty_name = tty.file_name().to_string_lossy().to_string();
                                    tty_port = Some(format!("/dev/{}", tty_name));
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            devices.push(CanableUsbInfo {
                sysfs_path: dev_path,
                vendor_id,
                product_id,
                manufacturer,
                product_name,
                serial_number,
                tty_port,
            });
        }
    }

    devices
}

/// Discovers all available CAN adapters (CANable and SocketCAN) on the system
pub fn discover_adapters() -> Vec<AdapterDeviceInfo> {
    let mut adapters = Vec::new();
    let canables = scan_canable_usb_devices();
    let net_dir = Path::new("/sys/class/net");

    if !net_dir.exists() || !net_dir.is_dir() {
        return adapters;
    }

    let entries = match fs::read_dir(net_dir) {
        Ok(e) => e,
        Err(_) => return adapters,
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

        let rx_packets = fs::read_to_string(path.join("statistics/rx_packets"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

        let tx_packets = fs::read_to_string(path.join("statistics/tx_packets"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

        let rx_bytes = fs::read_to_string(path.join("statistics/rx_bytes"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

        let tx_bytes = fs::read_to_string(path.join("statistics/tx_bytes"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

        // Check if this CAN interface is associated with a detected CANable USB adapter
        let matching_canable = canables.first();

        let (adapter_type, display_name, serial_port, usb_vendor, usb_prod, mfg, prod_name, serial_no) =
            if let Some(c) = matching_canable {
                let dev_name = c
                    .product_name
                    .as_deref()
                    .unwrap_or("CANable 2.0 Adapter");
                (
                    AdapterType::Canable,
                    format!("{} ({})", dev_name, name),
                    c.tty_port.clone(),
                    Some(c.vendor_id.clone()),
                    Some(c.product_id.clone()),
                    c.manufacturer.clone(),
                    c.product_name.clone(),
                    c.serial_number.clone(),
                )
            } else {
                (
                    AdapterType::SocketCan,
                    format!("SocketCAN ({})", name),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
            };

        adapters.push(AdapterDeviceInfo {
            name: display_name,
            adapter_type,
            interface: name,
            serial_port,
            usb_vendor_id: usb_vendor,
            usb_product_id: usb_prod,
            manufacturer: mfg,
            product_name: prod_name,
            serial_number: serial_no,
            is_up,
            operstate,
            bitrate,
            rx_packets,
            tx_packets,
            rx_bytes,
            tx_bytes,
        });
    }

    // Sort: 'can0' or Canable first, then alphabetical
    adapters.sort_by(|a, b| {
        if a.adapter_type == AdapterType::Canable && b.adapter_type != AdapterType::Canable {
            std::cmp::Ordering::Less
        } else if a.adapter_type != AdapterType::Canable && b.adapter_type == AdapterType::Canable {
            std::cmp::Ordering::Greater
        } else if a.interface == "can0" {
            std::cmp::Ordering::Less
        } else if b.interface == "can0" {
            std::cmp::Ordering::Greater
        } else {
            a.interface.cmp(&b.interface)
        }
    });

    adapters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_type_display() {
        assert_eq!(AdapterType::Canable.to_string(), "CANable (USB)");
        assert_eq!(AdapterType::SocketCan.to_string(), "SocketCAN");
        assert_eq!(AdapterType::Mock.to_string(), "Mock Adapter");
    }
}
