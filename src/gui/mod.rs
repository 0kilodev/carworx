pub mod app;

pub use app::CarworxApp;

use eframe::egui;

use crate::core::Database;
use crate::error::{CarworxError, Result};

use crate::pages::Page;

/// Launch the Carworx native desktop GUI application
pub fn run_gui(
    db: Database,
    initial_page: Option<Page>,
    auto_scan: bool,
    routine: Option<String>,
    confirm: bool,
) -> Result<()> {
    // Check if a display server is available (X11 or Wayland)
    let has_x11 = std::env::var("DISPLAY").map(|s| !s.trim().is_empty()).unwrap_or(false);
    let has_wayland = std::env::var("WAYLAND_DISPLAY").map(|s| !s.trim().is_empty()).unwrap_or(false);
    if !has_x11 && !has_wayland {
        return Err(CarworxError::hardware_not_found(
            "No graphical display detected ($DISPLAY and $WAYLAND_DISPLAY are unset or empty).\n\
             If you are connected via SSH or a headless terminal, launch Carworx in CLI mode using:\n\n    carworx --cli\n\n\
             Or run headless commands such as:\n\n    carworx status\n    carworx car list\n    carworx discover\n",
        ));
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Carworx - Universal Automotive Diagnostic & CAN Bus Platform")
            .with_inner_size([980.0, 680.0])
            .with_min_inner_size([820.0, 520.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Carworx Automotive Diagnostics",
        native_options,
        Box::new(move |cc| Ok(Box::new(CarworxApp::new(cc, db, initial_page, auto_scan, routine, confirm)))),
    )
    .map_err(|e| CarworxError::other(format!("Failed to initialize Carworx GUI window: {}", e)))
}
