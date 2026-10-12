use std::ffi::CString;
use std::time::Duration;

use crate::error::{CarworxError, Result};

const AF_CAN: i32 = 29;
const CAN_ISOTP: i32 = 6;
const SOL_CAN_ISOTP: i32 = 106;
const CAN_ISOTP_OPTS: i32 = 1;
const CAN_ISOTP_TX_PADDING: u32 = 0x004;

#[repr(C)]
struct CanIsotpOptions {
    flags: u32,
    frame_txtime: u32,
    ext_address: u8,
    txpad_content: u8,
    rxpad_content: u8,
    rx_ext_address: u8,
}

#[repr(C)]
struct SockaddrCan {
    can_family: u16,
    can_ifindex: i32,
    rx_id: u32,
    tx_id: u32,
}

/// Communicates over Linux SocketCAN ISO-TP transport to query vehicle identification
pub struct IsotpTransport {
    fd: i32,
}

impl Drop for IsotpTransport {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe {
                libc::close(self.fd);
            }
        }
    }
}

impl IsotpTransport {
    /// Opens an ISO-TP socket bound to the specified network interface and request/response CAN IDs
    pub fn open(interface: &str, tx_id: u32, rx_id: u32, timeout: Duration) -> Result<Self> {
        let fd = unsafe { libc::socket(AF_CAN, libc::SOCK_DGRAM, CAN_ISOTP) };
        if fd < 0 {
            return Err(CarworxError::hardware_config(
                interface,
                "Failed to create SocketCAN ISO-TP socket. Ensure can-isotp kernel module is loaded.",
            ));
        }

        // Configure padding
        let opts = CanIsotpOptions {
            flags: CAN_ISOTP_TX_PADDING,
            frame_txtime: 0,
            ext_address: 0,
            txpad_content: 0,
            rxpad_content: 0,
            rx_ext_address: 0,
        };

        let ret = unsafe {
            libc::setsockopt(
                fd,
                SOL_CAN_ISOTP,
                CAN_ISOTP_OPTS,
                &opts as *const _ as *const libc::c_void,
                std::mem::size_of::<CanIsotpOptions>() as libc::socklen_t,
            )
        };
        if ret < 0 {
            unsafe { libc::close(fd) };
            return Err(CarworxError::hardware_config(
                interface,
                "Failed to set CAN_ISOTP_OPTS socket options",
            ));
        }

        // Set timeout
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

        let ifname = CString::new(interface)
            .map_err(|_| CarworxError::validation("interface", "Interface name contains null byte"))?;
        let ifindex = unsafe { libc::if_nametoindex(ifname.as_ptr()) };
        if ifindex == 0 {
            unsafe { libc::close(fd) };
            return Err(CarworxError::hardware_not_found(interface));
        }

        let addr = SockaddrCan {
            can_family: AF_CAN as u16,
            can_ifindex: ifindex as i32,
            rx_id,
            tx_id,
        };

        let bind_ret = unsafe {
            libc::bind(
                fd,
                &addr as *const _ as *const libc::sockaddr,
                std::mem::size_of::<SockaddrCan>() as libc::socklen_t,
            )
        };

        if bind_ret < 0 {
            unsafe { libc::close(fd) };
            return Err(CarworxError::hardware_config(
                interface,
                format!("Failed to bind ISO-TP socket to {}: tx=0x{:X}, rx=0x{:X}", interface, tx_id, rx_id),
            ));
        }

        Ok(Self { fd })
    }

    /// Sends a diagnostic payload and awaits response
    pub fn send_recv(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let written = unsafe {
            libc::write(
                self.fd,
                payload.as_ptr() as *const libc::c_void,
                payload.len(),
            )
        };
        if written < 0 {
            return Err(CarworxError::protocol("CAN", "Failed to write payload to ISO-TP socket"));
        }

        let mut buf = [0u8; 4096];
        let bytes_read = unsafe {
            libc::read(
                self.fd,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
            )
        };

        if bytes_read <= 0 {
            return Err(CarworxError::protocol("CAN", "Response timeout or empty read"));
        }

        Ok(buf[..bytes_read as usize].to_vec())
    }
}

/// Query vehicle VIN over CAN using SAE J1979 Mode $09 PID $02 or UDS ReadDataByIdentifier ($22 $F1 $90)
pub fn query_live_vin(interface: &str, timeout_ms: u64) -> Result<String> {
    let timeout = Duration::from_millis(timeout_ms);

    // Primary target: ECM at 0x7E0 (tx) -> 0x7E8 (rx)
    // 1. Try J1979 Mode 09 PID 02 (Standard OBD-II VIN request)
    if let Ok(transport) = IsotpTransport::open(interface, 0x7E0, 0x7E8, timeout) {
        if let Ok(resp) = transport.send_recv(&[0x09, 0x02]) {
            // Positive response: 0x49 0x02 ... VIN ASCII
            if resp.len() >= 19 && resp[0] == 0x49 && resp[1] == 0x02 {
                let vin_bytes = &resp[resp.len() - 17..];
                if let Ok(vin_str) = std::str::from_utf8(vin_bytes) {
                    let cleaned = vin_str.trim();
                    if cleaned.len() == 17 {
                        return Ok(cleaned.to_string());
                    }
                }
            }
        }

        // 2. Try UDS Service 0x22 DID 0xF190 (VIN)
        if let Ok(resp) = transport.send_recv(&[0x22, 0xF1, 0x90]) {
            // Positive response: 0x62 0xF1 0x90 ... VIN ASCII
            if resp.len() >= 20 && resp[0] == 0x62 && resp[1] == 0xF1 && resp[2] == 0x90 {
                let vin_bytes = &resp[3..20];
                if let Ok(vin_str) = std::str::from_utf8(vin_bytes) {
                    let cleaned = vin_str.trim();
                    if cleaned.len() == 17 {
                        return Ok(cleaned.to_string());
                    }
                }
            }
        }
    }

    Err(CarworxError::protocol(
        "AutoDiscovery",
        "No vehicle response received to VIN identification queries",
    ))
}
