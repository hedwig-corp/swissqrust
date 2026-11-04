/*
 * Copyright (c) 2026 Thomas Prosser 
 * Licensed under MIT License
 * https://opensource.org/licenses/MIT
 */
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fmt;
use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;
use crate::Address;
use crate::address::AddressError;
use crate::bill::qr_bill::{QRBillError, QrBill, MAX_QR_DATA_BYTES};
use crate::bill::reference_type::ReferenceType;
use crate::input::InputBill;
use crate::support::utils::is_qr_iban;
use crate::support::validators::*;

/// Spec 4.2.2 (Amt): two decimal places, "." as separator, no leading
/// zeroes (a single "0" before the separator is fine, as in "0.10").
pub static AMOUNT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(0|[1-9]\d{0,8})\.\d{2}$").unwrap());

/// Spec 4.4, Table 10: the unstructured message of a "DO NOT USE FOR
/// PAYMENT" notification, which is the only bill allowed an amount of 0.00.
pub const NOTIFICATION_MESSAGES: [&str; 5] = [
    "NICHT ZUR ZAHLUNG VERWENDEN",
    "NE PAS UTILISER POUR LE PAIEMENT",
    "NON UTILIZZARE PER IL PAGAMENTO",
    "DO NOT USE FOR PAYMENT",
    "BETG DUVRAR PER IL PAJAMENT",
];

#[derive(Debug, Error)]
pub enum BillError{
    #[error(transparent)]
    ReferenceError(#[from] ReferenceError),
    #[error(transparent)]
    IbanError(#[from] IbanError),
    #[error(transparent)]
    AddressError(#[from] AddressError),
    #[error(transparent)]
    SPSCharsetError(#[from] SPSCharsetError),
    #[error("Invalid currency")]
    InvalidCurrency,
    #[error("Amount does not match amount specification")]
    InvalidAmount,
    #[error("QR-IBAN requires a QR reference (QRR)")]
    QrIbanRequiresQrReference,
    #[error("QR reference (QRR) requires a QR-IBAN")]
    QrReferenceRequiresQrIban,
    #[error("Unstructured message and billing information together must not exceed 140 characters")]
    AdditionalInformationTooLong,
    #[error("Alternative procedure parameters must not exceed 100 characters")]
    AlternativeProcedureTooLong,
    #[error("Provide either bill_information or swico_bill_information, not both")]
    AmbiguousBillingInformation,
    #[error("Invalid date '{0}' in Swico billing information (expected YYYY-MM-DD)")]
    InvalidSwicoDate(String),
    #[error("QR code data is {bytes} bytes, the maximum is {max}")]
    QrDataTooLong { bytes: usize, max: usize },
    #[error(transparent)]
    QrBillError(#[from] QRBillError),
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum Currency {
    #[default]
    CHF,
    EUR,
}

impl Display for Currency {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Currency::CHF => f.write_str("CHF"),
            Currency::EUR => f.write_str("EUR"),
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum QRCountry {
    #[default]
    CH,
    LI
}
impl Display for QRCountry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            QRCountry::CH => f.write_str("CH"),
            QRCountry::LI => f.write_str("LI"),
        }
    }
}
/// A validated Swiss QR-bill, ready to render.
///
/// Build one via `BillData::try_from(input_bill)` or [`BillData::new`] -
/// both enforce the spec's structural rules (IBAN checksum, QR-IBAN/QR-
/// reference pairing, amount format, message length limits, ...), returning
/// a [`BillError`] on the first violation. There is no way to construct an
/// invalid `BillData` outside this module.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BillData {
    pub iban: String,
    pub creditor_address: Address,
    pub debtor_address: Option<Address>,
    pub currency: Currency,
    pub amount: Option<String>,
    pub reference_type: ReferenceType,
    pub unstructured_message: Option<String>,
    pub bill_information: Option<String>,
    pub alternative_schemes: [Option<String>; 2],
} impl BillData {
    pub fn new (
        iban: String,
        creditor_address: Address,
        debtor_address: Option<Address>,
        currency: Currency,
        amount: Option<String>,
        reference_type: ReferenceType,

        // Belongs to additional Information
        unstructured_message: Option<String>,

        // Belongs to additional Information, but not to QR code
        bill_information: Option<String>,

        // Belongs to Further Information
        alternative_schemes: [Option<String>;2],
    ) -> Result<Self, BillError> {

        if let Some(ref amt) = amount {
            if !AMOUNT_REGEX.is_match(amt) {
                return Err(BillError::InvalidAmount);
            }
            // Spec 4.2.2: amount must be between 0.01 and 999999999.99,
            // except on a "DO NOT USE FOR PAYMENT" notification (spec 4.4),
            // which must carry exactly 0.00.
            let is_notification = unstructured_message
                .as_deref()
                .is_some_and(|msg| NOTIFICATION_MESSAGES.contains(&msg));
            if amt == "0.00" && !is_notification {
                return Err(BillError::InvalidAmount);
            }
        }

        // Spec 4.2.2 (Ref): the reference goes into the QR code without
        // spaces. Validated here too, since a `ReferenceType` can be built
        // directly rather than through `ReferenceType::infer`.
        let reference_type = match reference_type {
            ReferenceType::NoRef => ReferenceType::NoRef,
            ReferenceType::QrRef(reference) => {
                let reference: String = reference.chars().filter(|c| !c.is_whitespace()).collect();
                is_valid_qr_reference(&reference)?;
                ReferenceType::QrRef(reference)
            }
            ReferenceType::Creditor(reference) => {
                let reference: String = reference.chars().filter(|c| !c.is_whitespace()).collect();
                is_valid_iso11649_reference(&reference)?;
                ReferenceType::Creditor(reference)
            }
        };

        is_valid_iban(&iban)?;

        // TODO: This should have happened before here.
        let iban: String =
            iban
                .chars()
                .filter(|s| !s.is_whitespace())
                .collect();

        // Spec 4.2.2 (RmtInf.Tp) / 4.3.2: a QR-IBAN must carry a QR
        // reference, and a QR reference must not be paired with a plain IBAN.
        let has_qr_reference = matches!(reference_type, ReferenceType::QrRef(_));
        match (is_qr_iban(&iban), has_qr_reference) {
            (true, false) => return Err(BillError::QrIbanRequiresQrReference),
            (false, true) => return Err(BillError::QrReferenceRequiresQrIban),
            _ => {}
        }
        // Spec v2.4 2.10 / 2.12.1 restricts QR-IBAN and QR reference to
        // CHF, but v2.3 still allows EUR and stays valid until November
        // 2027, so EUR + QRR is accepted until then.
        // TODO: reject it once v2.3 expires - https://github.com/thomi137/swissqrust/issues/2

        // Spec 4.1.1: character set restriction applies to all free-text fields.
        if let Some(ref msg) = unstructured_message {
            is_valid_sps_charset(msg)?;
        }
        if let Some(ref info) = bill_information {
            is_valid_sps_charset(info)?;
        }
        for scheme in alternative_schemes.iter().flatten() {
            is_valid_sps_charset(scheme)?;
        }

        // Spec 4.2.2 (AddInf comment): Ustrd + StrdBkgInf share a common
        // 140-character budget.
        let combined_len = unstructured_message.as_deref().unwrap_or("").chars().count()
            + bill_information.as_deref().unwrap_or("").chars().count();
        if combined_len > 140 {
            return Err(BillError::AdditionalInformationTooLong);
        }

        // Spec 3.5.5 / 4.2.2 (AltPmt): max 100 characters per occurrence.
        if alternative_schemes
            .iter()
            .flatten()
            .any(|scheme| scheme.chars().count() > 100)
        {
            return Err(BillError::AlternativeProcedureTooLong);
        }

        let bill =  BillData{
            iban,
            creditor_address,
            debtor_address,
            currency,
            amount,
            reference_type,
            unstructured_message,
            bill_information,
            alternative_schemes,
        };

        // Spec 6.2: the field limits count characters, so multi-byte text
        // can still push the payload past what QR version 25 holds.
        let bytes = QrBill::new(&bill)?.create_qr_text()?.len();
        if bytes > MAX_QR_DATA_BYTES {
            return Err(BillError::QrDataTooLong { bytes, max: MAX_QR_DATA_BYTES });
        }
        Ok(bill)
    }
}

impl TryFrom<InputBill> for BillData {
    type Error = BillError;

    fn try_from(input: InputBill) -> Result<Self, Self::Error> {

        let currency = input.currency.parse()?;

        let creditor_address = Address::try_from(input.creditor_address)?;
        let debtor_address =
            match input.debtor_address {
                Some(addr) => Some(Address::try_from(addr)?),
                None => None,
            };
        let alternative_schemes = input
            .alternative_schemes
            .unwrap_or([None, None]);

        let reference_type =
            ReferenceType::infer(input.reference.unwrap_or("".to_string()).as_str())?;

        let bill_information = match (input.bill_information, input.swico_bill_information) {
            (Some(_), Some(_)) => return Err(BillError::AmbiguousBillingInformation),
            (Some(raw), None) => Some(raw),
            (None, Some(swico_input)) => {
                crate::bill::swico::SwicoBillInformation::try_from(swico_input)?.encode_as_text()
            }
            (None, None) => None,
        };

        BillData::new(
            input.iban,
            creditor_address,
            debtor_address,
            currency,
            input.amount,
            reference_type,
            input.unstructured_message,
            bill_information,
            alternative_schemes,
        )
    }
}

/// Generates Bill Data for use in a main function which is testing this library:
/// 1. Create a new Address
/// 2. Create a new BillData
///
/// Note: This is not a real bill, it is just a test case. Validating against the
/// SIX Swiss Bank Master will fail as the IBAN is not valid.
///
/// # Returns
/// `Result<BillData>` - Bill Data Result for use as a test case
///
/// # Example
/// ```
/// # use swiss_qrust::*;
/// let bill_data = build_bill().unwrap();
/// assert_eq!(bill_data.creditor_address.name, "Health insurance fit&kicking");
/// assert_eq!(bill_data.reference_type, ReferenceType::QrRef("000008207791225857421286694".to_string()));
/// ```
pub fn build_bill()  -> Result<BillData, BillError> {
    let  creditor = Address::new(
        "Health insurance fit&kicking",
        Some("Am Wasser"),
        Some("1"),
        "3000",
        "Bern",
        "CH"
    )?;

    let debtor = Address::new(
        "Sarah Beispiel",
        Some("Mustergasse"),
        Some("1"),
        "3600",
        "Thun",
        "CH"
    )?;

    let bill_data = BillData::new(
        "CH64 3196 1000 0044 2155 7".to_string(),
        creditor,
        Some(debtor),
        Currency::CHF,
        None, //Some(String::from("32111.00")),
        ReferenceType::infer("000008207791225857421286694")?,
        Some(String::from("Premium calculation July 2020")),
        None,
        [None, None],
    );

    Ok(bill_data?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const QR_IBAN: &str = "CH64 3196 1000 0044 2155 7";
    const PLAIN_IBAN: &str = "CH93 0076 2011 6238 5295 7";
    const QRR: &str = "000008207791225857421286694";

    fn address() -> Address {
        Address::new("Health insurance fit&kicking", Some("Am Wasser"), Some("1"), "3000", "Bern", "CH").unwrap()
    }

    #[test]
    fn qr_iban_without_qr_reference_is_rejected() {
        let err = BillData::new(
            QR_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            None,
            ReferenceType::NoRef,
            None,
            None,
            [None, None],
        )
        .unwrap_err();

        assert!(matches!(err, BillError::QrIbanRequiresQrReference));
    }

    #[test]
    fn qr_reference_without_qr_iban_is_rejected() {
        let err = BillData::new(
            PLAIN_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            None,
            ReferenceType::QrRef(QRR.to_string()),
            None,
            None,
            [None, None],
        )
        .unwrap_err();

        assert!(matches!(err, BillError::QrReferenceRequiresQrIban));
    }

    /// Still allowed by IG v2.3 until November 2027, see
    /// https://github.com/thomi137/swissqrust/issues/2.
    #[test]
    fn qr_reference_in_eur_is_accepted_until_v2_3_expires() {
        assert!(BillData::new(
            QR_IBAN.to_string(),
            address(),
            None,
            Currency::EUR,
            None,
            ReferenceType::QrRef(QRR.to_string()),
            None,
            None,
            [None, None],
        )
        .is_ok());
    }

    #[test]
    fn plain_iban_with_scor_or_no_reference_is_accepted() {
        assert!(BillData::new(
            PLAIN_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            None,
            ReferenceType::NoRef,
            None,
            None,
            [None, None],
        )
        .is_ok());
    }

    #[test]
    fn zero_amount_is_rejected() {
        let err = BillData::new(
            QR_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            Some("0.00".to_string()),
            ReferenceType::QrRef(QRR.to_string()),
            None,
            None,
            [None, None],
        )
        .unwrap_err();

        assert!(matches!(err, BillError::InvalidAmount));
    }

    #[test]
    fn zero_amount_is_accepted_on_a_notification() {
        for message in NOTIFICATION_MESSAGES {
            assert!(BillData::new(
                PLAIN_IBAN.to_string(),
                address(),
                None,
                Currency::CHF,
                Some("0.00".to_string()),
                ReferenceType::NoRef,
                Some(message.to_string()),
                None,
                [None, None],
            )
            .is_ok(), "{message}");
        }
    }

    #[test]
    fn amount_with_leading_zeroes_is_rejected() {
        for amount in ["0050.00", "01.00", "00.50"] {
            let err = BillData::new(
                PLAIN_IBAN.to_string(),
                address(),
                None,
                Currency::CHF,
                Some(amount.to_string()),
                ReferenceType::NoRef,
                None,
                None,
                [None, None],
            )
            .unwrap_err();

            assert!(matches!(err, BillError::InvalidAmount), "{amount}");
        }
    }

    #[test]
    fn reference_built_directly_is_validated_and_stored_without_spaces() {
        let bill = BillData::new(
            PLAIN_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            None,
            ReferenceType::Creditor("RF18 5390 0754 7034".to_string()),
            None,
            None,
            [None, None],
        )
        .unwrap();
        assert_eq!(bill.reference_type, ReferenceType::Creditor("RF18539007547034".to_string()));

        let err = BillData::new(
            QR_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            None,
            ReferenceType::QrRef("000008207791225857421286695".to_string()),
            None,
            None,
            [None, None],
        )
        .unwrap_err();
        assert!(matches!(err, BillError::ReferenceError(ReferenceError::InvalidQrChecksum)));
    }

    #[test]
    fn qr_data_over_997_bytes_is_rejected() {
        // 140 + 2 x 100 characters, each 3 bytes in UTF-8.
        let err = BillData::new(
            QR_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            None,
            ReferenceType::QrRef(QRR.to_string()),
            Some("€".repeat(140)),
            None,
            [Some("€".repeat(100)), Some("€".repeat(100))],
        )
        .unwrap_err();

        assert!(matches!(err, BillError::QrDataTooLong { max: 997, .. }));
    }

    #[test]
    fn non_zero_amount_is_accepted() {
        assert!(BillData::new(
            QR_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            Some("0.10".to_string()),
            ReferenceType::QrRef(QRR.to_string()),
            None,
            None,
            [None, None],
        )
        .is_ok());
    }

    #[test]
    fn invalid_charset_in_unstructured_message_is_rejected() {
        let err = BillData::new(
            QR_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            None,
            ReferenceType::QrRef(QRR.to_string()),
            Some("Hello 🤣".to_string()),
            None,
            [None, None],
        )
        .unwrap_err();

        assert!(matches!(err, BillError::SPSCharsetError(_)));
    }

    #[test]
    fn combined_additional_information_over_140_chars_is_rejected() {
        let err = BillData::new(
            QR_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            None,
            ReferenceType::QrRef(QRR.to_string()),
            Some("a".repeat(100)),
            Some("b".repeat(41)),
            [None, None],
        )
        .unwrap_err();

        assert!(matches!(err, BillError::AdditionalInformationTooLong));
    }

    #[test]
    fn combined_additional_information_at_140_chars_is_accepted() {
        assert!(BillData::new(
            QR_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            None,
            ReferenceType::QrRef(QRR.to_string()),
            Some("a".repeat(100)),
            Some("b".repeat(40)),
            [None, None],
        )
        .is_ok());
    }

    #[test]
    fn alternative_procedure_over_100_chars_is_rejected() {
        let err = BillData::new(
            QR_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            None,
            ReferenceType::QrRef(QRR.to_string()),
            None,
            None,
            [Some("a".repeat(101)), None],
        )
        .unwrap_err();

        assert!(matches!(err, BillError::AlternativeProcedureTooLong));
    }

    #[test]
    fn alternative_procedure_at_100_chars_is_accepted() {
        assert!(BillData::new(
            QR_IBAN.to_string(),
            address(),
            None,
            Currency::CHF,
            None,
            ReferenceType::QrRef(QRR.to_string()),
            None,
            None,
            [Some("a".repeat(100)), None],
        )
        .is_ok());
    }
}
