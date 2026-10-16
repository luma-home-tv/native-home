# Ordinary artwork cache in 0.58

The 0.58 source connects the durable icon cache to the native dashboard. Normal
interactive Preview uses its existing image worker for fixed producer-profile
loading, cache indexing, bounded reads and decoding. Cached artwork retains its
own alpha and aspect on OLED black; unavailable data falls back to packaged art
and then the existing missing-art presentation. Normal navigation creates no
privileged artwork runtime or helper.

This is a candidate, not a retained TV delivery. The G2 still has verified 0.57;
the C2 remains unchanged. Native population, cache-hit presentation, performance
and promotion require matching artifact and device evidence.

## Association and freshness

The worker loads one fixed packaged producer file and checks the actual dashboard
executable. The packager generates that file after both dashboard and helper
builds, using their actual receipts, shared source revision/snapshot and matching
Git input blobs. Normal and diagnostic packages share the same executable and
producer-file bytes. The file contains no command, path selector or Root grant.
Missing or invalid producer data leaves packaged artwork usable.

Each image remains associated with its complete current SourceKey, independently
expected producer, selected record digest/length and adopted generation. Late or
withdrawn results cannot reach GL admission. The existing single worker, bounded
job/result slots, render-thread texture ownership and joined shutdown remain.

Shell retains an independent full source map and its own 15-second wrapping
clock. Failed, partial, simulated or expired catalogue data withdraws those
associations even when the privileged runtime is absent. Equal updates renew
the clock without resetting the worker or missing-art state. Withdrawal removes
borrowed drawing references immediately, including while paused, without
polling, uploading, deleting textures, rebuilding focus or writing preferences.
Actual texture deletion remains with the controller.

## Separately admitted population

Only the existing explicit Preview artwork diagnostic and successful preflight
can select the optional population factory. The factory loads its own expected
producer file on its existing worker, checks the diagnostic marker and opens
ordinary cache storage before any fetch. Existing factories remain publisher-free.
A producer file by itself cannot create a Root helper.

Publication borrows a successful encoded result only after authenticated positive
transfer, joined observation, strict lifecycle/readback grading, current-Ticket
finish/sync and successful required result writing. Final cancellation/deadline
checks still apply. Cache and optional publication-receipt errors do not change
the image result, quarantine or privileged settlement. Persistence precedes the
ordinary image decoder, so a stored encoded entry is not proof of decoded pixels.

The new executable/helper pair needs its own native positive qualification.
Earlier PlxNative/Luma experiments and the ordinary held-window result do not
establish these new permissions or transfers. No firmware, Home/boot policy,
system library, jail policy, media service or OLED protection is changed.

## Bounded observation for device testing

The explicit normal session-capture mode can save the first eligible cached
Home frame and Apps frame, at most two in total. Metadata intersects actual
drawn/clipped cards with a current owned cache texture, its full fresh source
and exact handle/dimensions. It is sampled after draw and before swap, alongside
the existing framebuffer capture. JSON requests private mode 0600 and is capped
at 128 KiB including its final newline; the existing framebuffer writer requests
0640. Actual installed modes still require readback.

This observation adds no wait, refresh, navigation or worker. Ordinary sessions
without the capture marker perform none of this capture I/O. Metadata readiness
means an expected producer and index were adopted, not that every icon loaded.
Upload counters count successful owned uploads. Foreground/process identity,
framebuffer contents, Root provenance and cleanup remain independently checked;
capture work is not a performance measurement.

## Verification and remaining work

The coordinator ran all 711 Rust host tests (701 library and ten UI cases), strict
Clippy and 72 Python packaging/provenance tests successfully. These include real
cache files/PNG decoding, held-result withdrawal, initialization failures,
fallback, source expiry, paused reference removal and fixed capture budgets.
Host Shell tests use packaged fallback because the native producer reader is
unsupported on the host; cached-hit files are tested separately in the worker.

A separate saved-byte parser passes twelve fixtures and an actual production
Rust-writer host capture. Canonical filename derivation, body hash/length and real
nonzero host file ownership agree. Its first offline dependency resolution was
rejected before compilation; the admitted harness uses only production lock
versions. This establishes host format compatibility, with every native/Root/
pixel/cleanup authority flag remaining false.

Matching ARM dashboard and helper builds pass from functional source
`aca11408628ea74e651ae0ed9427ca077918bb38`, with the same 124 selected input files.
The dashboard is 4,345,392 bytes, SHA-256
`260c42ff908adacba4c60c64b7137dc56a510c55e86e26ee78aed4c7b5a62441`;
the helper is 908,124 bytes, SHA-256
`a738757aad017b5eaae6de61c427aa5e34e93407abbc06d6ff4aa1fbe73faf43`.
Both normal and diagnostic 0.58 packages pass an independent local source,
SDK `ar`/USTAR and `readelf` audit, including the exact retained 0.57 rollback
archive. Normal and diagnostic dashboard/profile bytes match; ordinary payloads
contain no helper, protected pins, service or boot hook. Archive ownership/modes
are payload facts and still need installed readback.

The dashboard's direct loader/library/GLIBC requirements match retained 0.57.
The helper's requirements match the specifically hash-bound native-qualified
0.46 helper; this is an ABI comparison, not transferred permission or lifecycle
qualification. The first audit incorrectly compared the helper's symbol-version
set to dashboard-only references and refused it. That failure remains preserved;
the corrected auditor uses separate references and passes eight guard fixtures.
The earlier macOS temporary-ancestor guard fixture failure also remains preserved.
The final private audit report SHA-256 is
`8b768a0db92bb6a5138fb9582ba506957acd39446169c749aa61a9b2d893b6d6`.

A separate [read-only G2 prerequisite](ordinary-artwork-cache-058-prerequisite.md)
now passes with independently replayed saved evidence and known coordination
closeout. It confirms the current 0.57 baseline, unchanged preferences, prospective
Home metadata eligibility and the fixed Core reader's precise stat format. It
runs neither new executable and grants no ordinary cache or Root permission.

Backed-up G2 diagnostic population, ordinary cache-hit frames with zero Root
dispatch and exact 0.57 restoration are the next gates. Native testing must use fresh current state and new
leases/dates; the older 0.46 operator is not reusable. Exact restoration means
the retained bundle and preferences, with independent sampled Core checks. The
existing shared runtime event log legitimately appends when Preview starts; its
old byte prefix must remain intact, with appended bytes accounted for. Do not
truncate it or claim whole-runtime byte equality from that test. Other existing
runtime files need their own unchanged-byte or explicitly reviewed mutation rules.
Cache eviction, old-version
retirement after restart and interrupted-slot recovery remain unfinished. Same-key
source-file mutation requires explicit qualified refresh; there is no TTL or
continuous freshness promise.

Luma owns the cache, source associations and dashboard composition. Rendering,
input and the bounded image decoder continue to build on the explicitly
attributed PlxNative framework by Gleb Linnik and contributors. Its pinned
dependency and integration patch remain unchanged; Plex product features are
outside this application.
