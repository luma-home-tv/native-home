# Catalogue preparation planning: 0.59 foundation

The next preparation path can enumerate every preferred icon in the current
admitted catalogue, in batches of at most 16. It no longer needs to derive its
work list from the cards visible on Home or Apps. This is a pure planning
component: it does not launch a helper, read or write an icon, create a worker,
upload a texture, or change the installed preview.

The [0.58 native observation](ordinary-artwork-cache-058-native.md) established
six publications and bounded ordinary rendering evidence. It did not establish
all-app preparation. This component addresses enumeration only; a separate
execution runner and its native qualification are still required.

## Current-source contract

`artwork_cache_preparation::Plan` accepts the independently observed source map
exposed by `Shell::cache_preparation_catalogue`. The Shell view withdraws failed,
simulated or unavailable catalogue/source updates. A separate general catalogue
timestamp cannot renew the source map's freshness.

Every plan creation, batch enumeration and immediate-before-dispatch selection
rechecks the supplied current time against the existing inclusive 15-second
freshness rule. The timestamp and current time must come from the same wrapping
`live_origin.elapsed()` domain. Animation ticks or a reset runner clock cannot
be substituted.

The plan hashes the entire typed source map in deterministic order, including
entries without a preferred image and their rejection metadata. It checks app
identity, version and icon-slot coherence. Each preferred source is the existing
parser's sealed `Source`; the planner does not construct a Root path from a
cursor or public `SourceKey`. An observed stale, unavailable, simulated,
incoherent or changed map permanently withdraws that plan, including an observed
A→B→A change.

The metadata limit is 512 source entries and the enumeration limit is 16
preferred sources per batch. Synthetic cases demonstrate 24 as 16+8 and 512 as
32 batches. These are planning limits. The ordinary cache still defaults to 256
entries and 16 MiB of encoded data; enumeration does not establish available
capacity or promise that every planned image fits.

A bounded, strictly parsed cursor contains a schema, catalogue digest and next
ordinal. It grants only an enumeration range. It does not prove any previous
attempt, publication, helper closure, prepared coverage or continuous generation.
A fresh plan with equal metadata may accept the same cursor; neither that match
nor an unobserved A→B→A change establishes continuity with an earlier process.
Every planning manifest explicitly leaves execution, coverage, generation
continuity and Root authority unverified.

## Validation and boundary

The full host library suite passes 715 tests, including 12 planner cases and two
actual Shell integration cases. The 10 home-preview host tests and strict
all-target Clippy also pass. After the initial lint run rejected an uppercase
letter in one test name, only that identifier was corrected; strict Clippy and
all 12 planner cases were rerun. The original failed lint receipt is preserved.
The selected build inputs and attributed framework remained unchanged during
each successful command. The [machine-readable record](artwork-catalogue-planning-059-milestone.json)
binds source and check logs.

These checks do not qualify a new ARM executable, a diagnostic package, native
catalogue preparation, all-app coverage, performance or either TV. No TV was
contacted for this source milestone. The last verified retained G2 remains
0.57; its next device baseline still requires a fresh read.

## Next integration

The separate explicit diagnostic runner will dispatch one existing worker job
at a time, reselect the exact fresh source immediately before submission, and
match actual publication metadata to that job's known completion. A missing
observation or unknown Root lifecycle must retain its barrier. A cursor will
never serve as an execution ledger. Preparation must stop admitting work on
input, departure, catalogue withdrawal or budget exhaustion and drain/join
existing work before closeout.

Normal interaction continues to use the ordinary image worker with Root artwork
off. Broad preparation, capacity/partial results, same-key content refresh,
cross-process completion accounting, eviction and stale-slot recovery remain
unfinished. No LG Home, boot, firmware, jail, media, network or OLED changes are
introduced. Rendering and decoding continue to build on the attributed
PlxNative framework; this planning component adds no Plex functionality.
