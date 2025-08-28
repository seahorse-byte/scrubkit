// crates/scrubkit-core/src/lib.rs

pub mod jpeg;
pub mod png;
use jpeg::JpegScrubber;
use png::PngScrubber;
use thiserror::Error;

/// A universal error type for all scrubbing operations.
#[derive(Error, Debug)]
pub enum ScrubError {
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    #[error("File parsing failed: {0}")]
    ParsingError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("An unknown error occurred")]
    Unknown,
}

/// Represents a single piece of metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataEntry {
    pub key: String,
    pub value: String,
    pub category: String, // e.g., "EXIF", "GPS", "Document Properties"
}

/// The result of a successful scrub operation.
#[derive(Debug)]
pub struct ScrubResult {
    /// The bytes of the new, cleaned file.
    pub cleaned_file_bytes: Vec<u8>,
    /// A report of the metadata entries that were removed.
    pub metadata_removed: Vec<MetadataEntry>,
}

/// The central trait of our library.
/// Any file type we want to support must implement this trait.
pub trait Scrubber {
    /// Creates a new Scrubber instance from file bytes.
    /// This will also parse the file to ensure it's valid.
    fn new(file_bytes: Vec<u8>) -> Result<Self, ScrubError>
    where
        Self: Sized;

    /// Returns all found metadata in a structured format.
    fn view_metadata(&self) -> Result<Vec<MetadataEntry>, ScrubError>;

    /// Removes all identifiable metadata.
    fn scrub(&self) -> Result<ScrubResult, ScrubError>;
}

/// Detects the file type and returns the appropriate scrubber.
/// This is the main entry point for consumers of the library.
pub fn scrubber_for_file(file_bytes: Vec<u8>) -> Result<Box<dyn Scrubber>, ScrubError> {
    // PNG files start with a specific 8-byte signature.
    if file_bytes.len() > 8 && file_bytes[0..8] == [137, 80, 78, 71, 13, 10, 26, 10] {
        let scrubber = PngScrubber::new(file_bytes)?;
        return Ok(Box::new(scrubber));
    }

    // JPEG files start with 0xFFD8.
    if file_bytes.len() > 2 && file_bytes[0..2] == [0xFF, 0xD8] {
        let scrubber = JpegScrubber::new(file_bytes)?;
        return Ok(Box::new(scrubber));
    }

    Err(ScrubError::UnsupportedFileType(
        "Could not determine file type.".to_string(),
    ))
}
