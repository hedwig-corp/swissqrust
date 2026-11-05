/*
 * Copyright (c) 2026 Thomas Prosser
 * Licensed under MIT License
 * https://opensource.org/licenses/MIT
 */

//! Reads the data content of a Swiss QR Code back into a validated
//! [`BillData`], the reverse of [`QrBill::create_qr_text`].
//!
//! The structure of the payload (spec 4.1.4, 4.2.2) is checked here; every
//! value rule (IBAN checksum, QR-IBAN/QR-reference pairing, amount format,
//! field lengths, character set, ...) is left to [`Address::new`] and
//! [`BillData::new`], so a payload is accepted exactly when the library
//! would also generate it.
//!
//! [`QrBill::create_qr_text`]: crate::qr_bill::QrBill::create_qr_text

use thiserror::Error;
use crate::address::AddressError;
use crate::{Address, BillData, BillError, ReferenceType};

/// Lines up to and including the `EPD` trailer, which every payload carries.
const MANDATORY_LINES: usize = 31;
/// Bill information (32) plus up to two alternative procedures (33, 34).
const MAX_LINES: usize = 34;

#[derive(Debug, Error)]
pub enum QrParseError {
    #[error("line separators must be all CR+LF or all LF (4.1.4)")]
    MixedLineSeparators,
    #[error("payload has {0} lines, expected {MANDATORY_LINES} to {MAX_LINES}")]
    LineCount(usize),
    #[error("line {line} ({field}): expected {expected:?}, found {found:?}")]
    UnexpectedValue { line: usize, field: &'static str, expected: &'static str, found: String },
    #[error("line {line} ({field}): address type {found:?} is not supported, only structured addresses (S) are")]
    UnsupportedAddressType { line: usize, field: &'static str, found: String },
    #[error("line {line} ({field}): {source}")]
    InvalidAddress { line: usize, field: &'static str, source: AddressError },
    #[error("lines 12-18 (UltmtCdtr) are reserved and must be empty")]
    UltimateCreditorNotEmpty,
    #[error("line {line} ({field}) must not contain whitespace")]
    UnexpectedWhitespace { line: usize, field: &'static str },
    #[error("line 29 (Ref): reference type {code} requires {expected}")]
    ReferenceMismatch { code: &'static str, expected: &'static str },
    #[error(transparent)]
    InvalidBill(#[from] BillError),
}

/// Parses the data content of a Swiss QR Code.
///
/// ```
/// use swiss_qrust::qr_bill::QrBill;
/// use swiss_qrust::{build_bill, parse_qr_text};
///
/// let bill = build_bill()?;
/// let text = QrBill::new(&bill)?.create_qr_text()?;
/// assert_eq!(parse_qr_text(&text)?, bill);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_qr_text(text: &str) -> Result<BillData, QrParseError> {
    let lines = split_lines(text)?;
    if !(MANDATORY_LINES..=MAX_LINES).contains(&lines.len()) {
        return Err(QrParseError::LineCount(lines.len()));
    }
    // 1-based line number, as in the spec's tables.
    let line = |n: usize| lines[n - 1];
    let optional = |n: usize| lines.get(n - 1).copied().and_then(non_empty);

    expect(1, "QRType", "SPC", line(1))?;
    expect(2, "Version", "0200", line(2))?;
    expect(3, "Coding", "1", line(3))?;
    expect(31, "Trailer", "EPD", line(31))?;

    let iban = line(4);
    if iban.contains(char::is_whitespace) {
        return Err(QrParseError::UnexpectedWhitespace { line: 4, field: "IBAN" });
    }

    let creditor = address(&lines, 5, "Cdtr")?
        .ok_or(QrParseError::UnexpectedValue { line: 5, field: "Cdtr", expected: "S", found: String::new() })?;
    if lines[11..18].iter().any(|l| !l.is_empty()) {
        return Err(QrParseError::UltimateCreditorNotEmpty);
    }

    let currency = match line(20) {
        "CHF" | "EUR" => line(20).parse()?,
        found => return Err(QrParseError::UnexpectedValue { line: 20, field: "Ccy", expected: "CHF or EUR", found: found.into() }),
    };
    let debtor = address(&lines, 21, "UltmtDbtr")?;

    let reference = line(29);
    if reference.contains(char::is_whitespace) {
        return Err(QrParseError::UnexpectedWhitespace { line: 29, field: "Ref" });
    }
    let reference_type = match (line(28), reference.is_empty()) {
        ("NON", true) => ReferenceType::NoRef,
        ("NON", false) => return Err(QrParseError::ReferenceMismatch { code: "NON", expected: "an empty reference" }),
        ("QRR", false) => ReferenceType::QrRef(reference.into()),
        ("SCOR", false) => ReferenceType::Creditor(reference.into()),
        ("QRR", true) => return Err(QrParseError::ReferenceMismatch { code: "QRR", expected: "a reference" }),
        ("SCOR", true) => return Err(QrParseError::ReferenceMismatch { code: "SCOR", expected: "a reference" }),
        (found, _) => return Err(QrParseError::UnexpectedValue { line: 28, field: "Tp", expected: "QRR, SCOR or NON", found: found.into() }),
    };

    Ok(BillData::new(
        iban.into(),
        creditor,
        debtor,
        currency,
        optional(19).map(String::from),
        reference_type,
        optional(30).map(String::from),
        optional(32).map(String::from),
        [optional(33).map(String::from), optional(34).map(String::from)],
    )?)
}

/// Spec 4.1.4: either CR+LF or LF throughout.
///
/// A payload ending in a separator is not rejected: within 34 lines that is
/// just an empty StrdBkgInf or AltPmt, and real bills from bank and
/// accounting software carry them ("EPD", "", "", ""). The generator still
/// never emits one; anything past line 34 fails the line count.
fn split_lines(text: &str) -> Result<Vec<&str>, QrParseError> {
    let separator = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.split(separator).collect();
    if lines.iter().any(|l| l.contains(['\r', '\n'])) {
        return Err(QrParseError::MixedLineSeparators);
    }
    Ok(lines)
}

fn non_empty(value: &str) -> Option<&str> {
    Some(value).filter(|v| !v.is_empty())
}

fn expect(line: usize, field: &'static str, expected: &'static str, found: &str) -> Result<(), QrParseError> {
    if found == expected {
        Ok(())
    } else {
        Err(QrParseError::UnexpectedValue { line, field, expected, found: found.into() })
    }
}

/// Spec 4.2.2: an address is 7 lines starting with its type. All 7 empty
/// means no address; the combined type K was removed on 21 Nov 2025.
fn address(lines: &[&str], start: usize, field: &'static str) -> Result<Option<Address>, QrParseError> {
    let block = &lines[start - 1..start + 6];
    match block[0] {
        "" if block.iter().all(|l| l.is_empty()) => Ok(None),
        "S" => {
            Address::new(block[1], non_empty(block[2]), non_empty(block[3]), block[4], block[5], block[6])
                .map(Some)
                .map_err(|source| QrParseError::InvalidAddress { line: start, field, source })
        }
        found => Err(QrParseError::UnsupportedAddressType { line: start, field, found: found.into() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qr_bill::QrBill;
    use crate::build_bill;

    fn payload() -> String {
        QrBill::new(&build_bill().unwrap()).unwrap().create_qr_text().unwrap()
    }

    fn with_line(n: usize, value: &str) -> String {
        let mut lines: Vec<String> = payload().split("\r\n").map(String::from).collect();
        lines[n - 1] = value.into();
        lines.join("\r\n")
    }

    #[test]
    fn round_trips_a_generated_bill() {
        assert_eq!(parse_qr_text(&payload()).unwrap(), build_bill().unwrap());
    }

    #[test]
    fn accepts_lf_separators() {
        assert_eq!(parse_qr_text(&payload().replace("\r\n", "\n")).unwrap(), build_bill().unwrap());
    }

    #[test]
    fn rejects_mixed_separators() {
        let text = payload().replacen("\r\n", "\n", 1);
        assert!(matches!(parse_qr_text(&text), Err(QrParseError::MixedLineSeparators)));
    }

    #[test]
    fn accepts_empty_trailing_optional_elements() {
        for trailing in ["\r\n", "\r\n\r\n\r\n"] {
            let text = payload() + trailing;
            assert_eq!(parse_qr_text(&text).unwrap(), build_bill().unwrap(), "{trailing:?}");
        }
        let text = payload() + "\r\n\r\n\r\n\r\n";
        assert!(matches!(parse_qr_text(&text), Err(QrParseError::LineCount(35))));
    }

    #[test]
    fn rejects_wrong_header_and_trailer() {
        for (n, value) in [(1, "SPX"), (2, "0100"), (3, "2"), (31, "END")] {
            let err = parse_qr_text(&with_line(n, value)).unwrap_err();
            assert!(matches!(err, QrParseError::UnexpectedValue { line, .. } if line == n), "line {n}: {err}");
        }
    }

    #[test]
    fn rejects_combined_address() {
        let err = parse_qr_text(&with_line(5, "K")).unwrap_err();
        assert!(matches!(err, QrParseError::UnsupportedAddressType { line: 5, .. }), "{err}");
    }

    #[test]
    fn rejects_filled_ultimate_creditor() {
        let err = parse_qr_text(&with_line(12, "S")).unwrap_err();
        assert!(matches!(err, QrParseError::UltimateCreditorNotEmpty), "{err}");
    }

    #[test]
    fn rejects_reference_that_does_not_match_its_type() {
        let err = parse_qr_text(&with_line(28, "SCOR")).unwrap_err();
        assert!(matches!(err, QrParseError::InvalidBill(_)), "{err}");
        let err = parse_qr_text(&with_line(28, "NON")).unwrap_err();
        assert!(matches!(err, QrParseError::ReferenceMismatch { code: "NON", .. }), "{err}");
    }

    #[test]
    fn rejects_spaced_iban_and_reference() {
        let err = parse_qr_text(&with_line(4, "CH64 3196 1000 0044 2155 7")).unwrap_err();
        assert!(matches!(err, QrParseError::UnexpectedWhitespace { line: 4, .. }), "{err}");
        let err = parse_qr_text(&with_line(29, "00 00082 07791 22585 74212 86694")).unwrap_err();
        assert!(matches!(err, QrParseError::UnexpectedWhitespace { line: 29, .. }), "{err}");
    }

    #[test]
    fn rejects_wrong_line_count() {
        let text: String = payload().split("\r\n").take(30).collect::<Vec<_>>().join("\r\n");
        assert!(matches!(parse_qr_text(&text), Err(QrParseError::LineCount(30))));
    }

    #[test]
    fn leaves_value_rules_to_bill_data() {
        let err = parse_qr_text(&with_line(4, "CH6431961000004421558")).unwrap_err();
        assert!(matches!(err, QrParseError::InvalidBill(BillError::IbanError(_))), "{err}");
    }
}
