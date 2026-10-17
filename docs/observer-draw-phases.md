# Opt-in draw phase diagnostics

This source adds Phase A of the passive observer investigation. It does not
identify the cause of an earlier outlier or establish a performance improvement.
Host traces compare rendering operations, not pixels or GPU completion.

`Shell::draw` and `Shell::draw_observed` use one generic renderer. The ordinary
entry instantiates a zero-sized no-op probe with no clock or collector. The
ordinary loop selects no diagnostic allocation, input context, queue submission
or output without the installed observer marker. Existing ordinary clocks are
unchanged. This is a source invariant, not a claim of measured zero machine-code
overhead; no target build or disassembly was performed.

Only the strict installed Preview observer marker selects four ordered,
nonoverlapping wall spans: clear through `frame_clear`, chrome through rail and
status labels, content through painting before `into_stops`, then hit-map/footer
finalization. The enclosing draw timer retains its existing counter increment;
positive unattributed time remains explicit. Missing or unordered cuts, clock
regression, arithmetic overflow and incoherent sums are reported. Wall spans
include diagnostic overhead, scheduling and driver waits. They are not CPU-only
or GPU, photon, pointer or physical-remote timings. Input/tick and thread CPU
fields remain unavailable.

The fixed collector retains at most eight draws of at least 16667 microseconds,
ordered by descending draw duration and then earliest frame sequence. It reports
seen, retained, omitted, context-unavailable and saturation counts. Context is
limited to safe page/key enums and first-draw/page-change booleans. No dynamic
app/focus/category/text, query, path, pixel hash or texture identifier is copied
into the new diagnostics or snapshot envelope.

The observer uses marker schema 2 and snapshot schema 2 with the explicit
`preview-passive-observer-draw-v2` profile. Schema 1 markers and snapshots are
rejected by the new strict readers; the old strict native acceptance contract
must not accept or grade these outputs. Lifetime 240 seconds, snapshot interval
1000 milliseconds, Root-off selection, normal workers and cache provenance
selection are unchanged. This does not alter an existing carrier or admit a new
native session.

A fixed typed summary accompanies the existing monitor data through the same
nonblocking capacity-one worker. The worker converts and encodes new diagnostics,
validates the entire strict envelope and enforces the complete 16384-byte cap
before opening a temporary file. Unknown or malformed fields fail publication.
Monitor histogram/report construction still runs on the app thread after draw
and swap. One final nonblocking submission uses the same slot; a full or closed
slot drops it without retry. The terminal report states the last successfully
published frame count and claims diagnostic completeness only when it matches
the final collector count and the writer joined without error. This reports
coverage of retained top records, not a complete per-frame trace. Existing file
identity, publication, error, join and cleanup boundaries remain in force.

The client dev dependency enables the existing framework painter census solely
for host tests. The attributed PlxNative revision and integration patch are
unchanged. Synthetic fixtures compare ordinary and observed primitive/rectangle,
hit-map and presentation state traces, suppressing framebuffer clears through
an existing framework recording helper. This does not qualify native drawing,
fonts, textures, GL timing, a physical display or release-code overhead.

Cargo dev-feature semantics, marker/schema and selector changes require a fresh
resolved dependency graph, matching executable/producer descriptor, accepted-cache
strategy and package audit before any native qualification. No previous compiled
source, native receipt, cache association or rollback authority transfers to this
branch. ARM/SDK builds, packaging, device execution, retained installation and
matched uninstrumented baseline comparison remain unperformed. The separate
framework instrumentation follow-up is not implemented.
