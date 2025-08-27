use crate::{MetadataEntry, ScrubError, ScrubResult, Scrubber};
use std::io::Cursor;

/// A Scrubber implementation for JPEG files.
#[derive(Debug, Clone)]
pub struct JpegScrubber {
    file_bytes: Vec<u8>,
}

// Private helper functions for JpegScrubber
impl JpegScrubber {
    /// Finds the EXIF data segment (APP1) in the JPEG byte stream.
    /// Returns a tuple of (start_index, length) if found.
    fn find_exif_segment(&self) -> Option<(usize, usize)> {
        let mut offset = 2; // Skip the initial SOI marker (0xFFD8)
        while offset + 4 < self.file_bytes.len() {
            // Check for a marker prefix
            if self.file_bytes[offset] != 0xFF {
                return None; // Invalid JPEG structure
            }

            let marker = self.file_bytes[offset + 1];

            // Markers without size fields, just skip them.
            if (0xD0..=0xD9).contains(&marker) || marker == 0x01 {
                offset += 2;
                continue;
            }

            // Reached Start of Scan (SOS), header ends here.
            if marker == 0xDA {
                break;
            }

            // Read the length of the segment (big-endian u16)
            let length =
                u16::from_be_bytes([self.file_bytes[offset + 2], self.file_bytes[offset + 3]])
                    as usize;

            // The APP1 marker (0xE1) is used for EXIF data.
            // We also check for the "Exif\0\0" identifier.
            if marker == 0xE1 && self.file_bytes[offset + 4..offset + 10] == *b"Exif\0\0" {
                return Some((offset, length));
            }

            // Move to the next segment
            offset += length + 2;
        }

        None
    }
}

impl Scrubber for JpegScrubber {
    fn new(file_bytes: Vec<u8>) -> Result<Self, ScrubError> {
        // Check for JPEG magic numbers/SOI marker (0xFFD8)
        if file_bytes.len() < 2 || file_bytes[0..2] != [0xFF, 0xD8] {
            return Err(ScrubError::UnsupportedFileType(
                "Not a valid JPEG file.".to_string(),
            ));
        }
        Ok(Self { file_bytes })
    }

    fn view_metadata(&self) -> Result<Vec<MetadataEntry>, ScrubError> {
        let mut metadata = Vec::new();

        if let Some((start, length)) = self.find_exif_segment() {
            // The actual EXIF data starts after the "Exif\0\0" identifier (6 bytes)
            let exif_data_start = start + 10;
            let exif_data_end = start + 2 + length;
            let exif_segment = &self.file_bytes[exif_data_start..exif_data_end];

            let mut reader = Cursor::new(exif_segment);
            let exif_reader = exif::Reader::new();

            match exif_reader.read_from(&mut reader) {
                Ok(exif) => {
                    for field in exif.fields() {
                        metadata.push(MetadataEntry {
                            category: format!("EXIF-{}", field.ifd_num),
                            key: field.tag.to_string(),
                            value: field.display_value().with_unit(&exif).to_string(),
                        });
                    }
                }
                Err(e) => return Err(ScrubError::ParsingError(e.to_string())),
            }
        }

        Ok(metadata)
    }

    fn scrub(&self) -> Result<ScrubResult, ScrubError> {
        let metadata_removed = self.view_metadata()?;
        let mut cleaned_file_bytes = self.file_bytes.clone();

        if let Some((start, length)) = self.find_exif_segment() {
            // The full segment length includes the 2 bytes for the length field itself.
            let total_segment_length = length + 2;
            cleaned_file_bytes.drain(start..start + total_segment_length);
        }

        Ok(ScrubResult {
            cleaned_file_bytes,
            metadata_removed,
        })
    }
}
