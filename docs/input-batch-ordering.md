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
with all targets and warnings denied passed. Native compilation and physical G2
qualification are recorded separately when completed. The installed normal
preview remains version 0.28.0 until a separate backed-up update is verified.

This change does not add SDL event timestamps, calibrate physical hold duration,
change the framework's 500 ms release/hold boundary, or fix controller-time drift
caused by clamped frame deltas. Native observer event origins remain unclassified.
No LG Home, boot, firmware, settings adapter, or framework dependency is changed.
