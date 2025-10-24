mod exif;
mod file_writer;
mod organizer;
mod path_generator;
mod zip_reader;

use clap::Parser;
use exif::ExifDateExtractor;
use file_writer::RealFileSystemWriter;
use organizer::PhotoOrganizer;
use path_generator::PathGenerator;
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
}

fn main() {
    let args = Args::parse();

    println!("Organizing photos from: {}", args.input);
    println!("Output directory: {}", args.output);
    println!();

    // Create components
    let zip_reader = FileZipReader::new(args.input);
    let date_extractor = ExifDateExtractor::new();
    let path_generator = PathGenerator::new();
    let file_writer = RealFileSystemWriter::new(args.output);

    // Create organizer
    let organizer = PhotoOrganizer::new(
        &zip_reader,
        &date_extractor,
        &path_generator,
        &file_writer,
    );

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
