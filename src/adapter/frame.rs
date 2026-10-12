use serde::{Deserialize, Serialize};

pub const CAN_EFF_FLAG: u32 = 0x8000_0000;
pub const CAN_RTR_FLAG: u32 = 0x4000_0000;
pub const CAN_ERR_FLAG: u32 = 0x2000_0000;
pub const CAN_SFF_MASK: u32 = 0x0000_07FF;
pub const CAN_EFF_MASK: u32 = 0x1FFF_FFFF;

/// Low-level C structure representing a Linux SocketCAN frame (16 bytes)
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RawCanFrame {
    pub can_id: u32,
    pub can_dlc: u8,
    pub __pad: u8,
    pub __res0: u8,
    pub __len8_dlc: u8,
    pub data: [u8; 8],
}

impl Default for RawCanFrame {
    fn default() -> Self {
        Self {
            can_id: 0,
            can_dlc: 0,
            __pad: 0,
            __res0: 0,
            __len8_dlc: 0,
            data: [0u8; 8],
        }
    }
}

/// High-level automotive CAN frame
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanFrame {
    /// 11-bit standard or 29-bit extended CAN arbitration ID
    pub id: u32,
    /// Data payload (0 to 8 bytes for classic CAN)
    pub data: Vec<u8>,
    /// Extended (29-bit) identifier flag
    pub is_extended: bool,
    /// Remote Transmission Request flag
    pub is_rtr: bool,
}

impl CanFrame {
    /// Create a standard (11-bit) CAN frame
    pub fn new(id: u32, data: &[u8]) -> Self {
        let len = data.len().min(8);
        Self {
            id: id & CAN_SFF_MASK,
            data: data[..len].to_vec(),
            is_extended: false,
            is_rtr: false,
        }
    }

    /// Create an extended (29-bit) CAN frame
    pub fn new_extended(id: u32, data: &[u8]) -> Self {
        let len = data.len().min(8);
        Self {
            id: id & CAN_EFF_MASK,
            data: data[..len].to_vec(),
            is_extended: true,
            is_rtr: false,
        }
    }

    /// Create standard frame (alias for `new`)
    pub fn standard(id: u32, data: &[u8]) -> Self {
        Self::new(id, data)
    }

    /// Data length code (number of payload bytes)
    pub fn dlc(&self) -> usize {
        self.data.len()
    }

    /// Formats the CAN ID as a standard hexadecimal string (e.g. "0x7DF")
    pub fn id_hex(&self) -> String {
        if self.is_extended {
            format!("0x{:08X}", self.id)
        } else {
            format!("0x{:03X}", self.id)
        }
    }

    /// Formats data bytes as space-separated uppercase hex pairs (e.g. "02 3E 00 00 00 00 00 00")
    pub fn display_data_hex(&self) -> String {
        self.data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Formats the frame in cansend/candump notation (e.g. "7DF#023E000000000000")
    pub fn format_hex(&self) -> String {
        let id_str = if self.is_extended {
            format!("{:08X}", self.id)
        } else {
            format!("{:03X}", self.id)
        };
        let data_str: String = self.data.iter().map(|b| format!("{:02X}", b)).collect();
        format!("{}#{}", id_str, data_str)
    }

    /// Convert to Linux SocketCAN `RawCanFrame`
    pub fn to_raw_can_frame(&self) -> RawCanFrame {
        let mut raw = RawCanFrame::default();
        let mut can_id = if self.is_extended {
            (self.id & CAN_EFF_MASK) | CAN_EFF_FLAG
        } else {
            self.id & CAN_SFF_MASK
        };

        if self.is_rtr {
            can_id |= CAN_RTR_FLAG;
        }

        raw.can_id = can_id;
        raw.can_dlc = self.data.len().min(8) as u8;

        for (i, b) in self.data.iter().take(8).enumerate() {
            raw.data[i] = *b;
        }

        raw
    }

    /// Parse from Linux SocketCAN `RawCanFrame`
    pub fn from_raw_can_frame(raw: &RawCanFrame) -> Self {
        let is_extended = (raw.can_id & CAN_EFF_FLAG) != 0;
        let is_rtr = (raw.can_id & CAN_RTR_FLAG) != 0;
        let id = if is_extended {
            raw.can_id & CAN_EFF_MASK
        } else {
            raw.can_id & CAN_SFF_MASK
        };

        let dlc = (raw.can_dlc as usize).min(8);
        let data = raw.data[..dlc].to_vec();

        Self {
            id,
            data,
            is_extended,
            is_rtr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_frame_roundtrip() {
        let frame = CanFrame::new(0x7DF, &[0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(frame.id_hex(), "0x7DF");
        assert_eq!(frame.format_hex(), "7DF#0201000000000000");

        let raw = frame.to_raw_can_frame();
        assert_eq!(raw.can_id, 0x7DF);
        assert_eq!(raw.can_dlc, 8);

        let parsed = CanFrame::from_raw_can_frame(&raw);
        assert_eq!(parsed, frame);
    }

    #[test]
    fn test_extended_frame_roundtrip() {
        let frame = CanFrame::new_extended(0x18DAF110, &[0x02, 0x10, 0x01]);
        assert_eq!(frame.id_hex(), "0x18DAF110");
        assert_eq!(frame.format_hex(), "18DAF110#021001");

        let raw = frame.to_raw_can_frame();
        assert_eq!(raw.can_id & CAN_EFF_FLAG, CAN_EFF_FLAG);
        assert_eq!(raw.can_id & CAN_EFF_MASK, 0x18DAF110);
        assert_eq!(raw.can_dlc, 3);

        let parsed = CanFrame::from_raw_can_frame(&raw);
        assert_eq!(parsed, frame);
    }
}
