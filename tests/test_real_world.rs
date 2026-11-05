//! Real-world QR-bills: Swiss QR Codes cropped from invoices and receipt
//! photos that users uploaded to SwissSync.
//!
//! The dataset is real customer data, so it is NOT in this repository. It
//! lives in a private repository and is read at run time from
//! `$SWISSQRUST_REAL_WORLD_DIR`, or else from `tests/data/real_world`
//! (git-ignored). Without either, every test here skips:
//!
//! ```text
//! git clone https://git.hedwig.sh/vincenzopalazzo/swissqrust-test-dataset.git tests/data/real_world
//! cargo test --test test_real_world
//! ```
//!
//! Per case:
//!
//! 1. the image decodes (with `rqrr`) to the payload recorded in
//!    `cases.json`, which was read with zbar when the dataset was built;
//! 2. `parse_qr_text` accepts the payload, unless the case is marked
//!    `invalid` (a real bill that breaks the spec), which it must reject;
//! 3. the parsed bill regenerates the payload line for line.
//!
//! As in `test_six_compliance.rs`, a case carrying a `known_gap` or
//! `decoder_gap` marker documents a current deviation: it must still fail,
//! so that fixing it also forces removing the marker.

use std::path::{Path, PathBuf};
use serde::Deserialize;
use swiss_qrust::parse_qr_text;
use swiss_qrust::qr_bill::QrBill;

/// Line 31 (`Trailer`) is the last line every Swiss QR Code carries.
const TRAILER_LINE: usize = 31;

#[derive(Deserialize)]
struct CaseFile {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    image: String,
    origin: String,
    payload: Vec<String>,
    /// The bill breaks the spec, so `parse_qr_text` must reject it.
    invalid: Option<String>,
    /// The library cannot reproduce this payload yet.
    known_gap: Option<String>,
    /// `rqrr` cannot read this image (zbar could, when the dataset was built).
    decoder_gap: Option<String>,
}

/// Where the dataset is, if it is available on this machine.
fn dataset_dir() -> Option<PathBuf> {
    let dir = match std::env::var_os("SWISSQRUST_REAL_WORLD_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/real_world"),
    };
    dir.join("cases.json").is_file().then_some(dir)
}

/// The dataset's cases, or `None` (after saying so) when it is not present.
fn cases() -> Option<(PathBuf, Vec<Case>)> {
    let Some(dir) = dataset_dir() else {
        eprintln!("real-world dataset not found: set SWISSQRUST_REAL_WORLD_DIR or clone it into tests/data/real_world; skipping");
        return None;
    };
    let text = std::fs::read_to_string(dir.join("cases.json")).expect("cases.json is readable");
    let cases = serde_json::from_str::<CaseFile>(&text).expect("cases.json is valid JSON").cases;
    Some((dir, cases))
}

/// 4.1.4 lets unused status-A lines after the trailer be omitted, and
/// issuers differ on whether they do: compare both forms the same way.
fn normalized(lines: &[&str]) -> Vec<String> {
    let mut lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    while lines.len() > TRAILER_LINE && lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

fn compare_lines(actual: &str, expected: &[String]) -> Result<(), String> {
    let actual = normalized(&actual.split("\r\n").collect::<Vec<_>>());
    let expected = normalized(&expected.iter().map(String::as_str).collect::<Vec<_>>());
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

fn decode_image(dir: &Path, case: &Case) -> Result<String, String> {
    let path = dir.join(&case.image);
    let img = image::open(&path).map_err(|e| format!("{}: {e}", path.display()))?.to_luma8();
    // rqrr misses finder patterns when modules are only a few pixels wide,
    // which is what a 200 dpi page render gives: retry at twice the size.
    decode_luma(&img).or_else(|first| {
        let (w, h) = img.dimensions();
        let big = image::imageops::resize(&img, w * 2, h * 2, image::imageops::FilterType::Triangle);
        decode_luma(&big).map_err(|second| format!("{first} (at 2x: {second})"))
    })
}

fn decode_luma(img: &image::GrayImage) -> Result<String, String> {
    let mut prepared = rqrr::PreparedImage::prepare_from_greyscale(img.width() as usize, img.height() as usize, |x, y| {
        img.get_pixel(x as u32, y as u32).0[0]
    });
    let grids = prepared.detect_grids();
    if grids.is_empty() {
        return Err("no QR code found".into());
    }
    let mut errors = Vec::new();
    for grid in grids {
        match grid.decode() {
            Ok((_, text)) if text.starts_with("SPC") => return Ok(text),
            Ok((_, text)) => errors.push(format!("not a Swiss QR Code: {:?}", text.chars().take(20).collect::<String>())),
            Err(e) => errors.push(e.to_string()),
        }
    }
    Err(errors.join("; "))
}

fn check_decode(dir: &Path, case: &Case) -> Result<(), String> {
    let text = decode_image(dir, case)?;
    let lines: Vec<String> = text.replace("\r\n", "\n").split('\n').map(String::from).collect();
    let lines = normalized(&lines.iter().map(String::as_str).collect::<Vec<_>>());
    let expected = normalized(&case.payload.iter().map(String::as_str).collect::<Vec<_>>());
    if lines == expected {
        Ok(())
    } else {
        Err(format!("decoded {} lines that differ from the {} recorded ones", lines.len(), expected.len()))
    }
}

/// The payload as it was in the QR code: `cases.json` keeps it one line
/// per element, and zbar does not report which separator it used.
fn payload_text(case: &Case) -> String {
    case.payload.join("\r\n")
}

fn check_rejected(_dir: &Path, case: &Case) -> Result<(), String> {
    match parse_qr_text(&payload_text(case)) {
        Ok(_) => Err("accepted".into()),
        Err(e) => {
            eprintln!("{} rejected as expected ({}): {e}", case.id, case.invalid.as_deref().unwrap_or_default());
            Ok(())
        }
    }
}

fn check_round_trip(_dir: &Path, case: &Case) -> Result<(), String> {
    let bill = parse_qr_text(&payload_text(case)).map_err(|e| format!("rejected: {e}"))?;
    let text = QrBill::new(&bill)
        .and_then(|qr| qr.create_qr_text())
        .map_err(|e| format!("cannot regenerate: {e}"))?;
    compare_lines(&text, &case.payload)
}

type Check = fn(&Path, &Case) -> Result<(), String>;

fn assert_conforms(check: Check, gap: fn(&Case) -> Option<&str>) {
    let Some((dir, cases)) = cases() else { return };
    let failures: Vec<String> = cases
        .iter()
        .filter(|c| gap(c).is_none())
        .filter_map(|c| check(&dir, c).err().map(|e| format!("{} ({}): {}", c.id, c.origin, e)))
        .collect();
    assert!(failures.is_empty(), "{} case(s) failed:\n{}", failures.len(), failures.join("\n"));
}

fn assert_gaps_still_fail(check: Check, gap: fn(&Case) -> Option<&str>, marker: &str) {
    let Some((dir, cases)) = cases() else { return };
    for c in cases {
        let Some(reason) = gap(&c) else { continue };
        match check(&dir, &c) {
            Ok(()) => panic!("{} now passes - remove `{marker}: {reason}` from cases.json", c.id),
            Err(e) => eprintln!("{marker} {reason} - {}: {e}", c.id),
        }
    }
}

#[test]
fn real_world_images_decode_to_recorded_payload() {
    assert_conforms(check_decode, |c| c.decoder_gap.as_deref());
}

#[test]
fn real_world_decoder_gaps_still_fail() {
    assert_gaps_still_fail(check_decode, |c| c.decoder_gap.as_deref(), "decoder_gap");
}

#[test]
fn real_world_valid_payloads_parse_and_round_trip() {
    assert_conforms(check_round_trip, |c| c.invalid.as_deref().or(c.known_gap.as_deref()));
}

#[test]
fn real_world_invalid_payloads_are_rejected() {
    assert_conforms(check_rejected, |c| if c.invalid.is_some() { None } else { Some("valid") });
}

#[test]
fn real_world_known_gaps_still_fail() {
    assert_gaps_still_fail(check_round_trip, |c| c.known_gap.as_deref(), "known_gap");
}

#[test]
fn real_world_case_ids_are_unique_and_images_exist() {
    let Some((dir, cases)) = cases() else { return };
    let mut ids: Vec<&str> = cases.iter().map(|c| c.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), cases.len(), "duplicate case id");
    for c in &cases {
        let path = dir.join(&c.image);
        assert!(path.is_file(), "{}: missing {}", c.id, path.display());
    }
}
