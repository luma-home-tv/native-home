# Parallel development plan

Use one coordinator and up to three background agents. Parallelise independent
research, review, host work and per-device validation; reconcile shared rendering
and application changes through one integration owner. This extends the
[implementation plan](implementation-plan.md) and uses the
[controls coverage audit](controls-coverage-next.md) as the control-centre backlog.
It does not change device authorisation or the protections in `AGENTS.md`.

## Broad remaining delivery steps

1. **Finish interactive input and lifecycle.** Validate the actual normal event
   loop, not only its shared helpers: remote entry/reopening, Done/Back, same-batch
   and delayed input after dismissal, pointer/wheel/hold, idle/wake and foreground
   departure. Keep one keyboard active. Qualify composing-language, deletion and
   hardware-keyboard semantics separately; the available G2 mode offered no word
   predictions. G2 now has the backed-up normal update described below; remaining
   scenarios are broader qualification work. The
   [interactive acceptance contract](interactive-keyboard-acceptance.md) names
   the required scenarios and evidence.
2. **Complete the control-centre inventory and useful native controls.** Map
   every intended category to verified readings, native choices/ranges or an
   explicit LG-managed route. Start the next fixed adapter with pointer size;
   expand missing Sound, Accessibility and General readings. Each setter needs
   its own capability/context/readback/restoration proof. Network reconfiguration,
   boot/Home policy, firmware and panel protections remain protected/delegated.
3. **Validate real TV integration.** Finish live HDMI video/format/HDR evidence,
   the C2 source-restoration boundary, app-launch/return and LG Settings handoff
   regressions under actual use. Do not transfer G2 evidence to C2 or interpret
   an idle selected-source getter as live video proof.
4. **Close the visual and usability gaps.** Match the agreed Apple TV-inspired
   concepts with Luma branding, OLED-black space and graphite surfaces. Review
   long/localized labels, dynamic/new app icons, empty/stale/error states, focus
   hierarchy and remote legibility on real panels.
5. **Prove performance and reliability.** Measure matched idle/motion/input
   workloads, CPU/RSS/frame submission and settling on both TVs; panel latency
   and physical smoothness require separate evidence. Check repeated app and
   keyboard sessions, memory growth, service failure/recovery, durable favourites
   and controlled restart/reboot persistence without changing boot configuration.
6. **Promote and deliver.** After the interactive acceptance gate, perform backed-up
   retained updates, verify favourites/metadata and normal behaviour, exercise
   clean removal, publish scoped release/test documentation and separate generic
   framework changes for possible upstream contribution. G2 now retains 0.26.0
   after its separate normal-build qualification; C2 remains on 0.20.0. Component
   tests alone do not admit a retained update.

## How the parallel work shortens delivery

Run three background lanes alongside the coordinator, with small owned outputs
and explicit interfaces. Framework review and control-adapter development do not
wait for a TV. G2 and C2 validators can run the same immutable candidate at the
same time under different leases. While those tests run, host/review workers can
prepare the next independent change. This overlaps work that was previously
serial without making two sessions compete for a file, build target or TV.

The critical path remains central integration, a frozen source/binary pair,
affected host checks, independent per-device acceptance and retained rollout.
Do not publish or deploy a moving checkout. Merge module contracts and review
findings in small batches; request changes from a file's owner instead of editing
across assignments. More simultaneous sessions are useful only when there are
more independent outputs; shared shell/input integration remains one owner's work.

Use waves rather than keeping every agent on a permanent role. During device
validation, the coordinator owns G2, a second validator owns C2, a reviewer audits
evidence and a host-only worker prepares the next independent feature. During
implementation, reassign the device validator to a bounded UI or test module.
This keeps hardware tests running alongside development without asking two
agents to integrate the same event loop. Give each assignment one deliverable,
named owned files, a completion check and a handoff point; stop expanding its
scope once that output is ready.

Prefer one integration checkout and separate worktrees for substantial code
changes, with separate build targets and private output directories. A worktree
does not replace file ownership or a device lease. Agree a small interface before
parallel implementation, then integrate one reviewed change at a time. Preserve
the frozen artifact while the next source proposal develops elsewhere; unrelated
proposals need not invalidate the artifact already under test.

## Four working lanes

| Owner | Immediate work | Owned outputs and boundary |
| --- | --- | --- |
| Coordinator: native integration and G2 | Finish the normal Search → LG keyboard → Done/Back flow, explicit Luma-keypad fallback, compatibility/failure gates and long-query rendering. Reconcile reviews, build the shared candidate, validate it on G2, then review and publish qualified results. | Sole owner of `src/bin/home-preview.rs`, `src/shell.rs`, `src/shell_tests.rs`, `src/live.rs`, app-level keyboard/drawer state, packaging, dependency-lock reconciliation and central milestone/plan updates. Owns the G2 device lane. |
| Background 1: firmware, ABI and text review | Independently review actual webOS24 SDL layouts and event semantics, preedit/caret handling, literal commits versus predictions, font/script limits and generic widget behaviour. Produce specific findings and proposed isolated changes. | Own review notes and focused ABI/framework fixtures. During active integration, shared files are read-only. After explicit handoff, may own `sdl-events.c`, `src/runtime.rs` or the external framework patch as an exclusive bounded unit; never concurrently with the coordinator. Generic widget improvements retain PlxNative authorship/licences and an upstream-suitable patch. No TV calls unless separately assigned a device lease. |
| Background 2: controls coverage and next adapter | Maintain the concrete capability inventory, refine availability/context presentation, audit pointer size as the next candidate, and specify missing read groups and admission tests. | Own `docs/controls-coverage-next.md` and new operation-specific design/fixture files. A new fixed adapter can be assigned its own module after review. Shared model/live/shell/action changes remain coordinator-owned and are proposed as an interface contract. No arbitrary setter, protected operation or TV testing by this lane. |
| Background 3: independent C2 validation | Review the candidate's C2 gates and prepare the separate state-preserving test recipe. Once a stable, explicitly admitted candidate is available, independently qualify C2 behaviour, capture scoped evidence, remove the isolated package and restore state. | Own C2-specific private harness/output paths and a proposed public validation record. Sole C2 device owner during its lease. Does not change shared app code, copy G2 conclusions, override rollout gates or update the retained preview autonomously. |

File ownership is an assignment, not a suggestion. Before editing, inspect the
current worktree, active processes and owner messages. The coordinator records
the exact assigned files and handoff point. If a change crosses a boundary, send
a proposed contract or patch for its owner to integrate; do not opportunistically
edit that owner's file. A separately assigned checkout can hold a proposal, but
its existence does not resolve a shared build-target or device conflict.

## What can proceed now

The isolated 0.24.0 candidate at
`81213337bee7dde2d4808f058c9973957d56ab12` has now completed the actual-main-loop
keyboard/fallback scenario independently on both models. Each run passed its
31 scenario checks and its removal/preserved-state checks. Remote navigation
used network events, the two text commits used fixed synthetic fixtures, and
both LG dismissals used guarded Back. These runs establish neither physical
LG Done/typing on this candidate nor observed same-batch dismissal input, idle,
window departure, performance or retained-update acceptance. Both retained
previews remain unchanged.

The [actual-main-loop record](main-loop-milestone.json) now adds independent
plain idle/wake and programmatic departure results on both models. Each idle
hold is checked against advancing native timestamps/batches and unchanged
submission counters, not just host sleep. LEFT wake remains a narrow
no-observed-activation check. Both models also completed idle begun with LG
editing active, with the adjacent-state/close-cause limitation recorded. Physical
input and every other gap are scoped per scenario; these additions do not
promote the candidate.
The parallel controls lane has also delivered the
[presentation policy module](control-capability-design.md) and a
[guidance-read implementation](read-coverage-audit.md). The former has no UI caller;
the latter now has [independent ordinary-app/native component qualification](guidance-read-milestone.json)
on both models, with preserved-state/removal checks. The stronger
[intentional OK-wake scenario](ok-wake-milestone.json) now has independent native
results on both models: first OK wakes without editing, second OK opens one LG
panel. Native completion preceded a host interruption; reviewed same-run cleanup
recovered both closeouts without restarting the tests.

The [current G2 human check](manual-keyboard-milestone.json) adds typing/Done,
same-process reopen/Back and one same-batch drained closing-key release. Three
incomplete attempts remain recorded, including early startup focus loss and a
corrected host assertion. C2 physical input, later-batch/pointer drain races,
cold-entry reliability and performance remain open. A separate host lane has
integrated the [optional Sound readings](sound-read-design.md), with independent
review and freshness/error fixtures. The same frozen 0.26.0 binary now has
[independent ordinary-app/native Sound qualification](sound-read-milestone.json)
on both TVs. A second agent prepared the C2 driver while the coordinator ran G2;
both device jobs then overlapped under separate leases, followed by independent
receipt review. Both proof packages were removed and retained previews preserved.
Native stale/error transitions and retained integration remain separate gates.

The [G2 startup/relaunch record](startup-relaunch-milestone.json) adds two planned
0.26.0 actual-loop sessions, independent timed window observations and a new
process loading unchanged saved state after programmatic LG Home departure.
Implementation, sampler storage, independent review and receipt audit ran in
parallel around one device owner. A failed first observer profile remains
recorded; its corrected successor changes only host window classification.
No native focus-loss policy or OLED protection changed. General startup
reliability and the equivalent C2 profile remain open.

The [backed-up G2 update](preview-update-milestone.json) now delivers 0.26.0 as
the retained normal app. Host implementation, independent review and saved-run
audits ran alongside one coordinator-owned TV lane. The installer rehearsal
verified isolated lower-version admission and complete removal; a separate
same-run audit corrected only a request-metadata comparison while preserving the
original failed host record. The normal update then exited successfully, with
complete old/new bundles, saved-state ownership/modes, ten visually reviewed
Apps/Search/keyboard/Back frames and eighteen preservation domains checked.
This moves the G2 delivery milestone forward without turning its network smoke
into physical typing, broad reliability, performance or C2 evidence.

The next waves are:

| Wave | Coordinator / G2 | C2 validator | Independent review | Host-only implementation |
| --- | --- | --- | --- | --- |
| Finish input acceptance | Retain the startup investigation and completed G2 update; qualify remaining dismissal-order and normal-use gaps. | Qualify physical typing when available and repeat startup/relaunch independently with separate teardown. | Audit changed paths, rollback admission and actual-loop receipts; distinguish missing observations from passes. | Prepare controls coverage, pointer-size recovery fixtures and small visual proposals without changing the running candidate. |
| Deliver useful controls and visual improvements | Integrate the next reviewed adapter/UI slice and freeze a new candidate. | Prepare independent regression recipes while host work proceeds; validate only after handoff. | Review fixed operation admission, failure/recovery and visual acceptance. | Implement disjoint operation modules or visual components under explicit ownership; central shell/live wiring has one owner. |
| Qualify normal use and release | Validate app/Settings/HDMI handoffs and matched performance; perform backed-up rollout after acceptance. | Repeat relevant C2 workloads and preserve its independent source-restoration boundary. | Audit release evidence, limitations, privacy and generic upstream patch separation. | Prepare release documentation and upstream-ready generic framework changes while hardware tests run. |

Human remote/pointer/wheel and live-video checks can be grouped into short
sessions once the relevant artifact is stable. They do not block host-only
features, but remain explicit release gaps. Unsupported composing-language
behaviour or unavailable controls must be described honestly rather than
silently counted as complete.

The next controls assignments can be prepared independently: a pure capability
state model for consistent native/read-only/stale/protected presentation; a
pointer-size trial state machine that handles uncertain writes and guarded
restoration; and an evidence-based audit of the next missing read-only fields.
Each gets a new operation-specific module or fixture/document output. The
coordinator retains ownership of shared model, live-worker and shell integration.
Pointer size remains unconnected to the UI until its separate native trial and
restoration gate pass.

Track delivered, accepted slices and time blocked on integration/device access.
Do not promise a fixed multiplier from four agents: shared integration and
hardware-dependent acceptance remain serial boundaries. The useful gain comes
from overlapping host work, independent review and two-device validation while
keeping rework and repeated setup small.

The previous reconciled component candidate was
`760db0eb11217e3de7766bc7d3223d995d1fc634`; its evidence remains scoped to that
revision. The newer candidate above is not a release receipt.
The [text milestone](text-input-milestone.json) records isolated G2 activation,
typing/Done/Back observations and corrected dismissal/removal; its retained
versions are G2 0.21.0 and C2 0.20.0 using the earlier Luma keypad. Refresh source,
records and actual installations before making a current deployment claim.

The changing candidate now contains normal-flow ownership/fallback integration,
an explicit known-model/firmware gate and an SDL 2.0.14 gate. The [shared-controller record](keyboard-integration-milestone.json) now contains
independent G2/C2 isolated component-flow proof; normal interactive qualification
and retained updates remain separate. In particular, the reviewer should resolve these open boundaries:

- The typed event includes preedit text and start/length metadata, but the current
  normal consumer forwards only the preedit string. Selection/caret range
  semantics and actual composing-language behaviour remain to qualify.
- Current normal text commits are explicitly literal. The event's `source` field
  is opaque protocol metadata, not a verified prediction discriminator. The
  buffer's separate word-replacement operation must not be enabled by guessing
  from that field or commit length.
- Unicode draft bounds, grapheme edits and NFC matching do not establish font
  coverage, shaping/bidi, full case folding, hardware-keyboard semantics or
  long-query device performance. Review each under its actual workload.
- LG panel flags require lifecycle interpretation and independent window
  observations; an initial absent flag or a linked SDL symbol is not behavioural
  proof. Done/Back must end editing without exposing the fallback underneath.

## Candidate and test handoff

1. **Freeze an identifiable candidate.** Reconcile assigned edits first. Build
   from a clean reviewed revision where possible; if a bounded trial must use an
   uncommitted tree, preserve the complete source/patch manifest and diff hash.
   Record framework revision and patch hash, source identity, package version,
   exact binary SHA-256 and package SHA-256. Put each candidate in an immutable
   directory before taking a device lease. Never rebuild in place underneath a
   running validator.
2. **Gate the second device explicitly.** C2 validation starts only when the
   coordinator has supplied a stable candidate, scoped test contract and an
   appropriate C2 admission path. A model gate is never
   silently relaxed by the validator. Distinguish a test-only candidate from a
   promoted normal capability, and treat every binary change as new evidence.
3. **Assign one live owner per device.** The test agent acquires the existing
   per-device lease and owns its SSH agent, process/session handle, package and
   teardown through terminal completion. Poll that same handle on an observation
   timeout; a lock file or missed observation is not proof the job stopped.
   Other agents do not send keys, launch apps, remove packages or run recovery on
   that TV during the lease. Hand ownership over only with exact live state and
   remaining cleanup obligations.
4. **Verify preservation, behaviour and removal separately.** Save original
   foreground/source/last-input state, retained installation and favourites
   bytes/metadata, relevant typed settings and protected core hashes. Observe
   foreground/windows independently of the app's own receipt. Prove the requested
   interaction under an ordinary app identity; distinguish synthetic text/keys
   from human input. Remove only the isolated identity, then verify process,
   catalogue, launchpoint, runtime, persistent state, jail paths and mounts are
   absent. Compare saved state, verify LG Home and restore original foreground.
   No firmware, boot, core-library, networking, media/DRM or OLED changes.
5. **Return a bounded evidence bundle.** The owner reports source/binary/package
   identities, checks actually executed, failures/skips, native frame locations,
   independent observations and exact cleanup/restoration results. Native
   household frames, inventories, raw service replies, addresses, credentials
   and saved preferences stay private. Public records contain scoped facts and
   counts, not sensitive fixtures.

## Reconciliation and publication

The coordinator reviews every lane against current source and receipts, resolves
conflicts, runs affected shared checks once and inspects native frames. A report
or passing host build is evidence to assess, not device proof. Re-run only the
tests invalidated by a subsequent change, and label evidence by its actual
source/binary/device scope. G2 success does not qualify C2, and an isolated app
success does not prove a retained update or physical interaction.

Only after reconciliation should central plans/milestones, dependency locks and
release metadata be updated and privacy-audited for publication. Commit/push and
retained-preview updates remain coordinator-owned; workers deliver reviewable
outputs instead of competing publication histories. The independent Luma app
continues to contain no Plex client features, while generic native UI/input
improvements remain in the attributed PlxNative integration patch for possible
upstream contribution.
