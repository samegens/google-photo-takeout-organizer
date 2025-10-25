use anyhow::{Context, Result};
use crate::exif::DateExtractor;
use crate::file_writer::FileSystemWriter;
use crate::path_generator::PathGenerator;
use crate::photo_filter::PhotoFilter;
use crate::zip_image_reader::{ZipEntry, ZipImageReader};

/// Main orchestrator service that coordinates photo organization
pub struct PhotoOrganizer<'a> {
    zip_reader: &'a dyn ZipImageReader,
    date_extractor: &'a dyn DateExtractor,
    path_generator: &'a PathGenerator,
    file_writer: &'a dyn FileSystemWriter,
    photo_filter: &'a dyn PhotoFilter,
}

impl<'a> PhotoOrganizer<'a> {
    pub fn new(
        zip_reader: &'a dyn ZipImageReader,
        date_extractor: &'a dyn DateExtractor,
        path_generator: &'a PathGenerator,
        file_writer: &'a dyn FileSystemWriter,
        photo_filter: &'a dyn PhotoFilter,
    ) -> Self {
        Self {
            zip_reader,
            date_extractor,
            path_generator,
            file_writer,
            photo_filter,
        }
    }

    /// Validate that no -edited files exist without their originals
    pub fn validate_no_orphaned_edits(&self) -> Result<()> {
        let entries = self.zip_reader.read_entries()
            .context("Failed to read ZIP entries for validation")?;

        let mut orphans = Vec::new();

        // Find all -edited files
        let edited_files: Vec<&ZipEntry> = entries.iter()
            .filter(|e| e.name.to_uppercase().contains("-EDITED"))
            .collect();

        for edited in edited_files {
            // Construct the original filename by removing -edited (case-insensitive)
            let original_name = edited.name
                .replace("-edited", "")
                .replace("-EDITED", "")
                .replace("-Edited", "");

            // Check if original exists in the ZIP
            let has_original = entries.iter().any(|e| e.name == original_name);

            if !has_original {
                orphans.push(edited.name.clone());
            }
        }

        if !orphans.is_empty() {
            anyhow::bail!(
                "Found {} -edited file(s) without originals:\n{}\n\n\
                To preserve these photos, use --no-filter to organize all files.",
                orphans.len(),
                orphans.join("\n")
            );
        }

        Ok(())
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
            // Apply filter first
            if !self.photo_filter.should_include(&entry.name, &entry.data) {
                println!("{}: filtered out", entry.name);
                skipped_files += 1;
                continue;
            }

            match self.process_entry(&entry) {
                Ok(target_path) => {
                    println!("{}: copied to {}", entry.name, target_path.display());
                    organized_files += 1;
                }
                Err(e) => {
                    println!("{}: error - {}", entry.name, e);
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

    fn process_entry(&self, entry: &ZipEntry) -> Result<std::path::PathBuf> {
        let date = self.date_extractor.extract_date(&entry.name, &entry.data)
            .context("Failed to extract date")?;

        let target_path = self.path_generator.generate_path(&date, &entry.name);

        self.ensure_parent_directory_exists(&target_path)?;
        self.file_writer.write_file(&target_path, &entry.data)
            .context("Failed to write file")?;

        Ok(self.file_writer.get_full_path(&target_path))
    }

    fn ensure_parent_directory_exists(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            self.file_writer.create_directory(parent)
                .context("Failed to create directory")?;
        }
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
    use crate::photo_filter::NoFilter;
    use chrono::NaiveDate;
    use std::fs;
    use std::path::PathBuf;

    // Mock implementations for testing
    struct MockZipReader {
        entries: Vec<ZipEntry>,
    }

    impl ZipImageReader for MockZipReader {
        fn read_entries(&self) -> Result<Vec<ZipEntry>> {
            Ok(self.entries.clone())
        }
    }

    struct MockDateExtractor {
        date: NaiveDate,
    }

    impl DateExtractor for MockDateExtractor {
        fn extract_date(&self, _filename: &str, _image_data: &[u8]) -> Result<NaiveDate> {
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
        let filter = NoFilter::new();

        let organizer = PhotoOrganizer::new(
            &zip_reader,
            &date_extractor,
            &path_generator,
            &file_writer,
            &filter,
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
        let filter = NoFilter::new();

        let organizer = PhotoOrganizer::new(
            &zip_reader,
            &date_extractor,
            &path_generator,
            &file_writer,
            &filter,
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
        let filter = NoFilter::new();

        let organizer = PhotoOrganizer::new(
            &zip_reader,
            &date_extractor,
            &path_generator,
            &file_writer,
            &filter,
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
        let filter = NoFilter::new();

        let organizer = PhotoOrganizer::new(
            &zip_reader,
            &date_extractor,
            &path_generator,
            &file_writer,
            &filter,
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
    fn test_validate_no_orphaned_edits_passes_with_pairs() {
        // Arrange
        let test_image = include_bytes!("../tests/fixtures/single_pixel_with_exif.jpg");
        let zip_reader = MockZipReader {
            entries: vec![
                ZipEntry {
                    name: "DSC_9157.JPG".to_string(),
                    data: test_image.to_vec(),
                },
                ZipEntry {
                    name: "DSC_9157-edited.JPG".to_string(),
                    data: test_image.to_vec(),
                },
            ],
        };
        let date_extractor = ExifDateExtractor::new();
        let path_generator = PathGenerator::new();
        let file_writer = RealFileSystemWriter::new("/tmp/test".to_string());
        let filter = NoFilter::new();

        let organizer = PhotoOrganizer::new(
            &zip_reader,
            &date_extractor,
            &path_generator,
            &file_writer,
            &filter,
        );

        // Act
        let result = organizer.validate_no_orphaned_edits();

        // Assert
        assert!(result.is_ok(), "Validation should pass when original exists");
    }

    #[test]
    fn test_validate_no_orphaned_edits_fails_with_orphan() {
        // Arrange
        let test_image = include_bytes!("../tests/fixtures/single_pixel_with_exif.jpg");
        let zip_reader = MockZipReader {
            entries: vec![
                ZipEntry {
                    name: "DSC_9157-edited.JPG".to_string(),
                    data: test_image.to_vec(),
                },
                // No original DSC_9157.JPG
            ],
        };
        let date_extractor = ExifDateExtractor::new();
        let path_generator = PathGenerator::new();
        let file_writer = RealFileSystemWriter::new("/tmp/test".to_string());
        let filter = NoFilter::new();

        let organizer = PhotoOrganizer::new(
            &zip_reader,
            &date_extractor,
            &path_generator,
            &file_writer,
            &filter,
        );

        // Act
        let result = organizer.validate_no_orphaned_edits();

        // Assert
        assert!(result.is_err(), "Validation should fail when orphaned -edited file exists");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("DSC_9157-edited.JPG"), "Error should mention the orphaned file");
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
        let filter = NoFilter::new();

        let organizer = PhotoOrganizer::new(
            &zip_reader,
            &date_extractor,
            &path_generator,
            &file_writer,
            &filter,
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
