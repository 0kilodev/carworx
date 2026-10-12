use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{CarworxError, Result};
use crate::schema::models::{Car, CarConfig, HardwareInterface, NewCar, NewCarConfig};

const INIT_SCHEMA: &str = include_str!("../../schema/001_initial_schema.sql");

pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Database {
    /// Opens or creates the SQLite database at default path (~/.carworx/carworx.db) or specified path
    pub fn open(custom_path: Option<&Path>) -> Result<Self> {
        let db_path = match custom_path {
            Some(p) => p.to_path_buf(),
            None => {
                let home = std::env::var("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("."));
                let dir = home.join(".carworx");
                fs::create_dir_all(&dir)
                    .map_err(|e| CarworxError::DatabaseInit(format!("Failed to create database directory '{}': {}", dir.display(), e)))?;
                dir.join("carworx.db")
            }
        };

        let conn = Connection::open(&db_path)
            .map_err(|e| CarworxError::DatabaseInit(format!("Failed to open database file '{}': {}", db_path.display(), e)))?;

        let db = Database { conn, path: db_path };
        db.init()?;
        Ok(db)
    }

    /// In-memory database for testing
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Database {
            conn,
            path: PathBuf::from(":memory:"),
        };
        db.init()?;
        Ok(db)
    }

    /// Initializes schema and tables
    pub fn init(&self) -> Result<()> {
        self.conn
            .execute_batch(INIT_SCHEMA)
            .map_err(|e| CarworxError::DatabaseInit(format!("Migration failed: {}", e)))?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    // ==========================================
    // VEHICLE (CAR) OPERATIONS
    // ==========================================

    /// List all registered cars
    pub fn list_cars(&self) -> Result<Vec<Car>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, make, model, year, trim, vin, engine, transmission, protocol, is_active, notes, created_at, updated_at
             FROM cars
             ORDER BY is_active DESC, updated_at DESC, id DESC",
        )?;

        let car_iter = stmt.query_map([], |row| {
            Ok(Car {
                id: row.get(0)?,
                name: row.get(1)?,
                make: row.get(2)?,
                model: row.get(3)?,
                year: row.get(4)?,
                trim: row.get(5)?,
                vin: row.get(6)?,
                engine: row.get(7)?,
                transmission: row.get(8)?,
                protocol: row.get(9)?,
                is_active: row.get::<_, i64>(10)? != 0,
                notes: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })?;

        let mut cars = Vec::new();
        for car in car_iter {
            cars.push(car?);
        }
        Ok(cars)
    }

    /// Get active car if any
    pub fn get_active_car(&self) -> Result<Option<Car>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, make, model, year, trim, vin, engine, transmission, protocol, is_active, notes, created_at, updated_at
             FROM cars
             WHERE is_active = 1
             LIMIT 1",
        )?;

        let mut car_iter = stmt.query_map([], |row| {
            Ok(Car {
                id: row.get(0)?,
                name: row.get(1)?,
                make: row.get(2)?,
                model: row.get(3)?,
                year: row.get(4)?,
                trim: row.get(5)?,
                vin: row.get(6)?,
                engine: row.get(7)?,
                transmission: row.get(8)?,
                protocol: row.get(9)?,
                is_active: row.get::<_, i64>(10)? != 0,
                notes: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })?;

        if let Some(car) = car_iter.next() {
            Ok(Some(car?))
        } else {
            Ok(None)
        }
    }

    /// Get a car by its ID
    pub fn get_car_by_id(&self, id: i64) -> Result<Option<Car>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, make, model, year, trim, vin, engine, transmission, protocol, is_active, notes, created_at, updated_at
             FROM cars
             WHERE id = ?1",
        )?;

        let mut car_iter = stmt.query_map(params![id], |row| {
            Ok(Car {
                id: row.get(0)?,
                name: row.get(1)?,
                make: row.get(2)?,
                model: row.get(3)?,
                year: row.get(4)?,
                trim: row.get(5)?,
                vin: row.get(6)?,
                engine: row.get(7)?,
                transmission: row.get(8)?,
                protocol: row.get(9)?,
                is_active: row.get::<_, i64>(10)? != 0,
                notes: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })?;

        if let Some(car) = car_iter.next() {
            Ok(Some(car?))
        } else {
            Ok(None)
        }
    }

    /// Register a new car. If it's the first car, it is automatically marked active.
    pub fn add_car(&self, new_car: &NewCar) -> Result<Car> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM cars", [], |row| row.get(0))?;
        let is_first = count == 0;

        let protocol = new_car
            .protocol
            .as_deref()
            .unwrap_or("ISO15765-4 / CAN 500k");

        self.conn.execute(
            "INSERT INTO cars (name, make, model, year, trim, vin, engine, transmission, protocol, is_active, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, datetime('now'), datetime('now'))",
            params![
                new_car.name,
                new_car.make,
                new_car.model,
                new_car.year,
                new_car.trim,
                new_car.vin,
                new_car.engine,
                new_car.transmission,
                protocol,
                if is_first { 1 } else { 0 },
                new_car.notes,
            ],
        )?;

        let id = self.conn.last_insert_rowid();
        self.get_car_by_id(id)?
            .ok_or_else(|| CarworxError::not_found("Vehicle", id))
    }

    /// Set a car as the active vehicle
    pub fn set_active_car(&self, id: i64) -> Result<()> {
        let exists: bool = self
            .conn
            .query_row("SELECT COUNT(*) FROM cars WHERE id = ?1", params![id], |row| {
                Ok(row.get::<_, i64>(0)? > 0)
            })?;

        if !exists {
            return Err(CarworxError::not_found("Vehicle", id));
        }

        // Deactivate all, activate chosen
        self.conn.execute("UPDATE cars SET is_active = 0", [])?;
        self.conn.execute(
            "UPDATE cars SET is_active = 1, updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Delete a car by ID
    pub fn delete_car(&self, id: i64) -> Result<()> {
        let affected = self
            .conn
            .execute("DELETE FROM cars WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(CarworxError::not_found("Vehicle", id));
        }
        Ok(())
    }

    // ==========================================
    // HARDWARE INTERFACE OPERATIONS
    // ==========================================

    /// List all known hardware interfaces in the database
    pub fn list_interfaces(&self) -> Result<Vec<HardwareInterface>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, interface_type, bitrate, is_active, notes, last_seen_at
             FROM hardware_interfaces
             ORDER BY is_active DESC, name ASC",
        )?;

        let iface_iter = stmt.query_map([], |row| {
            Ok(HardwareInterface {
                id: row.get(0)?,
                name: row.get(1)?,
                interface_type: row.get(2)?,
                bitrate: row.get::<_, i64>(3)? as u32,
                is_active: row.get::<_, i64>(4)? != 0,
                notes: row.get(5)?,
                last_seen_at: row.get(6)?,
            })
        })?;

        let mut ifaces = Vec::new();
        for iface in iface_iter {
            ifaces.push(iface?);
        }
        Ok(ifaces)
    }

    /// Get active hardware interface
    pub fn get_active_interface(&self) -> Result<Option<HardwareInterface>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, interface_type, bitrate, is_active, notes, last_seen_at
             FROM hardware_interfaces
             WHERE is_active = 1
             LIMIT 1",
        )?;

        let mut iface_iter = stmt.query_map([], |row| {
            Ok(HardwareInterface {
                id: row.get(0)?,
                name: row.get(1)?,
                interface_type: row.get(2)?,
                bitrate: row.get::<_, i64>(3)? as u32,
                is_active: row.get::<_, i64>(4)? != 0,
                notes: row.get(5)?,
                last_seen_at: row.get(6)?,
            })
        })?;

        if let Some(iface) = iface_iter.next() {
            Ok(Some(iface?))
        } else {
            Ok(None)
        }
    }

    /// Register or update a hardware interface
    pub fn register_interface(
        &self,
        name: &str,
        iface_type: &str,
        bitrate: u32,
        is_active: bool,
        notes: Option<&str>,
    ) -> Result<HardwareInterface> {
        if is_active {
            self.conn
                .execute("UPDATE hardware_interfaces SET is_active = 0", [])?;
        }

        self.conn.execute(
            "INSERT INTO hardware_interfaces (name, interface_type, bitrate, is_active, notes, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
             ON CONFLICT(name) DO UPDATE SET
                interface_type = excluded.interface_type,
                bitrate = excluded.bitrate,
                is_active = CASE WHEN ?4 = 1 THEN 1 ELSE is_active END,
                notes = COALESCE(excluded.notes, hardware_interfaces.notes),
                last_seen_at = datetime('now')",
            params![name, iface_type, bitrate as i64, if is_active { 1 } else { 0 }, notes],
        )?;

        let mut stmt = self.conn.prepare(
            "SELECT id, name, interface_type, bitrate, is_active, notes, last_seen_at
             FROM hardware_interfaces WHERE name = ?1",
        )?;

        let iface = stmt.query_row(params![name], |row| {
            Ok(HardwareInterface {
                id: row.get(0)?,
                name: row.get(1)?,
                interface_type: row.get(2)?,
                bitrate: row.get::<_, i64>(3)? as u32,
                is_active: row.get::<_, i64>(4)? != 0,
                notes: row.get(5)?,
                last_seen_at: row.get(6)?,
            })
        })?;

        Ok(iface)
    }

    /// Set an interface as active by name
    #[allow(dead_code)]
    pub fn set_active_interface(&self, name: &str) -> Result<()> {
        let exists: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM hardware_interfaces WHERE name = ?1",
            params![name],
            |row| Ok(row.get::<_, i64>(0)? > 0),
        )?;

        if !exists {
            // Auto register with default 500k SocketCAN
            self.register_interface(name, "socketcan", 500000, true, None)?;
            return Ok(());
        }

        self.conn
            .execute("UPDATE hardware_interfaces SET is_active = 0", [])?;
        self.conn.execute(
            "UPDATE hardware_interfaces SET is_active = 1, last_seen_at = datetime('now') WHERE name = ?1",
            params![name],
        )?;
        Ok(())
    }

    // ==========================================
    // CONFIGURATION FILE OPERATIONS
    // ==========================================

    /// List configuration profiles, optionally filtered by vehicle ID
    pub fn list_configs(&self, car_id: Option<i64>) -> Result<Vec<CarConfig>> {
        let mut query = String::from(
            "SELECT id, car_id, name, config_type, file_path, description, is_enabled, created_at
             FROM car_configs ",
        );

        if let Some(cid) = car_id {
            query.push_str(&format!("WHERE car_id IS NULL OR car_id = {} ", cid));
        }
        query.push_str("ORDER BY created_at DESC");

        let mut stmt = self.conn.prepare(&query)?;
        let config_iter = stmt.query_map([], |row| {
            Ok(CarConfig {
                id: row.get(0)?,
                car_id: row.get(1)?,
                name: row.get(2)?,
                config_type: row.get(3)?,
                file_path: row.get(4)?,
                description: row.get(5)?,
                is_enabled: row.get::<_, i64>(6)? != 0,
                created_at: row.get(7)?,
            })
        })?;

        let mut configs = Vec::new();
        for cfg in config_iter {
            configs.push(cfg?);
        }
        Ok(configs)
    }

    /// Register a universal configuration file (such as a CAN map or DBC)
    pub fn add_config(&self, new_cfg: &NewCarConfig) -> Result<CarConfig> {
        self.conn.execute(
            "INSERT INTO car_configs (car_id, name, config_type, file_path, description, is_enabled, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, datetime('now'))",
            params![
                new_cfg.car_id,
                new_cfg.name,
                new_cfg.config_type,
                new_cfg.file_path,
                new_cfg.description,
            ],
        )?;

        let id = self.conn.last_insert_rowid();
        let mut stmt = self.conn.prepare(
            "SELECT id, car_id, name, config_type, file_path, description, is_enabled, created_at
             FROM car_configs WHERE id = ?1",
        )?;

        let cfg = stmt.query_row(params![id], |row| {
            Ok(CarConfig {
                id: row.get(0)?,
                car_id: row.get(1)?,
                name: row.get(2)?,
                config_type: row.get(3)?,
                file_path: row.get(4)?,
                description: row.get(5)?,
                is_enabled: row.get::<_, i64>(6)? != 0,
                created_at: row.get(7)?,
            })
        })?;

        Ok(cfg)
    }
}
