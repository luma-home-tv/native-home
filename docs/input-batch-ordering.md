# Input batch ordering candidate (0.29.0)

Luma now drains queued SDL input at the current controller time before advancing
its deferred hold and tap decisions. This lets a queued Back or focus-loss event
cancel an action due in that frame. Back from Home ends input ingestion, clears
unsent requests, and prevents subsequent activation. A pointer click cancels an
older keyboard press, including on the same focused card or when the click misses.
Hovering the same card preserves a keyboard tap.

Host regressions reproduced the previous failures: a same-card pointer click plus
keyboard release emitted two application requests; a due Home hold changed
favourites before queued focus loss; a due Apps tap launched before queued Back;
and a click after Home Back still requested a launch. The corrected paths pass.
An uninterrupted hold still opens the Apps menu, and a newly queued press starts
at the current frame time. Tests use the real Shell and shared focus/press/hit
machines without a GL context or TV actions.

Validation: 193 library tests and 5 home-preview binary tests passed; host clippy
with all targets and warnings denied passed. The ARM release build also passed (the existing target-feature warning remains).
Physical G2 qualification is partial, as described below. The installed normal
preview remains version 0.28.0 until a separate backed-up update is verified.

This change does not add SDL event timestamps, calibrate physical hold duration,
change the framework's 500 ms release/hold boundary, or fix controller-time drift
caused by clamped frame deltas. Native observer event origins remain unclassified.
No LG Home, boot, firmware, settings adapter, or framework dependency is changed.

## Isolated G2 attempt boundary

The 0.29.0 Proof installed and rendered under an ordinary app UID. During the
first attempt, the human reported that pointer selection opened Apps; native
observations independently recorded pointer motion, a click, and the Apps page.
The host acknowledgement publisher then hit a filename mismatch and missed the
phase deadline. This is not a completed workflow or a native crash. The versioned
publisher fix passed a real temporary-file integration test; a second finite run
expired awaiting input with no pointer click recorded.

Both attempts removed the isolated app, app state and mounts, left no bound app
process or keyboard, and verified all 30 failure-closeout checks. Retained normal
0.28.0 package, favourites and metadata were preserved, along with the checked
core files, input and setting baselines. Capture temporaries, upload, lease and
private SSH agent were cleaned up. Private device records remain outside Git.

Wheel navigation, physical remote-key hold/menu cancellation, pointer Search and
LG keyboard Back remain unqualified on 0.29.0. No normal update was attempted.
The corrected publisher has 15 host harness checks; these do not substitute for
a completed physical run.
