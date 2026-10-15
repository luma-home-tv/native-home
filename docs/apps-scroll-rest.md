# Apps scroll at rest

Status: the 0.28.0 candidate fixes a host-reproduced redraw tail. Native timing,
visual regression and retained deployment remain pending. G2 retains 0.26.0;
C2 is unchanged.

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

The next G2 test will keep the previous frozen workload, sampling, waits and
cleanup procedure. The baseline's one-second diagnostic snapshots leave broad
native-clock alignment bounds. If those bounds cannot support a matched effect,
publish the new profile independently rather than inventing a percentage gain.
No change to firmware, system libraries, Home behaviour or OLED policy is involved.
