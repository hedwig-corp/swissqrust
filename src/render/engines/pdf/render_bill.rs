/*
 * Copyright (c) 2026 Thomas Prosser
 * Licensed under MIT License
 * https://opensource.org/licenses/MIT
 */

use thiserror::Error;
use crate::{BillData, Language, PaymentPartLayout, ReceiptLayout};
use crate::render::layout::bill_layout::LayoutStrategy;
use crate::pdf::{execute_bill_ops, PDFBuilder};
use crate::qr_bill::qr_code;

#[cfg(feature = "pdf-debug")]
use crate::render::debug_overlay::draw_debug_overlay;

/// Errors from the rendering pipeline itself (PDF/font setup, QR encoding,
/// ...) - as opposed to [`crate::BillError`], which covers invalid bill
/// *data*. A [`BillData`] that was successfully constructed should never
/// fail to render; if it does, that's a bug in this crate.
#[derive(Debug, PartialEq, Error)]
pub enum RenderError {
    #[error("Failed to create PDF builder")]
    BuilderCreationError,
    #[error("Failed to setup PDF")]
    SetupPdfError,
    #[error("Failed to compute font metrics")]
    FontMetricsError,
    #[error("Failed to execute bill operations")]
    BillOpsExecutionError,
    #[error("Failed to generate QR code")]
    QrCodeGenerationError,
}

/// Renders `bill` to a complete, single-page A4 PDF document (receipt and
/// payment part, as raw bytes ready to write to a file or serve over HTTP).
///
/// [`crate::pdf::create_pdf`] wraps this and writes straight to a path if
/// you don't need the bytes themselves.
///
/// ```
/// use swiss_qrust::{BillData, InputBill, Language};
///
/// # let toml = r#"
/// # iban = "CH93 0076 2011 6238 5295 7"
/// # currency = "CHF"
/// # amount = "199.95"
/// # [creditor_address]
/// # name = "Robert Schneider AG"
/// # street = "Rue du Lac"
/// # house_num = "1268"
/// # plz = "2501"
/// # city = "Biel"
/// # country = "CH"
/// # "#;
/// let bill = BillData::try_from(toml::from_str::<InputBill>(toml)?)?;
/// let pdf_bytes = swiss_qrust::pdf::render_bill_to_pdf(&bill, Language::De)?;
/// assert!(pdf_bytes.starts_with(b"%PDF"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn render_bill_to_pdf(bill: &BillData, language: Language) -> Result<Vec<u8>, RenderError>  {

     // --- 1. Create PDF builder ---
    let mut builder = PDFBuilder::new();
    builder.setup_pdf()?;

    // --- 2. Layout: Payment Part --
    PaymentPartLayout::new()
        .render(bill, language, &builder.fonts, &mut builder.ops);

    // --- 3. Layout: Receipt --
    ReceiptLayout::new()
        .render(bill, language, &builder.fonts, &mut builder.ops);

    // A slip without its QR code is not a QR-bill, so fail instead of
    // silently drawing the rest of it.
    let qr = qr_code(bill).map_err(|_| RenderError::QrCodeGenerationError)?;
    execute_bill_ops(
        &mut builder.content,
        &builder.fonts,
        std::mem::take(&mut builder.ops),
        Some(&qr),
    );

    /// Draw a visual overlay grid 5x5 mm and positions and sizes of blocks.
    #[cfg(feature = "pdf-debug")]
    
    draw_debug_overlay(&mut builder);
    // --- 6. Attach content stream ---
    builder
        .pdf
        .stream(builder.content_id, &builder.content.finish());

    // --- 7. Write PDF -
    Ok(builder.pdf.finish())
}
