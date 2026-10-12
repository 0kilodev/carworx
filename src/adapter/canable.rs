use std::time::Duration;

use crate::adapter::device::{discover_adapters, AdapterDeviceInfo, AdapterTestResult, AdapterType};
use crate::adapter::frame::CanFrame;
use crate::adapter::socketcan::SocketCanAdapter;
use crate::error::Result;

/// Specialized driver for CANable and CANable 2.0 USB CAN adapters
pub struct CanableAdapter {
    pub device_info: AdapterDeviceInfo,
    socketcan: SocketCanAdapter,
}

impl CanableAdapter {
    /// Create a CANable adapter instance bound to a specific interface
    pub fn new(interface: &str, device_info: AdapterDeviceInfo) -> Self {
        let socketcan = SocketCanAdapter::new(interface, device_info.clone());
        Self {
            device_info,
            socketcan,
        }
    }

    /// Auto-detects the connected CANable adapter on the host system.
    /// If an interface name is provided (e.g. "can0"), it uses that interface.
    /// Otherwise, it scans USB and network devices for CANable hardware.
    pub fn detect_or_default(preferred_interface: Option<&str>) -> Result<Self> {
        let adapters = discover_adapters();

        // 1. Look for an adapter explicitly tagged as CANable
        if let Some(target) = preferred_interface {
            if let Some(dev) = adapters.iter().find(|a| a.interface == target) {
                return Ok(Self::new(target, dev.clone()));
            }
        }

        // 2. Look for any detected CANable hardware
        if let Some(dev) = adapters.iter().find(|a| a.adapter_type == AdapterType::Canable) {
            return Ok(Self::new(&dev.interface, dev.clone()));
        }

        // 3. Look for 'can0' or any active SocketCAN interface
        if let Some(dev) = adapters.iter().find(|a| a.interface == "can0" || a.is_up) {
            return Ok(Self::new(&dev.interface, dev.clone()));
        }

        // 4. If no adapters found, fallback to can0 defaults
        let fallback_dev = AdapterDeviceInfo {
            name: "CANable Adapter (can0)".to_string(),
            adapter_type: AdapterType::Canable,
            interface: preferred_interface.unwrap_or("can0").to_string(),
            serial_port: Some("/dev/ttyACM0".to_string()),
            usb_vendor_id: Some("16d0".to_string()),
            usb_product_id: Some("117e".to_string()),
            manufacturer: Some("Openlight Labs".to_string()),
            product_name: Some("CANable2".to_string()),
            serial_number: None,
            is_up: true,
            operstate: "up".to_string(),
            bitrate: Some(500000),
            rx_packets: 0,
            tx_packets: 0,
            rx_bytes: 0,
            tx_bytes: 0,
        };

        Ok(Self::new(
            preferred_interface.unwrap_or("can0"),
            fallback_dev,
        ))
    }

    /// Open connection to CANable
    pub fn open(&mut self) -> Result<()> {
        self.socketcan.open()
    }

    /// Close connection
    pub fn close(&mut self) -> Result<()> {
        self.socketcan.close()
    }

    /// Check if connection is active
    pub fn is_open(&self) -> bool {
        self.socketcan.is_open()
    }

    /// Transmit frame over CANable
    pub fn send(&mut self, frame: &CanFrame) -> Result<()> {
        self.socketcan.send(frame)
    }

    /// Receive frame from CANable with timeout
    pub fn recv(&mut self, timeout: Duration) -> Result<CanFrame> {
        self.socketcan.recv(timeout)
    }

    /// Run diagnostic communication test on CANable
    pub fn test_communication(&mut self) -> Result<AdapterTestResult> {
        let mut result = self.socketcan.test_communication()?;

        // Enrich test result with CANable hardware identification
        let product = self.device_info.product_name.as_deref().unwrap_or("CANable");
        let mfg = self.device_info.manufacturer.as_deref().unwrap_or("");
        let serial = self.device_info.serial_number.as_deref().unwrap_or("unknown");
        let port = self.device_info.serial_port.as_deref().unwrap_or("SocketCAN");

        result.adapter_name = format!("{} by {} [SN: {}, Port: {}]", product, mfg, serial, port);

        if result.tx_success {
            result.details = format!(
                "CANable hardware communication verified on interface '{}'. Transmit ping succeeded in {:.2}ms. Live bus traffic: {}.",
                self.device_info.interface,
                result.latency_ms.unwrap_or(0.0),
                if result.live_traffic_detected {
                    format!("ACTIVE ({} frames captured)", result.rx_frames_captured)
                } else {
                    "IDLE (no response within 300ms)".to_string()
                }
            );
        } else {
            result.details = format!(
                "CANable hardware detected on '{}', but transmit test failed: {}",
                self.device_info.interface,
                result.details
            );
        }

        Ok(result)
    }
}
