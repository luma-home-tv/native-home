# Artwork handoff ordering in 0.49

The first slice of the [transition lifecycle](artwork-transition-lifecycle.md)
is implemented and has passed one isolated G2 native component trial. The native
result covers a queued or unpolled worker slot, local keyboard intent, local Back
and polling to classified readiness. It does not start LG's keyboard or hand off
to an external application. Ordinary interactive Root artwork remains off, and
the retained G2 normal installation remains **0.47**.

Shell owns composed pause reasons. Keyboard intent and ownership, external
intent, foreground loss, idle, manual diagnostic pauses and quarantine have
separate owners. Focus return clears its foreground reason; it cannot clear a
keyboard, action or unknown-session pause. Deferred press commits run before
intent detection and the artwork pump, including direct tick callers.

The worker reports initialization and per-job settlement independently of image
success. Until initialization is observed, the runtime drains without starting a
fetch or handoff. An old pending record, initialization failure, unknown
completion or unclassified shutdown remains blocked. A known closed job can
finish without an image. Unknown image results are discarded before image-cache
admission; a bounded failed-result tombstone can remain. Cancellation revokes
image usability without inventing or erasing settlement. No UI-thread filesystem
check, cancellation, blocking join or timer supplies transition readiness.

Keyboard startup has an explicit readiness gate. Eligible waiting does not open
the LG panel or begin its appearance timeout. Genuine visibility, focus or
capability loss retains the existing unavailable/close behavior. External actions
retain one typed intent, anchored at activation. Back or changed intent can
cancel an unsubmitted request. Queue accounting starts at actual submission.
An unverified reply retains the action pause because it can follow an admitted
or uncertain launch. Matching verification and an observed departure/return are
both needed to complete that local lifecycle. The normal path that has never
enabled artwork retains its immediate action and keyboard behavior.

Root ran **505 Rust tests**: 498 library and seven dashboard tests. Real controlled
workers, channel-held jobs, the bounded decoder/cache and shared input-frame
ordering cover initialization barriers, same-frame intent, cancellation,
unknown results through shutdown, pause composition, deferred intent replacement,
matching replies and keyboard readiness. Earlier failed host runs exposed
fixtures that assumed synchronous initialization or had not yet scheduled the
framework's deferred press commit. The repaired fixtures observe actual worker
initialization and release/commit timing; the original failed logs are retained.
All 58 Python checks and Clippy with warnings denied pass. Both actual ARM
binaries were built from the frozen source and the same 102-input snapshot.

In the isolated native trial, the recipe prepared an app-local editor under a
manual pause, observed the first owned worker slot, then set local keyboard
intent before another pump. A required draining sample showed no new submission,
artwork GL upload or recorded runtime cancellation. The recipe added a separate
manual pause before local Back cancelled the editor, then continued drawing and
polling until that work was classified known. It returned to the existing live
Home and Applications observations. This is a scripted Shell/worker component
result; the slot can be queued or completed-but-unpolled, so it does not identify
the HBC stage at the pause. Real LG keyboard and main-loop external handoff
qualification remain separate gates.

The actual carrier and saved-evidence audit passed. **Seven completed transfers**
used seven helpers and 112 known Core child processes. Home showed three fully
visible live targets and Applications four, with **four distinct displayed
targets** across both pages. Transfer count is not distinct-app coverage. Only
the first helper has independent live external process binding; the later six
bind their kernel endpoint, self report and joined HBC completion. Both raw
framebuffers were retained. Independent RGB checks confirm contained nonblack
content and an unchanged branding region; they do not identify the encoded
image pixels or provide a complete visual oracle.

All 24 preservation/removal gates passed. The full retained normal bundle and
private state, favourites, ownership, modes, five Core files and 20 typed domains
were unchanged. Every known helper and child group was absent; the pending record
was independently absent before the supported Proof uninstall. The exact two
protected test files and directory were removed. Recorded own package, state,
runtime, jail, mount and registration scopes were absent; the verified upload,
lease and private agent were removed. This is not an all-namespace filesystem
survey. Existing
ordinary runtime logs remain retained without a new deep-byte preservation
claim.

The saved-only audit passed **28,677 assertions**. These counts describe the saved
audit, not coverage or reliability. Root independently replayed it:
all facts, 245 artifact bindings and the diagnostic-check multiset agreed. The
reports were not byte-identical because diagnostic checks appeared in a different
order. The original byte-comparison wrapper exited 1; the separate semantic
comparison passed. Both audit executions exited 0, and the original comparison
result is retained. The [milestone](artwork-transition-049-milestone.json) records
the source, artifact, audit and semantic replay hashes.

Packaging gives archive directories mode 0755, the selected executable 0755 and
other files 0644 independently of the host umask. Links and special entries are
rejected. An actual restrictive-umask regression covers normal packaging. This
changes archive metadata only; installed SDK metadata still needs device
readback.

The Root server, role policy, durable pending format and allowed platform
operations are unchanged. This slice does not implement the authenticated
closed-negative outcome needed for an external departure. It does not clear an
older pending record, enable a normal provider, qualify C2 Root resources,
physical input, performance, OLED behavior, reliability or a hard lifetime for
blocked calls. Luma Home continues to use the attributed shared
[PlxNative framework](../README.md#plxnative-framework-dependency).
