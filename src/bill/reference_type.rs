/*
 * Copyright (c) 2026 Thomas Prosser
 * Licensed under MIT License
 * https://opensource.org/licenses/MIT
 */

use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::support::validators::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReferenceType {
    #[serde(rename = "NON")]
    NoRef,

    #[serde(rename = "QRR")]
    QrRef(String),

    #[serde(rename = "SCOR")]
    Creditor(String),
}

/// There are three reference types:
///  * None - A normal IBAN should not have a QR reference
///  * QRR - If a QR IBAN is given, there MUST be a QR Reference
///  * SCOR - If there is a normal IBAN, a SCOR or creditor IBAN can be added.
impl ReferenceType {
    //! Returns a reference type based on the given value.
    //!
    //! # Arguments
    //!
    //! * `value` - The value to be parsed.
    //!
    //! # Errors
    //! 
    //! * Returns `ReferenceError::InvalidReference` if the value is not a valid reference.
    //! * Returns `ReferenceError::InvalidQrReference` if the value is a QR reference, but not valid.
    //! * Returns `ReferenceError::InvalidIso11649Reference` if the value is a creditor reference, but not valid.
    //!
    //! # Examples
    //! ```
    //! use swiss_qrust::ReferenceType;
    //! let ref_type = ReferenceType::infer("RF18539007547034").unwrap();
    //! assert_eq!(ref_type, ReferenceType::Creditor("RF18539007547034".into()));
    //!```
    //! Returns an error if the value is not a valid reference.
    pub fn infer(value: &str) -> Result<Self, ReferenceError> {

        // References are often written in groups ("21 00000 ..."), but the
        // QR code carries them without spaces (spec 4.2.2, Ref).
        let value: String = value.chars().filter(|c| !c.is_whitespace()).collect();
        let value = value.as_str();

        // If value is empty, then Reference Type is none
        if value.is_empty() {
            return Ok(Self::NoRef);
        }

        // If value has exactly 27 numeric digits
        if value.len() == 27usize && value.chars().all(|c| c.is_ascii_digit()) {
            is_valid_qr_reference(value)?;
            return Ok(Self::QrRef(value.to_owned()));
        }

        // ISO 11649 creditor reference: RF... + MOD97
        if value.starts_with("RF") {
            is_valid_iso11649_reference(value)?;
            return Ok(Self::Creditor(value.to_owned()));
        }

        Err(ReferenceError::InvalidReference)
    }

    /// Code for the QR data structure
    ///
    /// # Arguments
    /// * Self
    ///
    /// # Returns
    /// `NON`, `QRR`, `SCOR` based on Type.
    pub fn code(&self) -> &'static str {
        match self {
            ReferenceType::NoRef => "NON",
            ReferenceType::QrRef(_) => "QRR",
            ReferenceType::Creditor(_) => "SCOR",
        }
    }
    pub fn requires_references(&self) -> bool {
        matches!(self, ReferenceType::QrRef(_) | ReferenceType::Creditor(_))
    }

}

impl Display for ReferenceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReferenceType::NoRef => f.write_str("NON"),
            ReferenceType::QrRef(reference) => write!(f,"QRR: {}",reference),
            ReferenceType::Creditor(reference) => write!(f,"SCOR: {}",reference),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_non_reference() {
        let value = "";
        let ref_type = ReferenceType::infer(value).unwrap();
        assert_eq!(ref_type, ReferenceType::NoRef);
    }

    #[test]
    fn test_get_qr_reference() {
        let value = "210000000003139471430009017";
        let ref_type = ReferenceType::infer(value).unwrap();
        assert_eq!(ref_type, ReferenceType::QrRef(value.to_owned()));
    }

    #[test]
    fn test_iso11649_reference() {
        let value = "RF18539007547034";
        let ref_type = ReferenceType::infer(value).unwrap();
        assert_eq!(ref_type, ReferenceType::Creditor(value.to_owned()));
    }

    #[test]
    fn grouped_references_are_stored_without_spaces() {
        assert_eq!(
            ReferenceType::infer("21 00000 00003 13947 14300 09017").unwrap(),
            ReferenceType::QrRef("210000000003139471430009017".into())
        );
        assert_eq!(
            ReferenceType::infer("RF18 5390 0754 7034").unwrap(),
            ReferenceType::Creditor("RF18539007547034".into())
        );
    }

    #[test]
    fn reference_type_codes() {
        assert_eq!(ReferenceType::NoRef.code(), "NON");
        assert_eq!(ReferenceType::QrRef("210000000003139471430009017".into()).code(), "QRR");
        assert_eq!(ReferenceType::Creditor("RF18539007547034".into()).code(), "SCOR");
    }
}