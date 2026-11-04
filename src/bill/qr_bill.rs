/*
 * Copyright (c) 2026 Thomas Prosser
 * Licensed under MIT License
 * https://opensource.org/licenses/MIT
 */

use std::fmt::{Display, Formatter};
use qrcodegen::{QrCode, QrCodeEcc, QrSegment, Version as QrVersion};
use thiserror::Error;
use crate::{Address, BillData, ReferenceType};

#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum QRBillError {
    #[error("QR encoding failed")]
    QrEncodingFailed,
}

/// According to the [spec](https://www.six-group.com/dam/download/banking-services/standardization/qr-bill/ig-qr-bill-v2.4-de.pdf)
/// section 4.1.4, there are two allowed line separators.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum LineSeparator{
    // Carriage return and line feed. Unicode U+000D and U+000A.
    #[default]
    CrLF,
    // Line feed. Unicode U+000A.
    Lf
}
impl LineSeparator {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineSeparator::CrLF => "\r\n",
            LineSeparator::Lf => "\n",
        }
    }
}
impl Display for LineSeparator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LineSeparator::CrLF => f.write_str("\r\n"),
            LineSeparator::Lf => f.write_str("\n"),
        }
    }
}

/// At the moment, only version "0200" is accepted. Starting from
/// version 3, minors are accepted.
///
/// ```
/// use swiss_qrust::qr_bill::Version;
/// let version = Version{ major: 2, minor: 0};
/// assert_eq!(version.qr_code_version(), "0200");
/// ```
///
/// Alternatively, you can also use .to_string():
///
/// ```
/// use swiss_qrust::qr_bill::Version;
/// let version = Version{ major: 2, minor: 0};
/// assert_eq!(version.to_string(), "0200");
/// ```
///
/// Seems kind of redundant, but could be helpful internally to
/// implement Display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}
impl Version {
    pub fn qr_code_version(&self) -> String {
        format!("{:02}{:02}", self.major, self.minor)
    }
}
impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.qr_code_version())
    }
}

const QR_TYPE: &str = "SPC";
const VERSION: Version = Version { major: 2, minor: 0 };
const CODING_TYPE: &str = "1";
const TRAILER_EPD: &str = "EPD";

pub struct QrBill<'a> {
    pub bill_data: &'a BillData,
    qr_type: String,
    version: String,
    coding_type: String,
}
impl<'a> QrBill
<'a> {
    pub fn new(
        bill_data: &'a BillData,
    ) -> Result<Self, QRBillError> {
        let qr_type = QR_TYPE.to_string();
        let version = VERSION.to_string();
        let coding_type = CODING_TYPE.to_string();
        Ok(QrBill{
            bill_data,
            qr_type,
            version,
            coding_type,
        })
    }

    
    pub fn create_qr_text(&self) -> Result<String, QRBillError> {

        // TODO Make available for other line separator as well
        let mut qr_text = QRTextBuilder::new(LineSeparator::default());

        // Header Data. Not expected to change anytime soon.
        qr_text.append_header(&self.qr_type, &self.version, &self.coding_type);

        // IBAN - Mandatory
        qr_text.append_data_field(Some(self.bill_data.iban.as_str()));
        // Cdtr - Mandatory
        qr_text.append_person(Some(&self.bill_data.creditor_address));
        // UltmtCdtr - Has to be there, has to be empty
        qr_text.append_person(None);

        qr_text.append_data_field(self.bill_data.amount.as_deref());
        qr_text.append_data_field(Some(&self.bill_data.currency.to_string()));

        // UltmtDbtr - Debtor
        qr_text.append_person(self.bill_data.debtor_address.as_ref());

        let ref_type = &self.bill_data.reference_type;
        // RmtInf + Tp - Reference Type
        qr_text.append_data_field(Some(ref_type.code()));

        // Reference
        match ref_type {
            ReferenceType::NoRef => qr_text.append_data_field(None),
            ReferenceType::QrRef(reference) => qr_text.append_data_field(Some(reference)),
            ReferenceType::Creditor(reference) => qr_text.append_data_field(Some(reference)),
        }


        // Additional information
        qr_text.append_data_field(self.bill_data.unstructured_message.as_deref());

        //Trailer - End of Payment Data
        qr_text.append_data_field(Some(TRAILER_EPD));

        // Bill Information (StrdBkgInf) - status "A" (additional): may only
        // be omitted outright when no further field follows. Since AltPmt
        // may follow, an empty placeholder must be emitted here whenever any
        // alternative scheme is present, so positional ordering is preserved.
        let has_alt_scheme = self.bill_data.alternative_schemes.iter().any(Option::is_some);
        match &self.bill_data.bill_information {
            Some(billinfo) => qr_text.append_data_field(Some(billinfo)),
            None if has_alt_scheme => qr_text.append_data_field(None),
            None => {}
        }

        // Alternative Schemes - a repeating group; skipping absent
        // occurrences is fine since order between occurrences carries no
        // positional meaning.
        for scheme in &self.bill_data.alternative_schemes {
            if let Some(scheme) = scheme {
                qr_text.append_data_field(Some(scheme));
            }
        }

        Ok(qr_text.build())
    }
}

pub struct QRTextBuilder {
    text: String,
    separator: LineSeparator,
}
impl QRTextBuilder {
    pub fn new(separator: LineSeparator) -> Self {
        Self {
            text: String::new(),
            separator
        }
    }

    pub fn append_data_field(&mut self, value: Option<&str>) {
        let value = value.unwrap_or("");
        self.text.push_str(self.separator.as_str());
        self.text.push_str(value);
    }

    pub fn append_person(&mut self, address: Option<&Address>) {
        if let Some(addr) = address {
            self.append_data_field(Some(&addr.address_type));
            self.append_data_field(Some(&addr.name));
            self.append_data_field(addr.street.as_deref());
            self.append_data_field(addr.house_num.as_deref());
            self.append_data_field(Some(&addr.plz));
            self.append_data_field(Some(&addr.city));
            self.append_data_field(Some(&addr.country.meta().alpha2));
        } else {
            for _ in 0..7 {
                self.append_data_field(None);
            }
        }
    }

    pub fn append_header(&mut self, qr_type: &str, version: &str, coding_type: &str) {
        self.text.push_str(qr_type);
        self.append_data_field(Some(version));
        self.append_data_field(Some(coding_type));
    }

    pub fn build(self) -> String {
        self.text
    }
}

/// Spec 6.2: maximum Swiss QR Code data content, separators included. The
/// payload always contains CR/LF, so it is encoded in byte mode, where
/// version 25 at level M holds exactly this many UTF-8 bytes.
pub const MAX_QR_DATA_BYTES: usize = 997;
/// Spec 6.2: largest QR code version a Swiss QR Code may use.
pub const MAX_QR_VERSION: u8 = 25;

/// Encodes the payload per spec 6.1/6.2: error correction level M (never
/// raised, which `QrCode::encode_text` would do when it fits), smallest
/// version, at most version 25.
pub fn encode_text_to_qr_code(qr_text: &str) -> Result<QrCode, QRBillError> {
    let segments = QrSegment::make_segments(qr_text);
    QrCode::encode_segments_advanced(
        &segments,
        QrCodeEcc::Medium,
        QrVersion::MIN,
        QrVersion::new(MAX_QR_VERSION),
        None,
        false,
    )
    .map_err(|_| QRBillError::QrEncodingFailed)
}

pub fn qr_code(bill: &BillData) -> Result<QrCode, QRBillError> {
    QrBill::new(bill)
        .and_then(|b| b.create_qr_text())
        .and_then(|txt| encode_text_to_qr_code(&txt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, Currency};

    fn address() -> Address {
        Address::new("Health insurance fit&kicking", Some("Am Wasser"), Some("1"), "3000", "Bern", "CH").unwrap()
    }

    fn qr_bill_bill_data() -> BillData {
        BillData::new(
            "CH64 3196 1000 0044 2155 7".to_string(),
            address(),
            None,
            Currency::CHF,
            None,
            ReferenceType::QrRef("000008207791225857421286694".to_string()),
            None,
            None,
            [None, None],
        )
        .unwrap()
    }

    #[test]
    fn strd_bkg_inf_placeholder_preserves_alt_pmt_position_when_billing_info_absent() {
        let mut bill = qr_bill_bill_data();
        bill.alternative_schemes = [Some("eBill/B/sarah.beispiel@einfach-zahlen.ch".to_string()), None];

        let text = QrBill::new(&bill).unwrap().create_qr_text().unwrap();
        let fields: Vec<&str> = text.split("\r\n").collect();

        let trailer_idx = fields.iter().position(|&f| f == "EPD").unwrap();
        assert_eq!(
            fields[trailer_idx + 1], "",
            "StrdBkgInf must be an empty placeholder (not omitted) when AltPmt follows"
        );
        assert_eq!(fields[trailer_idx + 2], "eBill/B/sarah.beispiel@einfach-zahlen.ch");
    }

    #[test]
    fn strd_bkg_inf_omitted_when_nothing_follows() {
        let bill = qr_bill_bill_data();

        let text = QrBill::new(&bill).unwrap().create_qr_text().unwrap();
        assert!(text.ends_with("EPD"));
    }
}



