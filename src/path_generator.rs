use chrono::NaiveDate;
use std::path::PathBuf;

/// Generates target directory paths based on dates
/// Single Responsibility: Only concerned with path generation logic
pub struct PathGenerator;

impl PathGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generates path in format: YYYY/YYYY-MM-DD
    pub fn generate_path(&self, date: &NaiveDate, filename: &str) -> PathBuf {
        let year = date.format("%Y").to_string();
        let full_date = date.format("%Y-%m-%d").to_string();

        PathBuf::from(year)
            .join(full_date)
            .join(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_generate_path_correct_format() {
        // Arrange
        let generator = PathGenerator::new();
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
        let generator = PathGenerator::new();
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
        let generator = PathGenerator::new();
        let date = NaiveDate::from_ymd_opt(2024, 3, 7).unwrap();
        let filename = "test.jpg";

        // Act
        let path = generator.generate_path(&date, filename);

        // Assert
        // Should use zero-padding: 03 instead of 3
        assert_eq!(path, PathBuf::from("2024/2024-03-07/test.jpg"));
    }
}
