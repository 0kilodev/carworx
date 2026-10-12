use clap::Parser;

pub mod adapter;
mod cli;
pub mod components;
pub mod config;
mod core;
pub mod error;
pub mod gui;
pub mod pages;
mod schema;
pub mod ui;

use cli::{handle_cli_command, run_interactive_terminal, Cli, Commands};
use core::Database;
use error::Result;

fn run(cli: &Cli) -> Result<()> {
    let db = Database::open(cli.db.as_deref())?;

    if let Some(ref cmd) = cli.command {
        // Execute headless / scriptable subcommand
        handle_cli_command(&db, cli, cmd)?;
    } else if cli.headless {
        // Headless flag given without a subcommand: default to printing status
        handle_cli_command(&db, cli, &Commands::Status)?;
    } else if cli.cli {
        // Interactive terminal mode requested with --cli
        run_interactive_terminal(&db)?;
    } else {
        // Default mode: Launch Carworx native desktop GUI
        let initial_page = match cli.page.as_deref() {
            Some("calibrations") | Some("calibration") => Some(pages::Page::Calibrations),
            Some("diagnostics") | Some("dtc") => Some(pages::Page::Diagnostics),
            Some("vehicle-selector") | Some("vehicles") => Some(pages::Page::VehicleSelector),
            Some("auto-discovery") | Some("discovery") => Some(pages::Page::AutoDiscovery),
            Some("landing") => Some(pages::Page::Landing),
            _ => None,
        };
        gui::run_gui(db, initial_page, cli.auto_scan, cli.routine.clone(), cli.confirm)?;
    }

    Ok(())
}

fn main() {
    // On Linux (especially Raspberry Pi Broadcom VideoCore IV/V GPUs), hardware GLES
    // has driver limitations with glow shaders that result in an empty black screen.
    // Defaulting to Mesa software rasterization (llvmpipe) guarantees crystal-clear,
    // robust rendering on Raspberry Pi and Linux desktop environments.
    if std::env::var_os("LIBGL_ALWAYS_SOFTWARE").is_none() {
        std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
    }

    let _ = env_logger::try_init();
    let cli = Cli::parse();
    if let Err(err) = run(&cli) {
        err.render(cli.json);
        std::process::exit(1);
    }
}
