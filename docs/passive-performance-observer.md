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
foreground intervals and the latest dequeued input's next swap submission.
Idle, hidden and skipped intervals suspend history. A black idle presentation
has its own count. Context identifies foreground, hidden and idle-black states.

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
complete attributed PlxNative framework remains unchanged. Host checks do not
establish ARM linking, installed metadata, actual normal worker participation,
cache hits, native timing or performance improvements.

Next, build matching ARM dashboard/helper artifacts, verify the observer and
ordinary packages, and run a fresh backed-up G2 diagnostic scope. Require actual
normal selections and cache participation, independently sampled process
resources, known closure and exact restoration of the retained f5 package,
preferences, cache and measured Core state. The separate cache-consumer trial
and its original artifact identities remain unchanged. Neither TV has received
this observer build.

For the optional real-artifact integration test, explicitly set
`LUMA_TEST_PRIVATE_ARTIFACT_ROOT` to the private evidence directory. A configured
but missing or invalid fixture fails; an ordinary public checkout does not need
private TV assets or binaries.
