use crate::file_writer::FileSystemWriter;
use chrono::NaiveDate;
use std::path::PathBuf;

/// Generates target directory paths based on dates
/// Single Responsibility: Only concerned with path generation logic
pub struct PathGenerator<'a> {
    file_writer: &'a dyn FileSystemWriter,
}

impl<'a> PathGenerator<'a> {
    pub fn new(file_writer: &'a dyn FileSystemWriter) -> Self {
        Self { file_writer }
    }

    /// Generates path in format: YYYY/YYYY-MM-DD
    /// If a directory with the date prefix already exists (e.g., YYYY-MM-DD_event_name),
    /// it will reuse that directory instead of creating a plain YYYY-MM-DD directory
    pub fn generate_path(&self, date: &NaiveDate, filename: &str) -> PathBuf {
        let year = date.format("%Y").to_string();
        let full_date = date.format("%Y-%m-%d").to_string();

        // Check if a directory with this date prefix already exists
        let date_dir = if let Some(existing_dir) = self.file_writer.find_existing_date_directory(
            &PathBuf::from(&year),
            &full_date
        ) {
            existing_dir
        } else {
            full_date
        };

        PathBuf::from(year)
            .join(date_dir)
            .join(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_writer::MockFileSystemWriter;
    use chrono::NaiveDate;

    #[test]
    fn test_generate_path_correct_format() {
        // Arrange
        let mut mock_writer = MockFileSystemWriter::new();
        mock_writer
            .expect_find_existing_date_directory()
            .returning(|_, _| None);
        let generator = PathGenerator::new(&mock_writer);
        let date = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();
        let filename = "IMG_1234.jpg";

        // Act
        let path = generator.generate_path(&date, filename);

        // Assert
        assert_eq!(path, PathBuf::from("2024/2024-01-05/IMG_1234.jpg"));
    }

    #[test]
    fn test_generate_path_different_year() {
        // Arrange
        let mut mock_writer = MockFileSystemWriter::new();
        mock_writer
            .expect_find_existing_date_directory()
            .returning(|_, _| None);
        let generator = PathGenerator::new(&mock_writer);
        let date = NaiveDate::from_ymd_opt(2025, 10, 24).unwrap();
        let filename = "photo.png";

        // Act
        let path = generator.generate_path(&date, filename);

        // Assert
        assert_eq!(path, PathBuf::from("2025/2025-10-24/photo.png"));
    }

    #[test]
    fn test_generate_path_single_digit_month_and_day() {
        // Arrange
        let mut mock_writer = MockFileSystemWriter::new();
        mock_writer
            .expect_find_existing_date_directory()
            .returning(|_, _| None);
        let generator = PathGenerator::new(&mock_writer);
        let date = NaiveDate::from_ymd_opt(2024, 3, 7).unwrap();
        let filename = "test.jpg";

        // Act
        let path = generator.generate_path(&date, filename);

        // Assert
        // Should use zero-padding: 03 instead of 3
        assert_eq!(path, PathBuf::from("2024/2024-03-07/test.jpg"));
    }

    #[test]
    fn test_generate_path_reuses_existing_directory_with_suffix() {
        // Arrange
        let mut mock_writer = MockFileSystemWriter::new();
        mock_writer
            .expect_find_existing_date_directory()
            .withf(|year_path, date_prefix| {
                year_path == &PathBuf::from("2025") && date_prefix == "2025-10-28"
            })
            .returning(|_, _| Some("2025-10-28_special_event".to_string()));
        let generator = PathGenerator::new(&mock_writer);
        let date = NaiveDate::from_ymd_opt(2025, 10, 28).unwrap();
        let filename = "photo.jpg";

        // Act
        let path = generator.generate_path(&date, filename);

        // Assert
        assert_eq!(path, PathBuf::from("2025/2025-10-28_special_event/photo.jpg"));
    }
}
