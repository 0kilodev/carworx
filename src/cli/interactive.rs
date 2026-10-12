use console::{style, Term};
use dialoguer::{Confirm, Input, Select};

use crate::components::discovery::AutoDiscoveryComponent;
use crate::config::ProtocolManager;
use crate::core::hardware::detect_can_interfaces;
use crate::core::Database;
use crate::error::{CarworxError, Result};
use crate::schema::models::{Car, NewCar};

pub fn run_interactive_terminal(db: &Database) -> Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    println!("{}", style("==================================================").cyan());
    println!("{}", style("          🚗 CARWORX AUTOMOTIVE PLATFORM          ").bold().cyan());
    println!("{}", style("   Universal Diagnostic & CAN Bus Terminal Engine ").cyan());
    println!("{}", style("==================================================").cyan());
    println!();
    println!("Database: {}", style(db.path().display()).dim());
    println!();

    // STEP 1: VEHICLE SELECTION OR CREATION
    ensure_active_vehicle(db)?;

    // STEP 2: HARDWARE INTERFACE SELECTION
    ensure_active_interface(db)?;

    // STEP 3: MAIN DASHBOARD LOOP
    main_dashboard_loop(db)?;

    Ok(())
}

fn ensure_active_vehicle(db: &Database) -> Result<Car> {
    let cars = db.list_cars()?;

    if cars.is_empty() {
        println!("{}", style("No vehicles are currently registered.").yellow().bold());
        println!("Let's configure your first vehicle profile:");
        println!();

        let options = vec![
            "1. 🔍 Auto-Discover Hardware Interface & Vehicle over CAN",
            "2. Quick Preset: 2015 Subaru Forester SJ (2.5L FB25, Lineartronic CVT)",
            "3. Custom Vehicle Entry",
        ];

        let selection = Select::new()
            .with_prompt("Choose an option")
            .items(&options)
            .default(0)
            .interact()?;

        if selection == 0 {
            println!("\n{}", style("Running Auto-Discovery on active CAN bus...").cyan().bold());
            let component = AutoDiscoveryComponent::new();
            match component.discover(db, None, Some("JF2SJABC7FH552058"), true) {
                Ok(res) => {
                    println!("\n{}", style("✓ Vehicle Successfully Discovered!").green().bold());
                    println!("Interface: {} [{}]", res.interface, res.interface_state);
                    println!("VIN:       {}", res.decoded.vin);
                    println!("Vehicle:   {} {} {}", res.decoded.year, res.decoded.make, res.decoded.model);
                    println!("Origin:    {}", res.decoded.country);
                    if let Some(ref plant) = res.decoded.assembly_plant {
                        println!("Plant:     {}", plant);
                    }
                    println!();
                    let car = db.get_active_car()?.unwrap();
                    return Ok(car);
                }
                Err(e) => {
                    println!("\n{} {}", style("Auto-Discovery error:").yellow(), e);
                    println!("Falling back to manual vehicle entry...\n");
                }
            }
        }

        let new_car = if selection == 1 {
            NewCar {
                name: "Forester SJ (2015)".to_string(),
                make: "Subaru".to_string(),
                model: "Forester".to_string(),
                year: 2015,
                trim: Some("2.5i Premium".to_string()),
                vin: Some("JF2SJABC7FH552058".to_string()),
                engine: Some("2.5L DOHC FB25".to_string()),
                transmission: Some("Lineartronic CVT (TR580)".to_string()),
                protocol: Some("ISO15765-4 / CAN 500k".to_string()),
                notes: Some("Target development vehicle for Carworx project".to_string()),
            }
        } else {
            prompt_custom_vehicle()?
        };

        let car = db.add_car(&new_car)?;
        println!(
            "\n{} {} (ID: {})\n",
            style("✓ Successfully registered:").green().bold(),
            style(&car.name).bold(),
            car.id
        );
        Ok(car)
    } else {
        println!("{}", style("Select Active Vehicle:").bold());
        let mut items = Vec::new();
        let mut active_idx = 0;

        for (i, c) in cars.iter().enumerate() {
            let badge = if c.is_active {
                active_idx = i;
                format!("[{}] ", style("ACTIVE").green().bold())
            } else {
                String::new()
            };
            items.push(format!("{}{} {} {} (VIN: {})", badge, c.year, c.make, c.model, c.vin.as_deref().unwrap_or("-")));
        }
        items.push("+ Register a New Vehicle...".to_string());

        let selection = Select::new()
            .with_prompt("Vehicle Profile")
            .items(&items)
            .default(active_idx)
            .interact()?;

        if selection == items.len() - 1 {
            let new_car = prompt_custom_vehicle()?;
            let car = db.add_car(&new_car)?;
            db.set_active_car(car.id)?;
            println!("\n{} {}\n", style("✓ Registered & Activated:").green().bold(), style(&car.name).bold());
            Ok(car)
        } else {
            let selected_car = &cars[selection];
            if !selected_car.is_active {
                db.set_active_car(selected_car.id)?;
            }
            let active_car = db
                .get_active_car()?
                .ok_or_else(|| CarworxError::not_found("Vehicle", "active"))?;
            println!(
                "{} {} {} {}\n",
                style("✓ Active Vehicle:").green().bold(),
                active_car.year,
                active_car.make,
                active_car.model
            );
            Ok(active_car)
        }
    }
}

fn prompt_custom_vehicle() -> Result<NewCar> {
    let name: String = Input::new()
        .with_prompt("Profile Name (e.g. My Forester)")
        .interact_text()?;

    let make: String = Input::new()
        .with_prompt("Make (e.g. Subaru)")
        .default("Subaru".to_string())
        .interact_text()?;

    let model: String = Input::new()
        .with_prompt("Model (e.g. Forester)")
        .default("Forester".to_string())
        .interact_text()?;

    let year_str: String = Input::new()
        .with_prompt("Year")
        .default("2015".to_string())
        .interact_text()?;
    let year: i32 = year_str.parse().map_err(|_| {
        CarworxError::validation("year", format!("'{}' is not a valid number", year_str))
    })?;

    if year < 1900 || year > 2100 {
        return Err(CarworxError::validation(
            "year",
            format!("Vehicle year {} must be between 1900 and 2100", year),
        ));
    }

    let vin_input: String = Input::new()
        .with_prompt("VIN (optional)")
        .allow_empty(true)
        .interact_text()?;
    let vin = if vin_input.trim().is_empty() {
        None
    } else {
        let trimmed = vin_input.trim();
        if trimmed.len() != 17 {
            return Err(CarworxError::validation(
                "vin",
                format!("VIN must be 17 characters, got {}", trimmed.len()),
            ));
        }
        Some(trimmed.to_string())
    };

    let engine_input: String = Input::new()
        .with_prompt("Engine (e.g. 2.5L FB25)")
        .default("2.5L FB25".to_string())
        .interact_text()?;
    let engine = if engine_input.trim().is_empty() {
        None
    } else {
        Some(engine_input.trim().to_string())
    };

    let trans_input: String = Input::new()
        .with_prompt("Transmission (e.g. CVT, Manual)")
        .default("CVT".to_string())
        .interact_text()?;
    let transmission = if trans_input.trim().is_empty() {
        None
    } else {
        Some(trans_input.trim().to_string())
    };

    Ok(NewCar {
        name,
        make,
        model,
        year,
        trim: None,
        vin,
        engine,
        transmission,
        protocol: Some("ISO15765-4 / CAN 500k".to_string()),
        notes: None,
    })
}

fn ensure_active_interface(db: &Database) -> Result<()> {
    let detected = detect_can_interfaces();
    let current_active = db.get_active_interface()?;

    println!("{}", style("Select Hardware CAN Interface:").bold());
    let mut options = Vec::new();
    let mut default_idx = 0;

    for (i, d) in detected.iter().enumerate() {
        let is_curr = current_active.as_ref().map(|a| a.name == d.name).unwrap_or(false);
        if is_curr {
            default_idx = i;
        }
        let status = if d.is_up {
            style("UP").green()
        } else {
            style("DOWN").red()
        };
        options.push(format!("{} (State: {}, RX: {}, TX: {})", d.name, status, d.rx_bytes, d.tx_bytes));
    }

    options.push("can0 (Standard SocketCAN / CANable default)".to_string());
    options.push("vcan0 (Virtual CAN interface for offline testing)".to_string());
    options.push("Enter custom interface name...".to_string());

    if default_idx == 0 && detected.is_empty() && current_active.is_some() {
        // keep current active
    }

    let selection = Select::new()
        .with_prompt("Hardware Interface")
        .items(&options)
        .default(default_idx)
        .interact()?;

    let iface_name = if selection < detected.len() {
        detected[selection].name.clone()
    } else if selection == detected.len() {
        "can0".to_string()
    } else if selection == detected.len() + 1 {
        "vcan0".to_string()
    } else {
        Input::new().with_prompt("Interface Name").interact_text()?
    };

    db.register_interface(&iface_name, "socketcan", 500000, true, None)?;
    println!(
        "{} {}\n",
        style("✓ Active Hardware Interface:").green().bold(),
        style(&iface_name).bold()
    );

    Ok(())
}

fn main_dashboard_loop(db: &Database) -> Result<()> {
    loop {
        let active_car = db.get_active_car()?;
        let active_iface = db.get_active_interface()?;

        println!("{}", style("--------------------------------------------------").dim());
        if let Some(ref car) = active_car {
            println!(
                "🚗 Vehicle:   {} {} {} | VIN: {}",
                style(car.year).yellow(),
                style(&car.make).yellow(),
                style(&car.model).yellow(),
                car.vin.as_deref().unwrap_or("-")
            );
        } else {
            println!("🚗 Vehicle:   {}", style("None selected").red());
        }

        if let Some(ref iface) = active_iface {
            println!(
                "🔌 Interface: {} [{} bps, {}]",
                style(&iface.name).green().bold(),
                iface.bitrate,
                iface.interface_type
            );
        } else {
            println!("🔌 Interface: {}", style("None selected").red());
        }
        println!("{}", style("--------------------------------------------------").dim());

        let menu = vec![
            "1. 🚗 View Active Vehicle Profile Details",
            "2. 🔄 Switch Active Vehicle",
            "3. ➕ Register a New Vehicle",
            "4. 🔍 Auto-Discover Vehicle & Hardware over CAN",
            "5. 🔌 Switch / Configure Hardware Interface",
            "6. 📡 Scan / Detect Linux CAN Hardware",
            "7. 📜 View Diagnostic Protocols (UDS, J1979, J2534, SSM)",
            "8. 📁 View Registered Config Profiles & CAN Maps",
            "9. 🚪 Exit",
        ];

        let choice = Select::new()
            .with_prompt("Dashboard Action")
            .items(&menu)
            .default(0)
            .interact()?;

        match choice {
            0 => {
                // View vehicle details
                if let Some(car) = active_car {
                    println!("\n{}", style("=== Vehicle Profile Details ===").bold().cyan());
                    println!("ID:           {}", car.id);
                    println!("Profile:      {}", car.name);
                    println!("Make:         {}", car.make);
                    println!("Model:        {}", car.model);
                    println!("Year:         {}", car.year);
                    println!("Trim:         {}", car.trim.as_deref().unwrap_or("-"));
                    println!("VIN:          {}", car.vin.as_deref().unwrap_or("-"));
                    println!("Engine:       {}", car.engine.as_deref().unwrap_or("-"));
                    println!("Transmission: {}", car.transmission.as_deref().unwrap_or("-"));
                    println!("Protocol:     {}", car.protocol);
                    println!("Notes:        {}", car.notes.as_deref().unwrap_or("-"));
                    println!("Created:      {}", car.created_at);
                    println!();
                }
            }
            1 => {
                // Switch active vehicle
                let cars = db.list_cars()?;
                let mut car_items = Vec::new();
                for c in &cars {
                    let badge = if c.is_active { " [ACTIVE]" } else { "" };
                    car_items.push(format!("{} {} {}{}", c.year, c.make, c.model, badge));
                }
                let sel = Select::new()
                    .with_prompt("Select vehicle to activate")
                    .items(&car_items)
                    .interact()?;
                db.set_active_car(cars[sel].id)?;
                println!("{} {}", style("✓ Activated:").green(), cars[sel].name);
            }
            2 => {
                // Register new vehicle
                let new_car = prompt_custom_vehicle()?;
                let car = db.add_car(&new_car)?;
                if Confirm::new().with_prompt("Activate this vehicle now?").default(true).interact()? {
                    db.set_active_car(car.id)?;
                }
            }
            3 => {
                // Auto-Discover
                println!("\n{}", style("Running Auto-Discovery on active CAN bus...").cyan().bold());
                let component = AutoDiscoveryComponent::new();
                match component.discover(db, None, Some("JF2SJABC7FH552058"), true) {
                    Ok(res) => {
                        println!("\n{}", style("✓ Vehicle Discovered & Activated!").green().bold());
                        println!("Interface: {} [{}]", res.interface, res.interface_state);
                        println!("VIN:       {}", res.decoded.vin);
                        println!("Vehicle:   {} {} {}", res.decoded.year, res.decoded.make, res.decoded.model);
                        println!("Origin:    {}", res.decoded.country);
                        if let Some(ref plant) = res.decoded.assembly_plant {
                            println!("Plant:     {}", plant);
                        }
                        println!();
                    }
                    Err(e) => {
                        println!("\n{} {}\n", style("Auto-Discovery error:").red(), e);
                    }
                }
            }
            4 => {
                // Switch interface
                ensure_active_interface(db)?;
            }
            5 => {
                // Detect hardware
                let detected = detect_can_interfaces();
                println!("\n{}", style("=== Detected Linux CAN Hardware ===").bold().cyan());
                if detected.is_empty() {
                    println!("No active CAN interfaces found under /sys/class/net.");
                } else {
                    for d in detected {
                        let st = if d.is_up { style("UP").green() } else { style("DOWN").red() };
                        println!("- {} (State: {}, RX: {} bytes, TX: {} bytes)", style(d.name).bold(), st, d.rx_bytes, d.tx_bytes);
                    }
                }
                println!();
            }
            6 => {
                // View protocols
                let pm = ProtocolManager::new();
                let protocols = pm.list_protocols();
                println!("\n{}", style("=== Diagnostic Protocols ===").bold().cyan());
                for (name, _) in protocols {
                    if let Ok(info) = pm.load_protocol(&name) {
                        println!("• {} - {} ({})", style(info.name).bold().yellow(), info.standard, info.description);
                    }
                }
                println!();
            }
            7 => {
                // View config profiles
                let car_id = active_car.as_ref().map(|c| c.id);
                let configs = db.list_configs(car_id)?;
                println!("\n{}", style("=== Registered Configuration Profiles ===").bold().cyan());
                if configs.is_empty() {
                    println!("No configuration profiles currently registered for this car.");
                    println!("(CAN maps and DTC catalogs can be registered using `carworx config add`)");
                } else {
                    for cfg in configs {
                        println!("- [{}] {} -> {}", cfg.config_type, style(cfg.name).bold(), cfg.file_path);
                    }
                }
                println!();
            }
            8 => {
                // Exit
                println!("Exiting Carworx. Goodbye!");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
