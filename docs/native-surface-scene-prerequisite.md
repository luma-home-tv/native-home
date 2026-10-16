# Ordinary native surface prerequisite

Before the two [0.53 artwork context tests](artwork-context-qualification-053.md),
verify that an owned native Target can become the sole foreground CARD while an
owned native Proof process keeps its window alive. This prerequisite uses no
artwork broker or HBC session. Both packages now have a G2 installation/removal
record, with **no launch requests or native scene qualification**. The separate
[prepared-package cleanup milestone](native-surface-prepared-cleanup-milestone.json)
binds that limited result.

That prepared-installation record preserved the then-current G2 0.47 and C2
0.48. The current G2 baseline is separately verified [0.57](g2-retained-preview057.md);
normal Root artwork remains off. Historical dates, locks, package pins and
preference snapshots do not admit another run. Cleanup of the original prepared
packages is qualified separately below.

## Why a separate probe

The normal interactive dashboard exits shortly after observed focus loss. Its
scripted diagnostic path can continue drawing during a background interval.
Neither supplies the required held-window experiment unchanged.

The separate minimal Proof creates the same native window class and GL setup,
draws black while active, then permanently stops drawing and swapping on observed
focus loss or hiding. It continues polling while holding the window for its
fixed 60-second lifetime. Discrete input, Quit or an unknown event ends the test
early; focus gain never restarts rendering. Ready and Complete reports distinguish
a full lifetime from a courtesy abort. Window ownership at report publication
does not prove OS exit or a hard bound on blocked SDK calls.

The Target is the existing fixed 12-second native launch test. Its report and
capture, the Proof reports, actual process identities and actual LG foreground/
window replies must agree. A foreground reply alone cannot establish a sole CARD.
The test must first observe a clean Proof frame and sole Proof surface before
dispatching Target. After both roles have launched and been positively mapped,
an early abort can permit known ordinary cleanup without qualifying the scene
component. An abort before Target dispatch retains the operator's barrier; this
carrier has no partial-start recovery procedure.

The Probe contains only its executable, app metadata and icon. The Target also
contains its two fonts and their licence. The shared runtime eagerly initializes
text: the Probe can try the system font fallback or fail text initialization.
Consequently, identical window construction does not imply identical font startup
or text performance. The Probe draws no text. Application-level statements about
not creating threads or children do not characterize SDK internals.

## Verified preparation

| Item | Result |
| --- | --- |
| Probe host checks | Two Rust rendering/input gate tests; Python test covering 28 inherited build-override cases; Clippy with warnings denied |
| Scene carrier host checks | Initial 22 fixtures, then 26 after correcting grouped GET stdin handling; no empty reply is accepted as a scene |
| Builds | Both actual ARM32 release builds succeeded; existing SDK target-feature warning retained |
| Package audit | Exact SDK-created archives: three Probe files and six Target files; no added services, hooks or permissions |
| ELF audit | Same loader and a subset of the saved retained G2 binary's direct library and GLIBC requirements |
| Device outcome | Both SDK installations completed; the active-screen guard stopped before any launch request; a separate cleanup-only continuation completed |

The private Probe uses a 34-package lock seeded and pruned from the production
lock. An earlier unseeded lock was rejected during admission and remains a
recorded failure. Build override variables are rejected by presence, even when
empty; the build records the actual selected source, SDK compiler wrapper,
objects and output. This is not a fully hermetic toolchain claim.

| Frozen artifact | SHA-256 |
| --- | --- |
| Probe ELF | `2195b51d2a585e9ed153e7755b743c822d37e53dfa4e1b6fcc9770c5453f59b1` |
| Probe IPK | `535dde701364827c77f427b61c6ce0f1d6a6fa576e8590141cbbf5854446e8b9` |
| Target ELF | `9b8620aef4fdea0758a740881f40cbf581a65e6f9afe016a842918a2f3dd0cee` |
| Target IPK | `f6294283357ad34a1964f18229fd79dd15df3e7137287d6c7b467eee4ac3844e` |
| Root host consolidation | `efba7d194d07ba39f381c6a31dd9aa3182280dd38d3dc9262801ba13035cb502` |

## Prepared installation and separate cleanup

The original device operator exited 1. Early attempts stopped before installation
because of grouped-query EOF handling or an unsuitable foreground/idle state;
their exact coordination locks were closed separately. The corrected operator
then completed both original SDK installer subscriptions and verified the exact
installed inventories. After preparation, the prelaunch guard observed LG's
screensaver state before the first launch. Neither package received
a launch request. This does not prove that auto-launch is impossible or qualify
execution under a measured test UID.

A separately reviewed cleanup-only operation exited zero. It rebound the exact
known packages, uploads and original lease, checked executable and namespace
absence, issued one supported remove per identity, and verified registration,
bundle, runtime, state, jail and mount absence. It removed only the two positively
identified IPKs and their empty staging directory. Full original retained bundle
(32 entries/30 files), private state (two entries/one file), runtime (110 entries/
73 files), all 19 checked reply domains and the five Core hashes matched. Raw
preference bytes and ownership/modes were preserved. No Root broker/HBC session
started, and no unmeasured test-UID absence claim was made.

The independent saved review replayed those file/readback comparisons and the
actual successful-release output, return status, subsequent whole-lock absence
and host-agent closeout. A zero release exit status alone was insufficient:
the coordination tool also returns zero for already-absent or foreign locks.
Original failure and separate cleanup remain distinct records.

The next actor experiment needs a prepared-then-launch workflow, fresh actual
LG Home/Active confirmation immediately before dispatch, and enough time left
for the native lifetime and removal. Baseline preparation time must not consume
the actor's own lifetime allowance. Natural screensaver activity can be accepted
for final cleanup when the actual Home/window/power replies agree; it grants no
native admission, wake command or change to LG's idle/panel protections.

## Required device evidence

A separately reviewed dated operator must capture the complete retained bundle,
fresh current preference bytes and metadata, retained runtime and protected Core
baseline. Historical preference bytes cannot replace a new baseline after a
legitimate user change. Both temporary identities, runtimes and mounts must be
absent before installation. Use the supported installer, one observed launch per
role, fresh ordinary process identities and positively measured namespace paths.

Scene qualification requires both original actors alive around the actual sole
Target CARD observation, the expected stopped renderer and an unaborted complete
Probe lifetime. It grants no Root resource authority. Independently observed
process and assigned-UID absence, supported package removal, scoped runtime and
staging removal, registry/jail/mount absence, retained/Core preservation and exact
lease/host-agent closeout establish cleanup separately. Unknown installer,
identity, publication or lifecycle evidence retains the barrier without retries
or guessed deletion. Operator Root SSH staging and fixed reads are distinct from
the ordinary Probe's lack of a Root broker/HBC session.

## Responsiveness remains a separate release gate

Saved positive 0.43/0.46/0.49 traces contain 16 non-first helper durations of
2.562–2.875 seconds, with a pooled median of 2.744 seconds. These are the helper's
own elapsed measurements, not input-to-panel or complete fetch latency. The
historical diagnostics explicitly allowed 30 seconds per job; the current normal
runtime defaults to two seconds. Normal Root artwork is off, so this is a future
integration issue rather than a demonstrated retained-app regression.

The saved data does not isolate individual GET, file readback, decode or upload
time. App-controlled keyboard and launch intents currently wait for the owned job
to settle, which could add several seconds near job start. That is an inference,
not a measured handoff latency. Raising the timeout alone does not resolve it.
Before activation, compare matched Root-off, settled and confirmed-active workloads
with separate local response, settlement, dispatch and actual scene intervals.
The 0.53 diagnostic holds and this ordinary scene probe cannot serve as release
performance measurements. Existing per-operation checks and Unknown barriers must
remain intact while evaluating a faster design.

## Public source for the next ordinary experiment

The minimal [standalone probe](../probes/native-surface/README.md) is preserved
in the repository. Its main, scoped runtime, identity and SDL event bridge retain
the exact earlier source bytes. The new package adds the project's two font
files and OFL licence, for six files total, since the unchanged runtime eagerly
initializes text. The probe still draws black only. This package change and
repository-relative build paths require new artifact hashes and a new audit;
the earlier three-file package result does not transfer.

Root ran the two render-gate tests, 28 inherited-build-override cases and strict
Clippy. The five new host phase-boundary tests also pass. These checks do not
establish either native actor or scene. A new two-phase carrier is being prepared
against actual retained 0.57, with preparation completed before actor timing
starts. All TV operations remain with the coordinator.

Its proposed ordinary-launch precondition permits exact LG Home CARD with an
optional exact natural LG screensaver, with a consistent fresh Active/Screen
Saver power reply. It adds no wake, Home or settings request. Postlaunch sole
Proof readiness and both-live sole Target CARD qualification stay strict.
Blocked actor, namespace, installer or ownership evidence cannot be replaced by
this prelaunch predicate. Root artwork, negative settlement and normal activation
remain later steps.
