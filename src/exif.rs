use anyhow::{Context, Result};
use chrono::NaiveDate;
use exif::{In, Tag};

/// Trait for extracting date information from image data
/// Following Dependency Inversion Principle - depend on abstraction
pub trait DateExtractor {
    fn extract_date(&self, image_data: &[u8]) -> Result<NaiveDate>;
}

/// Concrete implementation that extracts dates from EXIF metadata
pub struct ExifDateExtractor;

impl ExifDateExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl DateExtractor for ExifDateExtractor {
    fn extract_date(&self, image_data: &[u8]) -> Result<NaiveDate> {
        let mut cursor = std::io::Cursor::new(image_data);
        let exif_reader = exif::Reader::new();

        let exif_data = exif_reader
            .read_from_container(&mut cursor)
            .context("Failed to read EXIF data from image")?;

        // Try to get DateTimeOriginal (when photo was taken)
        let date_field = exif_data
            .get_field(Tag::DateTimeOriginal, In::PRIMARY)
            .context("No DateTimeOriginal field found in EXIF data")?;

        // EXIF dates are in format: "YYYY:MM:DD HH:MM:SS"
        let date_str = date_field.display_value().to_string();

        // Parse the date portion (first 10 characters: YYYY:MM:DD)
        let date_part = date_str
            .split_whitespace()
            .next()
            .context("Invalid EXIF date format")?;

        // Replace colons with dashes for parsing: YYYY:MM:DD -> YYYY-MM-DD
        let normalized_date = date_part.replace(':', "-");

        NaiveDate::parse_from_str(&normalized_date, "%Y-%m-%d")
            .context("Failed to parse date from EXIF")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AAA Pattern: Arrange, Act, Assert

    #[test]
    fn test_extract_date_from_valid_exif() {
        // Arrange
        let extractor = ExifDateExtractor::new();

        // Real Google Photos image (1x1 pixel) with DateTimeOriginal: 2012:10:06 13:09:32
        let sample_image_data: &[u8] = include_bytes!("../tests/fixtures/single_pixel_with_exif.jpg");

        // Act
        let result = extractor.extract_date(sample_image_data);

        // Assert
        assert!(result.is_ok(), "Failed to extract date: {:?}", result.err());
        let date = result.unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2012, 10, 6).unwrap());
    }

    #[test]
    fn test_extract_date_missing_exif_returns_error() {
        // Arrange
        let extractor = ExifDateExtractor::new();
        let invalid_data: &[u8] = &[0, 1, 2, 3]; // Not a valid image

        // Act
        let result = extractor.extract_date(invalid_data);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_date_empty_data_returns_error() {
        // Arrange
        let extractor = ExifDateExtractor::new();
        let empty_data: &[u8] = &[];

        // Act
        let result = extractor.extract_date(empty_data);

        // Assert
        assert!(result.is_err());
    }
}
