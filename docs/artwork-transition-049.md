# Artwork handoff ordering in 0.49

The first slice of the [transition lifecycle](artwork-transition-lifecycle.md)
is implemented in source. It pauses new image work before an app-controlled
keyboard or external-app handoff, then keeps polling the existing job while
navigation and rendering continue. Ordinary interactive Root artwork remains
off. This is host implementation evidence; the native transition trial remains
pending.

Shell owns the composed pause reasons. Keyboard intent and ownership, an
external intent, foreground loss, idle, manual diagnostic pauses and quarantine
have separate owners. Focus return clears its foreground reason; it cannot
clear a keyboard, action or unknown-session pause. Deferred press commits run
before intent detection and the artwork pump, including direct tick callers.

The worker reports initialization and per-job settlement independently of image
success. Until its initialization result arrives, the runtime drains without
starting a fetch or handoff. An old pending record, initialization failure,
unknown completion or unclassified shutdown remains blocked. A known closed
job can finish without producing an image. Unknown image results are discarded
before image-cache admission; a bounded failed-result tombstone can remain.
Cancellation revokes image usability without inventing or erasing settlement.
No UI-thread filesystem check, cancellation, blocking join or timer supplies
transition readiness.

Keyboard startup has an explicit readiness gate. Waiting does not open the LG
panel or begin its appearance timeout. Genuine visibility, focus or capability
loss retains the existing unavailable/close behavior. External actions retain
one typed intent, anchored when the user activates its control. Back or changed
intent can cancel an unsubmitted request. Queue accounting starts at actual
submission. An unverified reply retains the action pause: a failed verification
can follow an admitted or uncertain launch. Matching verification and an
observed departure/return are both needed to complete that local lifecycle.
The normal path that has never enabled artwork retains its immediate action
and keyboard behavior.

Root ran 498 library and seven dashboard tests, including real controlled
workers, channel-held jobs, the bounded decoder/cache and shared input-frame
ordering. These cover initialization barriers, same-frame intent, cancellation,
unknown results through shutdown, pause composition, deferred intent replacement,
matching replies and keyboard readiness. The first combined run exposed older
fixtures that assumed synchronous worker initialization and new fixtures that
had not yet scheduled the framework's deferred press commit. Those fixtures
now observe the asynchronous initialization and actual release/commit sequence.
The original failed logs are preserved privately.

The isolated ARM Proof-mode recipe also has a bounded queued-work observation
candidate: local keyboard intent pauses one owned worker slot, local Back remains
available, and polling must reach classified readiness without new submission,
GL upload or a recorded runtime cancellation request. It never starts LG's
keyboard or an external app and does not claim an observed HBC stage. A missed
draining sample declines through the common drain/join/report path. Every actual
helper/job still needs external lifecycle reconciliation; earlier six-job proof
receipts cannot qualify this new recipe. This native recipe has not run.

All 58 Python tests also pass. Packaging now gives archive directories mode
0755, the selected native executable 0755 and other files 0644 independently
of the host umask; links and special entries are rejected. An actual restrictive
umask regression covers normal packaging. This changes archive metadata only;
installed SDK metadata still requires separate device readback. Clippy passes
for all host targets with warnings denied.

The Root server, role policy, durable pending format, positive image grader and
allowed platform operations are unchanged. This slice does not implement the
separate authenticated closed-negative outcome needed for an external departure.
It does not clear an older pending record, enable a production provider, qualify
C2 Root resources, or establish native input, performance, physical OLED behavior
or a hard lifetime for blocked calls. The retained TV builds use their separately
frozen artifacts; this source has not been delivered to them.
