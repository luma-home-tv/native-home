# Bounded ordinary packaged artwork

The 0.54 candidate replaces synchronous first-64 icon loading with a separate
ordinary worker and a render-thread demand cache. It reads only icons already
packaged with Luma. The retained G2 0.47 and C2 0.48 installations are unchanged;
its [isolated G2 rendering/shutdown test](packaged-icon-native-054.md) is now
qualified for four controlled fixtures. Physical input, performance and a
retained update remain separate. Normal Root artwork remains off.

The old loader read each whole file before checking its encoded length and
decoded before checking dimensions. It also omitted packaged images beyond its
first 64 catalogue entries. Those are source-level limitations; the retained
observations did not establish an over-64 failure or measured memory fault.

## Reading and demand

Luma captures the package's app-ID/icon bindings before live catalogue merges.
Only `icons/<leaf>` is admitted. A worker anchors the caller's own assets
directory, opens the icon directory and regular file without following links,
and bounds encoded input to 1 MiB before allocation. A one-byte sentinel and
full before/after file metadata detect sampled length/identity changes. Held and
named directory identities exclude unrelated entry timestamps, while retaining
device/inode/owner/mode checks. Readable installed modes and hardlinks are not
rejected solely as authority predicates.

The worker uses PlxNative's existing bounded image decoder: both axes are at most
512 pixels and the RGBA image buffer is at most 1 MiB. Alpha and aspect ratio are
preserved. Codec bookkeeping limits are best-effort; encoded input, decoded
output and codec buffers can coexist. These limits do not establish a total-heap,
GPU-memory, OOM or hard filesystem/codec shutdown-time guarantee.

There is one outstanding job, including an unpolled completion, and at most one
decoded result awaiting capacity. Jobs bind package-configuration epoch, app ID
and relative path. Changed, abandoned or no-longer-visible results are discarded
before upload. Failed sources are not retried every frame; an explicit package
configuration resets that state. Actual package/root replacement needs a new
controller, normally a new process.

The render thread owns all packaged texture handles. Its cache is limited to
64 handles and a 16 MiB RGBA payload estimate, with offscreen LRU eviction.
All-visible capacity pressure defers without evicting the visible set repeatedly.
It attempts at most one packaged upload per frame, using actual drawn placement,
clipping and the Apps grid's whole-tile/focused exception. The experimental live
store retains its separate upload budget; combined Root-enabled responsiveness
is still a separate gate. There is no prefetch in this slice.

## Interface and lifetime

Known packaged icons stay blank while warming, with a focus outline when needed.
They acquire no graphite backing or monogram merely because their worker has not
finished. A missing binding or known failure uses the existing initials/name
fallback. Available icons keep their own shape and transparency on OLED black.

Texture arrivals and evictions replace only borrowed card references. They do not
rebuild layout, change selection, cancel an armed press or write favourites.
Frontend hidden/idle/keyboard/external-intent states suppress dispatch and upload;
window close latches exit before the input batch can reach its tick. The normal
input path never waits for packaged work. This ordinary loader neither enables
Root artwork nor uses its Ticket, quarantine, provider or transition readiness.

Shutdown drops result/job channels, joins the ordinary worker, then clears owned
textures while the render context still exists. Shell's drawing map owns no GL
deletion. An explicit final report records that shutdown; Drop supplies the error
path. Capture-only diagnostics separately wait for demanded visible images under
a finite deadline. A fixed frame count is insufficient for an asynchronous worker,
and a timeout fails the capture rather than claiming decoded artwork.

## Evidence and remaining work

The [source/build record](packaged-icon-demand-milestone.json) binds functional
source `3833c5b2ad54c1154d7b7cb71aa59a13fe978ffb` to 620 passing Rust tests,
64 passing Python packaging tests, clean strict Clippy and two successful ARM32
builds. The frozen dashboard and Proof helper have identical snapshots of all
110 selected source inputs and the verified framework. Independent source and
scope review is clear. The build record predates the subsequent isolated G2
dashboard test; the Proof helper has not run. These builds do not replace the
separately frozen 0.53 diagnostic artifacts.

The component tests use real temporary files, the ordinary worker and actual PNG
decoder, with a simulated GL ownership backend. They cover bounded/escaped/
symlink/FIFO inputs, stable file reads and directory churn, alpha/aspect, over-64
demand, abandoned/changed epochs, pauses, capacity, aliased/recycled handles,
shutdown, and the actual Shell's focus/press/loading/close behavior. Host checks
do not prove native permissions, graphics-driver deletion or physical latency.

Before a retained update, verify normal flows and G2/C2 independently, and preserve each complete current
bundle, resource set and fresh favourites. Compare matched normal startup,
navigation, scrolling and idle/wake workloads. Newly installed or revised apps
still need a separately qualified fresh artwork route: an old packaged binding
is not a live app-version measurement or all-app byte-access permission.

Luma owns this package-binding, worker/cache and dashboard integration. PlxNative's
explicitly attributed native rendering, input/focus framework and bounded decoder
remain the pinned upstream dependency; no Plex client, player or account features
are introduced.
