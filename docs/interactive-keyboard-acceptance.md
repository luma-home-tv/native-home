# Interactive keyboard acceptance

This is the acceptance contract; individual results live in the linked receipts.
It was originally prepared from source `16b0194`, the [keyboard integration record](keyboard-integration-milestone.json)
and the [text compatibility audit](webos24-text-compatibility.md). The recorded
0.23.0 trials on both models exercised shared search/controller code inside a
bounded diagnostic helper. They establish the operations listed in that record;
they do not establish that the normal interactive event loop wires them together
correctly. The older human G2 typing report also does not qualify a newer binary.

Current results: the [actual-loop record](main-loop-milestone.json) covers
independent keyboard/fallback, idle and programmatic departure scenarios on both
models. The [0.25.0 OK-wake record](ok-wake-milestone.json) adds first-OK wake
suppression and a second distinct OK action on both. The
[current G2 human check](manual-keyboard-milestone.json) covers typing/Done,
same-process reopen/Back and one observed same-batch release drain, with no
fallback keypad. These are bounded results, not full acceptance: C2 physical
typing, pointer/later-batch dismissal races, performance
and retained promotion remain open. One failed G2 startup exited on early focus
loss before readiness; keep its unresolved cause and broader reliability explicit.
The [0.26.0 G2 startup/relaunch record](startup-relaunch-milestone.json) now adds
two planned actual-loop sessions: keyboard-active programmatic Home departure,
then a new process loading the preserved installation/preferences and completing
network Back close/exit. This profile remains unperformed on C2. Its successful
bounded startup observations do not explain the earlier failure or establish
general cold-entry reliability; no native departure policy was relaxed.

The coordinator owns the interactive implementation and G2 lease. The background
input/framework lane can review event ordering, fixtures and receipts without
occupying the TV. A second device lane may qualify the same immutable candidate
independently after handoff. Neither a shared library hash nor G2 success replaces
that device's acceptance and preservation evidence. See the
[parallel development rules](parallel-development.md).

## Exercise the actual loop

Use the isolated Proof identity, ordinary app UID and an immutable package. Set
interactive mode and start at the normal Home page. Reject the noninteractive
text/controller, launch, setting-write and other action-trial markers. The
execution must enter the same `while` loop, SDL poll/dispatch, live workers,
keyboard reconciliation, presentation gate, persistence handling and exit path
as the retained preview in [home-preview.rs](../src/bin/home-preview.rs).

Do not replace these paths with calls from a new scripted editing loop. The
observer may send the agreed remote inputs and inspect foreground/windows, but
it must not call shell navigation or keyboard-controller methods to advance a
stage. Separate physical Magic Remote input from network-injected remote events
and synthetic SDL text in every result.

Take the existing per-device lease and save the normal installation, favourites
bytes/owner/modes, original foreground/source/last input, typed relevant settings
and protected core hashes before installation. Proof action gates must prevent
unintended household app/HDMI/settings requests. Use fixed benign test queries;
do not alter accessibility aids, LG idle policies or OLED protections to make a
scenario convenient.

## Acceptance scenarios

A scenario passes only with its own observations and terminal result. An event
that never arrived, a skipped stage or an expired observer is not a pass.

| Scenario | Actions through the normal loop | Required evidence |
| --- | --- | --- |
| Cold entry | Launch, wait for readiness, navigate Home → Apps → Search with ordinary remote input. | Interactive-loop identity; real readiness; compatibility admitted; one SDL start request; independent LG panel appearance. The Luma keypad has neither painted keys nor active fallback targets while the panel is owned. Focus flags or linked symbols alone are insufficient. |
| Type and Done | Starting with an empty proof draft, type `lu` with the Magic Remote, then select LG Done. | Human outcome and event counts; the fixed expected-query comparison succeeds; query survives close; editing ends; own SDL request stops; independent panel absence; Search is focused and the fallback stays absent. Record any LG hide without a forwarded key. |
| Reopen and Back | After drain settles, activate Search again, type `ma`, then use Back. | A second real open; retained draft becomes the fixed expected `luma`; close/stop/focus invariants hold again. Reopening must work in the same process without a relaunch or hidden helper reset. |
| Explicit Keypad | Choose the separate Keypad control. Enter a fixed character, use Delete/Clear, and finish with Done; repeat with Back. | Native fallback renders and receives input; system-panel start count does not increase; only one keyboard is active; expected local draft edits occur; close returns focus to Search. Choosing Keypad is explicit, not a consequence of LG hiding. |
| Dismissal drain | Exercise Done/Back followed quickly by release, OK, direction and pointer/click input, then a fresh intentional action after settling. Include events observed in the same SDL drain batch and a later batch. | No late release/click activates a drawer card, opens a menu, reopens a keyboard or emits a TV action. Drop counters identify the same-batch and subsequent-batch cases; the fresh post-drain action works. If the same-batch sequence was not observed, retain that gap even if a component regression passes. |
| App idle | Leave the main loop without app input through its real 120-second idle boundary. Include a run begun with LG editing active if the system permits the panel to persist that long. | Own request stops, editing/fallback end, the app presents its black idle canvas once and ceases ordinary submissions. First ordinary wake input wakes without activating a card; the next intentional action works. If LG dismisses or backgrounds the app earlier, record that path; do not claim the active-keyboard idle branch was observed. |
| Window departure | With editing active, use LG Home to leave, without intercepting that key. | Independent foreground/window transition; own keyboard ownership stops; no hidden painting; bounded ordinary departure/exit. A subsequent normal launch starts cleanly. A verified LG Settings handoff, if tested separately, retains only the existing narrow policy and is not a general hidden-window keepalive. |
| Unavailable path | Run component negative cases for missing support/focus, unknown model/firmware/SDL and opening timeout. Exercise an actual unavailable path only if encountered safely. | Search finishes with an unavailable notice and explicit Keypad remains usable; no repeated open storm or guessed ABI fallback. Host fixtures qualify the branch logic, not unknown-device behaviour. |
| Exit and removal | Return through the ordinary Back flow, then remove only the proof package. | Final loop receipt and storage outcome; own panel/process/UID workers absent; catalogue, launchpoint, runtime, persistent state, jail paths and mounts absent. Compare every saved value/metadata/hash; verify LG Home and restore the original foreground. A timeout requires guarded cleanup and a failure/partial result, not a fabricated terminal receipt. |

### Required event ordering

The 0.23.0 helper trials did not resolve a main-loop batch-ordering hazard. Done
can end drawer editing before the post-poll keyboard reconciliation stops SDL.
The app-local input-drain latch must therefore be armed immediately when the
shared text-event handler processes Done/Back, before polling the next event.
Drain admission must remain effective even though `system_keyboard()` is now
false. SDL stop/reconciliation can then occur at the normal post-drain point.

The intended drain is 200 ms. Record the controller clock and monotonic elapsed
time so a delayed loop cannot masquerade as a precise wall-clock guarantee. A
regression must drive the real shared handler with Done followed by a release or
click at the same timestamp, then cover later input inside and outside the drain.
Pure assertions about the controller's timestamp helper are insufficient. The
current G2 human receipt observes an accepted closing key-down followed by a
drained release in the same main-loop SDL batch. This covers that event pair;
later-batch, pointer and broader race cases still need native evidence.

## Passive Proof instrumentation contract

The actual-loop observer now implements this contract; see the receipts above.
Enable it only when both the Proof identity and an explicit interactive-keyboard
observation marker are present. Refuse it under the retained identity or together
with an action-trial helper. It must observe the normal loop without changing
navigation, clocks, capabilities, keyboard flags, idle policy or action gates.

Produce a private, bounded change log plus an atomic current snapshot and final
summary. Suggested fields are:

- Source, binary/package identity, ordinary UID, schema and `loop=main_interactive`;
  interactive readiness, zero helper executions, monotonic elapsed/controller
  time, event sequence and SDL poll-batch sequence.
- Page and focus **roles** (Home, Apps, Search, Keypad, field, other), editing mode,
  visible/hidden and idle state; requested/owned SDK session; starts, stops,
  dismissals, failures and last transition reason.
- Received/accepted/rejected counts by event class and rejection reason, including
  drain, hidden, nonediting and malformed text. Keep synthetic-origin counters
  separate. Do not infer origin or prediction from opaque `inputSource` values.
- Committed byte count, caret byte position, preedit-present/byte-count,
  filtered-result count and fallback target count. A fixed approved fixture may
  produce an expected-query/unchanged-query boolean computed in memory. Record
  neither draft/preedit bytes nor hashes of household text, titles, app IDs,
  app inventories, raw SDL debug streams or screenshots by default.
- Draw/swap counters, hidden/idle submission counts, last presentation reason,
  live-read freshness/cycle counts, and counts of attempted external actions or
  preference writes. Keyboard-only stages expect zero such mutations.

Cap the change log at 128 records, report overflow explicitly, keep the final
summary separate, and update ordinary current snapshots at most once per second. Use the existing private/no-follow/atomic
storage conventions. Transition logging must not generate a repaint, wake or
new service call. Independent foreground/panel observations belong to the
external device owner and are correlated by timestamps; app-local panel flags
alone do not establish compositor visibility.

Optional diagnostic text may be a separate, fixed, bounded recipe armed only in
an already owned, visible Proof editing session. It may queue a known literal
commit and legacy preedit/clear through the existing native-only diagnostic
methods; host synthetic SDL text pushes remain impossible. Label every injected
event and resulting assertion as synthetic. It must not navigate, invoke the
controller out of band, write settings or advance a second scripted loop. A
successful injected sequence qualifies that delivery path, not human typing,
prediction, hardware keyboard or actual composing-language semantics.

## Matched performance workloads after lifecycle acceptance

Use the same device, package profile, window/render size, fonts/art cache state,
private catalogue/favourites snapshot, starting page/focus/query, live-read
configuration, instrumentation rate and timed input schedule. Build before the
lease. Record ambient foreground/source/output context; do not change TV power,
refresh, OLED protection or service settings for the benchmark. Compare immutable
baseline/candidate packages under the isolated identity; preserve the retained
preview. Feature-specific keyboard scenarios compare candidates that actually
implement the same feature. Older no-keyboard builds can only be compared on the
common Home/Apps workloads.

| Workload | Matched window | Measurements and exclusions |
| --- | --- | --- |
| Visible Home rest | Warm assets for 30 seconds, then observe 60 seconds before the 120-second blank boundary. | Own CPU/RSS and submission deltas; correlate minute-clock/live-read invalidations. Visible rest is not expected to forbid all legitimate redraws. |
| Black app idle | In a separate run, no input through the real idle boundary; observe the following 60 seconds. | One blank presentation, no continuing ordinary frames, CPU/RSS after blank. Distinguish external LG screensaver/backgrounding from Luma's idle branch. |
| Drawer motion and rest | Same known drawer size/layout; fixed left/right/up/down schedule for 20 seconds followed by 30 seconds rest; no OK/app launches. | Submission interval/draw/swap distributions and CPU/RSS; same movement extent and warm-cache policy on both artifacts. |
| Keyboard lifecycle | Same fixed draft and open/type/close/reopen schedule, with separate closed and panel-open holds. | Luma cost and controller transitions; separate LG panel/process cost from Luma's own process. Human variability is reported, not used as an exact timing comparator. |
| Long draft/caret | Same 256-byte synthetic draft, same three caret positions, fixed movement schedule then rest. | Luma texture/draw workload and memory; label synthetic input and compare the same visible result. This does not qualify every grapheme/font or IME. |

Use per-process user+system CPU tick deltas divided by measured elapsed time and
the actual tick rate; express CPU as percent of **one core**. Sample own process
and child/worker participation explicitly. Report phase RSS minimum/median/peak
sampled values; distinguish these from process-lifetime high-water RSS. Keep warm
and cold runs separate, use at least three matched pairs for a comparison, and
report individual results/spread before attributing a change. Avoid screenshots
and heavy observers during timed windows, or apply identical measured overhead.

The existing [performance monitor](../src/performance.rs) measures app-thread draw
submission, swap-call duration, submission intervals and dequeued input to the
next swap submission. It does not measure GPU completion, compositor presentation,
physical remote latency, panel FPS or photons. Include sample counts and dropped
input counts: the current monitor can coalesce input bursts, and dequeued input
is not necessarily accepted input. Exclude hidden/idle gaps from motion interval
claims. A slow frame requires the existing device profiling/trace workflow before
naming its cause; an aggregate CPU or swap-duration change is not a diagnosis.

## Completion and promotion

Publish a new receipt tied to the exact source/binary/device and each observed
scenario, with failures, skips, input origin and cleanup/restoration results.
Keep raw device material private. Do not retroactively expand the earlier helper
milestone. Full release admission requires the applicable interactive acceptance
above and the remaining integration/performance work. A retained-preview trial
also needs its own backed-up update/readback/favourites-preservation procedure;
removing an isolated build proves neither that update nor full dashboard completion.

The [preview rollout plan](preview-rollout-plan.md) defines the finite backed-up
G2 trial: affected-path evidence, documented unchanged-path history, isolated
installer rollback admission, fresh retained backups, normal-identity smoke and
saved-state verification. It keeps broader release qualifications open instead
of relabelling earlier physical results as new-binary proof. Full dashboard
completion still requires the remaining acceptance and integration work.
