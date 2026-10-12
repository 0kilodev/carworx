use console::style;
use serde::Serialize;
use std::fmt;
use thiserror::Error;

/// Universal Result type for the Carworx application
pub type Result<T> = std::result::Result<T, CarworxError>;

/// Strongly typed, domain-specific error hierarchy for Carworx
#[derive(Error, Debug)]
pub enum CarworxError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Database initialization failed: {0}")]
    DatabaseInit(String),

    #[error("{entity} with identifier '{identifier}' was not found")]
    NotFound {
        entity: &'static str,
        identifier: String,
    },

    #[error("Hardware interface '{interface}' was not found on this system")]
    HardwareNotFound { interface: String },

    #[error("Hardware interface '{interface}' is down or offline")]
    HardwareDown { interface: String },

    #[error("Hardware interface '{interface}' configuration error: {reason}")]
    HardwareConfig {
        interface: String,
        reason: String,
    },

    #[error("File or I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error in '{path}': {reason}")]
    ConfigInvalid { path: String, reason: String },

    #[error("JSON serialization/deserialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Input validation failed for field '{field}': {reason}")]
    Validation {
        field: &'static str,
        reason: String,
    },

    #[error("Diagnostic protocol error on module '{module}': {reason}")]
    Protocol {
        module: String,
        code: Option<u8>,
        reason: String,
    },

    #[error("Terminal UI error: {0}")]
    Terminal(String),

    #[error("{0}")]
    Other(String),
}

/// Structured JSON representation of an error for scripting and programmatic API callers
#[derive(Serialize)]
struct JsonErrorPayload<'a> {
    error: bool,
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<&'a str>,
}

impl CarworxError {
    /// Machine-readable error code
    pub fn code(&self) -> &'static str {
        match self {
            CarworxError::Database(_) => "ERR_DATABASE",
            CarworxError::DatabaseInit(_) => "ERR_DB_INIT",
            CarworxError::NotFound { .. } => "ERR_NOT_FOUND",
            CarworxError::HardwareNotFound { .. } => "ERR_HARDWARE_NOT_FOUND",
            CarworxError::HardwareDown { .. } => "ERR_HARDWARE_DOWN",
            CarworxError::HardwareConfig { .. } => "ERR_HARDWARE_CONFIG",
            CarworxError::Io(_) => "ERR_IO",
            CarworxError::ConfigInvalid { .. } => "ERR_CONFIG_INVALID",
            CarworxError::Serialization(_) => "ERR_SERIALIZATION",
            CarworxError::Validation { .. } => "ERR_VALIDATION",
            CarworxError::Protocol { .. } => "ERR_PROTOCOL",
            CarworxError::Terminal(_) => "ERR_TERMINAL",
            CarworxError::Other(_) => "ERR_GENERAL",
        }
    }

    /// Actionable suggestion for the user, if applicable
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            CarworxError::HardwareDown { interface } if interface == "can0" => {
                Some("Try bringing up the CAN bus with: sudo ip link set can0 up type can bitrate 500000")
            }
            CarworxError::HardwareNotFound { interface } if interface == "can0" => {
                Some("Check that your CANable / USB-CAN adapter is plugged in and recognized (lsusb, dmesg)")
            }
            CarworxError::NotFound { entity, .. } if *entity == "Vehicle" => {
                Some("Run `carworx car list` to view valid vehicles, or `carworx car add` to create one.")
            }
            CarworxError::NotFound { entity, .. } if *entity == "Interface" => {
                Some("Run `carworx interface detect` to discover active CAN interfaces.")
            }
            _ => None,
        }
    }

    /// Renders the error either as structured JSON or as formatted terminal output
    pub fn render(&self, json_mode: bool) {
        if json_mode {
            let payload = JsonErrorPayload {
                error: true,
                code: self.code(),
                message: self.to_string(),
                details: match self {
                    CarworxError::NotFound { entity, identifier } => Some(serde_json::json!({
                        "entity": entity,
                        "identifier": identifier,
                    })),
                    CarworxError::HardwareNotFound { interface }
                    | CarworxError::HardwareDown { interface } => Some(serde_json::json!({
                        "interface": interface,
                    })),
                    CarworxError::Validation { field, reason } => Some(serde_json::json!({
                        "field": field,
                        "reason": reason,
                    })),
                    CarworxError::Protocol { module, code, reason } => Some(serde_json::json!({
                        "module": module,
                        "code": code,
                        "reason": reason,
                    })),
                    _ => None,
                },
                suggestion: self.suggestion(),
            };

            let json_str = serde_json::to_string_pretty(&payload)
                .unwrap_or_else(|_| r#"{"error":true,"code":"ERR_SERIALIZATION"}"#.to_string());
            eprintln!("{json_str}");
        } else {
            eprintln!();
            eprintln!(
                "{} [{}] {}",
                style("ERROR:").red().bold(),
                style(self.code()).yellow().bold(),
                style(self).bold()
            );

            if let Some(hint) = self.suggestion() {
                eprintln!("  {} {}", style("Hint:").cyan().bold(), style(hint).dim());
            }
            eprintln!();
        }
    }

    // ==========================================
    // CONSTRUCTOR HELPERS
    // ==========================================

    pub fn not_found(entity: &'static str, id: impl fmt::Display) -> Self {
        CarworxError::NotFound {
            entity,
            identifier: id.to_string(),
        }
    }

    pub fn validation(field: &'static str, reason: impl fmt::Display) -> Self {
        CarworxError::Validation {
            field,
            reason: reason.to_string(),
        }
    }

    pub fn hardware_not_found(interface: impl fmt::Display) -> Self {
        CarworxError::HardwareNotFound {
            interface: interface.to_string(),
        }
    }

    pub fn hardware_down(interface: impl fmt::Display) -> Self {
        CarworxError::HardwareDown {
            interface: interface.to_string(),
        }
    }

    pub fn hardware_config(interface: impl fmt::Display, reason: impl fmt::Display) -> Self {
        CarworxError::HardwareConfig {
            interface: interface.to_string(),
            reason: reason.to_string(),
        }
    }

    pub fn communication_timeout(interface: impl fmt::Display, reason: impl fmt::Display) -> Self {
        CarworxError::HardwareConfig {
            interface: interface.to_string(),
            reason: format!("Communication timed out: {}", reason),
        }
    }

    pub fn config_invalid(path: impl fmt::Display, reason: impl fmt::Display) -> Self {
        CarworxError::ConfigInvalid {
            path: path.to_string(),
            reason: reason.to_string(),
        }
    }

    pub fn protocol(module: impl fmt::Display, reason: impl fmt::Display) -> Self {
        CarworxError::Protocol {
            module: module.to_string(),
            code: None,
            reason: reason.to_string(),
        }
    }

    pub fn other(msg: impl fmt::Display) -> Self {
        CarworxError::Other(msg.to_string())
    }
}

// Convert dialoguer errors into CarworxError::Terminal
impl From<dialoguer::Error> for CarworxError {
    fn from(err: dialoguer::Error) -> Self {
        CarworxError::Terminal(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(CarworxError::not_found("Vehicle", 1).code(), "ERR_NOT_FOUND");
        assert_eq!(CarworxError::hardware_not_found("can0").code(), "ERR_HARDWARE_NOT_FOUND");
        assert_eq!(CarworxError::hardware_down("can0").code(), "ERR_HARDWARE_DOWN");
        assert_eq!(CarworxError::validation("vin", "invalid length").code(), "ERR_VALIDATION");
        assert_eq!(CarworxError::config_invalid("map.json", "missing").code(), "ERR_CONFIG_INVALID");
        assert_eq!(CarworxError::protocol("ECM", "timeout").code(), "ERR_PROTOCOL");
    }

    #[test]
    fn test_suggestions() {
        let err = CarworxError::hardware_down("can0");
        assert!(err.suggestion().is_some());
        assert!(err.suggestion().unwrap().contains("ip link set can0 up"));

        let err_nf = CarworxError::not_found("Vehicle", 99);
        assert!(err_nf.suggestion().is_some());
        assert!(err_nf.suggestion().unwrap().contains("carworx car list"));
    }

    #[test]
    fn test_error_display() {
        let err = CarworxError::not_found("Vehicle", 42);
        assert_eq!(err.to_string(), "Vehicle with identifier '42' was not found");

        let val_err = CarworxError::validation("year", "must be >= 1900");
        assert_eq!(val_err.to_string(), "Input validation failed for field 'year': must be >= 1900");
    }
}
