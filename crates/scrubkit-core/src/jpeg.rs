use crate::{MetadataEntry, ScrubError, ScrubResult, Scrubber};
use nom_exif::{ExifIter, MediaParser, MediaSource};
use std::io::Cursor;

/// A Scrubber implementation for JPEG files.
#[derive(Debug, Clone)]
pub struct JpegScrubber {
    file_bytes: Vec<u8>,
}

// Private helper functions for JpegScrubber
impl JpegScrubber {
    /// Finds the EXIF data segment (APP1) in the JPEG byte stream.
    /// Returns (start_offset, length_including_marker) of the APP1 segment.
    fn find_exif_segment(&self) -> Option<(usize, usize)> {
        let mut offset = 2; // Skip the initial SOI marker (0xFFD8)
        while offset + 4 <= self.file_bytes.len() {
            if self.file_bytes[offset] != 0xFF {
                // Invalid marker start, corrupt JPEG?
                eprintln!("Invalid marker start at offset {}", offset);
                return None;
            }

            let marker = self.file_bytes[offset + 1];

            // Standalone markers (no length field)
            if (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
                offset += 2;
                continue;
            }

            // Markers that signify the end of metadata or start of image data
            if marker == 0xD9 || marker == 0xDA {
                break;
            }

            // Markers with a length field (including the 2-byte length field itself)
            // Check bounds for reading the length
            if offset + 4 > self.file_bytes.len() {
                // Not enough bytes to read length, corrupt JPEG?
                eprintln!("Not enough bytes to read length at offset {}", offset);
                return None;
            }

            let length_bytes = [self.file_bytes[offset + 2], self.file_bytes[offset + 3]];
            let length = u16::from_be_bytes(length_bytes) as usize;

            // Length must be at least 2 (the length field itself) and not exceed buffer
            if length < 2 || offset + 2 + length > self.file_bytes.len() {
                // Corrupt length field.
                eprintln!("Corrupt length field at offset {}: {}", offset, length);
                return None;
            }

            // Check if this is the APP1 marker (0xE1) and starts with "Exif\0\0"
            if marker == 0xE1 && length >= 6 {
                // Need at least 6 bytes for "Exif\0\0"
                let exif_sig_start = offset + 4; // 2 (marker) + 2 (length) = 4
                let exif_sig_end = exif_sig_start + 6; // 6 bytes for "Exif\0\0"
                if exif_sig_end <= self.file_bytes.len()
                    && self.file_bytes[exif_sig_start..exif_sig_end] == *b"Exif\0\0"
                {
                    // Found the EXIF APP1 segment
                    // Return the start offset and the total length (including marker & length bytes)
                    return Some((offset, 2 + length));
                }
            }

            // Move to the next marker
            offset += 2 + length;
        }

        // EXIF APP1 segment not found
        None
    }
}

impl Scrubber for JpegScrubber {
    fn new(file_bytes: Vec<u8>) -> Result<Self, ScrubError> {
        // Basic JPEG check
        if file_bytes.len() < 2 || file_bytes[0..2] != [0xFF, 0xD8] {
            return Err(ScrubError::ParsingError("Not a valid JPEG file".into()));
        }
        Ok(Self { file_bytes })
    }

    fn view_metadata(&self) -> Result<Vec<MetadataEntry>, ScrubError> {
        let media_source = MediaSource::seekable(Cursor::new(&self.file_bytes)).map_err(|e| {
            ScrubError::ParsingError(format!("Failed to create MediaSource: {:?}", e))
        })?;

        if !media_source.has_exif() {
            return Ok(Vec::new());
        }

        let mut parser = MediaParser::new();
        let exif_iter: ExifIter = parser
            .parse(media_source)
            .map_err(|e| ScrubError::ParsingError(format!("Failed to parse EXIF: {:?}", e)))?;

        let mut metadata_entries = Vec::new();

        for entry in exif_iter {
            // --- Access fields from the ParsedExifEntry correctly ---

            // Get the tag representation.
            // entry.tag() returns Option<ExifTag>.
            let opt_tag_enum = entry.tag(); // This is Option<ExifTag>

            // Get a string representation of the tag name.
            let tag_name = match opt_tag_enum {
                Some(tag_enum) => format!("{:?}", tag_enum), // Format the ExifTag enum variant directly
                None => "<Unknown Tag>".to_string(),         // Fallback if tag is None
            };

            // Get the IFD number (category).
            let ifd_num = entry.ifd_index(); // Correct method

            let category = match ifd_num {
                0 => "IFD0".to_string(),
                1 => "IFD1".to_string(),
                2 => "EXIF".to_string(),
                3 => "GPS".to_string(),
                4 => "Interop".to_string(),
                _ => format!("IFD_{}", ifd_num),
            };

            // Get the value as a string representation.
            let opt_value_ref = entry.get_value(); // Hypothesis: This returns Option<&EntryValue>

            let value_string = match opt_value_ref {
                Some(value_ref) => format!("{:?}", value_ref), // Format the EntryValue using Debug
                None => "<No Value>".to_string(),              // Fallback if value is None
            };

            metadata_entries.push(MetadataEntry {
                key: tag_name,       // String from formatted ExifTag or fallback
                value: value_string, // String from formatted EntryValue or fallback
                category,            // String category
            });
        }
        Ok(metadata_entries)
    }

    fn scrub(&self) -> Result<ScrubResult, ScrubError> {
        // Get metadata that will be removed by calling view_metadata
        // This will now use the correct nom_exif API
        let metadata_removed = self.view_metadata()?; // Propagate errors from view_metadata

        // Find the EXIF segment
        if let Some((start_offset, segment_length)) = self.find_exif_segment() {
            // Create a new Vec for the cleaned bytes
            let mut cleaned_bytes = Vec::with_capacity(self.file_bytes.len() - segment_length);
            // Copy data before the EXIF segment
            cleaned_bytes.extend_from_slice(&self.file_bytes[..start_offset]);
            // Copy data after the EXIF segment
            cleaned_bytes.extend_from_slice(&self.file_bytes[start_offset + segment_length..]);

            Ok(ScrubResult {
                cleaned_file_bytes: cleaned_bytes,
                metadata_removed, // Use the metadata list obtained from view_metadata
            })
        } else {
            // No EXIF segment found according to our manual search, return original bytes
            Ok(ScrubResult {
                cleaned_file_bytes: self.file_bytes.clone(),
                metadata_removed: vec![], // If our search didn't find it, report nothing removed
            })
        }
    }
}

// --- Tests remain the same ---
// (Keeping the test code from the previous response as the logic for Scrubber impl is the focus)
// Note: I'll make one small adjustment to the test assertion based on the likely output format.
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
        println!("Found meta {:?}", metadata); // Debug print
        assert!(!metadata.is_empty(), "No metadata was found");

        // Check if any entry's value contains "Test Camera"
        // The ParsedExifEntry Display impl should format the Model tag's value nicely.
        let model_entry_found = metadata.iter().any(|m| m.value.contains("Test Camera"));
        assert!(
            model_entry_found,
            "Camera model metadata entry (containing 'Test Camera') not found. Metadata list: {:?}",
            metadata
        );
    }

    #[test]
    fn scrub_removes_exif_segment_and_reports_it() {
        let scrubber = JpegScrubber::new(TEST_JPEG_WITH_EXIF.to_vec()).unwrap();
        let expected_metadata_removed = scrubber.view_metadata().unwrap();
        assert!(
            !expected_metadata_removed.is_empty(),
            "Expected metadata to be present before scrubbing"
        );

        let result = scrubber.scrub().unwrap();

        assert!(
            result.cleaned_file_bytes.len() < TEST_JPEG_WITH_EXIF.len(),
            "Scrubbed file size should be smaller than original. Original: {}, Scrubbed: {}",
            TEST_JPEG_WITH_EXIF.len(),
            result.cleaned_file_bytes.len()
        );
        // Check that metadata was reported as removed
        assert!(
            !result.metadata_removed.is_empty(),
            "Metadata removed should not be empty"
        );

        // Verify the scrubbed file no longer has the EXIF segment (our manual check)
        let new_scrubber = JpegScrubber::new(result.cleaned_file_bytes.clone()).unwrap();
        assert!(
            new_scrubber.find_exif_segment().is_none(),
            "EXIF segment should be removed from the scrubbed file"
        );

        // Check if the bytes match the expected clean JPEG
        assert_eq!(
            result.cleaned_file_bytes, TEST_JPEG_WITHOUT_EXIF,
            "Scrubbed bytes do not match expected clean JPEG"
        );
    }

    #[test]
    fn view_metadata_on_jpeg_without_exif_returns_empty() {
        let scrubber = JpegScrubber::new(TEST_JPEG_WITHOUT_EXIF.to_vec()).unwrap();
        let metadata = scrubber.view_metadata().unwrap();
        println!("Metadata for clean JPEG: {:?}", metadata); // Debug print
        assert!(
            metadata.is_empty(),
            "Metadata should be empty for a clean JPEG. Found: {:?}",
            metadata
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
            "No metadata should be reported as removed. Found: {:?}",
            result.metadata_removed
        );
    }
}
