// crates/scrubkit-core/src/jpeg.rs

use crate::{MetadataEntry, ScrubError, ScrubResult, Scrubber};

/// A Scrubber implementation for JPEG files.
#[derive(Debug, Clone)]
pub struct JpegScrubber {
    file_bytes: Vec<u8>,
}

// Private helper functions for JpegScrubber
impl JpegScrubber {
    /// Finds the EXIF data segment (APP1) in the JPEG byte stream.
    fn find_exif_segment(&self) -> Option<(usize, usize)> {
        let mut offset = 2; // Skip the initial SOI marker (0xFFD8)
        while offset + 4 <= self.file_bytes.len() {
            if self.file_bytes[offset] != 0xFF {
                return None;
            }

            let marker = self.file_bytes[offset + 1];

            if (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
                offset += 2;
                continue;
            }

            if marker == 0xD9 || marker == 0xDA {
                break;
            }

            let length =
                u16::from_be_bytes([self.file_bytes[offset + 2], self.file_bytes[offset + 3]])
                    as usize;

            if length < 2 || offset + 2 + length > self.file_bytes.len() {
                return None; // Corrupt length.
            }

            if marker == 0xE1
                && length >= 8
                && self.file_bytes[offset + 4..offset + 10] == *b"Exif\0\0"
            {
                return Some((offset, 2 + length));
            }

            offset += 2 + length;
        }

        None
    }
} // <-- FIX #1: This closing brace was missing.

impl Scrubber for JpegScrubber {
    fn new(file_bytes: Vec<u8>) -> Result<Self, ScrubError> {
        if file_bytes.len() < 2 || file_bytes[0..2] != [0xFF, 0xD8] {
            return Err(ScrubError::ParsingError("Not a valid JPEG file".into()));
        }
        Ok(Self { file_bytes })
    }

    fn view_metadata(&self) -> Result<Vec<MetadataEntry>, ScrubError> {
        // FIX #2: Use the correct `parse_exif` function and handle its Result.
        match nom_exif::parse_exif(&self.file_bytes) {
            Ok((_, exif_data)) => Ok(exif_data
                .entries()
                .iter()
                .map(|entry| MetadataEntry {
                    category: entry.ifd.to_string(),
                    key: entry.tag.to_string(),
                    value: entry.value.to_string(),
                })
                .collect()),
            // If parsing fails, it's likely because there's no EXIF data.
            // We treat this as a success and return an empty list.
            Err(_) => Ok(Vec::new()),
        }
    }

    fn scrub(&self) -> Result<ScrubResult, ScrubError> {
        let metadata_removed = self.view_metadata()?;

        if let Some((start, length)) = self.find_exif_segment() {
            let mut cleaned_bytes = Vec::with_capacity(self.file_bytes.len());
            cleaned_bytes.extend_from_slice(&self.file_bytes[..start]);
            cleaned_bytes.extend_from_slice(&self.file_bytes[start + length..]);

            Ok(ScrubResult {
                cleaned_file_bytes: cleaned_bytes,
                metadata_removed,
            })
        } else {
            // No EXIF data to remove, return original bytes
            Ok(ScrubResult {
                cleaned_file_bytes: self.file_bytes.clone(),
                metadata_removed,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A 1x1 pixel JPEG with EXIF data. Camera Model: "Test Camera"
    const TEST_JPEG_WITH_EXIF: &[u8] = &[
        0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x4A, 0x45, 0x78, 0x69, 0x66, 0x00, 0x00, 0x4D, 0x4D, 0x00,
        0x2A, 0x00, 0x00, 0x00, 0x08, 0x00, 0x02, 0x01, 0x0F, 0x00, 0x02, 0x00, 0x00, 0x00, 0x0D,
        0x00, 0x00, 0x00, 0x1A, 0x01, 0x10, 0x00, 0x02, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00,
        0x28, 0x00, 0x00, 0x00, 0x00, 0x54, 0x65, 0x73, 0x74, 0x20, 0x43, 0x61, 0x6D, 0x65, 0x72,
        0x61, 0x00, 0x54, 0x65, 0x73, 0x74, 0x20, 0x4D, 0x6F, 0x64, 0x65, 0x6C, 0x00, 0xFF, 0xDB,
        0x00, 0x43, 0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x01, 0x00, 0x01, 0x03, 0x01,
        0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01,
        0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xDA, 0x00, 0x0C,
        0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00, 0xF7, 0xC8, 0xFF, 0xD9,
    ];

    const TEST_JPEG_WITHOUT_EXIF: &[u8] = &[
        0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x01, 0x00, 0x01,
        0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01, 0xFF, 0xC4, 0x00, 0x1F, 0x00,
        0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xDA,
        0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00, 0xF7, 0xC8, 0xFF,
        0xD9,
    ];

    #[test]
    fn new_jpeg_scrubber_works() {
        assert!(JpegScrubber::new(TEST_JPEG_WITH_EXIF.to_vec()).is_ok());
        let invalid_bytes = vec![0x01, 0x02, 0x03];
        assert!(JpegScrubber::new(invalid_bytes).is_err());
    }

    #[test]
    fn view_metadata_finds_exif_data() {
        let scrubber = JpegScrubber::new(TEST_JPEG_WITH_EXIF.to_vec()).unwrap();
        let metadata = scrubber.view_metadata().unwrap();
        assert!(!metadata.is_empty(), "No metadata was found");

        let model_entry = metadata.iter().find(|m| m.key == "Model");
        assert!(model_entry.is_some(), "Camera model metadata not found");
        assert_eq!(
            model_entry.unwrap().value.trim_matches(char::from(0)),
            "Test Model"
        );
    }

    #[test]
    fn scrub_removes_exif_segment_and_reports_it() {
        let scrubber = JpegScrubber::new(TEST_JPEG_WITH_EXIF.to_vec()).unwrap();
        let expected_metadata_removed = scrubber.view_metadata().unwrap();
        assert!(!expected_metadata_removed.is_empty());

        let result = scrubber.scrub().unwrap();

        assert!(
            result.cleaned_file_bytes.len() < TEST_JPEG_WITH_EXIF.len(),
            "Scrubbed file size should be smaller than original"
        );
        assert_eq!(
            result.metadata_removed, expected_metadata_removed,
            "The removed metadata should be reported correctly"
        );

        let new_scrubber = JpegScrubber::new(result.cleaned_file_bytes.clone()).unwrap();
        let new_metadata = new_scrubber.view_metadata().unwrap();
        assert!(
            new_metadata.is_empty(),
            "Scrubbed file should have no metadata"
        );
        assert_eq!(result.cleaned_file_bytes, TEST_JPEG_WITHOUT_EXIF);
    }

    #[test]
    fn view_metadata_on_jpeg_without_exif_returns_empty() {
        let scrubber = JpegScrubber::new(TEST_JPEG_WITHOUT_EXIF.to_vec()).unwrap();
        let metadata = scrubber.view_metadata().unwrap();
        assert!(
            metadata.is_empty(),
            "Metadata should be empty for a clean JPEG"
        );
    }

    #[test]
    fn scrub_on_jpeg_without_exif_does_nothing() {
        let original_bytes = TEST_JPEG_WITHOUT_EXIF.to_vec();
        let scrubber = JpegScrubber::new(original_bytes.clone()).unwrap();
        let result = scrubber.scrub().unwrap();

        assert_eq!(
            result.cleaned_file_bytes, original_bytes,
            "File bytes should not change when no EXIF data is present"
        );
        assert!(
            result.metadata_removed.is_empty(),
            "No metadata should be reported as removed"
        );
    }
}
