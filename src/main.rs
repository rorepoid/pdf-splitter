use anyhow::{Context, Result};
use clap::Parser;
use image::imageops::FilterType;
use indicatif::{ProgressBar, ProgressStyle};
use lopdf::Document;
use rayon::prelude::*;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author, version, about = "Splits, compresses and generates WebP images from PDFs.", long_about = None)]
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

    let has_gs = check_ghostscript().is_ok();
    if !has_gs {
        println!("⚠️  Ghostscript not found. Compression and Image generation will be skipped.");
        println!("   To enable, place 'gs' (Linux) or 'gswin32c.exe' (Windows) in this folder.");
    } else {
        println!("✅ Ghostscript detected. Compression and Images enabled.");
    }

    // Process in parallel
    let output_root = PathBuf::from(&args.output_dir);

    pdf_files.par_iter().for_each(|pdf_path| {
        if let Err(e) = process_single_pdf(pdf_path, &output_root, has_gs) {
            eprintln!("Failed to process {:?}: {}", pdf_path, e);
        }
    });

    Ok(())
}

fn check_ghostscript() -> Result<()> {
    let gs_bin = get_gs_binary();
    Command::new(gs_bin)
        .arg("--version")
        .output()
        .context("GS not found")?;
    Ok(())
}

fn get_gs_binary() -> String {
    if cfg!(windows) {
        if Path::new("gswin32c.exe").exists() {
            "gswin32c.exe".to_string()
        } else if Path::new("gswin64c.exe").exists() {
            "gswin64c.exe".to_string()
        } else {
            "gswin32c".to_string()
        }
    } else {
        if Path::new("./gs").exists() {
            "./gs".to_string()
        } else {
            "gs".to_string()
        }
    }
}

fn process_single_pdf(file_path: &Path, output_root: &Path, has_gs: bool) -> Result<()> {
    let filename = file_path.file_stem().unwrap().to_string_lossy();
    // let filename_full = file_path.file_name().unwrap().to_string_lossy();

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

    let output_dir = output_root
        .join(&full_year)
        .join(&month)
        .join(&day)
        .join("lima")
        .join("pages");

    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)?;
    }

    // --- Load PDF ---
    let mut doc = Document::load(file_path).context("Failed to load PDF")?;
    doc.renumber_objects();
    let pages = doc.get_pages();
    let total_pages = pages.len();

    if total_pages == 0 {
        return Ok(());
    }

    let limit = std::cmp::min(total_pages, 30);

    let pb = ProgressBar::new(limit as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} {spinner:.green} [{bar:40.cyan/blue}] {pos}/{len}")?
            .progress_chars("#>-"),
    );
    pb.set_message(format!("{}", filename));

    for i in 1..=limit {
        let p_str = format!("{:0>2}", i);
        let pdf_out_path = output_dir.join(format!("{}.pdf", p_str));
        let pdf_compress_path = output_dir.join(format!("{}_compress.pdf", p_str));
        let img_full_path = output_dir.join(format!("{}.webp", p_str));
        let img_thumb_path = output_dir.join(format!("{}_thumb.webp", p_str));

        // 1. Split
        let mut new_doc = doc.clone();
        let pages_to_delete: Vec<u32> = (1..=total_pages as u32)
            .filter(|&p| p != i as u32)
            .collect();
        new_doc.delete_pages(&pages_to_delete);
        new_doc.prune_objects();
        new_doc.save(&pdf_out_path)?;

        if has_gs {
            // 2. Compress (Ghostscript)
            if let Err(_) = compress_pdf_file(&pdf_out_path, &pdf_compress_path) {
                fs::copy(&pdf_out_path, &pdf_compress_path)?;
            }

            // 3. Generate Images (GS + Rust)
            // Use the COMPRESSED pdf for rendering as it's smaller and faster to read
            if let Err(_e) =
                generate_images_from_pdf(&pdf_compress_path, &img_full_path, &img_thumb_path)
            {
                // eprintln!("Img error: {}", _e);
            }
        } else {
            fs::copy(&pdf_out_path, &pdf_compress_path)?;
        }

        pb.inc(1);
    }
    pb.finish();

    Ok(())
}

fn compress_pdf_file(input: &Path, output: &Path) -> Result<()> {
    let gs_bin = get_gs_binary();
    let status = Command::new(gs_bin)
        .arg("-sDEVICE=pdfwrite")
        .arg("-dCompatibilityLevel=1.4")
        .arg("-dPDFSETTINGS=/ebook")
        .arg("-dNOPAUSE")
        .arg("-dQUIET")
        .arg("-dBATCH")
        .arg(format!("-sOutputFile={}", output.to_string_lossy()))
        .arg(input)
        .status()?;

    if !status.success() {
        anyhow::bail!("GS failed");
    }
    Ok(())
}

fn generate_images_from_pdf(pdf_path: &Path, full_out: &Path, thumb_out: &Path) -> Result<()> {
    let gs_bin = get_gs_binary();
    // Temp PNG file in the same dir
    let temp_png = pdf_path.with_extension("temp.png");

    // A. Render to PNG using GS
    // Optimized: -r100 is faster and sufficient for web
    let status = Command::new(gs_bin)
        .arg("-sDEVICE=png16m")
        .arg("-r100")
        .arg("-dTextAlphaBits=4")
        .arg("-dGraphicsAlphaBits=4")
        .arg("-dNOPAUSE")
        .arg("-dQUIET")
        .arg("-dBATCH")
        .arg(format!("-sOutputFile={}", temp_png.to_string_lossy()))
        .arg(pdf_path)
        .status()?;

    if !status.success() {
        anyhow::bail!("GS image render failed");
    }

    // B. Convert to WebP using Rust
    let img = image::open(&temp_png)?;

    // Save Full WebP
    img.save_with_format(full_out, image::ImageFormat::WebP)?;

    // Save Thumbnail WebP
    // Resize to fixed width 310px, auto height (maintain aspect ratio)
    let thumb = img.resize(310, u32::MAX, FilterType::Lanczos3);
    thumb.save_with_format(thumb_out, image::ImageFormat::WebP)?;
    // Cleanup
    let _ = fs::remove_file(temp_png);

    Ok(())
}
