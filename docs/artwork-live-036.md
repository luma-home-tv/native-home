# Isolated live artwork lifecycle (0.36.0)

An isolated C2 run qualified the ordinary native worker, decode/cache and GL
texture lifecycle for one owned synthetic application. It read and displayed an
initial icon, refreshed changed bytes at the same path and version, refreshed a
package update, then removed the source and its CPU/GPU residency. The worker
joined and its outstanding work resolved. This extends the
[0.35 encoded-transfer result](artwork-transfer-035.md); it does not deliver live
artwork to the retained preview or qualify a production resource launcher.

## Worker and texture ownership

One worker thread creates and owns its provider, performs ordinary file I/O and
decodes images. The direct provider uses the sealed app/source metadata and
component-wise file reader; it does not start a root helper. A job must match its
full source key. Actual encoded bytes are bounded to 1 MiB, checked against file
identity and independently hashed before an unchanged-content shortcut. The
bounded framework decoder admits at most 512 pixels per axis and 1 MiB of RGBA.

The cache retains generation/source identity across catalogue updates. Removed or
replaced entries reject stale results; a same-content refresh preserves its
resident revision. Only the main thread creates, replaces and deletes GL
textures. A new image changes card drawing references without rebuilding focus
or layout. Each frame permits at most one upload attempt. Suspension drains
results into bounded pending storage without dispatching or applying artwork;
shutdown joins the worker, resolves cache ownership and deletes live textures.

Configured limits cover one outstanding worker job, eight queued cache requests,
two pending results and 8 MiB of reserved resident pixels. They do not measure
total heap, RSS or GPU memory. Cancellation/deadline checks surround I/O and
decode; they cannot interrupt a blocked kernel read. Joining is cooperative and
has no hard bound against such a blockage.

Rendering, focus/input, image decoding and platform support build on the
explicitly attributed [PlxNative framework](../README.md#plxnative-framework-dependency)
by Gleb Linnik and its contributors. The generic bounded decoder remains part of
the separately identified framework integration patch. Luma Home's worker,
cache/controller and lifecycle proof are application code; no Plex features are
included.

## First attempt and capture correction

The first isolated attempt failed the first frame's colour predicate. Its carrier
validated the framebuffer before saving it, so the failed frame was not retained.
That result cannot distinguish a rendering issue from a capture/region issue and
does not qualify the GL lifecycle or graceful worker shutdown. Its saved-evidence
failure audit passed 539 assertions: all 30 failure cleanup checks were true,
with no closeout errors. Nineteen preservation domains had saved before/after
replies; several critical final observations, including Core hashes, retained
bytes/metadata and process absence, were recorded only as checks by the executed
driver. These evidence limits remain explicit.

The subsequent source draws a fresh frame and captures it before swapping the
buffer, following the established capture order. The versioned carrier saves
exact native stage JSON before frame retrieval, then saves the framebuffer bytes
before schema or pixel classification. A failure therefore retains evidence for
inspection. The region and colour predicates were unchanged. The successful
second run does not retrospectively diagnose the unsaved first frame.

## C2 result

The second isolated run completed all four phases on webOS 24. Its 128 × 128
opaque synthetic icons gave the following independently recomputed framebuffer
interior measurements:

| Phase | Change | Mean RGB | Expected-colour pixels |
| --- | --- | --- | --- |
| A | Initial installed icon | 224 / 36 / 36 | 100% |
| B | New bytes, same path and version | 36 / 224 / 36 | 100% |
| C | New icon through package update | 36 / 36 / 224 | 100% |
| Removed | Supported fixture uninstall | No resident target texture | — |

The three captured interiors had distinct digests. Actual fixture PNG bytes and
known raw RGBA were independently reconstructed; native decoded digests and
source keys matched them. Focus and own preferences stayed unchanged through
the three image phases. A later direct comparison of the original RGB bytes
also verifies that every pixel outside the fixture icon is unchanged between
A, B and C; see the correction below. The final receipt reported four uploads, four deletes and
zero upload, deletion or observed GL errors. The worker joined without panic,
resolved outstanding work and left no live textures, resident pixels, pending
results, queued work or in-flight work. The native component reported 18,579 ms;
this is not a UI latency or performance measurement.

All 24 positive cleanup/preservation gates passed, covering 20 selected state
domains and five selected Core hashes. The retained C2 preview remained 0.20.0
with its saved favourites and byte/metadata checks preserved. The synthetic
fixture was never launched. Both isolated application registrations, Proof
process/runtime/jail/state, owned uploads, the device lease and private host agent
were verified removed. LG Home was current; no reboot was required. Retained
final byte/metadata checks partly rely on the executed driver's fresh equality
checks rather than separately saved final bytes.

The run used the user's dated C2 window. The executed source enforces the
latest-start, phase and cleanup cutoffs; independent UTC telemetry for every
individual contact was not saved. No root resource broker was used.

The success audit replayed 912 assertions, including 84 committed build inputs,
artifact/build receipts, six raw process-identity records, four stage/frame
bindings, actual image regions, both fixture installer streams and the complete
cleanup receipts. Its result hash is
`8c62ae2157f41b3c550422b1055ac46f92e241ab0cd6718fd0777406c85f3094`.
The [scoped milestone record](artwork-live-036-milestone.json) records the artifact
identities and checks. Raw catalogues, frames, app details and household state
remain private.

## Correction to the initial visual reading

An earlier reading of repeated image previews incorrectly reported missing
header and app-label text in B and C. Direct checks of the original P6/RGB
bytes contradict that reading: each differs from A at exactly 52,832 pixels,
all inside the fixture rectangle `[758, 340, 988, 570]`. The other 2,020,768
pixels are identical, including all header, logo and label content. The
header region `[96, 60, 1200, 136]` is byte-identical across A/B/C/removal and
all six later 0.37 frames, with SHA-256
`2dc0b0c047b5df14943d3e39e2a8f84ce137112ea25149123eb2cbacb30cdc38`.

The independent direct-byte recheck and a separate coordinator replay agree;
the saved recheck receipt SHA-256 is
`6aaaff726e600d8709a011d514b13b57f3383c55a1d3a670252ed5256a6cf23c`.
The original frames and 912-check lifecycle audit are unchanged. The missing-text
report is withdrawn; these bytes do not justify a text-renderer fix. This
controlled pixel-preservation result is still narrower than complete product
or physical-panel acceptance.

## Remaining scope

The runtime in this proof is deliberately filtered to the owned fixture. Other
cards showing initials in its drawer do not demonstrate a failed attempt to
load those applications' icons. This result establishes neither class-wide icon
access nor coverage of every installed app. The protected-resource route,
production startup and a backed-up normal rollout remain separate work.

The requested design follow-up is to show real icons without a graphite backing
plate, using initials only when artwork is missing. That styling and broader icon
integration are a subsequent slice, not part of this result. Physical remote
input, sustained performance, idle power and full release readiness are also
unqualified by this test.
