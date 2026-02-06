use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use lopdf::Document;
use rayon::prelude::*;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author, version, about = "Splits and compresses PDFs using pure Rust.", long_about = None)]
struct Args {
    /// Input path (file or directory)
    #[arg(short, long)]
    path: String,

    /// Output root directory
    #[arg(short, long, default_value = "output")]
    output_dir: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // File Discovery
    // We treat the user provided path as the source of truth.
    // It can be relative to CWD or absolute.
    let search_path = PathBuf::from(&args.path);

    if !search_path.exists() {
        anyhow::bail!("Error: Input path not found: {:?}", search_path);
    }

    let mut pdf_files: Vec<PathBuf> = Vec::new();
    if search_path.is_dir() {
        for entry in WalkDir::new(&search_path)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().extension().map_or(false, |ext| ext == "pdf") {
                pdf_files.push(entry.path().to_path_buf());
            }
        }
    } else {
        pdf_files.push(search_path.clone());
    }

    if pdf_files.is_empty() {
        println!("No PDF files found in {:?}", search_path);
        return Ok(());
    }

    println!("Found {} files to process.", pdf_files.len());
    println!("Output directory: {}", args.output_dir);

    // Process in parallel
    let output_root = PathBuf::from(&args.output_dir);

    pdf_files.par_iter().for_each(|pdf_path| {
        if let Err(e) = process_single_pdf(pdf_path, &output_root) {
            eprintln!("Failed to process {:?}: {}", pdf_path, e);
        }
    });

    Ok(())
}

fn process_single_pdf(file_path: &Path, output_root: &Path) -> Result<()> {
    let filename = file_path.file_stem().unwrap().to_string_lossy();
    let filename_full = file_path.file_name().unwrap().to_string_lossy();

    // --- Date Logic ---
    let now = chrono::Local::now();
    let mut day = now.format("%d").to_string();
    let mut month = now.format("%m").to_string();
    let mut year_suffix = now.format("%y").to_string();

    let re_compact = Regex::new(r"REPLIM(\d{2})(\d{2})(\d{2})").unwrap();
    let re_slash = Regex::new(r"REPLIM\s*(\d+)/(\d+)").unwrap();

    if let Some(caps) = re_compact.captures(&filename) {
        day = caps[1].to_string();
        month = caps[2].to_string();
        year_suffix = caps[3].to_string();
    } else if let Some(caps) = re_slash.captures(&filename) {
        day = format!("{:0>2}", &caps[1]);
        month = format!("{:0>2}", &caps[2]);
    }

    let year_val: i32 = year_suffix.parse().unwrap_or(0);
    let full_year = if year_val > 60 {
        format!("19{}", year_suffix)
    } else {
        format!("20{}", year_suffix)
    };

    // Construct dynamic output path
    let output_dir = output_root
        .join(&full_year)
        .join(&month)
        .join(&day)
        .join("lima")
        .join("pages");

    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)?;
    }

    println!("Processing: {}", filename_full);

    // --- Load PDF ---
    let mut doc = Document::load(file_path).context("Failed to load PDF")?;

    // Initial cleanup
    doc.renumber_objects();

    // Get pages map: { page_number (1-based) -> object_id }
    let pages = doc.get_pages();
    let total_pages = pages.len();

    if total_pages == 0 {
        return Ok(());
    }

    let limit = std::cmp::min(total_pages, 30);
    println!(
        "  > Pages: {} (Limit {}) -> {}",
        total_pages,
        limit,
        output_dir.display()
    );

    let pb = ProgressBar::new(limit as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} {spinner:.green} [{bar:40.cyan/blue}] {pos}/{len}")?
            .progress_chars("#>-"),
    );
    pb.set_message(format!("{}", filename));

    // Sequential page extraction per file
    for i in 1..=limit {
        let p_str = format!("{:0>2}", i);
        let pdf_out_path = output_dir.join(format!("{}.pdf", p_str));
        let pdf_compress_path = output_dir.join(format!("{}_compress.pdf", p_str));

        let mut new_doc = doc.clone();

        // Calculate which pages to DELETE (Keep only 'i')
        let pages_to_delete: Vec<u32> = (1..=total_pages as u32)
            .filter(|&p| p != i as u32)
            .collect();

        new_doc.delete_pages(&pages_to_delete);
        new_doc.prune_objects();

        new_doc.save(&pdf_out_path)?;

        // Copy for compressed version
        fs::copy(&pdf_out_path, &pdf_compress_path)?;

        pb.inc(1);
    }
    pb.finish();

    Ok(())
}
