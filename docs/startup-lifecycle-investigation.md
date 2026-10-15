# Early startup focus loss

Status: observed once on the isolated G2 0.25.0 actual-main-loop candidate;
cause unresolved. The departure policy is unchanged. See the failed second
attempt in the [human keyboard record](manual-keyboard-milestone.json).

The receipt for functional source `a46ddcf9be8d3bf5d06eb6af72ddc531cb547fca`
records shown/focus-gained events at 0 and 125 ms, followed by focus loss at
194 ms. It exits with reason `window_hidden` at 697 ms. The runtime distinguishes
focus loss (event kind 7) from hidden/minimized (kind 8), but the application
intentionally suspends both and applies the same 500 ms departure deadline.
There was no kind-8 event, text input, keyboard opening or external action in
this attempt, and no hidden presentations. This is evidence of an orderly policy
exit, not evidence of a native crash.

No independent compositor/foreground sample covers the 194..697 ms interval.
Later Home/screensaver observations do not identify what took focus or establish
that focus would have returned. Increasing a startup grace period would therefore
be speculative. The successful fourth human attempt used the same frozen binary;
it establishes that the sequence can work, not that cold entry is reliable.

## Bounded follow-up

The [0.26.0 G2 follow-up](startup-relaunch-milestone.json) now records two planned
ordinary interactive sessions with independent startup observations. Each reaches
a fresh focused Home endpoint beyond eight seconds of native runtime. The first
opens the LG keyboard and leaves through a guarded programmatic LG Home launch;
the second starts a distinct process from the same installation and unchanged
saved preferences, then closes its keyboard and exits with network Back. Complete
proof removal and retained/core/settings preservation were verified.

The first host observer profile stopped after six records because it classified
LG's exactly typed launch splash as unrelated. No navigation or second session
followed, and cleanup completed. A separately versioned observer recognizes that
system window only during the timed launch observation, while preserving strict
prelaunch and input ownership checks. The window service supplies no target-app
association; timing is correlation. The failed profile remains unaccepted.

The native binary and its 500 ms departure policy were unchanged between these
attempts. The earlier rare focus-loss exit did not recur. These two successful
sessions establish the bounded sequence, rather than general startup reliability,
continuous focus or the cause of that earlier exit. Further investigations use
the same observation constraints:

Use an isolated immutable interactive package and the ordinary startup path, with
the existing passive native observer and a bounded independent host sampler:

- Record actual host monotonic start/end times around launch, foreground/window
  queries and observer retrieval. Keep native elapsed/event-batch times alongside
  them; do not pretend two clocks or successive service queries are atomic.
- Sample only during a short startup window. Capture app/window roles and native
  focus/hidden events, producer identity, readiness, drawing and exit. Raw device
  material stays private. A diagnostic component's scripted captures cannot
  substitute for this actual-loop observation.
- Preserve the 500 ms departure rule and the existing verified Settings-only
  exception. Do not retry a failed entry, force Home as recovery, retain a hidden
  window indefinitely or change LG services to obtain a passing run. A second
  predetermined session begins only after a fully verified first departure.
- Treat intervening use as an external departure. Close/remove only the owned
  proof and verify preserved state; record a partial result if the observation
  did not cover the transition. A run without the rare event cannot explain its
  cause or close the reliability gap.

If a policy change is supported by that evidence, exercise the production handler
for immediate input/render suspension, recovery before the bound, repeated loss
without deadline extension, persistent-loss exit, immediate close and the narrow
Settings exception. A pure timer fixture alone does not establish native recovery.
