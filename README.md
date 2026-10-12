# Carworx Core Application

A high-performance, compact automotive diagnostic and CAN bus application built with **Rust** designed to run efficiently on embedded hardware (such as Raspberry Pi 3) and desktop workstations.

---

## Key Highlights

- **Embedded SQLite Core:** Zero-overhead, zero-server SQLite database statically bundled into the binary via `rusqlite` (`bundled`).
- **Universal Vehicle Profiles:** Supports any car. Pre-seeded with the 2015 Subaru Forester SJ (2.5L FB25, CVT).
- **Dual Mode (Headless & Interactive):**
  - **Interactive Terminal:** Menu-driven workflow with vehicle selection, interface detection, and dashboard management.
  - **Headless / CLI:** Every function is fully available via non-interactive subcommands and scriptable JSON output (`--headless --json`).
- **Automatic Linux CAN Interface Detection:** Auto-discovers physical SocketCAN adapters (`can0`), virtual interfaces (`vcan0`), and serial slcan devices via `/sys/class/net`.
- **Extensible Configuration Engine:** Supports attaching external CAN maps (e.g. `can-map.json`), DTC databases, and DBC definitions per vehicle or globally.
- **Ultra Compact Binary:** Cross-compiles to a **1.9 MB** stripped native `aarch64` binary optimized with LTO and size profile (`opt-level = "z"`).

---

## Directory Structure

```
app/carworx/
├── Cargo.toml                  # Rust dependencies & size-optimization profiles
├── .cargo/
│   └── config.toml             # aarch64 cross-linker configuration
├── build_and_deploy_pi.sh      # Automated 1-click build and deploy to Pi
├── schema/
│   ├── 001_initial_schema.sql  # DDL migrations (cars, interfaces, configs, app_state)
│   └── README.md
└── src/
    ├── main.rs                 # CLI parser & mode switcher (interactive vs headless)
    ├── error.rs                # Universal error hierarchy & structured JSON/terminal rendering
    ├── schema/
    │   ├── mod.rs
    │   └── models.rs           # Rust data models (Car, HardwareInterface, CarConfig)
    ├── core/
    │   ├── mod.rs
    │   ├── db.rs               # SQLite storage engine & CRUD operations
    │   └── hardware.rs         # Linux CAN interface auto-detection engine
    └── cli/
        ├── mod.rs
        ├── commands.rs         # Headless CLI command handlers
        └── interactive.rs      # Interactive terminal TUI wizard
```

---

## Universal Error Handling

Carworx provides a strongly-typed, domain-specific error module in [`src/error.rs`](src/error.rs). Every operation returns `error::Result<T>`.

### Error Codes
| Code | Error Variant | Description / Example |
|---|---|---|
| `ERR_NOT_FOUND` | `CarworxError::NotFound` | Vehicle or interface not found |
| `ERR_HARDWARE_NOT_FOUND` | `CarworxError::HardwareNotFound` | CAN interface not detected on system |
| `ERR_HARDWARE_DOWN` | `CarworxError::HardwareDown` | CAN interface is offline / down |
| `ERR_CONFIG_INVALID` | `CarworxError::ConfigInvalid` | Configuration / CAN map file missing or invalid |
| `ERR_VALIDATION` | `CarworxError::Validation` | Invalid input parameters (year, VIN length, bitrate) |
| `ERR_DATABASE` | `CarworxError::Database` | SQLite query or constraint violation |
| `ERR_DB_INIT` | `CarworxError::DatabaseInit` | SQLite directory creation or migration failure |
| `ERR_PROTOCOL` | `CarworxError::Protocol` | Diagnostic/UDS transport errors |
| `ERR_TERMINAL` | `CarworxError::Terminal` | Interactive terminal dialog failure |

When running with `--json`, errors are rendered as machine-readable JSON:
```json
{
  "error": true,
  "code": "ERR_NOT_FOUND",
  "message": "Vehicle with identifier '9999' was not found",
  "suggestion": "Run `carworx car list` to view valid vehicles, or `carworx car add` to create one."
}
```

---

## Usage

### 1. Interactive Terminal Mode (Default)
When run without subcommands (in any terminal or over SSH):
```bash
carworx
```
1. Displays registered vehicles for selection, with an option to add a new car.
2. Auto-detects available CAN hardware (e.g. `can0`, `vcan0`).
3. Launches the main diagnostic dashboard.

### 2. Headless / Terminal CLI Mode
Every function is scriptable and supports `--json`:

#### Vehicle Management
```bash
# List all cars
carworx --headless car list

# Register a new car
carworx --headless car add \
  --name "2015 Subaru Forester SJ" \
  --make "Subaru" \
  --model "Forester" \
  --year 2015 \
  --vin "JF2SJABC7FH552058" \
  --engine "2.5L FB25" \
  --transmission "Lineartronic CVT"

# Set active vehicle by ID
carworx --headless car select 1

# Show active vehicle
carworx --headless car active
```

#### Hardware Interface Selection
```bash
# Auto-detect CAN hardware on system
carworx --headless interface detect

# Select active CAN interface and bitrate
carworx --headless interface select can0 --bitrate 500000

# Show active interface
carworx --headless interface active
```

#### Universal Configuration Files & CAN Maps
```bash
# Attach a CAN map to a vehicle profile
carworx --headless config add \
  --name "Subaru Forester CAN Map" \
  --type "can_map" \
  --path "/home/esel/Documents/carworx/docs/can-map.json" \
  --car-id 1 \
  --description "177 reverse-engineered 11-bit CAN IDs and SSM endpoints"

# List configurations
carworx --headless config list
```

#### System Status
```bash
# Formatted terminal status
carworx --headless status

# JSON status for scripting / integrations
carworx --headless --json status
```

---

## Cross-Compiling & Deploying to Raspberry Pi

The workspace is pre-configured with a self-contained cross-compilation pipeline:
```bash
# From app/carworx:
./build_and_deploy_pi.sh
```
This builds an optimized `aarch64` native binary and synchronizes it to `esel@192.168.1.120:~/Documents/carworx/bin/carworx`.
