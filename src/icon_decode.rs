//! Shared worker-only ordinary icon decode policy. No file, GL, Root or URI access.
//! Encoded digests/SourceKey/provenance remain those of the original bytes.
//! Larger SOURCE admission is separate from unchanged <=512/1MiB OUTPUT bounds.

pub const MAX_INPUT_AXIS: u32 = 2048;
pub const MAX_INPUT_RGBA_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_ENCODED_BYTES: usize = 1024 * 1024;
pub const MAX_OUTPUT_AXIS: u32 = 512;
pub const MAX_OUTPUT_RGBA_BYTES: u64 = 1024 * 1024;

/// Actual codec and CPU resampling must stay on the existing image worker.
/// Independent native/source-RGBA buffers each fit8MiB; codec bookkeeping limits
/// are best-effort, so this is not a process-heap/OOM or96MiB fit guarantee.
pub fn decode(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if bytes.is_empty() || bytes.len() > MAX_ENCODED_BYTES {
        return None;
    }
    plx_gfx::img::img_decode_owned_thumbnail_bounded(
        bytes,
        MAX_INPUT_AXIS,
        MAX_INPUT_RGBA_BYTES,
        MAX_OUTPUT_AXIS,
        MAX_OUTPUT_RGBA_BYTES,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png() -> Vec<u8> {
        vec![
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 244, 34, 127, 138, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99, 224, 18,
            145, 99, 208, 48, 178, 105, 0, 0, 3, 199, 1, 83, 206, 227, 180, 69, 0, 0, 0, 0, 73, 69,
            78, 68, 174, 66, 96, 130,
        ]
    }
    #[test]
    fn shared_icon_policy_keeps_original_tiny_pixels_and_encoded_admission() {
        let mut encoded = png();
        let original = decode(&encoded).expect("synthetic valid tiny PNG");
        assert_eq!((original.0, original.1, original.2.len()), (2, 1, 8));
        encoded.resize(MAX_ENCODED_BYTES + 1, 0);
        // The generic framework API has no encoded admission policy. The actual
        // Luma entry does, even if a codec tolerates trailing bytes.
        assert!(plx_gfx::img::img_decode_owned_thumbnail_bounded(
            &encoded,
            MAX_INPUT_AXIS,
            MAX_INPUT_RGBA_BYTES,
            MAX_OUTPUT_AXIS,
            MAX_OUTPUT_RGBA_BYTES,
        )
        .is_some());
        assert!(decode(&encoded).is_none());
    }
}
