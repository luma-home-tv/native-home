# Current dashboard measurement

Status: the frozen 0.26.0 performance-only Proof profile completed on G2, with
[current results and complete removal/preservation](current-performance-milestone.json).
The roughly 199-second run collected 73 process/native samples. The old
[0.14 record](responsiveness-milestone.json) remains historical evidence, not
current dashboard performance proof. The later
[0.28.0 profile and normal delivery](apps-scroll-rest-milestone.json) records the
next independent result; G2 now retains 0.28.0. This document preserves the
0.26.0 measurement's artifact and interpretation boundaries.

The measurement uses functional source
`7f29884be4c5a0c48794e71110744676c78bb371`, executable SHA256
`d63472f5264f6a2c696b1ef6bdddc2ea811771bfb9174582befd74dad22c71bf`,
and an isolated ordinary-app package with only interactive and performance
markers. It has no action proof, keyboard observer, synthetic text recipe or
capture marker. The 0.27.0 row-description and 0.28.0 scroll-fix candidates are
different artifacts and are not mixed into this measurement.

The sampled process CPU midpoints were 1.61% of one core for Home, 4.88% for
Apps, 1.49% for Controls/Sound, 1.22% while blanked and 1.89% after wake. These
come from approximately 7.4–7.6-second CPU endpoint intervals inside the host
holds, with retrieval bounds in the record. Peak sampled RSS was 38,512 KiB;
process lifetime HWM was 38,848 KiB. Black idle submitted no further ordinary
frames. Apps submitted 247 frames in its input-free window, concentrated in
the earlier samples; only one appeared between the final two samples. Its
animation-free steady state is not established, and its CPU average must not
be presented as steady-state cost. Investigate the burst before optimising.

## Workload and resources

Use a finite Home → Apps → Controls/Sound navigation sequence, settled windows
of at least eight seconds per view, then the actual native two-minute idle,
eight seconds after blanking, wake-only Back and ordinary Back exit. Require
fresh native page/category/focus and expected key counters at each boundary.
Do not capture screenshots during the timed run. Stop on unrelated foreground,
overlays, unexpected input, identity drift or inadequate exit margin.

Bind every process sample to its ordinary UID fields, PID, start ticks and
executable hash. Save raw process stat/status observations with host time
brackets. CPU is `delta(utime + stime) / observed CLK_TCK / elapsed time`, as a
percentage of one logical core, with bounds for retrieval uncertainty. This is
the measured process, excluding companion TV services and the SSH daemon. Use the
same sampling cadence across phases and report sample counts and durations.
Report maximum sampled RSS over every phase sample and the process lifetime
high-water mark separately; neither establishes leak freedom.

## Metric boundaries

- The existing `dequeued_input_to_next_swap_submission` name is historical.
  `Monitor::presented` receives its timestamp **after** `window.swap()` returns.
  The actual interval ends after that return. It excludes transport before app
  dequeue and does not measure GPU completion or panel photons.
- `Monitor` retains the latest dequeued key before a measured frame; subsequent
  keys overwrite it and suspension clears it. Latency samples are not one per
  key. Missing samples cannot be counted as lost input.
- Frame intervals cover consecutive actively painted iterations. Deliberately
  skipped iterations clear the history. Their percentiles are not whole-window
  FPS, physical refresh or comprehensive stall coverage.
- The one black-idle clear/swap is counted separately from ordinary frames,
  draws and swaps. Stable ordinary frame counts after blanking do not mean that
  no presentation ever occurred.
- Settled views may repaint for live updates and the framework keepalive. Low
  settled submission rate is expected; zero is not a required result.
- Cumulative histograms provide whole-session percentiles. Phase comparisons
  may subtract integer counters, but never percentiles or maxima.

Diagnostic snapshots, SSH sampling and process inspection add overhead. The
isolated Proof profile also differs from the normal app's enabled action
workers. Report the measured profile directly; do not infer normal-app total
cost, physical smoothness or improvement against another version, LG Home,
PlxNative's Plex app or a WebView without a matched comparison.

Package removal and retained favourites/core/settings/source preservation remain
required after measurement. Preserve failed observations and unknown outcomes;
neither a local installer timeout nor transport EOF authorises a conflicting
rollback/removal. Keep household inventories, raw samples and frames private.

## Next investigation

The later [host replay and fix](apps-scroll-rest.md) confirmed the numerical
tail through the real Shell/gate path. Its separate 0.28.0 device profile and
normal update are recorded without a percentage-effect claim.

Source inspection gives one plausible explanation for the early Apps paints.
Luma steps its external drawer-scroll spring each tick, then passes its position
to PlxNative's `Grid::set_page`. That calls the framework's `Spring::jump`, whose
exact float-change check reports damage. Small numerical scroll changes could
therefore keep invalidating after Luma's visible-motion threshold is crossed.
The finite paint burst is consistent with that path; this run did not record
scroll position, velocity or dirty reasons, so it does not prove the cause.

Replay the same arrows on host while inspecting those fields and the real
presentation gate, then compare a matched window after settling. Preserve the
shared framework's change-reporting contract and the intended smooth scrolling;
do not suppress damage or alter the framework solely from this hypothesis.
