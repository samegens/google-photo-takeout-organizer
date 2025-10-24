use anyhow::{Context, Result};
use std::fs::File;
use std::io::Read;

/// Represents a file entry in a ZIP archive
#[derive(Debug)]
pub struct ZipEntry {
    pub name: String,
    pub data: Vec<u8>,
}

/// Trait for reading ZIP archives
pub trait ZipReader {
    fn read_entries(&self) -> Result<Vec<ZipEntry>>;
}

/// Concrete implementation that reads ZIP files
pub struct FileZipReader {
    path: String,
}

impl FileZipReader {
    pub fn new(path: String) -> Self {
        Self { path }
    }
}

impl ZipReader for FileZipReader {
    fn read_entries(&self) -> Result<Vec<ZipEntry>> {
        let file = File::open(&self.path)
            .with_context(|| format!("Failed to open ZIP file: {}", self.path))?;

        let mut archive = zip::ZipArchive::new(file)
            .context("Failed to read ZIP archive")?;

        let mut entries = Vec::new();

        for i in 0..archive.len() {
            let mut zip_file = archive.by_index(i)
                .with_context(|| format!("Failed to read entry at index {}", i))?;

            // Skip directories
            if zip_file.is_dir() {
                continue;
            }

            let name = zip_file.name().to_string();
            let mut data = Vec::new();
            zip_file.read_to_end(&mut data)
                .with_context(|| format!("Failed to read data for file: {}", name))?;

            entries.push(ZipEntry { name, data });
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use zip::write::{FileOptions, ZipWriter};

    fn create_test_zip(path: &str, files: &[(&str, &[u8])]) -> Result<()> {
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);

        let options: FileOptions<()> = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for (name, data) in files {
            zip.start_file(*name, options)?;
            zip.write_all(data)?;
        }

        zip.finish()?;
        Ok(())
    }

    #[test]
    fn test_read_empty_zip() {
        // Arrange
        let zip_path = "/tmp/test_empty.zip";
        create_test_zip(zip_path, &[]).expect("Failed to create test zip");
        let reader = FileZipReader::new(zip_path.to_string());

        // Act
        let result = reader.read_entries();

        // Assert
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 0);

        // Cleanup
        std::fs::remove_file(zip_path).ok();
    }

    #[test]
    fn test_read_zip_with_single_file() {
        // Arrange
        let zip_path = "/tmp/test_single.zip";
        let test_data = b"Hello, World!";
        create_test_zip(zip_path, &[("test.txt", test_data)])
            .expect("Failed to create test zip");
        let reader = FileZipReader::new(zip_path.to_string());

        // Act
        let result = reader.read_entries();

        // Assert
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "test.txt");
        assert_eq!(entries[0].data, test_data);

        // Cleanup
        std::fs::remove_file(zip_path).ok();
    }

    #[test]
    fn test_read_zip_with_multiple_files() {
        // Arrange
        let zip_path = "/tmp/test_multiple.zip";
        create_test_zip(
            zip_path,
            &[
                ("photo1.jpg", b"fake jpg data 1"),
                ("photo2.jpg", b"fake jpg data 2"),
                ("photo3.png", b"fake png data"),
            ],
        )
        .expect("Failed to create test zip");
        let reader = FileZipReader::new(zip_path.to_string());

        // Act
        let result = reader.read_entries();

        // Assert
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "photo1.jpg");
        assert_eq!(entries[1].name, "photo2.jpg");
        assert_eq!(entries[2].name, "photo3.png");

        // Cleanup
        std::fs::remove_file(zip_path).ok();
    }

    #[test]
    fn test_read_nonexistent_zip_returns_error() {
        // Arrange
        let reader = FileZipReader::new("/tmp/nonexistent_file.zip".to_string());

        // Act
        let result = reader.read_entries();

        // Assert
        assert!(result.is_err());
    }
}
