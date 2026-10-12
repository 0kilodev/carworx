use std::time::Duration;

pub mod canable;
pub mod device;
pub mod frame;
pub mod mock;
pub mod socketcan;

pub use canable::CanableAdapter;
pub use device::{
    discover_adapters, scan_canable_usb_devices, AdapterDeviceInfo, AdapterTestResult, AdapterType,
    CanableUsbInfo,
};
pub use frame::{
    CanFrame, RawCanFrame, CAN_EFF_FLAG, CAN_EFF_MASK, CAN_ERR_FLAG, CAN_RTR_FLAG, CAN_SFF_MASK,
};
pub use mock::MockCanAdapter;
pub use socketcan::SocketCanAdapter;

use crate::error::Result;

/// Core interface for CAN bus hardware adapters
pub trait CanAdapter {
    /// Human-readable device name
    fn name(&self) -> &str;

    /// Discovered hardware device metadata
    fn device_info(&self) -> &AdapterDeviceInfo;

    /// Open connection to the adapter
    fn open(&mut self) -> Result<()>;

    /// Close connection
    fn close(&mut self) -> Result<()>;

    /// Check if connection is currently open
    fn is_open(&self) -> bool;

    /// Transmit a CAN frame onto the bus
    fn send(&mut self, frame: &CanFrame) -> Result<()>;

    /// Receive a CAN frame within the timeout
    fn recv(&mut self, timeout: Duration) -> Result<CanFrame>;

    /// Run diagnostic communication self-test with the adapter
    fn test_communication(&mut self) -> Result<AdapterTestResult>;
}

impl CanAdapter for SocketCanAdapter {
    fn name(&self) -> &str {
        &self.device_info.name
    }

    fn device_info(&self) -> &AdapterDeviceInfo {
        &self.device_info
    }

    fn open(&mut self) -> Result<()> {
        self.open()
    }

    fn close(&mut self) -> Result<()> {
        self.close()
    }

    fn is_open(&self) -> bool {
        self.is_open()
    }

    fn send(&mut self, frame: &CanFrame) -> Result<()> {
        self.send(frame)
    }

    fn recv(&mut self, timeout: Duration) -> Result<CanFrame> {
        self.recv(timeout)
    }

    fn test_communication(&mut self) -> Result<AdapterTestResult> {
        self.test_communication()
    }
}

impl CanAdapter for CanableAdapter {
    fn name(&self) -> &str {
        &self.device_info.name
    }

    fn device_info(&self) -> &AdapterDeviceInfo {
        &self.device_info
    }

    fn open(&mut self) -> Result<()> {
        self.open()
    }

    fn close(&mut self) -> Result<()> {
        self.close()
    }

    fn is_open(&self) -> bool {
        self.is_open()
    }

    fn send(&mut self, frame: &CanFrame) -> Result<()> {
        self.send(frame)
    }

    fn recv(&mut self, timeout: Duration) -> Result<CanFrame> {
        self.recv(timeout)
    }

    fn test_communication(&mut self) -> Result<AdapterTestResult> {
        self.test_communication()
    }
}

impl CanAdapter for MockCanAdapter {
    fn name(&self) -> &str {
        &self.device_info.name
    }

    fn device_info(&self) -> &AdapterDeviceInfo {
        &self.device_info
    }

    fn open(&mut self) -> Result<()> {
        self.open()
    }

    fn close(&mut self) -> Result<()> {
        self.close()
    }

    fn is_open(&self) -> bool {
        self.is_open()
    }

    fn send(&mut self, frame: &CanFrame) -> Result<()> {
        self.send(frame)
    }

    fn recv(&mut self, timeout: Duration) -> Result<CanFrame> {
        self.recv(timeout)
    }

    fn test_communication(&mut self) -> Result<AdapterTestResult> {
        self.test_communication()
    }
}
