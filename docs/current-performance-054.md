# G2 isolated dashboard measurement — 0.54

The current performance-only Proof profile completed a native G2 workload on
webOS24, then exited through ordinary Back and was removed. The
[milestone](current-performance-054-milestone.json) binds the frozen executable,
package, original operator outcome and independently replayed saved evidence.
Neither retained preview was updated: G2 remains 0.47 and C2 remains 0.48.

The approximately 208-second native session covered Home, six drawer-navigation
arrows, a settled Apps view, Controls/Sound, the actual two-minute idle threshold,
an idle hold, wake-only Back and ordinary Back exit. There were 17 original SDK
key requests, 89 accepted raw process/native samples and 82 samples retained for
workload aggregation. No screenshots were taken during measurement.

| Phase | Process CPU midpoint, % of one core | Retrieval bounds, % | CPU endpoint midpoint interval |
| --- | ---: | ---: | ---: |
| Home | 2.01 | 1.68–2.49 | 6.47 s |
| Settled Apps | 2.34 | 1.96–2.91 | 6.40 s |
| Controls/Sound | 1.64 | 1.42–1.94 | 7.92 s |
| Native idle blank | 1.37 | 1.14–1.70 | 6.59 s |
| After wake | 1.97 | 1.71–2.31 | 8.64 s |

Each source hold loop runs for at least eight seconds. Its persisted CPU endpoint
interval is narrower and separately bracketed; the table does not claim five
eight-second CPU denominators. CPU covers the measured process, excluding
companion services and SSH. Sampling and diagnostic output impose overhead.
Peak sampled RSS was 35,452 KiB (34.6 MiB); the maximum sampled kernel lifetime
high-water value was 35,568 KiB. These values do not establish leak freedom.

The app recorded 670 ordinary frames and one separate idle-blank presentation.
Ordinary frame counts stayed unchanged through the observed idle hold, and the
first Back woke the same Controls/Sound route. This is source/counter evidence,
without idle pixel capture or panel-power measurement.

The whole-session input-dequeue-to-after-swap metric had 17 samples, a 1.98 ms
mean and a 14 ms histogram p95 upper bound. It excludes transport before dequeue,
physical remote latency, GPU completion and panel response. The monitor retains
only the latest key before a painted frame; it does not guarantee one timing
sample per key. Frame intervals cover actively painted iterations, not
whole-window FPS.

A **183.667 ms draw-submission outlier remains**. It was already present in the
first accepted Home sample, at native elapsed 6.039 seconds and 36 frames. No
later cumulative maximum exceeded it. This does not identify the exact frame or
cause, or establish the absence of smaller stalls. Draw-submission p95 and p99
histogram upper bounds were 2 ms and 19 ms respectively.

An earlier attempt stopped after eight inputs when observer overhead consumed
the remaining-workload reserve. Its ninth input was refused before dispatch;
one supported close ended the owned app, followed by verified removal and
preservation. Its operator exited zero for known cleanup while performance
qualification remained false. The later driver reuses only an immediately
preceding validated packet selected within 200 ms, otherwise reading again.
Focus, key counts, lease and final deadline checks remain mandatory. Recorded
age is at selection, not delivery; global input targeting remains sampled.
This changes test overhead, without establishing a dashboard speedup.

The completed run joined the packaged-image worker, uploaded and deleted seven
owned packaged textures, and reported zero remaining packaged textures or backend
errors. That excludes separate branding/navigation textures and total GPU memory.
One initial preference seed belonged only to the temporary app; its preferences
did not change afterwards. Original retained bundle, private preferences and
runtime fingerprints matched before/after, including 32, 2 and 113 entries
(30, 1 and 75 files). The 19 fixed getter reply groups and five protected-file
hashes matched. Test registrations, paths, mounts and staging were absent, and
the owned lease and host agent closed.

Independent replay reconstructed raw process/scene/namespace samples, CPU bounds,
input requests, idle/wake, native shutdown, recorded removals and preservation.
The coordinator repeated the audit and obtained a byte-identical report. These
are sampled observations and later readbacks, rather than an atomic guarantee
about every intervening state.

This qualifies the particular isolated workload. The Proof action worker had
all application/Home/input/settings capabilities disabled; pointer and
description setters were absent. Normal Preview total cost, physical smoothness,
C2, long-run reliability, all-app artwork and matched comparisons remain open.
No Root artwork broker or HBC session ran. The
[measurement method](current-performance-method.md) records further metric
boundaries. PlxNative's attributed renderer, decoder and input/UI framework remain
the foundation; this profile adds no Plex features.
