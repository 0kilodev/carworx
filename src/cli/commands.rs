use clap::{Args, Parser, Subcommand};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, Row, Table};
use std::path::{Path, PathBuf};

use crate::adapter::{discover_adapters, CanableAdapter};
use crate::components::diagnostic::{get_dtc_description, perform_diagnostic_scan};
use crate::components::discovery::AutoDiscoveryComponent;
use crate::config::{ProtocolManager, VehicleManager};
use crate::core::hardware::detect_can_interfaces;
use crate::core::Database;
use crate::error::{CarworxError, Result};
use crate::schema::models::{NewCar, NewCarConfig};

#[derive(Parser, Debug)]
#[command(name = "carworx")]
#[command(author = "Carworx Team")]
#[command(version = "0.1.0")]
#[command(about = "Universal automotive diagnostic and CAN bus platform", long_about = None)]
pub struct Cli {
    /// Custom path to the SQLite database
    #[arg(short, long, global = true)]
    pub db: Option<PathBuf>,

    /// Run in headless / scriptable mode without interactive prompts
    #[arg(long, global = true)]
    pub headless: bool,

    /// Output results in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    /// Run in interactive terminal / CLI mode instead of GUI
    #[arg(long, global = true)]
    pub cli: bool,

    /// Initial GUI page to open (landing, vehicle-selector, auto-discovery, diagnostics, calibrations)
    #[arg(long, global = true)]
    pub page: Option<String>,

    /// Automatically run diagnostic DTC scan on startup if opening diagnostics page
    #[arg(long, global = true)]
    pub auto_scan: bool,

    /// Initial calibration routine key to select when opening calibrations page
    #[arg(long, global = true)]
    pub routine: Option<String>,

    /// Pre-open the confirmation dialog for the selected routine (for validation/testing)
    #[arg(long, global = true)]
    pub confirm: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Automatically discover CAN hardware and retrieve vehicle metadata (VIN, Make, Model, Year)
    Discover(DiscoverArgs),
    /// Manage vehicle profiles
    Car {
        #[command(subcommand)]
        cmd: CarCommands,
    },
    /// Manage CAN hardware interfaces
    Interface {
        #[command(subcommand)]
        cmd: InterfaceCommands,
    },
    /// Manage CAN bus adapters and test hardware communication (e.g. CANable)
    Adapter {
        #[command(subcommand)]
        cmd: AdapterCommands,
    },
    /// Manage universal configuration files (CAN maps, DTC catalogs)
    Config {
        #[command(subcommand)]
        cmd: ConfigCommands,
    },
    /// Inspect supported protocols (SAE J2534, UDS, J1979, Subaru SSM)
    Protocol {
        #[command(subcommand)]
        cmd: ProtocolCommands,
    },
    /// Browse vehicle makes and models catalog (Subaru, Toyota, Honda, Ford, Chevrolet)
    Vehicles {
        #[command(subcommand)]
        cmd: VehicleCommands,
    },
    /// Query and decode Diagnostic Trouble Codes (DTCs) across 6 automotive domains
    Dtc {
        #[command(subcommand)]
        cmd: DtcCommands,
    },
    /// Stream and decode real-time CAN bus telemetry (steering angle, G-sensors, velocity, RPM)
    Telemetry {
        /// Target CAN interface (default: can0)
        #[arg(short, long, default_value = "can0")]
        interface: String,

        /// Number of seconds to monitor streaming telemetry
        #[arg(short, long, default_value_t = 3)]
        seconds: u64,
    },
    /// Show current system, active vehicle, and interface status
    Status,
}

#[derive(Subcommand, Debug)]
pub enum CarCommands {
    /// List all registered vehicles
    List,
    /// Add a new vehicle profile
    Add(AddCarArgs),
    /// Set the active vehicle by ID
    Select {
        /// ID of the vehicle to activate
        id: i64,
    },
    /// Show the currently active vehicle
    Active,
    /// Delete a vehicle by ID
    Delete {
        /// ID of the vehicle to delete
        id: i64,
    },
}

#[derive(Args, Debug)]
pub struct AddCarArgs {
    #[arg(short, long)]
    pub name: String,

    #[arg(short, long)]
    pub make: String,

    #[arg(short = 'o', long)]
    pub model: String,

    #[arg(short, long)]
    pub year: i32,

    #[arg(long)]
    pub vin: Option<String>,

    #[arg(long)]
    pub trim: Option<String>,

    #[arg(long)]
    pub engine: Option<String>,

    #[arg(long)]
    pub transmission: Option<String>,

    #[arg(long)]
    pub protocol: Option<String>,

    #[arg(long)]
    pub notes: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum InterfaceCommands {
    /// List registered hardware interfaces from database
    List,
    /// Auto-detect available CAN interfaces on the Linux system
    Detect,
    /// Set the active hardware interface by name (e.g. can0)
    Select {
        /// Interface name (e.g. can0, vcan0)
        name: String,
        /// Bitrate in bps (default: 500000)
        #[arg(short, long, default_value = "500000")]
        bitrate: u32,
    },
    /// Show the currently active hardware interface
    Active,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// List configuration profiles
    List {
        #[arg(short, long)]
        car_id: Option<i64>,
    },
    /// Register a configuration file (CAN map, DTC catalog, etc.)
    Add {
        #[arg(short, long)]
        name: String,
        #[arg(short = 't', long = "type", alias = "config-type", default_value = "can_map")]
        config_type: String,
        #[arg(short, long)]
        path: String,
        #[arg(short, long)]
        car_id: Option<i64>,
        #[arg(long)]
        description: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct DiscoverArgs {
    /// Specific CAN interface to probe (e.g. can0). Defaults to auto-detecting active interface.
    #[arg(short, long)]
    pub interface: Option<String>,

    /// Fallback VIN if vehicle ignition is off or bench testing
    #[arg(long)]
    pub fallback_vin: Option<String>,

    /// Save the discovered vehicle profile and hardware interface to database
    #[arg(short, long)]
    pub save: bool,
}

#[derive(Subcommand, Debug)]
pub enum AdapterCommands {
    /// List all discovered CAN adapters (CANable, SocketCAN)
    List,
    /// Test communication with CAN adapter (sends test frame, measures latency, captures live traffic)
    Test(AdapterTestArgs),
}

#[derive(Args, Debug)]
pub struct AdapterTestArgs {
    /// Optional interface name to test (default: auto-detected CANable or can0)
    #[arg(short, long)]
    pub interface: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum ProtocolCommands {
    /// List all loaded protocol definitions
    List,
    /// Show full specification of a protocol (e.g. uds, j1979_obd2, j2534_isotp, subaru_ssm)
    Show {
        /// Protocol name or identifier
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum VehicleCommands {
    /// List supported vehicle makes and their top models
    List,
    /// Show detailed specifications, generations, engines, and ECUs for a make
    Show {
        /// Name or ID of the vehicle make (e.g. subaru, toyota, honda, ford, chevrolet)
        make: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum DtcCommands {
    /// Perform multi-domain DTC scan across 6 domains (Engine, Transmission, ABS, EPS, BIU, SRS)
    Scan(DtcScanArgs),
    /// Lookup description of a Diagnostic Trouble Code in the catalog
    Lookup {
        /// DTC code to look up (e.g. P2404, C1422, U0073)
        code: String,
    },
}

#[derive(Args, Debug)]
pub struct DtcScanArgs {
    /// Specific CAN interface to use (defaults to active interface or "can0")
    #[arg(short, long)]
    pub interface: Option<String>,

    /// Specific vehicle ID to use (defaults to active vehicle in database)
    #[arg(short, long)]
    pub car_id: Option<i64>,
}

pub fn handle_cli_command(db: &Database, cli: &Cli, cmd: &Commands) -> Result<()> {
    match cmd {
        Commands::Discover(args) => {
            let component = AutoDiscoveryComponent::new();
            let result = component.discover(
                db,
                args.interface.as_deref(),
                args.fallback_vin.as_deref(),
                args.save,
            )?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("=== 🔍 Vehicle & Hardware Auto-Discovery ===");
                println!();
                println!("Hardware Interface: {} (State: {})", result.interface, result.interface_state);
                println!("Protocol Used:      {}", result.protocol_used);
                println!();

                let mut table = Table::new();
                table.load_preset(UTF8_FULL).apply_modifier(UTF8_ROUND_CORNERS);
                table.set_header(vec!["Attribute", "Discovered Value"]);

                table.add_row(Row::from(vec![Cell::new("VIN"), Cell::new(&result.decoded.vin).fg(Color::Yellow)]));
                table.add_row(Row::from(vec![Cell::new("Make"), Cell::new(&result.decoded.make)]));
                table.add_row(Row::from(vec![Cell::new("Model"), Cell::new(&result.decoded.model).fg(Color::Green)]));
                table.add_row(Row::from(vec![Cell::new("Year"), Cell::new(result.decoded.year)]));
                if let Some(ref bt) = result.decoded.body_type {
                    table.add_row(Row::from(vec![Cell::new("Body Type"), Cell::new(bt)]));
                }
                if let Some(ref plant) = result.decoded.assembly_plant {
                    table.add_row(Row::from(vec![Cell::new("Assembly Plant"), Cell::new(plant)]));
                }
                table.add_row(Row::from(vec![Cell::new("Serial Number"), Cell::new(&result.decoded.serial_number)]));
                table.add_row(Row::from(vec![Cell::new("Origin"), Cell::new(&result.decoded.country)]));

                println!("{table}");
                println!();

                if let Some(id) = result.saved_car_id {
                    println!("✓ Successfully saved and activated in database as Vehicle ID: {}", id);
                } else {
                    println!("ℹ Note: Run with `--save` to persist this discovered vehicle profile into the database.");
                }
            }
        }
        Commands::Protocol { cmd } => match cmd {
            ProtocolCommands::List => {
                let pm = ProtocolManager::new();
                let protocols = pm.list_protocols();

                if cli.json {
                    let mut list = Vec::new();
                    for (name, path) in &protocols {
                        if let Ok(info) = pm.load_protocol(name) {
                            list.push(serde_json::json!({
                                "id": name,
                                "name": info.name,
                                "standard": info.standard,
                                "description": info.description,
                                "path": path.display().to_string(),
                            }));
                        }
                    }
                    println!("{}", serde_json::to_string_pretty(&list)?);
                } else if protocols.is_empty() {
                    println!("No protocol configuration files found under config/protocol/.");
                } else {
                    println!("Supported Diagnostic Protocols:");
                    let mut table = Table::new();
                    table.load_preset(UTF8_FULL).apply_modifier(UTF8_ROUND_CORNERS);
                    table.set_header(vec!["Identifier", "Protocol Name", "Standard", "Default Speed"]);

                    for (name, _) in protocols {
                        if let Ok(info) = pm.load_protocol(&name) {
                            table.add_row(Row::from(vec![
                                Cell::new(name).fg(Color::Cyan),
                                Cell::new(info.name),
                                Cell::new(info.standard),
                                Cell::new(format!("{} bps", info.default_bitrate)),
                            ]));
                        }
                    }
                    println!("{table}");
                }
            }
            ProtocolCommands::Show { name } => {
                let pm = ProtocolManager::new();
                let info = pm.load_protocol(name)?;

                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&info)?);
                } else {
                    println!("=== Protocol Specification: {} ===", info.name);
                    println!("Standard:       {}", info.standard);
                    println!("Description:    {}", info.description);
                    println!("Default Speed:  {} bps", info.default_bitrate);
                    if let Some(ref t) = info.transport {
                        println!("Transport:      {}", t);
                    }
                    println!();
                    println!("{}", serde_json::to_string_pretty(&info.extra)?);
                }
            }
        },
        Commands::Vehicles { cmd } => match cmd {
            VehicleCommands::List => {
                let vm = VehicleManager::new();
                let makes = vm.list_makes()?;

                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&makes)?);
                } else {
                    println!("Supported Vehicle Makes & Top Models Catalog:");
                    let mut table = Table::new();
                    table.load_preset(UTF8_FULL).apply_modifier(UTF8_ROUND_CORNERS);
                    table.set_header(vec!["Make", "Country", "WMI Prefixes", "Default Protocol", "Top 3 Models"]);

                    for make in makes {
                        let models_str: Vec<String> = make.models.iter().map(|m| m.name.clone()).collect();
                        table.add_row(Row::from(vec![
                            Cell::new(&make.name).fg(Color::Green),
                            Cell::new(&make.country),
                            Cell::new(make.wmi_prefixes.join(", ")),
                            Cell::new(&make.default_protocol),
                            Cell::new(models_str.join(", ")),
                        ]));
                    }
                    println!("{table}");
                }
            }
            VehicleCommands::Show { make } => {
                let vm = VehicleManager::new();
                let info = vm.get_make(make)?;

                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&info)?);
                } else {
                    println!("=== Vehicle Make: {} ({}) ===", info.name, info.country);
                    println!("Default Protocol: {} (Default CAN Speed: {} bps)", info.default_protocol, info.default_bitrate);
                    println!("WMI Prefixes:     {}", info.wmi_prefixes.join(", "));
                    println!();

                    for model in info.models {
                        println!("▸ Model: {} [{}]", model.name, model.category);
                        for gen in model.generations {
                            println!("   • Generation: {} ({})", gen.generation, gen.years);
                            println!("     Engines:       {}", gen.engines.join(", "));
                            println!("     Transmissions: {}", gen.transmissions.join(", "));
                            println!("     Protocol:      {} ({} bps)", gen.standard_protocol, gen.can_speed);
                        }
                        if !model.common_ecus.is_empty() {
                            println!("   • Common Diagnostic CAN Endpoints:");
                            for ecu in model.common_ecus {
                                println!("     - {} ({}) -> Req: {}, Resp: {}", ecu.id, ecu.description, ecu.request_id, ecu.response_id);
                            }
                        }
                        println!();
                    }
                }
            }
        },
        Commands::Car { cmd } => match cmd {
            CarCommands::List => {
                let cars = db.list_cars()?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&cars)?);
                } else if cars.is_empty() {
                    println!("No registered vehicles found. Use `carworx car add` to register one.");
                } else {
                    let mut table = Table::new();
                    table.load_preset(UTF8_FULL).apply_modifier(UTF8_ROUND_CORNERS);
                    table.set_header(vec!["ID", "Status", "Name", "Make / Model", "Year", "VIN", "Protocol"]);

                    for car in cars {
                        let status_cell = if car.is_active {
                            Cell::new("ACTIVE").fg(Color::Green)
                        } else {
                            Cell::new("inactive").fg(Color::DarkGrey)
                        };

                        table.add_row(Row::from(vec![
                            Cell::new(car.id),
                            status_cell,
                            Cell::new(car.name),
                            Cell::new(format!("{} {}", car.make, car.model)),
                            Cell::new(car.year),
                            Cell::new(car.vin.unwrap_or_else(|| "-".to_string())),
                            Cell::new(car.protocol),
                        ]));
                    }
                    println!("{table}");
                }
            }
            CarCommands::Add(args) => {
                let new_car = NewCar {
                    name: args.name.clone(),
                    make: args.make.clone(),
                    model: args.model.clone(),
                    year: args.year,
                    trim: args.trim.clone(),
                    vin: args.vin.clone(),
                    engine: args.engine.clone(),
                    transmission: args.transmission.clone(),
                    protocol: args.protocol.clone(),
                    notes: args.notes.clone(),
                };

                // Validate year
                if args.year < 1900 || args.year > 2100 {
                    return Err(CarworxError::validation(
                        "year",
                        format!("Vehicle year {} must be between 1900 and 2100", args.year),
                    ));
                }

                // Validate VIN if provided
                if let Some(ref vin) = args.vin {
                    if vin.trim().len() != 17 {
                        return Err(CarworxError::validation(
                            "vin",
                            format!("Standard VIN must be exactly 17 characters, got {}", vin.trim().len()),
                        ));
                    }
                }

                let car = db.add_car(&new_car)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&car)?);
                } else {
                    println!("Successfully registered vehicle: {} (ID: {})", car.name, car.id);
                    if car.is_active {
                        println!("(Automatically marked as ACTIVE vehicle)");
                    }
                }
            }
            CarCommands::Select { id } => {
                db.set_active_car(*id)?;
                let car = db.get_car_by_id(*id)?.ok_or_else(|| CarworxError::not_found("Vehicle", id))?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&car)?);
                } else {
                    println!("Active vehicle set to: {} {} {} (ID: {})", car.year, car.make, car.model, car.id);
                }
            }
            CarCommands::Active => {
                let car = db.get_active_car()?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&car)?);
                } else if let Some(car) = car {
                    println!("Active Vehicle:");
                    println!("  ID:           {}", car.id);
                    println!("  Name:         {}", car.name);
                    println!("  Vehicle:      {} {} {}", car.year, car.make, car.model);
                    println!("  VIN:          {}", car.vin.as_deref().unwrap_or("-"));
                    println!("  Engine:       {}", car.engine.as_deref().unwrap_or("-"));
                    println!("  Transmission: {}", car.transmission.as_deref().unwrap_or("-"));
                    println!("  Protocol:     {}", car.protocol);
                } else {
                    println!("No active vehicle selected. Run `carworx car list` to see available profiles.");
                }
            }
            CarCommands::Delete { id } => {
                db.delete_car(*id)?;
                println!("Deleted vehicle ID: {}", id);
            }
        },
        Commands::Interface { cmd } => match cmd {
            InterfaceCommands::List => {
                let ifaces = db.list_interfaces()?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&ifaces)?);
                } else if ifaces.is_empty() {
                    println!("No registered interfaces. Run `carworx interface detect` to discover hardware.");
                } else {
                    let mut table = Table::new();
                    table.load_preset(UTF8_FULL).apply_modifier(UTF8_ROUND_CORNERS);
                    table.set_header(vec!["Status", "Name", "Type", "Bitrate", "Last Seen"]);

                    for iface in ifaces {
                        let status_cell = if iface.is_active {
                            Cell::new("ACTIVE").fg(Color::Green)
                        } else {
                            Cell::new("inactive").fg(Color::DarkGrey)
                        };

                        table.add_row(Row::from(vec![
                            status_cell,
                            Cell::new(iface.name),
                            Cell::new(iface.interface_type),
                            Cell::new(format!("{} bps", iface.bitrate)),
                            Cell::new(iface.last_seen_at.unwrap_or_else(|| "-".to_string())),
                        ]));
                    }
                    println!("{table}");
                }
            }
            InterfaceCommands::Detect => {
                let detected = detect_can_interfaces();
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&detected)?);
                } else if detected.is_empty() {
                    println!("No active CAN interfaces found under /sys/class/net.");
                    println!("Make sure your CAN adapter (e.g. CANable) is connected and brought up with `ip link set can0 up`.");
                } else {
                    println!("Detected Linux CAN Interfaces:");
                    let mut table = Table::new();
                    table.load_preset(UTF8_FULL).apply_modifier(UTF8_ROUND_CORNERS);
                    table.set_header(vec!["Interface", "Link State", "Bitrate", "RX Bytes", "TX Bytes"]);

                    for iface in detected {
                        let state_cell = if iface.is_up {
                            Cell::new(&iface.operstate).fg(Color::Green)
                        } else {
                            Cell::new(&iface.operstate).fg(Color::Red)
                        };

                        let bitrate_str = iface
                            .bitrate
                            .map(|b| format!("{} bps", b))
                            .unwrap_or_else(|| "N/A".to_string());

                        table.add_row(Row::from(vec![
                            Cell::new(iface.name),
                            state_cell,
                            Cell::new(bitrate_str),
                            Cell::new(iface.rx_bytes),
                            Cell::new(iface.tx_bytes),
                        ]));
                    }
                    println!("{table}");
                }
            }
            InterfaceCommands::Select { name, bitrate } => {
                if *bitrate == 0 || *bitrate > 1_000_000 {
                    return Err(CarworxError::validation(
                        "bitrate",
                        format!("Bitrate {} is invalid. Typical CAN speeds: 125000, 250000, 500000, 1000000 bps", bitrate),
                    ));
                }

                db.register_interface(name, "socketcan", *bitrate, true, None)?;
                println!("Active hardware interface set to: {} ({} bps)", name, bitrate);
            }
            InterfaceCommands::Active => {
                let iface = db.get_active_interface()?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&iface)?);
                } else if let Some(iface) = iface {
                    println!("Active Interface: {} (Type: {}, Bitrate: {} bps)", iface.name, iface.interface_type, iface.bitrate);
                } else {
                    println!("No active interface selected. Run `carworx interface detect` or `carworx interface select <name>`.");
                }
            }
        },
        Commands::Config { cmd } => match cmd {
            ConfigCommands::List { car_id } => {
                let configs = db.list_configs(*car_id)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&configs)?);
                } else if configs.is_empty() {
                    println!("No configuration files registered. Use `carworx config add` to register one.");
                } else {
                    let mut table = Table::new();
                    table.load_preset(UTF8_FULL).apply_modifier(UTF8_ROUND_CORNERS);
                    table.set_header(vec!["ID", "Name", "Type", "Car ID", "File Path", "Description"]);

                    for cfg in configs {
                        table.add_row(Row::from(vec![
                            Cell::new(cfg.id),
                            Cell::new(cfg.name),
                            Cell::new(cfg.config_type),
                            Cell::new(cfg.car_id.map(|id| id.to_string()).unwrap_or_else(|| "Global".to_string())),
                            Cell::new(cfg.file_path),
                            Cell::new(cfg.description.unwrap_or_else(|| "-".to_string())),
                        ]));
                    }
                    println!("{table}");
                }
            }
            ConfigCommands::Add {
                name,
                config_type,
                path,
                car_id,
                description,
            } => {
                // Verify car_id exists if specified
                if let Some(cid) = car_id {
                    if db.get_car_by_id(*cid)?.is_none() {
                        return Err(CarworxError::not_found("Vehicle", cid));
                    }
                }

                // Verify configuration file exists on disk
                let p = Path::new(path);
                if !p.exists() {
                    return Err(CarworxError::config_invalid(
                        path,
                        "Configuration file does not exist on disk",
                    ));
                }

                let new_cfg = NewCarConfig {
                    car_id: *car_id,
                    name: name.clone(),
                    config_type: config_type.clone(),
                    file_path: path.clone(),
                    description: description.clone(),
                };
                let cfg = db.add_config(&new_cfg)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&cfg)?);
                } else {
                    println!("Registered config profile: {} (ID: {}, Path: {})", cfg.name, cfg.id, cfg.file_path);
                }
            }
        },
        Commands::Adapter { cmd } => handle_adapter_command(cli, &cmd)?,
        Commands::Dtc { cmd } => handle_dtc_command(db, cli, &cmd)?,
        Commands::Status => {
            let active_car = db.get_active_car()?;
            let active_iface = db.get_active_interface()?;
            let detected = detect_can_interfaces();

            if cli.json {
                let status_obj = serde_json::json!({
                    "database_path": db.path().display().to_string(),
                    "active_car": active_car,
                    "active_interface": active_iface,
                    "detected_interfaces": detected,
                });
                println!("{}", serde_json::to_string_pretty(&status_obj)?);
            } else {
                println!("=== Carworx Core Status ===");
                println!("Database: {}", db.path().display());
                println!();

                println!("Active Vehicle:");
                if let Some(car) = active_car {
                    println!("  {} {} {} (VIN: {})", car.year, car.make, car.model, car.vin.as_deref().unwrap_or("-"));
                    println!("  Protocol: {}", car.protocol);
                } else {
                    println!("  None selected (Run `carworx car list` or `carworx car add`)");
                }
                println!();

                println!("Active Hardware Interface:");
                if let Some(iface) = active_iface {
                    println!("  {} [{} bps, {}]", iface.name, iface.bitrate, iface.interface_type);
                } else {
                    println!("  None selected (Run `carworx interface select <name>`)");
                }
                println!();

                println!("Detected System CAN Interfaces:");
                if detected.is_empty() {
                    println!("  None detected under /sys/class/net");
                } else {
                    for d in detected {
                        println!("  - {}: state={} rx={} tx={}", d.name, d.operstate, d.rx_bytes, d.tx_bytes);
                    }
                }

                println!();
                println!("CAN Telemetry Service:");
                let svc = crate::components::calibrations::telemetry::get_telemetry_service();
                let stats = svc.get_stats();
                let is_active = svc.is_bus_active();
                let status_str = if is_active { "Active (Streaming)" } else { "Idle / Waiting" };
                let iface_display = if stats.active_interface.is_empty() { "can0" } else { &stats.active_interface };
                println!("  Status: {} | Monitored Interface: {}", status_str, iface_display);
                println!("  Frames Received: {} | Rate: {:.1} fps | Active Kernel Filters: {}", stats.total_frames_received, stats.frames_per_second, stats.active_filter_count);
            }
        }
        Commands::Telemetry { interface, seconds } => {
            handle_telemetry_command(interface, *seconds, cli.json)?;
        }
    }
    Ok(())
}

fn handle_telemetry_command(interface: &str, seconds: u64, json: bool) -> Result<()> {
    use crate::components::calibrations::telemetry::get_telemetry_service;
    use std::thread;
    use std::time::Duration;

    let svc = get_telemetry_service();
    svc.start(interface);

    if !json {
        println!("=== CAN Telemetry Service ===");
        println!("Monitoring {} with kernel CAN filters for {} seconds...", interface, seconds);
    }

    thread::sleep(Duration::from_secs(seconds));

    let stats = svc.get_stats();
    let signals = svc.get_all_signals();

    if json {
        let obj = serde_json::json!({
            "interface": interface,
            "connected": stats.is_connected,
            "total_frames": stats.total_frames_received,
            "fps": stats.frames_per_second,
            "signals": signals.iter().map(|(k, v, live, cnt)| {
                serde_json::json!({
                    "name": k,
                    "value": v,
                    "is_live": live,
                    "updates": cnt,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&obj)?);
    } else {
        println!();
        println!("Status: {} | Received: {} frames ({:.1} fps)",
            if stats.is_connected { "CONNECTED / STREAMING" } else { "NO FRAMES DETECTED" },
            stats.total_frames_received, stats.frames_per_second
        );
        println!();

        if signals.is_empty() {
            println!("No vehicle signals received yet. Ensure ignition is ON.");
        } else {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL).apply_modifier(UTF8_ROUND_CORNERS);
            table.set_header(vec!["Signal Name", "Current Value", "Status", "Frame Count"]);

            for (name, val, live, cnt) in signals {
                let status_cell = if live {
                    Cell::new("LIVE CAN").fg(Color::Green)
                } else {
                    Cell::new("OFFLINE").fg(Color::Yellow)
                };

                table.add_row(Row::from(vec![
                    Cell::new(name).fg(Color::Cyan),
                    Cell::new(format!("{:+.2}", val)),
                    status_cell,
                    Cell::new(cnt.to_string()),
                ]));
            }
            println!("{table}");
        }
    }
    Ok(())
}

fn handle_adapter_command(cli: &Cli, cmd: &AdapterCommands) -> Result<()> {
    match cmd {
        AdapterCommands::List => {
            let adapters = discover_adapters();
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&adapters)?);
            } else {
                println!("=== Discovered CAN Bus Adapters ===");
                if adapters.is_empty() {
                    println!("No CAN adapters detected under /sys/class/net or USB.");
                    println!("Connect a CANable or configure a SocketCAN interface with 'sudo ip link set can0 up'.");
                    return Ok(());
                }

                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .apply_modifier(UTF8_ROUND_CORNERS)
                    .set_header(vec![
                        "Adapter Name",
                        "Type",
                        "Interface",
                        "Serial Port",
                        "State",
                        "Bitrate",
                        "RX / TX Packets",
                    ]);

                for a in adapters {
                    let port_str = a.serial_port.as_deref().unwrap_or("-");
                    let bitrate_str = a.bitrate.map(|b| format!("{} bps", b)).unwrap_or_else(|| "-".to_string());
                    let state_color = if a.is_up { Color::Green } else { Color::Red };
                    let packets_str = format!("{} / {}", a.rx_packets, a.tx_packets);

                    table.add_row(Row::from(vec![
                        Cell::new(&a.name).fg(Color::Cyan),
                        Cell::new(a.adapter_type.to_string()),
                        Cell::new(&a.interface),
                        Cell::new(port_str),
                        Cell::new(&a.operstate).fg(state_color),
                        Cell::new(bitrate_str),
                        Cell::new(packets_str),
                    ]));
                }
                println!("{table}");
            }
        }
        AdapterCommands::Test(args) => {
            if !cli.json {
                println!("Probing CAN adapter and verifying hardware communication...");
            }
            let mut adapter = CanableAdapter::detect_or_default(args.interface.as_deref())?;
            let result = adapter.test_communication()?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!();
                println!("=== CAN Adapter Communication Test ===");
                println!("Device:       {}", result.adapter_name);
                println!("Interface:    {}", result.interface);
                println!(
                    "Result:       {}",
                    if result.success { "PASSED" } else { "FAILED" }
                );
                if let Some(lat) = result.latency_ms {
                    println!("TX Latency:   {:.2} ms", lat);
                }
                println!("Test Frame:   {}", result.test_frame.format_hex());
                println!(
                    "Live Traffic: {}",
                    if result.live_traffic_detected {
                        format!("ACTIVE ({} frames captured)", result.rx_frames_captured)
                    } else {
                        "IDLE (no response within 300ms)".to_string()
                    }
                );
                println!("Details:      {}", result.details);

                if !result.sample_frames.is_empty() {
                    println!();
                    println!("Sample Bus Frames Captured:");
                    for frame in &result.sample_frames {
                        println!(
                            "  {}  [{}]  {}",
                            frame.id_hex(),
                            frame.dlc(),
                            frame.display_data_hex()
                        );
                    }
                }
                println!();
            }
        }
    }
    Ok(())
}

fn handle_dtc_command(db: &Database, cli: &Cli, cmd: &DtcCommands) -> Result<()> {
    match cmd {
        DtcCommands::Scan(args) => {
            let car = if let Some(id) = args.car_id {
                db.get_car_by_id(id)?
                    .ok_or_else(|| CarworxError::not_found("Vehicle", id))?
            } else {
                db.get_active_car()?
                    .ok_or_else(|| CarworxError::validation(
                        "vehicle",
                        "No active vehicle selected. Select a vehicle with `carworx car select <id>` or run `carworx discover`."
                    ))?
            };

            let iface = if let Some(ref i) = args.interface {
                i.clone()
            } else if let Ok(Some(active_iface)) = db.get_active_interface() {
                active_iface.name
            } else {
                "can0".to_string()
            };

            if !cli.json {
                println!("=== 🔧 Automotive Diagnostic (DTC) Scan ===");
                println!("Vehicle:   {} {} {}", car.year, car.make, car.model);
                if let Some(ref vin) = car.vin {
                    println!("VIN:       {}", vin);
                }
                println!("Interface: {}", iface);
                println!("Scanning 6 vehicle domains for confirmed DTCs...");
                println!();
            }

            let scan_result = perform_diagnostic_scan(&car, &iface)?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&scan_result)?);
            } else {
                println!("Scan Mode:      {}", if scan_result.is_live { "LIVE CAN BUS" } else { "BASELINE SNAPSHOT" });
                println!("Confirmed DTCs: {}", scan_result.total_confirmed_dtcs);
                println!("Status Notes:   {}", scan_result.notes);
                println!();

                for group in &scan_result.domains {
                    let header_status = if group.has_faults() {
                        format!("⚠️  {} CONFIRMED FAULT(S)", group.fault_count())
                    } else {
                        "✓ CLEAN (Normal)".to_string()
                    };

                    println!("--------------------------------------------------");
                    println!(
                        "{} {} ({})",
                        group.domain.icon(),
                        group.domain.name(),
                        group.ecu_name
                    );
                    println!(
                        "CAN Addressing: Req 0x{:03X} -> Resp 0x{:03X} | Status: {}",
                        group.request_id, group.response_id, header_status
                    );

                    if group.has_faults() {
                        let mut table = Table::new();
                        table.load_preset(UTF8_FULL).apply_modifier(UTF8_ROUND_CORNERS);
                        table.set_header(vec!["DTC Code", "Status", "Description", "Raw Bytes", "Flags"]);

                        for dtc in &group.confirmed_dtcs {
                            table.add_row(Row::from(vec![
                                Cell::new(&dtc.code).fg(Color::Red),
                                Cell::new("CONFIRMED").fg(Color::Red),
                                Cell::new(&dtc.description),
                                Cell::new(&dtc.raw_hex).fg(Color::DarkGrey),
                                Cell::new(dtc.status_labels.join(", ")).fg(Color::Yellow),
                            ]));
                        }
                        println!("{table}");
                    } else {
                        println!("  No confirmed DTCs reported.");
                    }
                    println!();
                }
            }
        }
        DtcCommands::Lookup { code } => {
            let code_upper = code.to_uppercase();
            let desc = get_dtc_description(&code_upper);
            if cli.json {
                let obj = serde_json::json!({
                    "code": code_upper,
                    "description": desc,
                });
                println!("{}", serde_json::to_string_pretty(&obj)?);
            } else {
                println!("=== Diagnostic Trouble Code Lookup ===");
                println!("DTC Code:    {}", code_upper);
                println!("Description: {}", desc);
            }
        }
    }
    Ok(())
}

