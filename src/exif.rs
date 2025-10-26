use anyhow::{Context, Result};
use chrono::NaiveDate;
use exif::{In, Tag};

/// Trait for extracting date information from image data
pub trait DateExtractor {
    fn extract_date(&self, filename: &str, image_data: &[u8]) -> Result<NaiveDate>;
}

/// Concrete implementation that extracts dates from EXIF metadata
pub struct ExifDateExtractor;

impl ExifDateExtractor {
    pub fn new() -> Self {
        Self
    }

    fn read_exif_from_image(image_data: &[u8]) -> Result<exif::Exif> {
        let mut cursor = std::io::Cursor::new(image_data);
        let exif_reader = exif::Reader::new();
        exif_reader
            .read_from_container(&mut cursor)
            .context("Failed to read EXIF data from image")
    }

    fn get_datetime_original_field(exif_data: &exif::Exif) -> Result<&exif::Field> {
        exif_data
            .get_field(Tag::DateTimeOriginal, In::PRIMARY)
            .context("No DateTimeOriginal field found in EXIF data")
    }

    fn parse_exif_date_string(exif_date_string: &str) -> Result<NaiveDate> {
        let date_part = exif_date_string
            .split_whitespace()
            .next()
            .context("Invalid EXIF date format")?;

        let normalized_date = date_part.replace(':', "-");

        NaiveDate::parse_from_str(&normalized_date, "%Y-%m-%d")
            .context("Failed to parse date from EXIF")
    }
}

impl DateExtractor for ExifDateExtractor {
    fn extract_date(&self, _filename: &str, image_data: &[u8]) -> Result<NaiveDate> {
        let exif_data = Self::read_exif_from_image(image_data)?;
        let datetime_original_field = Self::get_datetime_original_field(&exif_data)?;
        let date_string = datetime_original_field.display_value().to_string();
        Self::parse_exif_date_string(&date_string)
    }
}

/// Extracts dates from filename patterns
pub struct FilenameBasedDateExtractor;

impl FilenameBasedDateExtractor {
    pub fn new() -> Self {
        Self
    }

    fn try_parse_patterns(filename: &str) -> Option<NaiveDate> {
        Self::try_parse_date_with_dashes(filename)
            .or_else(|| Self::try_parse_compact_datetime_pattern(filename))
            .or_else(|| Self::try_parse_img_underscore_pattern(filename))
            .or_else(|| Self::try_parse_img_dash_pattern(filename))
    }

    fn try_parse_date_with_dashes(filename: &str) -> Option<NaiveDate> {
        let pattern = regex::Regex::new(r"(\d{4})-(\d{2})-(\d{2})").ok()?;
        let captures = pattern.captures(filename)?;

        let year: i32 = captures.get(1)?.as_str().parse().ok()?;
        let month: u32 = captures.get(2)?.as_str().parse().ok()?;
        let day: u32 = captures.get(3)?.as_str().parse().ok()?;

        NaiveDate::from_ymd_opt(year, month, day)
    }

    fn try_parse_compact_datetime_pattern(filename: &str) -> Option<NaiveDate> {
        let pattern = regex::Regex::new(r"(\d{8})_\d{6}").ok()?;
        let captures = pattern.captures(filename)?;
        let date_str = captures.get(1)?.as_str();
        NaiveDate::parse_from_str(date_str, "%Y%m%d").ok()
    }

    fn try_parse_img_underscore_pattern(filename: &str) -> Option<NaiveDate> {
        let pattern = regex::Regex::new(r"IMG_(\d{8})_\d{6}").ok()?;
        let captures = pattern.captures(filename)?;
        let date_str = captures.get(1)?.as_str();
        NaiveDate::parse_from_str(date_str, "%Y%m%d").ok()
    }

    fn try_parse_img_dash_pattern(filename: &str) -> Option<NaiveDate> {
        let pattern = regex::Regex::new(r"IMG-(\d{8})").ok()?;
        let captures = pattern.captures(filename)?;
        let date_str = captures.get(1)?.as_str();
        NaiveDate::parse_from_str(date_str, "%Y%m%d").ok()
    }
}

impl DateExtractor for FilenameBasedDateExtractor {
    fn extract_date(&self, filename: &str, _image_data: &[u8]) -> Result<NaiveDate> {
        Self::try_parse_patterns(filename)
            .context("Failed to extract date from filename")
    }
}

/// Composite extractor that tries EXIF first, then falls back to filename
pub struct CompositeDateExtractor {
    exif_extractor: ExifDateExtractor,
    filename_extractor: FilenameBasedDateExtractor,
}

impl CompositeDateExtractor {
    pub fn new() -> Self {
        Self {
            exif_extractor: ExifDateExtractor::new(),
            filename_extractor: FilenameBasedDateExtractor::new(),
        }
    }
}

impl DateExtractor for CompositeDateExtractor {
    fn extract_date(&self, filename: &str, image_data: &[u8]) -> Result<NaiveDate> {
        self.exif_extractor
            .extract_date(filename, image_data)
            .or_else(|_| self.filename_extractor.extract_date(filename, image_data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_date_from_valid_exif() {
        // Arrange
        let extractor = ExifDateExtractor::new();

        // Real Google Photos image (1x1 pixel) with DateTimeOriginal: 2012:10:06 13:09:32
        let sample_image_data: &[u8] = include_bytes!("../tests/fixtures/single_pixel_with_exif.jpg");

        // Act
        let result = extractor.extract_date("photo.jpg", sample_image_data);

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
        let result = extractor.extract_date("photo.jpg", invalid_data);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_date_empty_data_returns_error() {
        // Arrange
        let extractor = ExifDateExtractor::new();
        let empty_data: &[u8] = &[];

        // Act
        let result = extractor.extract_date("photo.jpg", empty_data);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_filename_extractor_screenshot_pattern() {
        // Arrange
        let extractor = FilenameBasedDateExtractor::new();
        let filename = "Screenshot_2013-04-19-19-46-43.png";

        // Act
        let result = extractor.extract_date(filename, &[]);

        // Assert
        assert!(result.is_ok(), "Failed to extract date: {:?}", result.err());
        let date = result.unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2013, 4, 19).unwrap());
    }

    #[test]
    fn test_filename_extractor_animation_pattern() {
        // Arrange
        let extractor = FilenameBasedDateExtractor::new();
        let filename = "20151115_143914-ANIMATION.gif";

        // Act
        let result = extractor.extract_date(filename, &[]);

        // Assert
        assert!(result.is_ok(), "Failed to extract date: {:?}", result.err());
        let date = result.unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2015, 11, 15).unwrap());
    }

    #[test]
    fn test_filename_extractor_img_underscore_pattern() {
        // Arrange
        let extractor = FilenameBasedDateExtractor::new();
        let filename = "IMG_20130106_160818.JPG";

        // Act
        let result = extractor.extract_date(filename, &[]);

        // Assert
        assert!(result.is_ok(), "Failed to extract date: {:?}", result.err());
        let date = result.unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2013, 1, 6).unwrap());
    }

    #[test]
    fn test_filename_extractor_img_dash_pattern() {
        // Arrange
        let extractor = FilenameBasedDateExtractor::new();
        let filename = "IMG-20150130-WA0001.jpg";

        // Act
        let result = extractor.extract_date(filename, &[]);

        // Assert
        assert!(result.is_ok(), "Failed to extract date: {:?}", result.err());
        let date = result.unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2015, 1, 30).unwrap());
    }

    #[test]
    fn test_filename_extractor_simple_date_pattern() {
        // Arrange
        let extractor = FilenameBasedDateExtractor::new();
        let filename = "2014-09-29.jpg";

        // Act
        let result = extractor.extract_date(filename, &[]);

        // Assert
        assert!(result.is_ok(), "Failed to extract date: {:?}", result.err());
        let date = result.unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2014, 9, 29).unwrap());
    }

    #[test]
    fn test_filename_extractor_no_pattern_returns_error() {
        // Arrange
        let extractor = FilenameBasedDateExtractor::new();
        let filename = "random_file.jpg";

        // Act
        let result = extractor.extract_date(filename, &[]);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_composite_extractor_uses_exif_first() {
        // Arrange
        let extractor = CompositeDateExtractor::new();
        let sample_image_data: &[u8] = include_bytes!("../tests/fixtures/single_pixel_with_exif.jpg");
        // Filename suggests 2015, but EXIF says 2012 - EXIF should win
        let filename = "IMG_20150130_000000.jpg";

        // Act
        let result = extractor.extract_date(filename, sample_image_data);

        // Assert
        assert!(result.is_ok());
        let date = result.unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2012, 10, 6).unwrap(), "Should use EXIF date, not filename");
    }

    #[test]
    fn test_composite_extractor_falls_back_to_filename() {
        // Arrange
        let extractor = CompositeDateExtractor::new();
        let no_exif_data: &[u8] = &[0xFF, 0xD8, 0xFF, 0xD9]; // Minimal JPEG without EXIF
        let filename = "Screenshot_2013-04-19-19-46-43.png";

        // Act
        let result = extractor.extract_date(filename, no_exif_data);

        // Assert
        assert!(result.is_ok(), "Failed to extract date: {:?}", result.err());
        let date = result.unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2013, 4, 19).unwrap(), "Should fall back to filename");
    }

    #[test]
    fn test_composite_extractor_fails_when_both_missing() {
        // Arrange
        let extractor = CompositeDateExtractor::new();
        let no_exif_data: &[u8] = &[0xFF, 0xD8, 0xFF, 0xD9];
        let filename = "random_file.jpg";

        // Act
        let result = extractor.extract_date(filename, no_exif_data);

        // Assert
        assert!(result.is_err(), "Should fail when both EXIF and filename patterns are missing");
    }
}
