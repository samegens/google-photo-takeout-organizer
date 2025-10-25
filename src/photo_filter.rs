use exif::{In, Tag};

/// Trait for filtering photos based on criteria
/// Following Interface Segregation Principle
pub trait PhotoFilter {
    fn should_include(&self, image_data: &[u8]) -> bool;
}

/// Filter that skips photos processed by Lightroom
pub struct LightroomFilter;

impl LightroomFilter {
    pub fn new() -> Self {
        Self
    }

    fn check_software_field(&self, image_data: &[u8]) -> Option<String> {
        let mut cursor = std::io::Cursor::new(image_data);
        let exif_reader = exif::Reader::new();

        let exif_data = exif_reader.read_from_container(&mut cursor).ok()?;

        // Get Software field (Tag 0x0131)
        let software_field = exif_data.get_field(Tag::Software, In::PRIMARY)?;

        Some(software_field.display_value().to_string())
    }
}

impl PhotoFilter for LightroomFilter {
    fn should_include(&self, image_data: &[u8]) -> bool {
        match self.check_software_field(image_data) {
            Some(software) => {
                // Reject if software contains "Lightroom"
                !software.to_lowercase().contains("lightroom")
            }
            None => {
                // No Software field, accept the photo
                true
            }
        }
    }
}

/// Filter that accepts all photos (no filtering)
pub struct NoFilter;

impl NoFilter {
    pub fn new() -> Self {
        Self
    }
}

impl PhotoFilter for NoFilter {
    fn should_include(&self, _image_data: &[u8]) -> bool {
        true // Accept everything
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_filter_accepts_all() {
        // Arrange
        let filter = NoFilter::new();
        let any_data = b"any data";

        // Act
        let result = filter.should_include(any_data);

        // Assert
        assert!(result);
    }

    #[test]
    fn test_lightroom_filter_rejects_lightroom_photos() {
        // Arrange
        let filter = LightroomFilter::new();
        // This photo has "Adobe Photoshop Lightroom 3.6 (Windows)" in Software field
        let lightroom_photo = include_bytes!("../tests/fixtures/single_pixel_with_exif.jpg");

        // Act
        let result = filter.should_include(lightroom_photo);

        // Assert
        assert!(!result, "Lightroom photo should be rejected (should_include = false)");
    }

    #[test]
    fn test_lightroom_filter_accepts_non_lightroom_photos() {
        // Arrange
        let filter = LightroomFilter::new();
        // Create a minimal JPEG that will pass (we'll need a non-Lightroom photo)
        // For now, test that photos without Software field are accepted
        let no_software_photo = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include(no_software_photo);

        // Assert
        assert!(result, "Photo without Software field should be accepted");
    }

    #[test]
    fn test_lightroom_filter_accepts_photos_without_software_field() {
        // Arrange
        let filter = LightroomFilter::new();
        // Minimal JPEG without EXIF
        let no_exif_photo = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include(no_exif_photo);

        // Assert
        assert!(result); // Accept if no Software field
    }
}
