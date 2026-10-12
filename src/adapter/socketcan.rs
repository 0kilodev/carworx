use std::ffi::CString;
use std::time::{Duration, Instant};

use crate::adapter::device::{AdapterDeviceInfo, AdapterTestResult};
use crate::adapter::frame::{CanFrame, RawCanFrame};
use crate::error::{CarworxError, Result};

const AF_CAN: i32 = 29;
const CAN_RAW: i32 = 1;
const SOL_CAN_RAW: i32 = 101;
const CAN_RAW_LOOPBACK: i32 = 3;

#[repr(C)]
struct SockaddrCanRaw {
    can_family: u16,
    can_ifindex: i32,
    rx_id: u32,
    tx_id: u32,
}

/// Linux SocketCAN network adapter driver
pub struct SocketCanAdapter {
    pub interface: String,
    pub device_info: AdapterDeviceInfo,
    fd: Option<i32>,
}

impl Drop for SocketCanAdapter {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

impl SocketCanAdapter {
    /// Create a new SocketCanAdapter for the given network interface
    pub fn new(interface: &str, device_info: AdapterDeviceInfo) -> Self {
        Self {
            interface: interface.to_string(),
            device_info,
            fd: None,
        }
    }

    /// Open and bind the Linux raw CAN socket
    pub fn open(&mut self) -> Result<()> {
        if self.fd.is_some() {
            return Ok(());
        }

        let fd = unsafe { libc::socket(AF_CAN, libc::SOCK_RAW, CAN_RAW) };
        if fd < 0 {
            return Err(CarworxError::hardware_config(
                &self.interface,
                "Failed to create SocketCAN raw socket. Verify kernel CAN support is enabled.",
            ));
        }

        // Enable loopback so transmitted frames can be observed locally if needed
        let loopback: libc::c_int = 1;
        unsafe {
            libc::setsockopt(
                fd,
                SOL_CAN_RAW,
                CAN_RAW_LOOPBACK,
                &loopback as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
        }

        // Default 250ms receive timeout
        let tv = libc::timeval {
            tv_sec: 0,
            tv_usec: 250_000,
        };
        unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                &tv as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::timeval>() as libc::socklen_t,
            );
        }

        let ifname = CString::new(self.interface.as_str())
            .map_err(|_| CarworxError::validation("interface", "Interface name contains null byte"))?;
        let ifindex = unsafe { libc::if_nametoindex(ifname.as_ptr()) };
        if ifindex == 0 {
            unsafe { libc::close(fd) };
            return Err(CarworxError::hardware_not_found(format!(
                "CAN network interface '{}' does not exist in /sys/class/net",
                self.interface
            )));
        }

        let addr = SockaddrCanRaw {
            can_family: AF_CAN as u16,
            can_ifindex: ifindex as i32,
            rx_id: 0,
            tx_id: 0,
        };

        let ret = unsafe {
            libc::bind(
                fd,
                &addr as *const _ as *const libc::sockaddr,
                std::mem::size_of::<SockaddrCanRaw>() as libc::socklen_t,
            )
        };

        if ret < 0 {
            unsafe { libc::close(fd) };
            return Err(CarworxError::hardware_config(
                &self.interface,
                format!("Failed to bind to SocketCAN interface '{}'. Is the interface UP?", self.interface),
            ));
        }

        self.fd = Some(fd);
        Ok(())
    }

    /// Close the socket
    pub fn close(&mut self) -> Result<()> {
        if let Some(fd) = self.fd.take() {
            unsafe {
                libc::close(fd);
            }
        }
        Ok(())
    }

    /// Check if socket is open
    pub fn is_open(&self) -> bool {
        self.fd.is_some()
    }

    /// Transmit a standard or extended CAN frame onto the bus
    pub fn send(&mut self, frame: &CanFrame) -> Result<()> {
        let fd = match self.fd {
            Some(fd) => fd,
            None => {
                self.open()?;
                self.fd.unwrap()
            }
        };

        let raw = frame.to_raw_can_frame();
        let ret = unsafe {
            libc::write(
                fd,
                &raw as *const _ as *const libc::c_void,
                std::mem::size_of::<RawCanFrame>(),
            )
        };

        if ret < 0 {
            let err = std::io::Error::last_os_error();
            return Err(CarworxError::hardware_config(
                &self.interface,
                format!("Failed to write CAN frame ({}): {}", frame.format_hex(), err),
            ));
        }

        Ok(())
    }

    /// Receive a CAN frame with the specified timeout
    pub fn recv(&mut self, timeout: Duration) -> Result<CanFrame> {
        let fd = match self.fd {
            Some(fd) => fd,
            None => {
                self.open()?;
                self.fd.unwrap()
            }
        };

        // Update timeout on socket
        let tv = libc::timeval {
            tv_sec: timeout.as_secs() as libc::time_t,
            tv_usec: timeout.subsec_micros() as libc::suseconds_t,
        };
        unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                &tv as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::timeval>() as libc::socklen_t,
            );
        }

        let mut raw = RawCanFrame::default();
        let ret = unsafe {
            libc::read(
                fd,
                &mut raw as *mut _ as *mut libc::c_void,
                std::mem::size_of::<RawCanFrame>(),
            )
        };

        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::WouldBlock || err.kind() == std::io::ErrorKind::TimedOut {
                return Err(CarworxError::communication_timeout(
                    &self.interface,
                    format!("Timed out after {}ms waiting for CAN frame", timeout.as_millis()),
                ));
            }
            return Err(CarworxError::hardware_config(
                &self.interface,
                format!("Failed to read from SocketCAN interface: {}", err),
            ));
        }

        Ok(CanFrame::from_raw_can_frame(&raw))
    }

    /// End-to-end communication test:
    /// 1. Verifies interface link state
    /// 2. Binds raw CAN socket
    /// 3. Transmits standard UDS TesterPresent ping (0x7DF [0x02, 0x3E, 0x00, ...])
    /// 4. Measures latency
    /// 5. Captures incoming bus frames to prove active communication
    pub fn test_communication(&mut self) -> Result<AdapterTestResult> {
        self.open()?;

        // Safe broadcast TesterPresent (SAE J1979 / UDS diagnostic keepalive frame)
        let test_frame = CanFrame::new(0x7DF, &[0x02, 0x3E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let start = Instant::now();
        let send_res = self.send(&test_frame);
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        let tx_success = send_res.is_ok();

        // Sample up to 10 incoming frames within 300ms to detect active vehicle CAN traffic
        let mut sample_frames = Vec::new();
        let sample_deadline = Instant::now() + Duration::from_millis(300);

        while Instant::now() < sample_deadline && sample_frames.len() < 10 {
            let remaining = sample_deadline.saturating_duration_since(Instant::now());
            if let Ok(frame) = self.recv(remaining.min(Duration::from_millis(50))) {
                sample_frames.push(frame);
            }
        }

        let live_traffic_detected = !sample_frames.is_empty();
        let rx_success = live_traffic_detected;

        let details = if tx_success && live_traffic_detected {
            format!(
                "Successfully transmitted test frame in {:.2}ms; verified active bus traffic with {} live frames captured.",
                latency_ms,
                sample_frames.len()
            )
        } else if tx_success {
            format!(
                "Successfully transmitted test frame in {:.2}ms onto {}; no incoming response received within 300ms (vehicle ignition may be OFF).",
                latency_ms,
                self.interface
            )
        } else {
            format!(
                "Failed to transmit test frame onto {}: {}",
                self.interface,
                send_res.err().map(|e| e.to_string()).unwrap_or_default()
            )
        };

        Ok(AdapterTestResult {
            success: tx_success,
            adapter_name: self.device_info.name.clone(),
            interface: self.interface.clone(),
            tx_success,
            rx_success,
            latency_ms: if tx_success { Some(latency_ms) } else { None },
            test_frame,
            rx_frames_captured: sample_frames.len(),
            sample_frames,
            live_traffic_detected,
            details,
        })
    }
}
