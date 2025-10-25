mod exif;
mod file_writer;
mod organizer;
mod path_generator;
mod photo_filter;
mod zip_reader;

use clap::Parser;
use exif::ExifDateExtractor;
use file_writer::RealFileSystemWriter;
use organizer::PhotoOrganizer;
use path_generator::PathGenerator;
use photo_filter::{ExistingCollectionFilter, NoFilter};
use zip_reader::FileZipReader;

/// Organize Google Photos ZIP exports into date-based directory structure

#[derive(Parser, Debug)]
#[command(name = "organize-photo-zip")]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the Google Photos ZIP file
    #[arg(short, long)]
    input: String,

    /// Output directory for organized photos
    #[arg(short, long, default_value = "./organized_photos")]
    output: String,

    /// Disable filtering (by default, DSLR/Lightroom/Google -MIX/-edited files are skipped)
    #[arg(short, long)]
    no_filter: bool,
}

fn main() {
    let args = Args::parse();

    println!("Organizing photos from: {}", args.input);
    println!("Output directory: {}", args.output);
    if args.no_filter {
        println!("Filtering: Disabled (organizing all photos)");
    } else {
        println!("Filtering: Skipping existing collection photos (DSLR, Lightroom, Google -MIX/-edited files)");
    }
    println!();

    // Create components
    let zip_reader = FileZipReader::new(args.input);
    let date_extractor = ExifDateExtractor::new();
    let path_generator = PathGenerator::new();
    let file_writer = RealFileSystemWriter::new(args.output);
    let existing_collection_filter = ExistingCollectionFilter::new();
    let no_filter = NoFilter::new();

    // Choose filter based on CLI flag
    // By default (no_filter = false), skip existing collection files
    let filter: &dyn photo_filter::PhotoFilter = if args.no_filter {
        &no_filter // Don't filter anything
    } else {
        &existing_collection_filter // Skip existing collection files (default)
    };

    // Create organizer
    let organizer = PhotoOrganizer::new(
        &zip_reader,
        &date_extractor,
        &path_generator,
        &file_writer,
        filter,
    );

    // Validate ZIP contents before organizing
    if !args.no_filter {
        if let Err(e) = organizer.validate_no_orphaned_edits() {
            eprintln!("✗ Validation failed: {}", e);
            std::process::exit(1);
        }
    }

    // Organize photos
    match organizer.organize() {
        Ok(result) => {
            println!("✓ Organization complete!");
            println!("  Total files: {}", result.total_files);
            println!("  Organized: {}", result.organized_files);
            println!("  Skipped: {}", result.skipped_files);

            if !result.errors.is_empty() {
                println!("\nErrors:");
                for error in &result.errors {
                    println!("  - {}", error);
                }
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to organize photos: {}", e);
            std::process::exit(1);
        }
    }
}
