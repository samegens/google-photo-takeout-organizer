use exif::{In, Tag};

/// Trait for filtering photos based on criteria
/// Following Interface Segregation Principle
pub trait PhotoFilter {
    fn should_include(&self, filename: &str, image_data: &[u8]) -> bool;
}

/// Filter that skips photos already in your existing collection
/// (Lightroom-processed, DSLR cameras like Nikon, or Google-generated -MIX files)
pub struct ExistingCollectionFilter;

impl ExistingCollectionFilter {
    pub fn new() -> Self {
        Self
    }

    fn get_exif_field(&self, image_data: &[u8], tag: Tag) -> Option<String> {
        let mut cursor = std::io::Cursor::new(image_data);
        let exif_reader = exif::Reader::new();

        let exif_data = exif_reader.read_from_container(&mut cursor).ok()?;
        let field = exif_data.get_field(tag, In::PRIMARY)?;

        Some(field.display_value().to_string())
    }
}

impl PhotoFilter for ExistingCollectionFilter {
    fn should_include(&self, filename: &str, image_data: &[u8]) -> bool {
        // Check filename for Google-generated -MIX files
        if filename.to_uppercase().contains("-MIX") {
            return false; // Reject Google-generated MIX files
        }

        // Check Software field for Lightroom
        if let Some(software) = self.get_exif_field(image_data, Tag::Software) {
            if software.to_lowercase().contains("lightroom") {
                return false; // Reject Lightroom photos
            }
        }

        // Check Make field for NIKON
        if let Some(make) = self.get_exif_field(image_data, Tag::Make) {
            if make.to_uppercase().contains("NIKON") {
                return false; // Reject Nikon photos
            }
        }

        // Check Model field for NIKON
        if let Some(model) = self.get_exif_field(image_data, Tag::Model) {
            if model.to_uppercase().contains("NIKON") {
                return false; // Reject Nikon photos
            }
        }

        true // Accept if none of the filters match
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
    fn should_include(&self, _filename: &str, _image_data: &[u8]) -> bool {
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
        let result = filter.should_include("any_file.jpg", any_data);

        // Assert
        assert!(result);
    }

    #[test]
    fn test_existing_collection_filter_rejects_lightroom_photos() {
        // Arrange
        let filter = ExistingCollectionFilter::new();
        // This photo has "Adobe Photoshop Lightroom 3.6 (Windows)" in Software field
        let lightroom_photo = include_bytes!("../tests/fixtures/single_pixel_with_exif.jpg");

        // Act
        let result = filter.should_include("DSC_9157.JPG", lightroom_photo);

        // Assert
        assert!(!result, "Lightroom photo should be rejected (should_include = false)");
    }

    #[test]
    fn test_existing_collection_filter_accepts_mobile_photos() {
        // Arrange
        let filter = ExistingCollectionFilter::new();
        // Create a minimal JPEG that will pass (we'll need a non-Lightroom photo)
        // For now, test that photos without Software field are accepted
        let no_software_photo = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include("phone_photo.jpg", no_software_photo);

        // Assert
        assert!(result, "Photo without Software field should be accepted");
    }

    #[test]
    fn test_existing_collection_filter_accepts_photos_without_exif() {
        // Arrange
        let filter = ExistingCollectionFilter::new();
        // Minimal JPEG without EXIF
        let no_exif_photo = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include("photo.jpg", no_exif_photo);

        // Assert
        assert!(result); // Accept if no Software field
    }

    #[test]
    fn test_existing_collection_filter_rejects_google_mix_files() {
        // Arrange
        let filter = ExistingCollectionFilter::new();
        // Any image data (doesn't matter since we're testing filename)
        let any_data = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include("DSC_9157-MIX.jpg", any_data);

        // Assert
        assert!(!result, "Google-generated MIX files should be rejected");
    }

    #[test]
    fn test_existing_collection_filter_rejects_mix_files_case_insensitive() {
        // Arrange
        let filter = ExistingCollectionFilter::new();
        let any_data = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result_lowercase = filter.should_include("photo-mix.jpg", any_data);
        let result_uppercase = filter.should_include("PHOTO-MIX.JPG", any_data);
        let result_mixed = filter.should_include("Photo-MiX.jpg", any_data);

        // Assert
        assert!(!result_lowercase, "Should reject lowercase -mix");
        assert!(!result_uppercase, "Should reject uppercase -MIX");
        assert!(!result_mixed, "Should reject mixed case -MiX");
    }
}
