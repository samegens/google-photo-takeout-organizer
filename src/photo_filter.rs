use exif::{In, Tag};
use std::collections::HashSet;

/// Google duplicate file patterns to filter (uppercase versions)
const GOOGLE_DUPLICATE_PATTERNS: &[&str] = &[
    "-MIX",
    "-EDITED",
    "-EFFECTS",
    "-ANIMATION",
    "-COLLAGE",
    "-SMILE",
    "-PANO",
];

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

    fn has_original_file(&self, duplicate_filename: &str) -> bool {
        let mut original_name = duplicate_filename.to_string();

        for pattern in GOOGLE_DUPLICATE_PATTERNS {
            original_name = original_name
                .replace(pattern, "")
                .replace(&pattern.to_lowercase(), "");
        }

        self.all_filenames.contains(&original_name)
    }
}

impl PhotoFilter for ExistingCollectionFilter {
    fn should_include(&self, filename: &str, image_data: &[u8]) -> bool {
        let filename_upper = filename.to_uppercase();

        if filename_upper.ends_with(".GIF") {
            return false;
        }

        for pattern in GOOGLE_DUPLICATE_PATTERNS {
            if filename_upper.contains(pattern) {
                return !self.has_original_file(filename);
            }
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
    use rstest::rstest;

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
        assert!(
            !result,
            "Lightroom photo should be rejected (should_include = false)"
        );
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
        assert!(
            !result,
            "Google-edited files should be rejected when original exists"
        );
    }

    #[test]
    fn test_existing_collection_filter_keeps_orphaned_edited_files() {
        // Arrange
        let all_filenames = vec!["photo1.jpg".to_string(), "photo2-EDITED.jpg".to_string()];
        let filter = ExistingCollectionFilter::new(all_filenames);
        let any_data = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include("photo2-EDITED.jpg", any_data);

        // Assert
        assert!(
            result,
            "Should keep -EDITED file when original doesn't exist"
        );
    }

    #[rstest]
    #[case("animation.gif")]
    #[case("PHOTO.GIF")]
    #[case("Image.Gif")]
    fn test_existing_collection_filter_rejects_gif_files(#[case] filename: &str) {
        // Arrange
        let filter = ExistingCollectionFilter::new(vec![]);
        let gif_data = &[0x47, 0x49, 0x46, 0x38, 0x39, 0x61]; // GIF89a header

        // Act
        let result = filter.should_include(filename, gif_data);

        // Assert
        assert!(!result, "Should always reject GIF file: {}", filename);
    }

    #[rstest]
    #[case("photo-EFFECTS.jpg", "photo.jpg")]
    #[case("IMG_1234-ANIMATION.jpg", "IMG_1234.jpg")]
    #[case("DSC_9876-COLLAGE.jpg", "DSC_9876.jpg")]
    #[case("pic-SMILE.jpg", "pic.jpg")]
    #[case("sunset-PANO.jpg", "sunset.jpg")]
    #[case("sunset-MIX.jpg", "sunset.jpg")]
    #[case("DSC_9157-edited.JPG", "DSC_9157.JPG")]
    fn test_existing_collection_filter_rejects_google_duplicates_when_original_exists(
        #[case] duplicate_filename: &str,
        #[case] original_filename: &str,
    ) {
        // Arrange
        let all_filenames = vec![
            original_filename.to_string(),
            duplicate_filename.to_string(),
        ];
        let filter = ExistingCollectionFilter::new(all_filenames);
        let any_data = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include(duplicate_filename, any_data);

        // Assert
        assert!(
            !result,
            "Should reject {} when {} exists",
            duplicate_filename, original_filename
        );
    }

    #[rstest]
    #[case("photo-EFFECTS.jpg")]
    #[case("IMG_1234-ANIMATION.jpg")]
    #[case("DSC_9876-COLLAGE.jpg")]
    #[case("pic-SMILE.jpg")]
    #[case("sunset-PANO.jpg")]
    #[case("DSC_9157-edited.JPG")]
    fn test_existing_collection_filter_keeps_orphaned_google_duplicates(#[case] filename: &str) {
        // Arrange - no original file exists
        let filter = ExistingCollectionFilter::new(vec![filename.to_string()]);
        let any_data = &[0xFF, 0xD8, 0xFF, 0xD9];

        // Act
        let result = filter.should_include(filename, any_data);

        // Assert
        assert!(
            result,
            "Should keep {} when original doesn't exist",
            filename
        );
    }
}
