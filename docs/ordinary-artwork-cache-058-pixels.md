# Saved 0.58 icon content comparison

A separate saved-only analysis independently decodes six actual cache payloads
and compares the four cached, unfocused Apps icons with their internal PPM and
two SDK BMP captures. The coordinator replay produces byte-identical results.
This is content evidence for those four opaque icons; it adds no TV execution,
Root authority, retained delivery or performance result.

The cache magic, big-endian header length, encoded length and SHA-256 are checked
before independent PNG decoding with Pillow. All six images are 130×130 and fully
opaque. Three available native CPU RGBA digests match the independent decoded
bytes. Three of the four rendered Apps icons have those witnesses; the fourth
has no observed native CPU digest. Cache/frame inputs agree with the earlier
composite audit, and both SDK BMP bodies match its audited command readbacks.

All four selected draws use recorded integer 230×230 rectangles, full UVs, white
tint, radius zero and the existing contain geometry. The analysis computes the
four GL_LINEAR texel weights at fragment pixel centres with edge clamping; it
does not resize through Pillow or search for an alignment.

Of 211,600 quad pixels, 183,988 have four identical contributing source texels.
Their RGB values match exactly. Ideal float64 bilinear predictions across each
whole quad have mean absolute channel errors of 0.021–0.044 and a maximum below
1.320 on the 0–255 scale. These are measured residuals, not an exact GPU precision
emulator. Both SDK captures' four icon rectangles equal the internal PPM crops.

Using every other decoded payload as a wrong-image control yields nearest mean
channel errors of 37.34, 34.45, 113.04 and 59.35. Vertical flips and one-pixel
cyclic horizontal shifts also differ clearly. These comparisons support content
and placement rather than only matching file/texture metadata.

The analysis is retrospective. Its first reader assumed a hex digest where Rust
wrote a byte array; a successor corrected that schema. A second attempt applied
an arbitrary minimum of 500 nonmodal constant-colour pixels. Actual counts are
3,061, 925, 988 and 152, so the fourth image does not meet that floor. The final
analysis reports all four flags and counts, removes the floor as an acceptance
gate, and introduces no lower replacement. Both failed analyses remain preserved;
no TV rerun followed them. Exact RGB matching, residuals and wrong-image controls
remain separate evidence rather than a preregistered general grader.

Report SHA-256:
`e44908aace43b2d26ae4f46740748ce287385a9b6e409724140a254373251cff`.
Analysis-source SHA-256:
`6bd113bc1a0b11eca57d594161453c7aa663599d4396df229f2c054964abcd51`.

Focused Home art, transparency, other dimensions/filter paths, all-app coverage,
the physical panel and performance remain outside this comparison. The original
native operator still exited 1; its successful cleanup remains separate in the
[native record](ordinary-artwork-cache-058-native.md). G2 retains 0.57.
