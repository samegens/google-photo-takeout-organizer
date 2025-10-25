use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use zip::write::{FileOptions, ZipWriter};

// Import the library modules
use organize_photo_zip::exif::ExifDateExtractor;
use organize_photo_zip::file_writer::RealFileSystemWriter;
use organize_photo_zip::organizer::PhotoOrganizer;
use organize_photo_zip::path_generator::PathGenerator;
use organize_photo_zip::photo_filter::NoFilter;
use organize_photo_zip::zip_reader::FileZipReader;

#[test]
fn test_end_to_end_photo_organization() {
    // Arrange: Create a test ZIP file with our sample image
    let test_zip_path = "/tmp/integration_test_photos.zip";
    let output_dir = "/tmp/integration_test_output";

    // Clean up any previous test artifacts
    fs::remove_file(test_zip_path).ok();
    fs::remove_dir_all(output_dir).ok();

    // Load the test image with EXIF data
    let test_image = include_bytes!("fixtures/single_pixel_with_exif.jpg");

    // Create a ZIP file with the test image
    let file = File::create(test_zip_path).expect("Failed to create test ZIP");
    let mut zip = ZipWriter::new(file);
    let options: FileOptions<()> = FileOptions::default();

    zip.start_file("photo1.jpg", options)
        .expect("Failed to start file");
    zip.write_all(test_image).expect("Failed to write image");

    zip.start_file("photo2.jpg", options)
        .expect("Failed to start file");
    zip.write_all(test_image).expect("Failed to write image");

    zip.finish().expect("Failed to finish ZIP");

    // Act: Run the full organization workflow
    let zip_reader = FileZipReader::new(test_zip_path.to_string());
    let date_extractor = ExifDateExtractor::new();
    let path_generator = PathGenerator::new();
    let file_writer = RealFileSystemWriter::new(output_dir.to_string());
    let filter = NoFilter::new();

    let organizer = PhotoOrganizer::new(
        &zip_reader,
        &date_extractor,
        &path_generator,
        &file_writer,
        &filter,
    );

    let result = organizer.organize().expect("Organization failed");

    // Assert: Verify results
    assert_eq!(result.total_files, 2);
    assert_eq!(result.organized_files, 2);
    assert_eq!(result.skipped_files, 0);
    assert!(result.errors.is_empty());

    // Verify directory structure
    let expected_dir = PathBuf::from(output_dir).join("2012").join("2012-10-06");
    assert!(expected_dir.exists(), "Expected directory not created");

    // Verify files exist
    assert!(expected_dir.join("photo1.jpg").exists());
    assert!(expected_dir.join("photo2.jpg").exists());

    // Verify file contents
    let written_data = fs::read(expected_dir.join("photo1.jpg"))
        .expect("Failed to read written file");
    assert_eq!(written_data, test_image);

    // Cleanup
    fs::remove_file(test_zip_path).ok();
    fs::remove_dir_all(output_dir).ok();

    println!("✓ End-to-end integration test passed!");
}
