mod exif;
mod file_writer;
mod organizer;
mod path_generator;
mod photo_filter;
mod zip_image_reader;

use clap::Parser;
use exif::CompositeDateExtractor;
use file_writer::RealFileSystemWriter;
use organizer::PhotoOrganizer;
use path_generator::PathGenerator;
use photo_filter::{ExistingCollectionFilter, NoFilter};
use zip_image_reader::{FileZipImageReader, ZipImageReader};

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
    display_configuration(&args);

    let result = organize_photos_from_zip(&args);

    display_results_and_exit(result);
}

fn display_configuration(args: &Args) {
    println!("Organizing photos from: {}", args.input);
    println!("Output directory: {}", args.output);
    display_filter_status(args.no_filter);
    println!();
}

fn display_filter_status(filtering_disabled: bool) {
    if filtering_disabled {
        println!("Filtering: Disabled (organizing all photos)");
    } else {
        println!("Filtering: Skipping existing collection photos (DSLR, Lightroom, Google -MIX/-edited files)");
    }
}

fn organize_photos_from_zip(args: &Args) -> Result<organizer::OrganizeResult, anyhow::Error> {
    let zip_reader = FileZipImageReader::new(args.input.clone());
    let date_extractor = CompositeDateExtractor::new();
    let path_generator = PathGenerator::new();
    let file_writer = RealFileSystemWriter::new(args.output.clone());

    let all_filenames = collect_filenames_from_zip(&zip_reader)?;
    let existing_collection_filter = ExistingCollectionFilter::new(all_filenames);
    let no_filter = NoFilter::new();

    let filter: &dyn photo_filter::PhotoFilter = if args.no_filter {
        &no_filter
    } else {
        &existing_collection_filter
    };

    let organizer = PhotoOrganizer::new(
        &zip_reader,
        &date_extractor,
        &path_generator,
        &file_writer,
        filter,
    );

    organizer.organize()
}

fn collect_filenames_from_zip(zip_reader: &FileZipImageReader) -> Result<Vec<String>, anyhow::Error> {
    let entries = zip_reader.read_entries()?;
    Ok(entries.into_iter().map(|entry| entry.name).collect())
}

fn display_results_and_exit(result: Result<organizer::OrganizeResult, anyhow::Error>) -> ! {
    match result {
        Ok(result) => {
            display_success_summary(&result);
            std::process::exit(0);
        }
        Err(e) => {
            display_failure_message(&e);
            std::process::exit(1);
        }
    }
}

fn display_success_summary(result: &organizer::OrganizeResult) {
    println!("✓ Organization complete!");
    println!("  Total files: {}", result.total_files);
    println!("  Organized: {}", result.organized_files);
    println!("  Skipped: {}", result.skipped_files);

    display_errors_if_any(&result.errors);
}

fn display_errors_if_any(errors: &[String]) {
    if !errors.is_empty() {
        println!("\nErrors:");
        for error in errors {
            println!("  - {}", error);
        }
    }
}

fn display_failure_message(error: &anyhow::Error) {
    eprintln!("✗ Failed to organize photos: {}", error);
}
