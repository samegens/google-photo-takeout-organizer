use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Trait for writing files to the filesystem
pub trait FileSystemWriter {
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()>;
    fn create_directory(&self, path: &Path) -> Result<()>;
    fn get_full_path(&self, path: &Path) -> PathBuf;
}

/// Concrete implementation that writes to the actual filesystem
pub struct RealFileSystemWriter {
    base_output_dir: String,
}

impl RealFileSystemWriter {
    pub fn new(base_output_dir: String) -> Self {
        Self { base_output_dir }
    }
}

impl FileSystemWriter for RealFileSystemWriter {
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        let full_path = PathBuf::from(&self.base_output_dir).join(path);

        let mut file = fs::File::create(&full_path)
            .with_context(|| format!("Failed to create file: {}", full_path.display()))?;

        file.write_all(data)
            .with_context(|| format!("Failed to write data to file: {}", full_path.display()))?;

        Ok(())
    }

    fn create_directory(&self, path: &Path) -> Result<()> {
        let full_path = PathBuf::from(&self.base_output_dir).join(path);

        fs::create_dir_all(&full_path)
            .with_context(|| format!("Failed to create directory: {}", full_path.display()))?;

        Ok(())
    }

    fn get_full_path(&self, path: &Path) -> PathBuf {
        PathBuf::from(&self.base_output_dir).join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_create_directory() {
        // Arrange
        let temp_dir = "/tmp/test_photo_org";
        let writer = RealFileSystemWriter::new(temp_dir.to_string());
        let dir_path = PathBuf::from("2024/2024-01-05");

        // Act
        let result = writer.create_directory(&dir_path);

        // Assert
        assert!(result.is_ok());
        assert!(PathBuf::from(temp_dir).join(&dir_path).exists());

        // Cleanup
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_write_file() {
        // Arrange
        let temp_dir = "/tmp/test_photo_write";
        let writer = RealFileSystemWriter::new(temp_dir.to_string());
        let file_path = PathBuf::from("2024/2024-01-05/photo.jpg");
        let test_data = b"fake image data";

        // Create parent directory first
        writer.create_directory(&PathBuf::from("2024/2024-01-05")).ok();

        // Act
        let result = writer.write_file(&file_path, test_data);

        // Assert
        assert!(result.is_ok());
        let full_path = PathBuf::from(temp_dir).join(&file_path);
        assert!(full_path.exists());
        let written_data = fs::read(&full_path).unwrap();
        assert_eq!(written_data, test_data);

        // Cleanup
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_create_nested_directories() {
        // Arrange
        let temp_dir = "/tmp/test_nested_dirs";
        let writer = RealFileSystemWriter::new(temp_dir.to_string());
        let nested_path = PathBuf::from("2024/2024-01-05/subdir/deep");

        // Act
        let result = writer.create_directory(&nested_path);

        // Assert
        assert!(result.is_ok());
        assert!(PathBuf::from(temp_dir).join(&nested_path).exists());

        // Cleanup
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_create_directory_idempotent() {
        // Arrange
        let temp_dir = "/tmp/test_idempotent";
        let writer = RealFileSystemWriter::new(temp_dir.to_string());
        let dir_path = PathBuf::from("2024");

        // Act - create twice
        let result1 = writer.create_directory(&dir_path);
        let result2 = writer.create_directory(&dir_path);

        // Assert - both should succeed (idempotent)
        assert!(result1.is_ok());
        assert!(result2.is_ok());

        // Cleanup
        fs::remove_dir_all(temp_dir).ok();
    }
}
