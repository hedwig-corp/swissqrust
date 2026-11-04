/*
 * Copyright (c) 2026 Thomas Prosser
 * Licensed under MIT License
 * https://opensource.org/licenses/MIT
 */

//! The Rust "backend" exposed to the Alpine.js frontend via wasm-bindgen.
//! Alpine owns UI state and interaction; these functions are called for the
//! things that actually need the library: validation, checksums, the live
//! SVG preview, and PDF generation. Everything here routes through
//! `swiss_qrust`'s existing public API (in particular the `InputBill` /
//! `InputAddress` JSON-deserializable boundary types and their `TryFrom`
//! impls into `BillData` / `Address`) -- nothing in `src/` was changed for
//! this.

use strum::IntoEnumIterator;
use wasm_bindgen::prelude::*;

use swiss_qrust::pdf::render_bill_to_pdf;
use swiss_qrust::svg::render_bill_to_svg;
use swiss_qrust::{
    generate_iso11649_with_checksum, generate_qrr_with_checksum, is_qr_iban, is_valid_iban, label,
    Country, InputBill, Language, LabelKey, ReferenceType,
};

fn parse_lang(lang: &str) -> Language {
    match lang {
        "Fr" => Language::Fr,
        "It" => Language::It,
        "En" => Language::En,
        "Rm" => Language::Rm,
        _ => Language::De,
    }
}

fn to_js_err<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}

/// Renders the live bill preview to an SVG string.
///
/// `bill_json` must deserialize to `swiss_qrust::InputBill`.
#[wasm_bindgen]
pub fn render_preview_svg(bill_json: &str, lang: &str) -> Result<String, JsValue> {
    let input: InputBill = serde_json::from_str(bill_json).map_err(to_js_err)?;
    let bill = swiss_qrust::BillData::try_from(input).map_err(to_js_err)?;
    render_bill_to_svg(&bill, parse_lang(lang)).map_err(to_js_err)
}

/// Renders the bill to PDF bytes, returned to JS as a `Uint8Array`.
///
/// `bill_json` must deserialize to `swiss_qrust::InputBill`.
#[wasm_bindgen]
pub fn render_pdf(bill_json: &str, lang: &str) -> Result<Vec<u8>, JsValue> {
    let input: InputBill = serde_json::from_str(bill_json).map_err(to_js_err)?;
    let bill = swiss_qrust::BillData::try_from(input).map_err(to_js_err)?;
    render_bill_to_pdf(&bill, parse_lang(lang)).map_err(to_js_err)
}

/// Whether `iban` (whitespace-insensitive) is a structurally valid IBAN.
#[wasm_bindgen]
pub fn is_valid_iban_js(iban: &str) -> bool {
    is_valid_iban(&clean(iban)).is_ok()
}

/// Whether `iban` (whitespace-insensitive) is a Swiss/LI QR-IBAN, i.e.
/// requires a QR reference rather than an optional SCOR one.
#[wasm_bindgen]
pub fn is_qr_iban_js(iban: &str) -> bool {
    is_qr_iban(&clean(iban))
}

fn clean(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Generates a valid 27-digit QR reference from up to 26 raw digits (the
/// checksum digit is computed and appended; the raw digits are zero-padded
/// on the left to 26).
#[wasm_bindgen]
pub fn generate_qr_reference(raw: &str) -> Result<String, JsValue> {
    generate_qrr_with_checksum(raw).map_err(to_js_err)
}

/// Generates a valid SCOR (ISO 11649) reference from an alphanumeric
/// identifier (checksum digits are computed and prepended with "RF").
#[wasm_bindgen]
pub fn generate_scor_reference(raw: &str) -> Result<String, JsValue> {
    generate_iso11649_with_checksum(raw).map_err(to_js_err)
}

/// Validates a reference number against the given IBAN's QR-ness: returns
/// `{"valid": bool, "error": string | null}` as JSON. A QR-IBAN requires a
/// QR reference; a normal IBAN must not have one (SCOR or none are fine).
#[wasm_bindgen]
pub fn validate_reference_json(reference: &str, is_qr: bool) -> String {
    let (valid, error) = match ReferenceType::infer(reference.trim()) {
        Ok(ReferenceType::NoRef) if is_qr => (false, Some("A QR-IBAN requires a QR reference")),
        Ok(ReferenceType::QrRef(_)) if !is_qr => {
            (false, Some("A QR reference requires a QR-IBAN"))
        }
        Ok(ReferenceType::Creditor(_)) if is_qr => (
            false,
            Some("A QR-IBAN requires a QR reference, not a SCOR reference"),
        ),
        Ok(_) => (true, None),
        Err(_) => (false, Some("Not a valid reference")),
    };
    serde_json::json!({ "valid": valid, "error": error }).to_string()
}

const LABEL_KEYS: &[LabelKey] = &[
    LabelKey::PaymentPart,
    LabelKey::AccountPayableTo,
    LabelKey::Reference,
    LabelKey::AdditionalInformation,
    LabelKey::Currency,
    LabelKey::Amount,
    LabelKey::Receipt,
    LabelKey::AcceptancePoint,
    LabelKey::SeparateBeforePayingIn,
    LabelKey::PayableBy,
    LabelKey::PayableByNameAddress,
    LabelKey::InFavourOf,
    LabelKey::BillingInformation,
    LabelKey::AlternativeProcedures,
];

const GUI_LABELS: &[(&str, Language, &str)] = &[
    ("Name", Language::De, "Name"),
    ("Name", Language::Fr, "Nom"),
    ("Name", Language::It, "Nome"),
    ("Name", Language::En, "Name"),
    ("Street", Language::De, "Strasse"),
    ("Street", Language::Fr, "Rue"),
    ("Street", Language::It, "Via"),
    ("Street", Language::En, "Street"),
    ("HouseNo", Language::De, "Nr"),
    ("HouseNo", Language::Fr, "N\u{b0}"),
    ("HouseNo", Language::It, "N."),
    ("HouseNo", Language::En, "No"),
    ("City", Language::De, "Ort"),
    ("City", Language::Fr, "Localit\u{e9}"),
    ("City", Language::It, "Luogo"),
    ("City", Language::En, "Town"),
    ("PostalCode", Language::De, "PLZ"),
    ("PostalCode", Language::Fr, "NPA"),
    ("PostalCode", Language::It, "CAP"),
    ("PostalCode", Language::En, "Postal Code"),
    ("Country", Language::De, "Land"),
    ("Country", Language::Fr, "Pays"),
    ("Country", Language::It, "Paese"),
    ("Country", Language::En, "Country"),

    ("NamePlaceholder", Language::De, "Firma oder Name der Person"),
    ("NamePlaceholder", Language::Fr, "Nom de l'entreprise ou de la personne"),
    ("NamePlaceholder", Language::It, "Nome dell'azienda o della persona"),
    ("NamePlaceholder", Language::En, "Company or Person Name"),

    ("SearchCountry", Language::De, "Land suchen..."),
    ("SearchCountry", Language::Fr, "Rechercher un pays..."),
    ("SearchCountry", Language::It, "Cerca paese..."),
    ("SearchCountry", Language::En, "Search country..."),

    ("QrReferencePlaceholder", Language::De, "QR-Referenz (erforderlich)"),
    ("QrReferencePlaceholder", Language::Fr, "R\u{e9}f\u{e9}rence QR (obligatoire)"),
    ("QrReferencePlaceholder", Language::It, "Riferimento QR (obbligatorio)"),
    ("QrReferencePlaceholder", Language::En, "QR reference (required)"),

    ("ScorReferencePlaceholder", Language::De, "SCOR-Referenz (optional)"),
    ("ScorReferencePlaceholder", Language::Fr, "R\u{e9}f\u{e9}rence SCOR (facultative)"),
    ("ScorReferencePlaceholder", Language::It, "Riferimento SCOR (facoltativo)"),
    ("ScorReferencePlaceholder", Language::En, "SCOR reference (optional)"),

    ("Propose", Language::De, "Vorschlagen"),
    ("Propose", Language::Fr, "Proposer"),
    ("Propose", Language::It, "Proponi"),
    ("Propose", Language::En, "Propose"),

    ("Language", Language::De, "Sprache"),
    ("Language", Language::Fr, "Langue"),
    ("Language", Language::It, "Lingua"),
    ("Language", Language::En, "Language"),

    ("DownloadPdf", Language::De, "Offizielles PDF herunterladen"),
    ("DownloadPdf", Language::Fr, "T\u{e9}l\u{e9}charger le PDF officiel"),
    ("DownloadPdf", Language::It, "Scarica il PDF ufficiale"),
    ("DownloadPdf", Language::En, "Download Official PDF"),

    ("DebtorToggle", Language::De, "Zahlungspflichtige Person"),
    ("DebtorToggle", Language::Fr, "D\u{e9}biteur"),
    ("DebtorToggle", Language::It, "Debitore"),
    ("DebtorToggle", Language::En, "Debtor Details"),

    ("DebtorSubtitle", Language::De, "Optionale Empf\u{e4}ngeradresse hinzuf\u{fc}gen"),
    ("DebtorSubtitle", Language::Fr, "Ajouter une adresse de destinataire facultative"),
    ("DebtorSubtitle", Language::It, "Aggiungi un indirizzo del destinatario facoltativo"),
    ("DebtorSubtitle", Language::En, "Add an optional recipient address"),

    ("SwicoToggle", Language::De, "Swico-Rechnungsinformationen"),
    ("SwicoToggle", Language::Fr, "Informations de facture Swico"),
    ("SwicoToggle", Language::It, "Informazioni di fattura Swico"),
    ("SwicoToggle", Language::En, "Swico Billing Info"),

    ("SwicoSubtitle", Language::De, "Strukturierte Rechnungsdetails hinzuf\u{fc}gen"),
    ("SwicoSubtitle", Language::Fr, "Ajouter des d\u{e9}tails de facture structur\u{e9}s"),
    ("SwicoSubtitle", Language::It, "Aggiungi dettagli di fattura strutturati"),
    ("SwicoSubtitle", Language::En, "Add structured invoice details"),

    ("InvoiceNumber", Language::De, "Rechnungsnummer"),
    ("InvoiceNumber", Language::Fr, "Num\u{e9}ro de facture"),
    ("InvoiceNumber", Language::It, "Numero fattura"),
    ("InvoiceNumber", Language::En, "Invoice Number"),

    ("InvoiceDate", Language::De, "Rechnungsdatum"),
    ("InvoiceDate", Language::Fr, "Date de facture"),
    ("InvoiceDate", Language::It, "Data fattura"),
    ("InvoiceDate", Language::En, "Invoice Date"),

    ("CustomerReference", Language::De, "Kundenreferenz"),
    ("CustomerReference", Language::Fr, "R\u{e9}f\u{e9}rence client"),
    ("CustomerReference", Language::It, "Riferimento cliente"),
    ("CustomerReference", Language::En, "Customer Reference"),

    ("VatRate", Language::De, "MWST-Satz (%)"),
    ("VatRate", Language::Fr, "Taux de TVA (%)"),
    ("VatRate", Language::It, "Aliquota IVA (%)"),
    ("VatRate", Language::En, "VAT Rate (%)"),

    ("DiscountPercent", Language::De, "Skonto (%)"),
    ("DiscountPercent", Language::Fr, "Escompte (%)"),
    ("DiscountPercent", Language::It, "Sconto (%)"),
    ("DiscountPercent", Language::En, "Discount (%)"),

    ("DiscountDays", Language::De, "Innert Tagen"),
    ("DiscountDays", Language::Fr, "Dans les jours"),
    ("DiscountDays", Language::It, "Entro giorni"),
    ("DiscountDays", Language::En, "Within Days"),

    ("StatusReady", Language::De, "Bereit"),
    ("StatusReady", Language::Fr, "Pr\u{ea}t"),
    ("StatusReady", Language::It, "Pronto"),
    ("StatusReady", Language::En, "Ready"),

    ("StatusDownloaded", Language::De, "PDF heruntergeladen!"),
    ("StatusDownloaded", Language::Fr, "PDF t\u{e9}l\u{e9}charg\u{e9} !"),
    ("StatusDownloaded", Language::It, "PDF scaricato!"),
    ("StatusDownloaded", Language::En, "PDF Downloaded!"),
];

/// All library labels plus the GUI-only labels (form field names, which
/// aren't in `swiss_qrust`), as one JSON object of key -> translated string
/// for the given language.
#[wasm_bindgen]
pub fn labels_json(lang: &str) -> String {
    let lang = parse_lang(lang);
    let mut map = serde_json::Map::new();

    for k in LABEL_KEYS {
        let text = label(*k, lang).unwrap_or("");
        map.insert(format!("{k:?}"), serde_json::Value::String(text.to_string()));
    }
    // The form itself has no Romansh translation; only the slip labels
    // above do. Fall back to German for the form.
    let gui_lang = if lang == Language::Rm { Language::De } else { lang };
    for (key, _, value) in GUI_LABELS.iter().filter(|(_, l, _)| *l == gui_lang) {
        map.insert(key.to_string(), serde_json::Value::String(value.to_string()));
    }

    serde_json::Value::Object(map).to_string()
}

/// All countries as a JSON array of `{code, name, flag}`, sorted CH/LI
/// first then alphabetically, for the given language.
#[wasm_bindgen]
pub fn country_list_json(lang: &str) -> String {
    let lang = parse_lang(lang);
    let mut list: Vec<Country> = Country::iter().collect();
    list.sort_by(|a, b| {
        let priority = |c: &Country| match c {
            Country::CH => 0,
            Country::LI => 1,
            _ => 2,
        };
        priority(a)
            .cmp(&priority(b))
            .then_with(|| a.to_string().cmp(&b.to_string()))
    });

    let items: Vec<serde_json::Value> = list
        .into_iter()
        .map(|c| {
            let meta = c.meta();
            let name = match lang {
                // No Romansh country names in the source data.
                Language::De | Language::Rm => meta.name_de,
                Language::Fr => meta.name_fr,
                Language::It => meta.name_it,
                Language::En => meta.name,
            };
            serde_json::json!({
                "code": c.to_string(),
                "name": name,
                "flag": meta.flag.unwrap_or_default(),
            })
        })
        .collect();

    serde_json::Value::Array(items).to_string()
}
