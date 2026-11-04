# swiss_qrust vs. SIX Swiss Implementation Guidelines for the QR-bill

Evaluated on 2026-11-04 against the specifications published by SIX Interbank Clearing at
<https://www.six-group.com/en/products-services/banking-services/payment-standardization/standards/qr-bill.html>:

| Document | Status |
|---|---|
| [IG QR-bill v2.4](https://www.six-group.com/dam/download/banking-services/standardization/qr-bill/ig-qr-bill-v2.4-en.pdf) (24.02.2026) | Introduced with the SIC release of 13/14 November 2026. **Primary reference.** |
| [IG QR-bill v2.3](https://www.six-group.com/dam/download/banking-services/standardization/qr-bill/ig-qr-bill-v2.3-en.pdf) (21.11.2025) | Still valid until November 2027. Previously cited by the crate. |
| [Style Guide QR-bill](https://www.six-group.com/dam/download/banking-services/standardization/qr-bill/style-guide-qr-bill-en.pdf) | Layout cross-check only. |

The German v2.4 text was used for the Annex A example values, since the English tables add translation glosses such
as "(sample company)" to the names.

## How to run

```bash
cargo test --test test_six_compliance -- --nocapture
```

- `ig_qr_bill_v2_4_vectors.json` holds 72 bill vectors and 5 billing-information vectors. Each one names the spec
  section it comes from.
- `scripts/gen_six_vectors.py` regenerates the file. Expected payloads come from Table 8 and Annex A, and check digits
  are computed independently of the crate.
- `tests/test_six_compliance.rs` runs each vector the way a caller would: JSON → `InputBill` → `BillData` → QR text
  → `qr_code`. Every bill the crate accepts must also pass these symbol rules:
  - only CR+LF separators, with none after the final line (4.1.4)
  - only characters from the 4.1.1 set
  - at most 997 characters
  - error correction level M (6.1)
  - QR version 25 or lower (6.2)
- A vector marked `known_gap` must still fail. The test goes red once the crate conforms, so the marker gets
  removed along with the fix.

## Result

The first evaluation found 8 issues (G1–G8 below): 60 of 72 bill vectors conformed. G1–G6 and G8 have since been fixed,
and 71 of 72 bill vectors plus all 5 billing-information vectors now conform. The remaining vector, `QRR-EUR`, is G7,
which is deliberately deferred until IG v2.3 expires in November 2027
([thomi137/swissqrust#2](https://github.com/thomi137/swissqrust/issues/2)). G8 is covered by a unit test in
`src/language/language.rs` instead of a vector.

### What conforms

- **Header and line layout (Table 8):**
  - `SPC` / `0200` / `1` header
  - all 31 mandatory lines in order, CR+LF separators, nothing after the last line
  - Ultimate Creditor sent as 7 empty lines
  - Billing information and Alternative procedures (status A) left out when unused, but kept as an empty placeholder
    when an Alternative procedure follows
- **Annex A examples 1–6** produce the exact payload the spec shows. Example 2 was also built from structured S1
  fields.
- **Account and reference pairing (4.3.2, 7.1):** QR-IBAN (QR-IID 30000–31999) only with QRR; IBAN only with SCOR or
  NON; CH/LI only; mod-97 IBAN check.
- **QR reference (2.12.1, Annex B):** 27 digits, mod-10 recursive check digit, not all zeros.
- **Creditor Reference (2.12.2):** 5–25 alphanumeric characters, mod 97-10 check, `RF` prefix.
- **Amount:** two decimal places, `.` as separator, range 0.01–999 999 999.99.
- **Currency:** CHF/EUR only, case-sensitive.
- **Text lengths:** unstructured message ≤ 140; message + billing information ≤ 140 together; ≤ 2 Alternative
  procedures of ≤ 100 characters each.
- **Character set (4.1.1):**
  - the extended Latin set is accepted, including Ș ș Ț ț €
  - control characters are rejected, so a CR or LF inside a value can't shift the following lines
- **Addresses:** structured address only (`S`); name, postal code, town and an ISO 3166-1 country are mandatory;
  for an ultimate debtor, those lines are mandatory once the group is used.
- **SIX/Swico S1 billing information (Annex D):** all four Table 31 examples and the Annex A example 2 string encode
  byte-for-byte and decode back.
- **QR symbol (6.1, 6.4):**
  - ECC level M, smallest version
  - 46 × 46 mm symbol with a 7 × 7 mm cross
  - The crate calls `QrCode::encode_text`, which may raise the ECC level, but a sweep of real payloads never
    triggered that: at these data sizes, level Q capacity at version *v* is below level M capacity at *v*−1.
- **Layout constants (checked by reading, not by vectors):**
  - 210 × 105 mm slip with a 62 mm receipt
  - fonts 11 / 8 / 10 pt on the payment part, 6 / 8 pt on the receipt, 7 pt for Alternative procedures
  - Liberation Sans
  - blank boxes: amount 40 × 15 mm and debtor 65 × 25 mm on the payment part, 30 × 10 mm and 52 × 20 mm on the receipt

### Findings

| ID | Severity | Spec | Finding | Vectors | Status |
|---|---|---|---|---|---|
| G1 | High | 4.4, Table 10 | **"DO NOT USE FOR PAYMENT" bills are impossible.** The spec requires amount `0.00` with the fixed text (DE/FR/IT/EN/RM), but `BillData::new` rejects every all-zero amount (`src/bill/bill_data.rs`). The spec contradicts itself here (line 19 says 0.01–999 999 999.99, while 4.4 requires 0.00), and 4.4 is the explicit exception. | `NOTIFY-DE`, `NOTIFY-EN`, `NOTIFY-RM` | Fixed |
| G2 | Medium | 4.2.2 lines 6–10, 22–26 | **Address limits count UTF-8 bytes, not characters.** `Address::new` uses `str::len()` (`src/bill/address.rs`). A valid 70-character name or 35-character town with umlauts or accents is rejected. For example, any town over 17 characters made only of `ä`/`é` is over 35 bytes. | `ADDR-NAME-70-CHARS-UMLAUT`, `ADDR-TOWN-35-CHARS-ACCENT` | Fixed |
| G3 | Medium | 4.2.2 line 29, 4.1.2 | **Creditor Reference with spaces goes into the QR code with spaces.** `ReferenceType::infer` validates the reference with whitespace stripped but stores the original, so line 29 becomes `RF18 5390 0754 7034`, which is not alphanumeric. A QR reference with spaces is rejected instead, which is inconsistent. | `SCOR-SPACES` | Fixed |
| G4 | Medium | 6.2 | **No check of the version-25 / 997-character limit.** The message and Alternative procedure limits count characters, so text like `€` (3 bytes) produces a 903-character, 1583-byte payload, which encodes as **QR version 33**. Related: both renderers call `qr_code(bill).ok()`, so if encoding ever failed the bill would render **without a QR code** instead of returning an error (`src/render/engines/svg/render_svg_bill.rs`, `src/render/engines/pdf/render_bill.rs`). | `QR-MAX-VERSION-25` | Fixed |
| G5 | Low | 4.2.2 line 19 | **Leading zeros accepted in the amount.** `AMOUNT_REGEX` is `^\d{1,9}\.\d{2}$`, so `0050.00` and `01.00` pass; the spec says "without leading zeroes". | `AMT-REJECT-0050.00`, `AMT-REJECT-01.00` | Fixed |
| G6 | Low | 4.2.2 line 4 | **IBAN length not fixed at 21.** `is_valid_iban` accepts 15–34 characters, so a CH IBAN of 20 or 22 characters with valid check digits is written into the QR code. | `IBAN-LENGTH-20`, `IBAN-LENGTH-22` | Fixed |
| G7 | Medium (v2.4 only) | 1.1, 2.10, 2.12.1, 4.3.2 | **QR-IBAN + QR reference accepted with EUR.** v2.4 restricts QR-IBAN/QRR to CHF (euroSIC is discontinued); v2.3 still allowed it. | `QRR-EUR` | Deferred to November 2027 ([#2](https://github.com/thomi137/swissqrust/issues/2)) |
| G8 | Low (v2.4) | 3.2, Annex C | **Romansh missing.** Romansh is a correspondence language with its own headings (e.g. "Part da pajament", "Quittanza"), in force since 1 January 2026. `Language` only has De/Fr/It/En. Not written as a vector, because the test could not reference a variant that doesn't exist. | — | Fixed |

### Errata in the specification

- **E1 — Annex A, Table 20 (example 4).** The printed reference `RF720191230100405JSH0438` has invalid
  mod 97-10 check digits (the remainder is 49, not 1). The correct reference is `RF240191230100405JSH0438`, which SIX's
  own sample sheet Nr. 17 uses (`tests/data/expected/Nr. 17 Datenschema englisch.txt`). The error appears in both v2.3
  and v2.4. The crate correctly rejects the printed value (`ANNEX-A-4-AS-PRINTED`), and the golden vector
  `ANNEX-A-4` uses the corrected reference.

### Fixes applied

- **G1:** `BillData::new` accepts `0.00` only when the unstructured message is one of the five Table 10 texts
  (`NOTIFICATION_MESSAGES`). Any other zero amount is still rejected.
- **G2:** `Address::new` counts characters (`chars().count()`) for every length limit.
- **G3:**
  - `ReferenceType::infer` strips whitespace before classifying, so `21 00000 00003 …` is now accepted as a QR
    reference, and a SCOR is stored without spaces.
  - `BillData::new` also strips and validates the reference itself. Before, a `ReferenceType` built directly (not
    through `infer`) wasn't checked at all.
- **G4:**
  - `BillData::new` rejects a payload above 997 UTF-8 bytes with `BillError::QrDataTooLong`. In byte mode that is
    exactly the version-25, level-M capacity.
  - `encode_text_to_qr_code` uses `encode_segments_advanced` with level M fixed (no boosting) and versions 1–25.
  - Both renderers return `RenderError::QrCodeGenerationError` instead of drawing a slip without its QR code.
- **G5:** `AMOUNT_REGEX` is now `^(0|[1-9]\d{0,8})\.\d{2}$`.
- **G6:** `is_valid_iban` requires exactly 21 characters.

- **G8:**
  - `Language::Rm` prints the slip with the Annex C Romansh labels. It is available in the CLI (`--lang rm`), the
    GUI and the web app.
  - The web app's form labels and country names have no Romansh translation, so they fall back to German.
- **Also fixed:** the French "Billing information" label was "Information de facture"; Annex C, Table 24 says
  "Informations de facture".

### Deferred

- **G7:** QR-IBAN + QR reference in EUR is still accepted. v2.4 forbids it, but v2.3 allows it and stays valid
  until November 2027, so existing EUR bills keep working until then. The `QRR-EUR` vector stays marked
  `known_gap: G7`, and [thomi137/swissqrust#2](https://github.com/thomi137/swissqrust/issues/2) tracks adding
  the reject. The spec link in `src/bill/qr_bill.rs` now points at v2.4.

### Not covered

- Rendered layout: positions, 5 mm margins, perforation and scissors, IBAN and reference grouping in the printed
  text, truncation with "…".
- PDF specifics (3.7).
- Readability of the printed QR code.
- The LF-only separator option: the crate always writes CR+LF, which the spec permits.
- Postal codes written with a country prefix (for example `CH-8000`) are not rejected (4.2.2 line 9). No vector
  checks this, because it can't be detected reliably in general.
