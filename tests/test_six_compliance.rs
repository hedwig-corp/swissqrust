//! Spec-derived test vectors for the SIX "Swiss Implementation Guidelines
//! for the QR-bill" v2.4 (valid from 14 November 2026). The vectors live in
//! `data/six/ig_qr_bill_v2_4_vectors.json`; the findings they track are
//! written up in `data/six/COMPLIANCE.md`.
//!
//! Every vector goes through the same path a real caller uses: JSON ->
//! `InputBill` -> `BillData::try_from` -> `QrBill::create_qr_text` ->
//! `qr_code`. A vector with a `known_gap` marker documents a current
//! deviation from the spec: it must still fail, so that fixing the gap
//! also forces removing the marker.

use std::convert::TryFrom;
use qrcodegen::QrCodeEcc;
use serde::Deserialize;
use swiss_qrust::qr_bill::{qr_code, QrBill};
use swiss_qrust::support::utils::is_in_extended_sps_charset;
use swiss_qrust::{parse_qr_text, BillData, InputBill, InputSwicoBillInformation, SwicoBillInformation};

const VECTORS: &str = include_str!("data/six/ig_qr_bill_v2_4_vectors.json");

/// Line 31 (`Trailer`) is the last line every Swiss QR Code carries.
const TRAILER_LINE: usize = 31;
/// IG 6.2: maximum data content, including the separators.
const MAX_CHARACTERS: usize = 997;
/// IG 6.2: resulting maximum version at error correction level M.
const MAX_QR_VERSION: u8 = 25;

#[derive(Deserialize)]
struct VectorFile {
    vectors: Vec<Vector>,
    billing_information: Vec<BillingInformationVector>,
}

#[derive(Deserialize)]
struct Vector {
    id: String,
    spec: String,
    description: String,
    input: serde_json::Value,
    expect: Expect,
    known_gap: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Expect {
    Reject,
    AcceptOrReject,
    QrLines(Vec<String>),
    QrLinesOrReject(Vec<String>),
}

#[derive(Deserialize)]
struct BillingInformationVector {
    id: String,
    swico_input: serde_json::Value,
    text: String,
}

fn vector_file() -> VectorFile {
    serde_json::from_str(VECTORS).expect("vector file is valid JSON")
}

/// Runs the input through the library: `Err` if the bill is rejected
/// anywhere along the way, otherwise the bill and its QR payload.
fn build(input: &serde_json::Value) -> Result<(BillData, String), String> {
    let input: InputBill = serde_json::from_value(input.clone()).map_err(|e| e.to_string())?;
    let bill = BillData::try_from(input).map_err(|e| e.to_string())?;
    let text = QrBill::new(&bill)
        .and_then(|qr| qr.create_qr_text())
        .map_err(|e| e.to_string())?;
    Ok((bill, text))
}

/// Rules every accepted bill must satisfy, whatever its content.
fn check_symbol(bill: &BillData, text: &str) -> Result<(), String> {
    // 4.1.4: one separator type per document, so every line break is CR+LF.
    if text.replace("\r\n", "").contains(['\r', '\n']) {
        return Err("payload mixes line separators or has a line break inside a value".into());
    }
    if text.ends_with('\n') {
        return Err("carriage return after the final element (4.1.4)".into());
    }
    if let Some(ch) = text.replace("\r\n", "").chars().find(|&c| !is_in_extended_sps_charset(c as u32)) {
        return Err(format!("character {ch:?} outside the 4.1.1 set"));
    }
    let chars = text.chars().count();
    if chars > MAX_CHARACTERS {
        return Err(format!("payload has {chars} characters, maximum is {MAX_CHARACTERS} (6.2)"));
    }

    let qr = qr_code(bill).map_err(|e| format!("QR encoding failed: {e}"))?;
    if qr.error_correction_level() != QrCodeEcc::Medium {
        return Err(format!("error correction level {:?}, must be M (6.1)", qr.error_correction_level()));
    }
    if qr.version().value() > MAX_QR_VERSION {
        return Err(format!(
            "QR version {} ({} UTF-8 bytes), maximum is {MAX_QR_VERSION} (6.2)",
            qr.version().value(),
            text.len()
        ));
    }
    Ok(())
}

/// 4.1.4 lets unused status-A lines after the trailer be omitted; the
/// Annex A tables show them as empty lines instead. Compare both forms
/// the same way.
fn normalized_lines(text: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = text.split("\r\n").collect();
    while lines.len() > TRAILER_LINE && lines.last() == Some(&"") {
        lines.pop();
    }
    lines
}

fn compare_lines(actual: &str, expected: &[String]) -> Result<(), String> {
    let actual = normalized_lines(actual);
    let expected = expected.join("\r\n");
    let expected = normalized_lines(&expected);
    if actual == expected {
        return Ok(());
    }
    let first_diff = actual
        .iter()
        .zip(&expected)
        .position(|(a, e)| a != e)
        .unwrap_or(actual.len().min(expected.len()));
    Err(format!(
        "line {} differs: got {:?}, expected {:?} ({} vs {} lines)",
        first_diff + 1,
        actual.get(first_diff),
        expected.get(first_diff),
        actual.len(),
        expected.len()
    ))
}

fn evaluate(vector: &Vector) -> Result<(), String> {
    let outcome = build(&vector.input);
    match (&vector.expect, outcome) {
        (Expect::Reject, Err(_)) => Ok(()),
        (Expect::Reject, Ok((_, text))) => Err(format!("accepted, payload:\n{}", text.replace("\r\n", "⏎"))),
        (Expect::AcceptOrReject | Expect::QrLinesOrReject(_), Err(_)) => Ok(()),
        (Expect::AcceptOrReject, Ok((bill, text))) => check_symbol(&bill, &text),
        (Expect::QrLines(_), Err(e)) => Err(format!("rejected: {e}")),
        (Expect::QrLines(expected) | Expect::QrLinesOrReject(expected), Ok((bill, text))) => {
            compare_lines(&text, expected)?;
            check_symbol(&bill, &text)
        }
    }
}

#[test]
fn six_vectors_conform() {
    let failures: Vec<String> = vector_file()
        .vectors
        .iter()
        .filter(|v| v.known_gap.is_none())
        .filter_map(|v| {
            evaluate(v)
                .err()
                .map(|e| format!("{} ({}): {}\n    {}", v.id, v.spec, v.description, e))
        })
        .collect();

    assert!(failures.is_empty(), "{} vector(s) failed:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn six_known_gaps_still_fail() {
    let file = vector_file();
    let gaps: Vec<&Vector> = file.vectors.iter().filter(|v| v.known_gap.is_some()).collect();

    for v in &gaps {
        match evaluate(v) {
            Ok(()) => panic!(
                "{} now conforms - remove `known_gap: {}` from the vector and update COMPLIANCE.md",
                v.id,
                v.known_gap.as_deref().unwrap()
            ),
            Err(e) => eprintln!("known gap {} - {}: {}", v.known_gap.as_deref().unwrap(), v.id, e),
        }
    }
}

#[test]
fn six_vector_ids_are_unique() {
    let file = vector_file();
    let mut ids: Vec<&str> = file.vectors.iter().map(|v| v.id.as_str()).collect();
    ids.extend(file.billing_information.iter().map(|v| v.id.as_str()));
    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), total, "duplicate vector id");
}

/// Annex D, Table 31 examples and the billing information of Annex A,
/// example 2: encode from structured fields, then decode and re-encode.
#[test]
fn six_billing_information_vectors() {
    for v in vector_file().billing_information {
        let input: InputSwicoBillInformation = serde_json::from_value(v.swico_input).unwrap();
        let info = SwicoBillInformation::try_from(input).unwrap();
        assert_eq!(info.encode_as_text().as_deref(), Some(v.text.as_str()), "{}: encode", v.id);

        let decoded = SwicoBillInformation::decode_text(&v.text).unwrap_or_else(|| panic!("{}: decode", v.id));
        assert_eq!(decoded, info, "{}: decode", v.id);
    }
}

/// The reverse direction: every payload the spec shows (and every one the
/// library generates for an accepted vector) parses back into the same
/// bill. Rejected vectors are input-side rules, so they have no payload.
#[test]
fn six_payloads_parse_back() {
    let failures: Vec<String> = vector_file()
        .vectors
        .iter()
        .filter(|v| v.known_gap.is_none())
        .filter_map(|v| {
            let Ok((bill, text)) = build(&v.input) else { return None };
            let mut payloads = vec![("generated", text)];
            if let Expect::QrLines(lines) | Expect::QrLinesOrReject(lines) = &v.expect {
                let mut lines = lines.clone();
                // Annex A pads with empty status-A lines; a real payload omits them (4.1.4).
                while lines.len() > TRAILER_LINE && lines.last().is_some_and(|l| l.is_empty()) {
                    lines.pop();
                }
                payloads.push(("spec", lines.join("\r\n")));
            }
            let errors: Vec<String> = payloads
                .into_iter()
                .filter_map(|(which, text)| match parse_qr_text(&text) {
                    Ok(parsed) if parsed == bill => None,
                    Ok(parsed) => Some(format!("{which} payload parsed to a different bill: {parsed:?}")),
                    Err(e) => Some(format!("{which} payload rejected: {e}")),
                })
                .collect();
            (!errors.is_empty()).then(|| format!("{} ({}): {}", v.id, v.spec, errors.join("; ")))
        })
        .collect();

    assert!(failures.is_empty(), "{} vector(s) failed:\n{}", failures.len(), failures.join("\n"));
}
