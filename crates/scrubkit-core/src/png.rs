// File: crates/scrubkit-core/src/png.rs

use crate::{MetadataEntry, ScrubError, ScrubResult, Scrubber};
use std::io::Cursor;

/// A Scrubber implementation for PNG files.
#[derive(Debug, Clone)]
pub struct PngScrubber {
    file_bytes: Vec<u8>,
}

impl Scrubber for PngScrubber {
    fn new(file_bytes: Vec<u8>) -> Result<Self, ScrubError> {
        // The png::Decoder will fail if it's not a valid PNG, which is a robust check.
        let decoder = png::Decoder::new(Cursor::new(&file_bytes));
        if decoder.read_info().is_err() {
            return Err(ScrubError::UnsupportedFileType(
                "Not a valid PNG file.".to_string(),
            ));
        }
        Ok(Self { file_bytes })
    }

    fn view_metadata(&self) -> Result<Vec<MetadataEntry>, ScrubError> {
        let decoder = png::Decoder::new(Cursor::new(&self.file_bytes));
        let reader = decoder
            .read_info()
            .map_err(|e| ScrubError::ParsingError(e.to_string()))?;
        let mut metadata = Vec::new();

        // Correctly iterate over the decoded text chunks.
        for text_chunk in &reader.info().uncompressed_latin1_text {
            metadata.push(MetadataEntry {
                category: "tEXt/zTXt/iTXt".to_string(),
                key: text_chunk.keyword.clone(),
                value: text_chunk.text.clone(),
            });
        }

        Ok(metadata)
    }

    fn scrub(&self) -> Result<ScrubResult, ScrubError> {
        let metadata_removed = self.view_metadata()?;
        if metadata_removed.is_empty() {
            return Ok(ScrubResult {
                cleaned_file_bytes: self.file_bytes.clone(),
                metadata_removed: vec![],
            });
        }

        // To scrub, we must re-encode the image while skipping the metadata chunks.
        let decoder = png::Decoder::new(Cursor::new(&self.file_bytes));
        let mut reader = decoder
            .read_info()
            .map_err(|e| ScrubError::ParsingError(e.to_string()))?;

        // Read the image data itself.
        let mut img_data = vec![0; reader.output_buffer_size()];
        let info = reader
            .next_frame(&mut img_data)
            .map_err(|e| ScrubError::ParsingError(e.to_string()))?;

        // Create a new PNG in memory
        let mut cleaned_bytes = Vec::new();
        {
            // Create a new scope for the encoder and writer to ensure they are dropped
            // and release their borrow on `cleaned_bytes` before we return it.
            let mut encoder =
                png::Encoder::new(Cursor::new(&mut cleaned_bytes), info.width, info.height);
            encoder.set_color(info.color_type);
            encoder.set_depth(info.bit_depth);

            // Crucially, we do *not* write any of the textual metadata chunks to the new encoder.

            let mut writer = encoder
                .write_header()
                .map_err(|e| ScrubError::ParsingError(e.to_string()))?;

            writer
                .write_image_data(&img_data)
                .map_err(|e| ScrubError::ParsingError(e.to_string()))?;
        } // encoder and writer are dropped here

        // The `cleaned_bytes` vec now holds the scrubbed PNG.
        Ok(ScrubResult {
            cleaned_file_bytes: cleaned_bytes,
            metadata_removed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A simple 1x1 pixel PNG with a tEXt chunk for metadata.
    // Keyword: "Author", Text: "ScrubKit Tester"
    const TEST_PNG_WITH_METADATA: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 28, 116, 69, 88, 116, 65, 117, 116, 104, 111, 114, 0,
        83, 99, 114, 117, 98, 75, 105, 116, 32, 84, 101, 115, 116, 101, 114, 215, 122, 61, 248, 0,
        0, 0, 12, 73, 68, 65, 84, 8, 215, 99, 96, 96, 96, 248, 207, 192, 4, 0, 1, 10, 0, 255, 170,
        222, 158, 221, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    #[test]
    fn view_metadata_finds_png_text_chunk() {
        let scrubber = PngScrubber::new(TEST_PNG_WITH_METADATA.to_vec()).unwrap();
        let metadata = scrubber.view_metadata().unwrap();
        assert!(!metadata.is_empty());
        assert_eq!(metadata[0].key, "Author");
        assert_eq!(metadata[0].value, "ScrubKit Tester");
    }

    #[test]
    fn scrub_removes_png_text_chunk() {
        let scrubber = PngScrubber::new(TEST_PNG_WITH_METADATA.to_vec()).unwrap();
        let result = scrubber.scrub().unwrap();

        // The most important test is to verify that the *new* file has no metadata.
        assert!(!result.metadata_removed.is_empty());

        let new_scrubber = PngScrubber::new(result.cleaned_file_bytes).unwrap();
        let new_metadata = new_scrubber.view_metadata().unwrap();
        assert!(
            new_metadata.is_empty(),
            "Scrubbed file should have no metadata"
        );
    }
}
