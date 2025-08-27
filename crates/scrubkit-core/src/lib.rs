// crates/scrubkit-core/src/lib.rs

pub mod jpeg;
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
