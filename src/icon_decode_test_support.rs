//! Test-only synthetic PNGs. No filesystem, codec dependency, GL, URI or TV input.
//! Stored-DEFLATE RGBA makes full valid bodies, not only forged IHDR dimensions.
const COLOURS: [[u8; 4]; 4] = [
    [240, 10, 20, 255],
    [10, 240, 20, 128],
    [10, 20, 240, 64],
    [200, 100, 50, 0],
];
fn crc(bytes: &[u8]) -> u32 {
    let mut value = !0u32;
    for byte in bytes {
        value ^= u32::from(*byte);
        for _ in 0..8 {
            value = (value >> 1) ^ if value & 1 != 0 { 0xedb8_8320 } else { 0 };
        }
    }
    !value
}
fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    output.extend_from_slice(&(body.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(body);
    let mut checked = kind.to_vec();
    checked.extend_from_slice(body);
    output.extend_from_slice(&crc(&checked).to_be_bytes());
}
pub(crate) fn patterned_png(width: u32, height: u32) -> Vec<u8> {
    assert!(width > 0 && height > 0 && width <= 2049 && height <= 2049);
    assert!(u64::from(width) * u64::from(height) <= 128 * 1024);
    let mut raw = Vec::new();
    for y in 0..height {
        raw.push(0); // Every complete scanline uses PNG filter None.
        for x in 0..width {
            let (position, axis) = if width >= height {
                (x, width)
            } else {
                (y, height)
            };
            raw.extend_from_slice(&COLOURS[((position * 4) / axis) as usize]);
        }
    }
    let mut encoded = vec![0x78, 0x01];
    let blocks = raw.len().div_ceil(65535);
    for (index, block) in raw.chunks(65535).enumerate() {
        encoded.push(u8::from(index + 1 == blocks));
        let length = block.len() as u16;
        encoded.extend_from_slice(&length.to_le_bytes());
        encoded.extend_from_slice(&(!length).to_le_bytes());
        encoded.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &raw {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    encoded.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut output = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = width.to_be_bytes().to_vec();
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut output, b"IHDR", &header);
    chunk(&mut output, b"IDAT", &encoded);
    chunk(&mut output, b"IEND", &[]);
    assert!(
        output.len() < 1024 * 1024,
        "these integration fixtures stay inside encoded admission"
    );
    output
}
/// Sample interiors of four broad source bands. This is a colour/alpha and
/// nonconstant/aspect oracle, not a copy of the resampling algorithm.
pub(crate) fn assert_scaled_pattern(width: u32, height: u32, rgba: &[u8]) {
    assert!(matches!((width, height), (512, 128) | (128, 512)));
    assert_eq!(rgba.len(), (width * height * 4) as usize);
    for (band, colour) in COLOURS.iter().enumerate() {
        let (x, y) = if width >= height {
            (width * (2 * band as u32 + 1) / 8, height / 2)
        } else {
            (width / 2, height * (2 * band as u32 + 1) / 8)
        };
        let offset = ((y * width + x) * 4) as usize;
        let expected = if colour[3] == 0 { [0; 4] } else { *colour };
        assert_eq!(
            &rgba[offset..offset + 4],
            expected.as_slice(),
            "synthetic band {band}"
        );
    }
}
pub(crate) fn tiny_alpha_png() -> Vec<u8> {
    vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 244, 34, 127, 138, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99, 224, 18, 145, 99,
        208, 48, 178, 105, 0, 0, 3, 199, 1, 83, 206, 227, 180, 69, 0, 0, 0, 0, 73, 69, 78, 68, 174,
        66, 96, 130,
    ]
}
