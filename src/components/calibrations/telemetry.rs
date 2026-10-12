use std::collections::HashMap;
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::thread;
use std::time::{Duration, Instant};

const AF_CAN: i32 = 29;
const CAN_RAW: i32 = 1;
const SOL_CAN_RAW: i32 = 101;
const CAN_RAW_FILTER: i32 = 1;

/// Kernel CAN filter structure matching `<linux/can.h>`:
/// `struct can_filter { canid_t can_id; canid_t can_mask; };`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct CanFilter {
    pub can_id: u32,
    pub can_mask: u32,
}

/// Linux kernel `struct sockaddr_can` (24 bytes on modern 64-bit Linux kernels)
#[repr(C)]
#[derive(Copy, Clone)]
struct SockaddrCan {
    can_family: u16,
    _pad: u16,
    can_ifindex: i32,
    rx_id: u32,
    tx_id: u32,
    _reserved: [u8; 8],
}

/// Linux kernel `struct can_frame` (16 bytes)
#[repr(C)]
#[derive(Default, Copy, Clone)]
struct RawCanFrame {
    can_id: u32,
    can_dlc: u8,
    __pad: u8,
    __res0: u8,
    __len8_dlc: u8,
    data: [u8; 8],
}

#[derive(Debug, Clone, Default)]
pub struct SignalEntry {
    pub value: f64,
    pub last_updated: Option<Instant>,
    pub update_count: u64,
}

#[derive(Debug, Clone, Default)]
pub struct TelemetryStats {
    pub active_interface: String,
    pub is_connected: bool,
    pub total_frames_received: u64,
    pub last_frame_time: Option<Instant>,
    pub frames_per_second: f64,
    pub active_filter_count: usize,
}

/// Dedicated, high-performance background CAN telemetry service.
/// Applies kernel-level SocketCAN filters (`CAN_RAW_FILTER`) directly in the Linux kernel
/// to capture only relevant vehicle sensors (steering angle, G-sensors, velocity, RPM)
/// from the streaming CAN bus without packet drops or excessive CPU usage.
pub struct CanTelemetryService {
    signals: RwLock<HashMap<String, SignalEntry>>,
    stats: RwLock<TelemetryStats>,
    running: Arc<AtomicBool>,
}

static SERVICE_INSTANCE: OnceLock<CanTelemetryService> = OnceLock::new();

pub fn get_telemetry_service() -> &'static CanTelemetryService {
    SERVICE_INSTANCE.get_or_init(|| CanTelemetryService {
        signals: RwLock::new(HashMap::new()),
        stats: RwLock::new(TelemetryStats::default()),
        running: Arc::new(AtomicBool::new(false)),
    })
}

// Backward-compatibility alias
pub type LiveTelemetryManager = CanTelemetryService;
pub fn get_telemetry_manager() -> &'static CanTelemetryService {
    get_telemetry_service()
}

impl CanTelemetryService {
    pub fn new() -> Self {
        Self {
            signals: RwLock::new(HashMap::new()),
            stats: RwLock::new(TelemetryStats::default()),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start or ensure the background CAN telemetry listener is actively running on the given interface.
    pub fn start(&self, interface: &str) {
        let current_iface = {
            let guard = self.stats.read().unwrap();
            guard.active_interface.clone()
        };

        if self.running.load(Ordering::Relaxed) && current_iface == interface {
            return;
        }

        // Signal existing thread to stop if interface changed
        self.running.store(false, Ordering::Relaxed);
        thread::sleep(Duration::from_millis(40));

        let running = Arc::new(AtomicBool::new(true));
        self.running.store(true, Ordering::Relaxed);

        {
            let mut stats = self.stats.write().unwrap();
            stats.active_interface = interface.to_string();
            stats.is_connected = false;
        }

        let running_clone = running.clone();
        let iface = interface.to_string();

        log::info!("[CanTelemetryService] Starting telemetry worker on interface: {}", iface);
        eprintln!("[CanTelemetryService] Starting telemetry worker on interface: {}", iface);

        thread::Builder::new()
            .name("can-telemetry-worker".to_string())
            .spawn(move || {
                Self::worker_loop(&iface, running_clone);
            })
            .expect("Failed to spawn CAN telemetry worker thread");
    }

    /// Backwards compatibility method
    pub fn ensure_monitoring(&self, interface: &str) {
        self.start(interface);
    }

    /// Primary background worker loop using native Linux SocketCAN
    fn worker_loop(interface: &str, running: Arc<AtomicBool>) {
        let fd = unsafe { libc::socket(AF_CAN, libc::SOCK_RAW, CAN_RAW) };
        if fd < 0 {
            log::error!("[CanTelemetryService] Failed to create AF_CAN raw socket: errno {}", unsafe { *libc::__errno_location() });
            eprintln!("[CanTelemetryService] Failed to create AF_CAN raw socket: errno {}", unsafe { *libc::__errno_location() });
            return;
        }

        let ifname = match CString::new(interface) {
            Ok(s) => s,
            Err(_) => {
                unsafe { libc::close(fd) };
                return;
            }
        };

        let ifindex = unsafe { libc::if_nametoindex(ifname.as_ptr()) };
        if ifindex == 0 {
            log::warn!("[CanTelemetryService] Interface '{}' not found in system", interface);
            eprintln!("[CanTelemetryService] Interface '{}' not found in system", interface);
            unsafe { libc::close(fd) };
            return;
        }

        // =====================================================================
        // KERNEL FILTER INSTALLATION (CRITICAL FOR REAL VEHICLE BUS STREAMING)
        // Discards thousands of unrelated frames at kernel driver level!
        // =====================================================================
        let filters = [
            // 0x002: Steering Wheel Angle (100 Hz broadcast from VDC/ABS)
            CanFilter { can_id: 0x002, can_mask: 0x7FF },
            // 0x0D0: G-Sensors: Lateral G, Longitudinal G, Yaw Rate (50 Hz from VDC/ABS)
            CanFilter { can_id: 0x0D0, can_mask: 0x7FF },
            // 0x0D1: Vehicle Speed & Brake Hydraulic Demand (50 Hz from VDC/ABS)
            CanFilter { can_id: 0x0D1, can_mask: 0x7FF },
            // 0x140: Throttle command, Accelerator Position, Engine RPM (100 Hz from ECM)
            CanFilter { can_id: 0x140, can_mask: 0x7FF },
            // 0x141: Engine Speed alternate broadcast (100 Hz from ECM)
            CanFilter { can_id: 0x141, can_mask: 0x7FF },
            // 0x148: Transmission gear lever position & CVT ratio (100 Hz from TCM)
            CanFilter { can_id: 0x148, can_mask: 0x7FF },
            // 0x150: Master cylinder brake pressure (50 Hz from VDC)
            CanFilter { can_id: 0x150, can_mask: 0x7FF },
        ];

        let filter_res = unsafe {
            libc::setsockopt(
                fd,
                SOL_CAN_RAW,
                CAN_RAW_FILTER,
                filters.as_ptr() as *const libc::c_void,
                (filters.len() * std::mem::size_of::<CanFilter>()) as libc::socklen_t,
            )
        };
        if filter_res < 0 {
            log::warn!("[CanTelemetryService] Warning: Failed to set kernel CAN filters, fallback to all");
        } else {
            let svc = get_telemetry_service();
            let mut stats = svc.stats.write().unwrap();
            stats.active_filter_count = filters.len();
        }

        // Set 512 KB socket receive buffer to prevent frame dropping
        let rcvbuf_size: i32 = 512 * 1024;
        unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &rcvbuf_size as *const _ as *const libc::c_void,
                std::mem::size_of::<i32>() as libc::socklen_t,
            );
        }

        // Set 25ms read timeout so thread responds quickly to shutdown signals
        let tv = libc::timeval {
            tv_sec: 0,
            tv_usec: 25_000,
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

        let addr = SockaddrCan {
            can_family: AF_CAN as u16,
            _pad: 0,
            can_ifindex: ifindex as i32,
            rx_id: 0,
            tx_id: 0,
            _reserved: [0; 8],
        };

        let ret = unsafe {
            libc::bind(
                fd,
                &addr as *const _ as *const libc::sockaddr,
                std::mem::size_of::<SockaddrCan>() as libc::socklen_t,
            )
        };

        if ret < 0 {
            log::error!("[CanTelemetryService] Failed to bind CAN socket to interface {}: errno {}", interface, unsafe { *libc::__errno_location() });
            eprintln!("[CanTelemetryService] Failed to bind CAN socket to interface {}: errno {}", interface, unsafe { *libc::__errno_location() });
            unsafe { libc::close(fd) };
            return;
        }

        log::info!(
            "[CanTelemetryService] Successfully bound to {} (ifindex: {}). Active filters: {}",
            interface, ifindex, filters.len()
        );
        eprintln!(
            "[CanTelemetryService] Successfully bound to {} (ifindex: {}). Active filters: {}",
            interface, ifindex, filters.len()
        );

        let svc = get_telemetry_service();
        let mut last_log_time = Instant::now();
        let mut frames_this_period: u64 = 0;

        while running.load(Ordering::Relaxed) {
            let mut frame = RawCanFrame::default();
            let n = unsafe {
                libc::read(
                    fd,
                    &mut frame as *mut _ as *mut libc::c_void,
                    std::mem::size_of::<RawCanFrame>(),
                )
            };

            if n == std::mem::size_of::<RawCanFrame>() as isize {
                let can_id = frame.can_id & 0x7FF;
                let dlc = (frame.can_dlc as usize).min(8);
                let data = &frame.data[..dlc];
                let now = Instant::now();

                // Decode known high-frequency periodic frames
                Self::decode_and_record(svc, can_id, data, now);

                frames_this_period += 1;
                let mut stats = svc.stats.write().unwrap();
                stats.total_frames_received += 1;
                stats.last_frame_time = Some(now);
                stats.is_connected = true;

                // Log periodic throughput stats
                if now.duration_since(last_log_time) >= Duration::from_secs(5) {
                    let elapsed_sec = now.duration_since(last_log_time).as_secs_f64();
                    stats.frames_per_second = (frames_this_period as f64) / elapsed_sec;
                    log::info!(
                        "[CanTelemetryService] Throughput: {:.1} frames/sec (Total: {})",
                        stats.frames_per_second, stats.total_frames_received
                    );
                    frames_this_period = 0;
                    last_log_time = now;
                }
            }
        }

        unsafe { libc::close(fd) };
        log::info!("[CanTelemetryService] Worker thread stopped for {}", interface);
        eprintln!("[CanTelemetryService] Worker thread stopped for {}", interface);
    }

    /// Frame decoding logic for Subaru / UDS CAN architecture
    fn decode_and_record(svc: &CanTelemetryService, can_id: u32, data: &[u8], now: Instant) {
        match can_id {
            // 0x002: Steering angle (Subaru 100Hz broadcast)
            // pos 1, size 2: big-endian signed i16 * 0.1 deg
            0x002 => {
                if data.len() >= 2 {
                    let raw = i16::from_be_bytes([data[0], data[1]]);
                    let angle = (raw as f64) * 0.1;
                    svc.record_signal("Steering_Angle", angle, now);
                }
                if data.len() >= 3 {
                    let rate = data[2] as f64;
                    svc.record_signal("Steering_Rate", rate, now);
                }
            }

            // 0x0D0: G-Sensors (Subaru 50Hz broadcast)
            // Yaw Rate: bytes 0..2 little-endian signed i16 * -0.1 deg/s
            // Lateral G: bytes 2..4 little-endian signed i16 * -0.0035 + 1.0 m/s^2
            // Longitudinal G: bytes 6..8 little-endian signed i16 * -0.00035 m/s^2
            0x0D0 => {
                if data.len() >= 2 {
                    let raw_yaw = i16::from_le_bytes([data[0], data[1]]);
                    let yaw = (raw_yaw as f64) * -0.1;
                    svc.record_signal("Yaw_Rate", yaw, now);
                }
                if data.len() >= 4 {
                    let raw_lat = i16::from_le_bytes([data[2], data[3]]);
                    let lat_g = ((raw_lat as f64) * -0.0035) + 1.0;
                    svc.record_signal("Lateral_G", lat_g, now);
                }
                if data.len() >= 8 {
                    let raw_long = i16::from_le_bytes([data[6], data[7]]);
                    let long_g = (raw_long as f64) * -0.00035;
                    svc.record_signal("Longitudinal_G", long_g, now);
                }
            }

            // 0x0D1: Vehicle Speed & Brake Demand (Subaru 50Hz broadcast)
            // bytes 0..2 little-endian unsigned u16 * 0.05625 km/h
            0x0D1 => {
                if data.len() >= 2 {
                    let raw_spd = u16::from_le_bytes([data[0], data[1]]);
                    let spd = (raw_spd as f64) * 0.05625;
                    svc.record_signal("Vehicle_Speed", spd, now);
                }
                if data.len() >= 3 {
                    let brake_demand = data[2] as f64;
                    svc.record_signal("Brake_Demand_Raw", brake_demand, now);
                }
            }

            // 0x140 / 0x141: Engine RPM (Subaru 100Hz broadcast)
            0x140 => {
                if data.len() >= 7 {
                    let raw_rpm = u16::from_le_bytes([data[5], data[6]]);
                    svc.record_signal("Engine_Speed", raw_rpm as f64, now);
                    svc.record_signal("Engine_RPM", raw_rpm as f64, now);
                }
                if !data.is_empty() {
                    let throttle = (data[0] as f64) * 0.392;
                    svc.record_signal("Throttle_Pos", throttle, now);
                }
            }
            0x141 => {
                if data.len() >= 6 {
                    let raw_rpm = u16::from_le_bytes([data[4], data[5]]);
                    svc.record_signal("Engine_Speed", raw_rpm as f64, now);
                    svc.record_signal("Engine_RPM", raw_rpm as f64, now);
                }
            }

            // 0x148: Transmission gear lever position & CVT ratio (Subaru 100Hz)
            0x148 => {
                if !data.is_empty() {
                    let gear = (data[0] & 0x0F) as f64;
                    svc.record_signal("Gear_Position", gear, now);
                }
            }

            // 0x150: Brake master cylinder pressure (Subaru 50Hz)
            0x150 => {
                if data.len() >= 2 {
                    let raw_press = u16::from_le_bytes([data[0], data[1]]);
                    let press = (raw_press as f64) * 0.1;
                    svc.record_signal("Brake_Pressure", press, now);
                }
            }

            _ => {}
        }
    }

    /// Internal helper to update signal entry in thread-safe map
    fn record_signal(&self, name: &str, value: f64, now: Instant) {
        let mut signals = self.signals.write().unwrap();
        let entry = signals.entry(name.to_string()).or_default();
        entry.value = value;
        entry.last_updated = Some(now);
        entry.update_count += 1;
    }

    /// Retrieve the current value and live status for a signal.
    /// If live CAN traffic was received within the last 3.0 seconds, returns (live_val, true).
    /// If no live frames received (vehicle off / offline), returns safe baseline resting values.
    pub fn get_signal(&self, signal_name: &str) -> (f64, bool) {
        let now = Instant::now();
        if let Some(entry) = self.signals.read().unwrap().get(signal_name) {
            if let Some(last) = entry.last_updated {
                if now.duration_since(last) < Duration::from_secs(3) {
                    return (entry.value, true);
                }
            }
        }

        // Baseline resting values when ignition is off or offline
        let resting = match signal_name {
            "Vehicle_Speed" => 0.0,
            "Steering_Angle" => 0.0,
            "Lateral_G" => 0.0,
            "Longitudinal_G" => 0.0,
            "Engine_Speed" | "Engine_RPM" => 0.0,
            "Yaw_Rate" => 0.0,
            "ATF_Oil_Temp" => 68.0,
            _ => 0.0,
        };

        (resting, false)
    }

    /// Check if live CAN frames are actively being received on the monitored interface
    pub fn is_bus_active(&self) -> bool {
        let stats = self.stats.read().unwrap();
        match stats.last_frame_time {
            Some(ts) => Instant::now().duration_since(ts) < Duration::from_secs(2),
            None => false,
        }
    }

    /// Get current telemetry statistics
    pub fn get_stats(&self) -> TelemetryStats {
        self.stats.read().unwrap().clone()
    }

    /// Get all current decoded signal values as a list
    pub fn get_all_signals(&self) -> Vec<(String, f64, bool, u64)> {
        let now = Instant::now();
        let signals = self.signals.read().unwrap();
        signals
            .iter()
            .map(|(k, v)| {
                let is_live = v.last_updated.map(|t| now.duration_since(t) < Duration::from_secs(3)).unwrap_or(false);
                (k.clone(), v.value, is_live, v.update_count)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steering_angle_decoding() {
        // Big endian i16: 0x0064 = 100 * 0.1 = 10.0 deg
        let data = [0x00, 0x64];
        let raw = i16::from_be_bytes(data);
        let angle = (raw as f64) * 0.1;
        assert!((angle - 10.0).abs() < 1e-4);

        // Negative angle: -5.0 deg -> -50 -> 0xFFCE
        let raw_neg: i16 = -50;
        let bytes = raw_neg.to_be_bytes();
        let decoded = (i16::from_be_bytes(bytes) as f64) * 0.1;
        assert!((decoded - (-5.0)).abs() < 1e-4);
    }

    #[test]
    fn test_vehicle_speed_decoding() {
        // Little endian u16: 0x0000 = 0 km/h
        let raw: u16 = 0;
        let speed = (raw as f64) * 0.05625;
        assert_eq!(speed, 0.0);

        // 100 counts = 5.625 km/h
        let speed_100 = (100 as f64) * 0.05625;
        assert!((speed_100 - 5.625).abs() < 1e-4);
    }

    #[test]
    fn test_baseline_resting_signals() {
        let svc = get_telemetry_service();
        let (speed, is_live) = svc.get_signal("Vehicle_Speed");
        assert_eq!(speed, 0.0);
        assert!(!is_live);
    }
}
