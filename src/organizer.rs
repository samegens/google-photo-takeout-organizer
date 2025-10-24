use anyhow::{Context, Result};
use crate::exif::DateExtractor;
use crate::file_writer::FileSystemWriter;
use crate::path_generator::PathGenerator;
use crate::zip_reader::{ZipEntry, ZipReader};

/// Main orchestrator service that coordinates photo organization
pub struct PhotoOrganizer<'a> {
    zip_reader: &'a dyn ZipReader,
    date_extractor: &'a dyn DateExtractor,
    path_generator: &'a PathGenerator,
    file_writer: &'a dyn FileSystemWriter,
}

impl<'a> PhotoOrganizer<'a> {
    pub fn new(
        zip_reader: &'a dyn ZipReader,
        date_extractor: &'a dyn DateExtractor,
        path_generator: &'a PathGenerator,
        file_writer: &'a dyn FileSystemWriter,
    ) -> Self {
        Self {
            zip_reader,
            date_extractor,
            path_generator,
            file_writer,
        }
    }

    /// Organize photos from ZIP archive into date-based directory structure
    pub fn organize(&self) -> Result<OrganizeResult> {
        let entries = self.zip_reader.read_entries()
            .context("Failed to read ZIP entries")?;

        let total_files = entries.len();
        let mut organized_files = 0;
        let mut skipped_files = 0;
        let mut errors = Vec::new();

        for entry in entries {
            match self.process_entry(&entry) {
                Ok(_) => organized_files += 1,
                Err(e) => {
                    skipped_files += 1;
                    errors.push(format!("{}: {}", entry.name, e));
                }
            }
        }

        Ok(OrganizeResult {
            total_files,
            organized_files,
            skipped_files,
            errors,
        })
    }

    fn process_entry(&self, entry: &ZipEntry) -> Result<()> {
        // Extract date from EXIF
        let date = self.date_extractor.extract_date(&entry.data)
            .context("Failed to extract date from EXIF")?;

        // Generate target path
        let target_path = self.path_generator.generate_path(&date, &entry.name);

        // Create parent directory
        if let Some(parent) = target_path.parent() {
            self.file_writer.create_directory(parent)
                .context("Failed to create directory")?;
        }

        // Write file
        self.file_writer.write_file(&target_path, &entry.data)
            .context("Failed to write file")?;

        Ok(())
    }
}

/// Result of organization operation
#[derive(Debug, PartialEq)]
pub struct OrganizeResult {
    pub total_files: usize,
    pub organized_files: usize,
    pub skipped_files: usize,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exif::ExifDateExtractor;
    use crate::file_writer::RealFileSystemWriter;
    use crate::path_generator::PathGenerator;
    use crate::zip_reader::FileZipReader;
    use chrono::NaiveDate;
    use std::fs;
    use std::path::PathBuf;
    use zip::write::{FileOptions, ZipWriter};

    // Mock implementations for testing
    struct MockZipReader {
        entries: Vec<ZipEntry>,
    }

    impl ZipReader for MockZipReader {
        fn read_entries(&self) -> Result<Vec<ZipEntry>> {
            Ok(self.entries.clone())
        }
    }

    struct MockDateExtractor {
        date: NaiveDate,
    }

    impl DateExtractor for MockDateExtractor {
        fn extract_date(&self, _image_data: &[u8]) -> Result<NaiveDate> {
            Ok(self.date)
        }
    }

    #[test]
    fn test_organize_empty_zip() {
        // Arrange
        let temp_dir = "/tmp/test_org_empty";
        let zip_reader = MockZipReader { entries: vec![] };
        let date_extractor = ExifDateExtractor::new();
        let path_generator = PathGenerator::new();
        let file_writer = RealFileSystemWriter::new(temp_dir.to_string());

        let organizer = PhotoOrganizer::new(
            &zip_reader,
            &date_extractor,
            &path_generator,
            &file_writer,
        );

        // Act
        let result = organizer.organize();

        // Assert
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.organized_files, 0);

        // Cleanup
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_organize_single_photo() {
        // Arrange
        let temp_dir = "/tmp/test_org_single";
        let test_image = include_bytes!("../tests/fixtures/single_pixel_with_exif.jpg");

        let zip_reader = MockZipReader {
            entries: vec![ZipEntry {
                name: "photo1.jpg".to_string(),
                data: test_image.to_vec(),
            }],
        };
        let date_extractor = ExifDateExtractor::new();
        let path_generator = PathGenerator::new();
        let file_writer = RealFileSystemWriter::new(temp_dir.to_string());

        let organizer = PhotoOrganizer::new(
            &zip_reader,
            &date_extractor,
            &path_generator,
            &file_writer,
        );

        // Act
        let result = organizer.organize();

        // Assert
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.total_files, 1);
        assert_eq!(stats.organized_files, 1);

        // Verify file was written to correct location (2012-10-06 from EXIF)
        let expected_path = PathBuf::from(temp_dir)
            .join("2012")
            .join("2012-10-06")
            .join("photo1.jpg");
        assert!(expected_path.exists());

        // Cleanup
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_organize_multiple_photos_same_date() {
        // Arrange
        let temp_dir = "/tmp/test_org_multiple_same";
        let test_image = include_bytes!("../tests/fixtures/single_pixel_with_exif.jpg");

        let zip_reader = MockZipReader {
            entries: vec![
                ZipEntry {
                    name: "photo1.jpg".to_string(),
                    data: test_image.to_vec(),
                },
                ZipEntry {
                    name: "photo2.jpg".to_string(),
                    data: test_image.to_vec(),
                },
            ],
        };
        let date_extractor = ExifDateExtractor::new();
        let path_generator = PathGenerator::new();
        let file_writer = RealFileSystemWriter::new(temp_dir.to_string());

        let organizer = PhotoOrganizer::new(
            &zip_reader,
            &date_extractor,
            &path_generator,
            &file_writer,
        );

        // Act
        let result = organizer.organize();

        // Assert
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.organized_files, 2);

        // Both files should be in same directory
        let dir_path = PathBuf::from(temp_dir).join("2012").join("2012-10-06");
        assert!(dir_path.join("photo1.jpg").exists());
        assert!(dir_path.join("photo2.jpg").exists());

        // Cleanup
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_organize_photos_different_dates() {
        // Arrange
        let temp_dir = "/tmp/test_org_diff_dates";
        let test_image = include_bytes!("../tests/fixtures/single_pixel_with_exif.jpg");

        let zip_reader = MockZipReader {
            entries: vec![
                ZipEntry {
                    name: "photo_oct.jpg".to_string(),
                    data: test_image.to_vec(),
                },
            ],
        };
        let date_extractor = ExifDateExtractor::new();
        let path_generator = PathGenerator::new();
        let file_writer = RealFileSystemWriter::new(temp_dir.to_string());

        let organizer = PhotoOrganizer::new(
            &zip_reader,
            &date_extractor,
            &path_generator,
            &file_writer,
        );

        // Act
        let result = organizer.organize();

        // Assert
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.total_files, 1);
        assert_eq!(stats.organized_files, 1);

        // Cleanup
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_organize_file_without_exif_skipped() {
        // Arrange
        let temp_dir = "/tmp/test_org_no_exif";

        let zip_reader = MockZipReader {
            entries: vec![ZipEntry {
                name: "no_exif.jpg".to_string(),
                data: vec![0xFF, 0xD8, 0xFF, 0xD9], // Minimal JPEG without EXIF
            }],
        };
        let date_extractor = ExifDateExtractor::new();
        let path_generator = PathGenerator::new();
        let file_writer = RealFileSystemWriter::new(temp_dir.to_string());

        let organizer = PhotoOrganizer::new(
            &zip_reader,
            &date_extractor,
            &path_generator,
            &file_writer,
        );

        // Act
        let result = organizer.organize();

        // Assert
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.total_files, 1);
        assert_eq!(stats.organized_files, 0);
        assert_eq!(stats.skipped_files, 1);
        assert!(stats.errors.len() > 0);

        // Cleanup
        fs::remove_dir_all(temp_dir).ok();
    }
}
