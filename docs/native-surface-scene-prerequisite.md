# Ordinary native surface prerequisite

Before the two [0.53 artwork context tests](artwork-context-qualification-053.md),
verify that an owned native Target can become the sole foreground CARD while an
owned native Proof process keeps its window alive. This prerequisite uses no
artwork broker or HBC session. It has **host and package evidence only**; neither
new test package has been installed or run on a TV.

The retained G2 0.47 and C2 0.48 applications remain the current verified normal
installations, with normal Root artwork disabled. There is no new native,
performance, physical-input or cleanup qualification here.

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
| Scene carrier host checks | 22 fixtures, including parser strictness, publication races, empty held SDK stdin, actor identity, admission and cleanup barriers |
| Builds | Both actual ARM32 release builds succeeded; existing SDK target-feature warning retained |
| Package audit | Exact SDK-created archives: three Probe files and six Target files; no added services, hooks or permissions |
| ELF audit | Same loader and a subset of the saved retained G2 binary's direct library and GLIBC requirements |
| Device admission | Refused before agent or SSH because no dated device window is configured |

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
