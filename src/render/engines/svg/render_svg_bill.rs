/*
 * Copyright (c) 2026 Thomas Prosser
 * Licensed under MIT License
 * https://opensource.org/licenses/MIT
 */
use crate::{BillData, Language, PaymentPartLayout, ReceiptLayout};
use crate::pdf::render_bill::RenderError;
use crate::qr_bill::qr_code;
use crate::svg::{execute_bill_ops_svg, SvgFontLibrary};
use crate::render::layout::bill_layout::LayoutStrategy;

/// Renders `bill` to a self-contained SVG document (receipt and payment
/// part) suitable for inline display, e.g. in a browser preview.
///
/// Uses a lighter-weight font metrics implementation than the PDF engine,
/// which matters for wasm builds (see the `web/` example crate). Otherwise
/// produces the same layout as [`crate::pdf::render_bill_to_pdf`] - both
/// engines consume the same `BillData` and share all positioning logic;
/// only the output format differs.
///
/// Unlike the PDF engine, this intentionally omits print-only elements
/// (scissors icon, perforation guide) that only make sense on a physical
/// printout - the divider between receipt and payment part is a plain line.
pub fn render_bill_to_svg(bill: &BillData, language: Language) -> Result<String, RenderError> {

    // 1. Initialise SVG-specific metrics (Lightweight for WASM)
    let fonts = SvgFontLibrary::new();

    // 2. Generate the "Bytecode" (DrawOps) using your shared Layout engines
    let mut ops = Vec::new();

    // Render Receipt
    ReceiptLayout::new()
        .render(bill, language, &fonts, &mut ops);

    // Render Payment Part
    PaymentPartLayout::new()
        .render(bill, language, &fonts, &mut ops);

    // A slip without its QR code is not a QR-bill, so fail instead of
    // silently drawing the rest of it.
    let qr = qr_code(bill).map_err(|_| RenderError::QrCodeGenerationError)?;
    let svg = execute_bill_ops_svg(ops, Some(&qr))?;

    Ok(svg)

}

