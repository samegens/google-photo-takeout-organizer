use exif::{In, Tag};
use std::collections::HashSet;

/// Trait for filtering photos based on criteria
/// Following Interface Segregation Principle
pub trait PhotoFilter {
    fn should_include(&self, filename: &str, image_data: &[u8]) -> bool;
}

/// Filter that skips photos already in your existing collection
/// (Lightroom-processed, DSLR cameras like Nikon, or Google-generated -MIX files)
pub struct ExistingCollectionFilter {
    all_filenames: HashSet<String>,
}

impl ExistingCollectionFilter {
    pub fn new(filenames: Vec<String>) -> Self {
        Self {
            all_filenames: filenames.into_iter().collect(),
        }
    }

    fn get_exif_field(&self, image_data: &[u8], tag: Tag) -> Option<String> {
        let mut cursor = std::io::Cursor::new(image_data);
        let exif_reader = exif::Reader::new();

        let exif_data = exif_reader.read_from_container(&mut cursor).ok()?;
        let field = exif_data.get_field(tag, In::PRIMARY)?;

        Some(field.display_value().to_string())
    }

    fn has_original_file(&self, edited_filename: &str) -> bool {
        let original_name = edited_filename
            .replace("-edited", "")
            .replace("-EDITED", "")
            .replace("-Edited", "");

        self.all_filenames.contains(&original_name)
    }
}

impl PhotoFilter for ExistingCollectionFilter {
    fn should_include(&self, filename: &str, image_data: &[u8]) -> bool {
        let filename_upper = filename.to_uppercase();

        if filename_upper.contains("-MIX") {
            return false;
        }

        if filename_upper.contains("-EDITED") {
            return !self.has_original_file(filename);
        }

        if let Some(software) = self.get_exif_field(image_data, Tag::Software) {
            if software.to_lowercase().contains("lightroom") {
                return false;
            }
        }

        if let Some(make) = self.get_exif_field(image_data, Tag::Make) {
            if make.to_uppercase().contains("NIKON") {
                return false;
            }
        }

        if let Some(model) = self.get_exif_field(image_data, Tag::Model) {
            if model.to_uppercase().contains("NIKON") {
                return false;
            }
        }

        true
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
        let filter = ExistingCollectionFilter::new(vec!["DSC_9157.JPG".to_string()]);
        let lightroom_photo = include_bytes!("../tests/fixtures/single_pixel_with_exif.jpg");

        // Act
        let result = filter.should_include("DSC_9157.JPG", lightroom_photo);

        // Assert
        assert!(!result, "Lightroom photo should be rejected (should_include = false)");
    }

    #[test]
    fn test_existing_collection_filter_accepts_mobile_photos() {
        // Arrange
        let filter = ExistingCollectionFilter::new(vec!["phone_photo.jpg".to_string()]);
        let no_software_photo = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include("phone_photo.jpg", no_software_photo);

        // Assert
        assert!(result, "Photo without Software field should be accepted");
    }

    #[test]
    fn test_existing_collection_filter_accepts_photos_without_exif() {
        // Arrange
        let filter = ExistingCollectionFilter::new(vec!["photo.jpg".to_string()]);
        let no_exif_photo = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include("photo.jpg", no_exif_photo);

        // Assert
        assert!(result);
    }

    #[test]
    fn test_existing_collection_filter_rejects_google_mix_files() {
        // Arrange
        let filter = ExistingCollectionFilter::new(vec!["DSC_9157-MIX.jpg".to_string()]);
        let any_data = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include("DSC_9157-MIX.jpg", any_data);

        // Assert
        assert!(!result, "Google-generated MIX files should be rejected");
    }

    #[test]
    fn test_existing_collection_filter_rejects_mix_files_case_insensitive() {
        // Arrange
        let filter = ExistingCollectionFilter::new(vec![
            "photo-mix.jpg".to_string(),
            "PHOTO-MIX.JPG".to_string(),
            "Photo-MiX.jpg".to_string(),
        ]);
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

    #[test]
    fn test_existing_collection_filter_rejects_edited_files() {
        // Arrange
        let filter = ExistingCollectionFilter::new(vec![
            "DSC_9157.JPG".to_string(),
            "DSC_9157-edited.JPG".to_string(),
        ]);
        let any_data = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include("DSC_9157-edited.JPG", any_data);

        // Assert
        assert!(!result, "Google-edited files should be rejected when original exists");
    }

    #[test]
    fn test_existing_collection_filter_rejects_edited_files_case_insensitive() {
        // Arrange
        let filter = ExistingCollectionFilter::new(vec![
            "photo.jpg".to_string(),
            "photo-edited.jpg".to_string(),
            "PHOTO.JPG".to_string(),
            "PHOTO-EDITED.JPG".to_string(),
            "Photo.jpg".to_string(),
            "Photo-Edited.jpg".to_string(),
        ]);
        let any_data = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result_lowercase = filter.should_include("photo-edited.jpg", any_data);
        let result_uppercase = filter.should_include("PHOTO-EDITED.JPG", any_data);
        let result_mixed = filter.should_include("Photo-Edited.jpg", any_data);

        // Assert
        assert!(!result_lowercase, "Should reject lowercase -edited when original exists");
        assert!(!result_uppercase, "Should reject uppercase -EDITED when original exists");
        assert!(!result_mixed, "Should reject mixed case -Edited when original exists");
    }

    #[test]
    fn test_existing_collection_filter_keeps_orphaned_edited_files() {
        // Arrange
        let all_filenames = vec![
            "photo1.jpg".to_string(),
            "photo2-edited.jpg".to_string(),
        ];
        let filter = ExistingCollectionFilter::new(all_filenames);
        let any_data = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include("photo2-edited.jpg", any_data);

        // Assert
        assert!(result, "Should keep -edited file when original doesn't exist");
    }

    #[test]
    fn test_existing_collection_filter_rejects_edited_when_original_exists() {
        // Arrange
        let all_filenames = vec![
            "photo1.jpg".to_string(),
            "photo1-edited.jpg".to_string(),
        ];
        let filter = ExistingCollectionFilter::new(all_filenames);
        let any_data = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include("photo1-edited.jpg", any_data);

        // Assert
        assert!(!result, "Should reject -edited file when original exists");
    }
}

