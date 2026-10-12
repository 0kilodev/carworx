-- Carworx Core Initial Database Schema
-- Lightweight SQLite schema for universal vehicle and hardware management

PRAGMA foreign_keys = ON;

-- Vehicle Profiles
CREATE TABLE IF NOT EXISTS cars (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    make TEXT NOT NULL,
    model TEXT NOT NULL,
    year INTEGER NOT NULL,
    trim TEXT,
    vin TEXT,
    engine TEXT,
    transmission TEXT,
    protocol TEXT DEFAULT 'ISO15765-4 / CAN 500k',
    is_active INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Hardware / CAN Interfaces
CREATE TABLE IF NOT EXISTS hardware_interfaces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    interface_type TEXT NOT NULL DEFAULT 'socketcan',
    bitrate INTEGER NOT NULL DEFAULT 500000,
    is_active INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    last_seen_at TEXT DEFAULT (datetime('now'))
);

-- Universal Config Profiles (CAN Maps, DTC Catalogs, DBC files)
CREATE TABLE IF NOT EXISTS car_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    car_id INTEGER REFERENCES cars(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    config_type TEXT NOT NULL, -- 'can_map', 'dtc_catalog', 'protocol_spec', 'custom'
    file_path TEXT NOT NULL,
    description TEXT,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Persistent Application State
CREATE TABLE IF NOT EXISTS app_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Default Indices
CREATE INDEX IF NOT EXISTS idx_cars_active ON cars(is_active);
CREATE INDEX IF NOT EXISTS idx_interfaces_name ON hardware_interfaces(name);
CREATE INDEX IF NOT EXISTS idx_configs_car ON car_configs(car_id);
