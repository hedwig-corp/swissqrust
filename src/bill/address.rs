
/*
 * Copyright (c) 2026 Thomas Prosser 
 * Licensed under MIT License
 * https://opensource.org/licenses/MIT
 */

use serde::{Serialize, Deserialize};
use thiserror::Error;
use crate::Country;
use crate::support::validators::{is_valid_iso_3661_1_country, is_valid_sps_charset, SPSCharsetError};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddressIdentity {
    #[default]
    Cdtr,
    Dbtr,
    UltmtCdtr,
    UltmtDbtr,
}

const ADDRESS_TYPE: &str = "S";

#[derive(Error, Debug)]
pub enum AddressError {
    #[error("Invalid name: {0}")]
    AddressNameError(String),
    #[error("Invalid street address: {0}")]
    AddressStreetError(String),
    #[error("Invalid house nr.")]
    AddressHouseError,
    #[error("Invalid postal code")]
    AddressPostalCodeError,
    #[error("Invalid city character length")]
    AddressCityError,
    #[error("invalid country code: {0}")]
    AddressCountryError(String),
    #[error(transparent)]
    AddressCharsetError(#[from] SPSCharsetError),
}

/// No need for unstructured address
/// as deprecated on Nov 21, 2025
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub address_type: String,
    pub name: String,
    pub street: Option<String>,
    pub house_num: Option<String>,
    pub plz: String,
    pub city: String,
    pub country: Country,
}

impl Default for Address {
    fn default() -> Self {
        Self {
            address_type: ADDRESS_TYPE.into(),
            name: "".into(),
            street: None,
            house_num: None,
            plz: "".into(),
            city: "".into(),
            country: Country::CH,
        }
    }
}

impl Address {
    pub fn new(
        name: &str,
        street: Option<&str>,
        house_num: Option<&str>,
        plz: &str,
        city: &str,
        country: &str,
    ) -> Result<Self, AddressError> {
        // Spec 4.2.2: all limits are in characters, not UTF-8 bytes.

        // Name
        let name = name.trim().to_string();
        if !(1..=70).contains(&name.chars().count()) {
            return Err(AddressError::AddressNameError(
                "Name must be 1–70 chars".into(),
            ));
        }

        // Street
        let street = street
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        if let Some(ref s) = street
            && s.chars().count() > 70 {
                return Err(AddressError::AddressStreetError(
                    "Street must not be longer than 70 chars".into(),
                ));
            }

        // House number
        let house_num = house_num
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        if let Some(ref h) = house_num
            && h.chars().count() > 16 {
                return Err(AddressError::AddressHouseError);
            }

        // Postal code
        let plz = plz.trim().to_string();
        if plz.is_empty() || plz.chars().count() > 16 {
            return Err(AddressError::AddressPostalCodeError);
        }

        // City
        let city = city.trim().to_string();
        if city.is_empty() || city.chars().count() > 35 {
            return Err(AddressError::AddressCityError);
        }

        // Country
        let country = country.trim().to_uppercase();
        if country.len() != 2 {
            return Err(AddressError::AddressCountryError(
                "Country code must be 2 chars".into(),
            ));
        }

        // ISO validation
        let country_enum = is_valid_iso_3661_1_country(&country)
            .map_err(|e| AddressError::AddressCountryError(e.to_string()))?;

        // Character set (spec 4.1.1): applies to every general text field.
        is_valid_sps_charset(&name)?;
        if let Some(ref s) = street {
            is_valid_sps_charset(s)?;
        }
        if let Some(ref h) = house_num {
            is_valid_sps_charset(h)?;
        }
        is_valid_sps_charset(&plz)?;
        is_valid_sps_charset(&city)?;

        Ok(Self {
            address_type: ADDRESS_TYPE.into(),
            name,
            street,
            house_num,
            plz,
            city,
            country: country_enum,
        })
    }
     pub fn to_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        lines.push(self.name.clone());

        if let Some(street) = &self.street {
            let mut street_line = street.clone();
            if let Some(house) = &self.house_num {
                street_line.push(' ');
                street_line.push_str(house);
            }
            lines.push(street_line);
        }

        lines.push(format!("{} {}", self.plz, self.city));

        lines.push(self.country.meta().alpha2.to_string());

        lines
    }
}

#[derive(Debug, Deserialize)]
pub struct InputAddress {
    pub name: String,
    pub street: Option<String>,
    pub house_num: Option<String>,
    pub plz: String,
    pub city: String,
    pub country: String,
}

impl TryFrom<InputAddress> for Address {
    type Error = AddressError;

    fn try_from(input: InputAddress) -> Result<Self, Self::Error> {
        Address::new(
            &input.name,
            input.street.as_deref(),
            input.house_num.as_deref(),
            &input.plz,
            &input.city,
            &input.country,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_name_outside_permitted_charset() {
        let err = Address::new("Hello 🤣", None, None, "3000", "Bern", "CH").unwrap_err();
        assert!(matches!(err, AddressError::AddressCharsetError(_)));
    }

    #[test]
    fn limits_count_characters_not_bytes() {
        assert!(Address::new(&"ä".repeat(70), Some(&"é".repeat(70)), Some(&"ü".repeat(16)), "3000", &"ö".repeat(35), "CH").is_ok());
        assert!(Address::new(&"ä".repeat(71), None, None, "3000", "Bern", "CH").is_err());
        assert!(Address::new("Muster AG", None, None, "3000", &"ö".repeat(36), "CH").is_err());
    }

    #[test]
    fn accepts_extended_sps_charset_in_name() {
        assert!(Address::new("Șipi Müller € AG", None, None, "3000", "Bern", "CH").is_ok());
    }
}
