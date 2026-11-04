/*
 * Copyright (c) 2026 Thomas Prosser
 * Licensed under MIT License
 * https://opensource.org/licenses/MIT
 */

use std::fs;
use clap::Parser;
use anyhow::{Result};
use swiss_qrust::{BillData};
use swiss_qrust::pdf::create_pdf;

#[derive(Parser)]
#[command(name = "swiss_qrust")]
#[command(about = "Swiss QR Bill generator CLI", long_about = None)]
struct Cli {
    /// Input file (TOML or JSON)
    #[arg(short, long)]
    input: String,

    /// Output PDF file
    #[arg(short, long)]
    output: String,

    /// Language for PDF (e.g., "de", "fr", "it", "rm")
    #[arg(short, long, default_value = "de")]
    lang: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // --- 1. Load input file ---
    let content = fs::read_to_string(&cli.input)?;
    let ext = cli.input.split('.').last().unwrap_or("");
    let input_bill = swiss_qrust::parse_bill_data(&content, ext)?;

    // --- 2. Convert to internal BillData ---
    let bill_data: BillData = input_bill.try_into()?; // your TryFrom impl

    // --- 3. Convert language ---
    let language = match cli.lang.as_deref() {
        Some("de") => swiss_qrust::Language::De,
        Some("fr") => swiss_qrust::Language::Fr,
        Some("it") => swiss_qrust::Language::It,
        Some("rm") => swiss_qrust::Language::Rm,
        _ => swiss_qrust::Language::En,
    };

    // --- 4. Generate PDF ---
    create_pdf(&cli.output, language, &bill_data)?;

    println!("PDF successfully written to {}", &cli.output);
    Ok(())
}