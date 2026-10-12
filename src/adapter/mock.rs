use std::time::Duration;

use crate::adapter::device::{AdapterDeviceInfo, AdapterTestResult, AdapterType};
use crate::adapter::frame::CanFrame;
use crate::error::Result;

/// Mock CAN adapter for simulated environments and automated tests
pub struct MockCanAdapter {
    pub device_info: AdapterDeviceInfo,
    pub is_open: bool,
    pub sent_frames: Vec<CanFrame>,
    pub rx_queue: Vec<CanFrame>,
}

impl Default for MockCanAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCanAdapter {
    pub fn new() -> Self {
        let device_info = AdapterDeviceInfo {
            name: "Mock CANable Adapter (Simulated)".to_string(),
            adapter_type: AdapterType::Mock,
            interface: "vcan0".to_string(),
            serial_port: None,
            usb_vendor_id: Some("16d0".to_string()),
            usb_product_id: Some("117e".to_string()),
            manufacturer: Some("Openlight Labs (Mock)".to_string()),
            product_name: Some("CANable2 Mock".to_string()),
            serial_number: Some("MOCK-001".to_string()),
            is_up: true,
            operstate: "up".to_string(),
            bitrate: Some(500000),
            rx_packets: 128,
            tx_packets: 64,
            rx_bytes: 1024,
            tx_bytes: 512,
        };

        // Queue simulated incoming frames (e.g. Subaru live CAN bus broadcast frames)
        let rx_queue = vec![
            CanFrame::new(0x282, &[0xFE, 0x83, 0x00, 0xFE, 0xFE, 0x00, 0x24, 0x00]),
            CanFrame::new(0x152, &[0x81, 0xF4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
            CanFrame::new(0x7E8, &[0x02, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]), // TesterPresent positive response
        ];

        Self {
            device_info,
            is_open: false,
            sent_frames: Vec::new(),
            rx_queue,
        }
    }

    pub fn open(&mut self) -> Result<()> {
        self.is_open = true;
        Ok(())
    }

    pub fn close(&mut self) -> Result<()> {
        self.is_open = false;
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn send(&mut self, frame: &CanFrame) -> Result<()> {
        self.sent_frames.push(frame.clone());
        Ok(())
    }

    pub fn recv(&mut self, _timeout: Duration) -> Result<CanFrame> {
        if let Some(frame) = self.rx_queue.pop() {
            Ok(frame)
        } else {
            Err(crate::error::CarworxError::communication_timeout(
                "vcan0",
                "No simulated frame in queue",
            ))
        }
    }

    pub fn test_communication(&mut self) -> Result<AdapterTestResult> {
        let test_frame = CanFrame::new(0x7DF, &[0x02, 0x3E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        self.send(&test_frame)?;

        let sample_frames = self.rx_queue.clone();
        Ok(AdapterTestResult {
            success: true,
            adapter_name: self.device_info.name.clone(),
            interface: self.device_info.interface.clone(),
            tx_success: true,
            rx_success: true,
            latency_ms: Some(1.2),
            test_frame,
            rx_frames_captured: sample_frames.len(),
            sample_frames,
            live_traffic_detected: true,
            details: "Simulated communication test succeeded with positive response.".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_adapter() {
        let mut adapter = MockCanAdapter::new();
        assert!(adapter.open().is_ok());
        let res = adapter.test_communication().unwrap();
        assert!(res.success);
        assert!(res.tx_success);
        assert!(res.rx_success);
    }
}
