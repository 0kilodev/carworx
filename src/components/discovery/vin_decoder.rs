use serde::{Deserialize, Serialize};

use crate::error::{CarworxError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedVin {
    pub vin: String,
    pub make: String,
    pub model: String,
    pub year: i32,
    pub body_type: Option<String>,
    pub assembly_plant: Option<String>,
    pub serial_number: String,
    pub country: String,
}

pub struct VinDecoder;

impl VinDecoder {
    /// Decodes a 17-character VIN into vehicle metadata (Make, Model, Year, Plant)
    pub fn decode(raw_vin: &str) -> Result<DecodedVin> {
        let vin = raw_vin.trim().to_uppercase();

        if vin.len() != 17 {
            return Err(CarworxError::validation(
                "vin",
                format!("VIN must be exactly 17 characters, got {}", vin.len()),
            ));
        }

        // ISO 3779 forbids letters I, O, Q to avoid confusion with 1 and 0
        if vin.contains('I') || vin.contains('O') || vin.contains('Q') {
            return Err(CarworxError::validation(
                "vin",
                "VIN contains invalid characters (I, O, or Q)",
            ));
        }

        let wmi = &vin[0..3];
        let vds = &vin[3..8];
        let year_char = vin.chars().nth(9).unwrap();
        let plant_char = vin.chars().nth(10).unwrap();
        let serial = vin[11..17].to_string();

        let (make, country) = Self::decode_wmi(wmi);
        let year = Self::decode_year(year_char)?;
        let (model, body_type) = Self::decode_model(wmi, vds);
        let assembly_plant = Self::decode_plant(wmi, plant_char);

        Ok(DecodedVin {
            vin,
            make,
            model,
            year,
            body_type,
            assembly_plant,
            serial_number: serial,
            country,
        })
    }

    fn decode_wmi(wmi: &str) -> (String, String) {
        match wmi {
            "JF1" => ("Subaru".to_string(), "Japan (Passenger Car)".to_string()),
            "JF2" => ("Subaru".to_string(), "Japan (MPV / SUV)".to_string()),
            "4S3" => ("Subaru".to_string(), "USA (SIA Passenger Car)".to_string()),
            "4S4" => ("Subaru".to_string(), "USA (SIA MPV / SUV)".to_string()),
            "1HG" | "2HG" | "3HG" => ("Honda".to_string(), "North America".to_string()),
            "JH4" => ("Acura".to_string(), "Japan".to_string()),
            "1FA" | "1FB" | "1FC" | "1FD" | "1FM" | "1FT" => ("Ford".to_string(), "USA".to_string()),
            "WAU" | "WA1" => ("Audi".to_string(), "Germany".to_string()),
            "WBA" | "WBS" | "WBX" => ("BMW".to_string(), "Germany".to_string()),
            "JM1" | "JM3" => ("Mazda".to_string(), "Japan".to_string()),
            "JTD" | "JTE" | "JTJ" => ("Toyota".to_string(), "Japan".to_string()),
            "4T1" | "4T4" => ("Toyota".to_string(), "USA".to_string()),
            "1GC" | "1G1" => ("Chevrolet".to_string(), "USA".to_string()),
            "KMH" | "KM8" => ("Hyundai".to_string(), "South Korea".to_string()),
            "KNA" | "KND" => ("Kia".to_string(), "South Korea".to_string()),
            "JN1" | "JN6" | "JN8" => ("Nissan".to_string(), "Japan".to_string()),
            "WDB" | "WDD" => ("Mercedes-Benz".to_string(), "Germany".to_string()),
            "WVW" | "1VW" => ("Volkswagen".to_string(), "Germany / USA".to_string()),
            "WP0" => ("Porsche".to_string(), "Germany".to_string()),
            "SAL" => ("Land Rover".to_string(), "United Kingdom".to_string()),
            "SAJ" => ("Jaguar".to_string(), "United Kingdom".to_string()),
            "YV1" | "YV4" => ("Volvo".to_string(), "Sweden".to_string()),
            _ => (format!("Manufacturer ({})", wmi), "Unknown Country".to_string()),
        }
    }

    fn decode_year(code: char) -> Result<i32> {
        let year = match code {
            'A' => 2010,
            'B' => 2011,
            'C' => 2012,
            'D' => 2013,
            'E' => 2014,
            'F' => 2015,
            'G' => 2016,
            'H' => 2017,
            'J' => 2018,
            'K' => 2019,
            'L' => 2020,
            'M' => 2021,
            'N' => 2022,
            'P' => 2023,
            'R' => 2024,
            'S' => 2025,
            'T' => 2026,
            '1' => 2001,
            '2' => 2002,
            '3' => 2003,
            '4' => 2004,
            '5' => 2005,
            '6' => 2006,
            '7' => 2007,
            '8' => 2008,
            '9' => 2009,
            _ => {
                return Err(CarworxError::validation(
                    "year_code",
                    format!("Invalid or unrecognized 10th VIN year character: '{}'", code),
                ))
            }
        };
        Ok(year)
    }

    fn decode_model(wmi: &str, vds: &str) -> (String, Option<String>) {
        if wmi.starts_with("JF") || wmi.starts_with("4S") {
            // Subaru Models by Chassis Series
            if vds.starts_with("SJ") {
                return ("Forester (SJ)".to_string(), Some("Compact SUV / Crossover".to_string()));
            } else if vds.starts_with("SK") {
                return ("Forester (SK)".to_string(), Some("Compact SUV / Crossover".to_string()));
            } else if vds.starts_with("SH") {
                return ("Forester (SH)".to_string(), Some("Compact SUV / Crossover".to_string()));
            } else if vds.starts_with("SG") {
                return ("Forester (SG)".to_string(), Some("Compact Wagon / SUV".to_string()));
            } else if vds.starts_with("SF") {
                return ("Forester (SF)".to_string(), Some("Compact Wagon".to_string()));
            } else if vds.starts_with("VA") {
                return ("WRX / STI (VA)".to_string(), Some("Sport Sedan".to_string()));
            } else if vds.starts_with("VB") {
                return ("WRX (VB)".to_string(), Some("Sport Sedan".to_string()));
            } else if vds.starts_with("BS") {
                return ("Outback (BS)".to_string(), Some("Midsize Crossover".to_string()));
            } else if vds.starts_with("BT") {
                return ("Outback (BT)".to_string(), Some("Midsize Crossover".to_string()));
            } else if vds.starts_with("BN") {
                return ("Legacy (BN)".to_string(), Some("Midsize Sedan".to_string()));
            } else if vds.starts_with("BW") {
                return ("Legacy (BW)".to_string(), Some("Midsize Sedan".to_string()));
            } else if vds.starts_with("GP") {
                return ("Crosstrek / Impreza (GP)".to_string(), Some("Subcompact Crossover".to_string()));
            } else if vds.starts_with("GT") {
                return ("Crosstrek (GT)".to_string(), Some("Subcompact Crossover".to_string()));
            } else if vds.starts_with("GU") {
                return ("Crosstrek (GU)".to_string(), Some("Subcompact Crossover".to_string()));
            } else if vds.starts_with("ZC") || vds.starts_with("ZD") {
                return ("BRZ".to_string(), Some("Coupe".to_string()));
            } else if vds.starts_with("WN") {
                return ("Ascent".to_string(), Some("3-Row SUV".to_string()));
            }
            return (format!("Subaru ({})", &vds[0..2.min(vds.len())]), None);
        }

        // Generic fallback model
        (format!("Model ({})", vds), None)
    }

    fn decode_plant(wmi: &str, code: char) -> Option<String> {
        if wmi.starts_with("JF") || wmi.starts_with("4S") {
            match code {
                'H' => Some("Gunma Main Plant (Ota, Gunma, Japan)".to_string()),
                'G' => Some("Gunma Yajima Plant (Ota, Gunma, Japan)".to_string()),
                '3' => Some("Subaru of Indiana Automotive (Lafayette, IN, USA)".to_string()),
                _ => Some(format!("Plant Code '{}'", code)),
            }
        } else {
            Some(format!("Plant Code '{}'", code))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subaru_forester_vin() {
        let decoded = VinDecoder::decode("JF2SJABC7FH552058").expect("Valid VIN");
        assert_eq!(decoded.make, "Subaru");
        assert_eq!(decoded.model, "Forester (SJ)");
        assert_eq!(decoded.year, 2015);
        assert_eq!(decoded.serial_number, "552058");
        assert!(decoded.assembly_plant.unwrap().contains("Gunma"));
    }

    #[test]
    fn test_invalid_vin_length() {
        let err = VinDecoder::decode("12345").unwrap_err();
        assert_eq!(err.code(), "ERR_VALIDATION");
    }

    #[test]
    fn test_invalid_vin_characters() {
        let err = VinDecoder::decode("JF2SJABC7FH55205I").unwrap_err();
        assert_eq!(err.code(), "ERR_VALIDATION");
    }
}
