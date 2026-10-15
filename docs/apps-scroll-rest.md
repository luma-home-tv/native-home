# Apps scroll at rest

Status: 0.28.0 fixes a host-reproduced redraw tail and is now retained on G2
after isolated profiling and a backed-up normal update. The
[device and delivery record](apps-scroll-rest-milestone.json) separates each
qualification. C2 is unchanged.

The [0.26.0 native profile](current-performance-milestone.json) found a paint
burst after Apps navigation. A synthetic 70-app replay now reproduces a specific
cause through the real Shell updates and PlxNative presentation gate. After the
same six arrows and two-second waits, all sixty iterations in the final second
selected a frame even though the framework reported the scroll settled and
visible spring motion absent. Tiny numerical scroll changes still reached the
external grid as discrete jumps.

After stepping the drawer scroll, Luma now uses the existing framework
`idle::settled` criterion and, when settled, `Spring::jump(target)` before handing
the position to `Grid::set_page`. The jump requests a final damaged frame and
clears velocity. Subsequent exact-rest updates are quiet. Target selection,
content-end clamping, focus, input and framework implementation are unchanged.
PlxNative's shared spring and damage rules retain their original attribution.

The replay selects zero frames in that final second after the fix. Separate
checks cover the final damaged frame, high velocity near the target, retargeting
during motion, nonzero content-end rest and caption containment. All 190 library
tests, clippy with warnings denied, and the ARM release build pass.
[Host evidence](verification/apps-scroll-rest-host.json) records scope and trace
hashes. The test is `apps_six_arrow_replay_stops_subvisible_scroll_tail_presentations`
in `src/shell_tests.rs`; run it with `cargo test --lib` and `--nocapture` to retain
the synthetic trace. No draw or GL call is part of that replay, so its selected
frame count is not a TV performance or pixel result.

The G2 test kept the previous frozen workload, sampling, waits and cleanup
procedure. Its input-free Apps window submitted six frames and used about 1.89%
of one core, compared with the earlier independent profile's 247 frames and
4.88%. The baseline's diagnostic snapshots leave broad native-clock alignment
bounds, so the prespecified strict comparison rejected a percentage-effect
claim. These are independent profiles, not a claimed percentage gain.

The new whole-session draw/active-interval maxima reached about 136 ms, already
present in the first Home sample. That outlier remains recorded; its exact frame
and cause are unproven. This fix does not establish a blanket latency improvement.
The normal update verified all 32 bundle entries, preserved saved-state bytes/metadata and
passed a visually reviewed Apps/Search/LG-keyboard/Back smoke. Physical input,
full media behaviour, broader reliability and other conditional states remain open.
No change to firmware, system libraries, Home behaviour or OLED policy is involved.
