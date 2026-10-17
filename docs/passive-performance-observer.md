# Measuring the ordinary dashboard

An opt-in Preview observer can now measure the normal dashboard with its live,
action, pointer, description and ordinary artwork workers selected. The older
isolated Proof profile disables some of those workers, so its results cannot
establish normal-dashboard performance.

The observer uses the existing timing monitor and a separate, explicit package
marker. Default packages create no observer worker or metric files. They do gain
one startup marker-presence check. The observer does not change application
capabilities, privileged artwork admission, persistence, input handling or the
existing Proof selector.

The selected profile lasts at most 240 seconds and offers a snapshot once per
second through a one-slot queue. Monitor summaries and JSON-value construction
remain on the render thread; byte encoding and file I/O run on the snapshot
worker. A full queue drops a snapshot rather than waiting for I/O. The worker is
joined on normal completion. These bounds do not guarantee total heap use,
kernel I/O latency, shutdown time or recovery from an aborting native panic.

The installed marker must be Root-owned. Each publication checks the original
held and named private output directory and the held/named temporary and current
files. Observed ownership, mode, identity or epoch changes refuse publication;
the observer does not repair permissions or claim cleanup authority. These
checks are not atomic against another owner changing a path or its permissions.

## What the numbers mean

The monitor reports application-thread draw submission, the swap call, painted
foreground intervals and the latest dequeued key-down's next swap submission.
Idle, hidden and skipped intervals suspend history. A black idle presentation
has its own count. Context identifies foreground, hidden and idle-black states.
The input measurement does not cover pointer or wheel events, and multiple
key-downs before a presentation do not each receive a separate latency sample.

These are whole-session summaries. Subtracting percentile summaries does not
produce a phase percentile. The observations do not measure GPU completion,
physical remote latency or panel response. Rejected snapshot submissions are
counted; that count does not describe every accepted-but-unpublished snapshot or
lost input. Native source elapsed time is separate from the coordinator's clock.

## Current validation and next step

The integrated source passes 798 library tests, 15 UI tests, strict all-target
Clippy and 87 Python package checks. The portable contract suite runs 16 cases,
with one explicit skip for an optional private ARM-artifact fixture. That actual
private-artifact test also passes when configured separately. Its former guessed
filesystem path failed in this integration checkout; the fixture-only repair
preserves all production guards and removes an unrelated parent-directory write.

Independent review covers the original design and subsequent marker/publication
metadata fixes. All 143 functional inputs match the reviewed source, and the
complete attributed PlxNative framework remains unchanged. Host checks alone do not
establish ARM linking, installed metadata, actual normal worker participation,
cache hits, native timing or performance improvements.

Both actual ARM release builds now pass from immutable functional revision
`d2215d5d8761fe385ae7ae3dd76ab173793c5bb8`, with matching build receipts and
unchanged 143-input/framework snapshots. Independent checks match both actual
Cargo compiler descriptors to those receipts, and both direct SDK ABI sets are
subsets of their matching retained-build references. Nineteen artifact-gate
guard cases pass. This is local linking/contract evidence, not full transitive
loader, installed-permission, native-execution or performance evidence.

Both private packages are now assembled from those same actual binaries. An
independent complete archive/control/payload audit and 36 guard cases pass. The
ordinary package has 32 files across 34 application attributes; the observer has
33 across 35. Their existing payloads and 29 approved private resources are
identical; the fixed observer marker is the sole payload difference. Archive
ownership is distinct from installed metadata and grants no TV permission. The
build runner's original stop before packaging is preserved separately from the
later package step. Private resources and binaries are not published.

The [current ordinary prerequisite](current-ordinary-g2-native.md) now passes its
own matching G2 route and restoration. The
[first instrumented G2 attempt](passive-performance-observer-g2-first-attempt.md)
collected partial app-local timing data but reached the original marker limit
before full route acceptance. Independent saved review and coordinator replay
verify exact retained f5/state/cache/Core/runtime preservation and scoped cleanup.
The retained installation remains f5; the C2 has not received this observer.

The later [bounded navigation observation](passive-performance-observer-g2-native.md)
and separate [idle/wake observation](passive-performance-observer-g2-idle-wake.md)
now pass their own matching native routes, final output/state associations and
independently replayed restoration/cleanup. The intervening
[second partial attempt](passive-performance-observer-g2-second-attempt.md)
preserves coordinator review delay and grants no batching-speed comparison.
Both successful scopes are closed and retained f5 remains unchanged. They are
separate sessions, not combined full-product, physical-remote, GPU/panel, matched
observer-overhead or performance-improvement qualification. Ordinary retained
delivery, current physical input, daily actions/restart and broader reliability
remain separate work. The earlier cache-consumer identity/result stays distinct.

For the optional real-artifact integration test, explicitly set
`LUMA_TEST_PRIVATE_ARTIFACT_ROOT` to the private evidence directory. A configured
but missing or invalid fixture fails; an ordinary public checkout does not need
private TV assets or binaries.
