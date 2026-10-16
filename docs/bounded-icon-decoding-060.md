# Bounded icon thumbnails: 0.60 source integration

Large app icons can now be decoded and scaled on the existing image worker,
while the renderer still receives at most 512 pixels per axis and 1 MiB of RGBA.
This fixes the source limitation found by the partial 0.59 G2 batch. Source and
host validation pass; matching ARM builds and native qualification are pending.
The last verified retained G2 remains 0.57. C2 was not contacted.

## Shared policy and attributed framework

Luma's `icon_decode` policy is used by the fresh artwork worker, ordinary packaged
icon reader and durable-cache decoder. The original encoded bytes, digest, file
identity, full source key and producer association retain their existing meaning.
Cache generations, post-decode withdrawal, cancellation/deadline checks and GL
ownership remain in their existing paths. No codec or file work moves onto the
UI thread, and no extra image worker is introduced.

The new opt-in `img_decode_owned_thumbnail_bounded` API extends the explicitly
attributed PlxNative `plx_gfx::img` module. Existing decoder APIs and poster
defaults are unchanged. The tracked integration patch records this generic
upstream contribution candidate; it has not been submitted upstream. Luma adds
no Plex features and does not claim the framework as original Luma code.

Encoded input is limited to 1 MiB. Both source axes must fit 2048, and both native
decoded pixels and the eventual source RGBA buffer must independently fit 8 MiB
before full pixel allocation/conversion. Output limits are checked separately
before allocation. Small images preserve their original RGBA bytes without
upscaling. Larger images retain aspect within integer-pixel rounding.

The CPU filter uses disjoint integer source boxes, alpha-weighted colour and mean
alpha. It allocates a bounded output without a full-size floating-point or
premultiplied intermediate. This approximates fractional-area resampling in
gamma-encoded colour; it is not a linear-light or exact GPU filter. No EXIF
orientation transform is added. High-frequency artwork quality remains open.

These bounds are not a process-memory guarantee. Native and converted buffers
can coexist, as can encoded copies, codec bookkeeping, result buffers and the
existing graphics allocations. Codec allocation limits are best-effort, and
allocation aborts remain outside panic recovery. The 96 MiB package declaration
does not prove safe total memory use or bounded shutdown time. Actual ARM memory
and latency measurement still precede delivery.

## Validation and next step

All 744 library tests, 10 UI tests, 84 Python packaging/provenance tests and strict
Luma Clippy pass against the same 134 selected input hashes. Seventeen tests pass
in the actual framework decoder crate. The [milestone](bounded-icon-decoding-060-milestone.json)
binds the source inputs and logs. Independent source reviews cover the policy,
three callers, regression fixtures and attributed patch boundary.

Six new production-path tests use complete synthetic PNGs through the actual
worker, packaged-file reader and durable-cache snapshot. They check wide/tall
scaling, nonconstant colour and partial transparency, original encoded
provenance, late generation withdrawal and refusal of valid complete 2049-axis
images. Tiny transparent pixels retain their exact bytes. Synthetic provider
success retains Unknown settlement and never invents Root closure.

The independent noninteger-ratio fixture scales 1079×719 to 512×341. Exact colour
and alpha interiors match, source ranges are preserved, and seven flipped,
one-pixel-shifted or erased-region controls fail the same invariants. Against a
separate continuous premultiplied-area oracle, channel MAE is 0.05421, RMS is
1.23623 and maximum error is 32.69694 on a 0–255 scale. The geometry-derived
whole-image MAE bound is 0.40444. These are synthetic CPU observations, not native
panel or arbitrary-artwork quality proof.

The first integrated library run preserved one failure: an old test required a
valid 513-axis image to fail. That expectation moved to the actual new 2049-axis
refusal boundary, with separate positive larger-image tests. A separate strict
lint attempt across the framework remains unsuccessful on upstream C-ABI/style
diagnostics and two new test-only `chunks_exact` suggestions. No framework-wide
lint-clean result is claimed, and unrelated upstream code was left intact.

Next: freeze matching ARM dashboard/helper artifacts, inspect their packages and
ABI, then run an isolated G2 batch and ordinary consumer with a fresh baseline,
lease and verified restoration. Host success does not establish native memory,
pixels, all-catalogue coverage, normal performance or retained promotion. LG Home,
core services and OLED protections remain untouched; normal interaction keeps
Root artwork off.
