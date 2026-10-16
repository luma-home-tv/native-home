# Parallel development plan

Use one coordinator and up to three background agents. Parallelise independent
research, review, host work and per-device validation; reconcile shared rendering
and application changes through one integration owner. This extends the
[implementation plan](implementation-plan.md) and uses the
[controls coverage audit](controls-coverage-next.md) as the control-centre backlog.
It does not change device authorisation or the protections in `AGENTS.md`.
G2 remains primary and the user has explicitly made it always available. C2 is
available only from 00:00–06:00 Europe/London or when explicitly made available
by the user. This supersedes the earlier dated availability windows; historical
operators retain their own expired execution limits and cannot be reused as
current admission. All TV access stays with the coordinator, with distinct
leases, current foreground/baseline checks and private agents. Current retained deliveries are
[G2 0.57](g2-retained-preview057.md) and [C2 0.48](c2-retained-preview-048.md),
with device-specific resources, saved preferences and fresh normal sessions.
C2's ordinary-owner metadata roundtrip/restoration precedes its retained result;
G2 evidence never establishes C2 permissions. The isolated 0.44 C2 static result
and its preserved 0.20 baseline remain historical evidence.
The [normal055 owned-app observation](g2-normal055-app-launch-return.md) now adds
one real drawer launch and a later explicitly launched fresh Luma session, with
independently replayed cleanup and preservation. The original operator remains
exit1; its same-lease continuation observed removal completion without retry.
Automatic return, arbitrary-app compatibility and broader reliability remain open.
The separate [ordinary caption getter](caption-read-native.md) adds one fixed
public-API read, independent caller/report evidence and full removal/preservation.
Its source/build, carrier and independent audit were prepared in parallel around
one device owner. The [0.57 row integration](caption-read-057.md) followed the
same split: private implementation, independent review and coordinator-run checks,
with a separate native carrier lane. The [normal057 G2 delivery](g2-retained-preview057.md)
now verifies captured presentation and closure with an independently replayed audit.
Regional semantics and native failure/expiry remain open.
The [0.49 isolated G2 scheduling component](artwork-transition-049.md) now adds
queued-work/local-intent/Back and known provider closure, with seven helper
sessions and complete cleanup. Actual LG keyboard/external-app transition,
closed-negative lifecycle and normal Root artwork activation remain open.
The [0.50 transport probe](artwork-transport-eof-050.md) now records four ordinary
G2 socket cases and scoped cleanup separately from the dashboard/helper builds.
Live artwork work retains separate worker, decoder, integration and review lanes. HDMI detail and broader control inventory
remain subsequent feature work; the release gates below remain open.

## Current host foundation wave

This wave's reviewed provider/helper proposals are integrated as the
[0.51 host foundation](artwork-attempt-foundation-051.md), with host checks and
both ARM builds complete. They add evidence for failures without enabling
closed-negative acceptance or changing either retained TV installation. Further
proposals stay outside the integration checkout until reviewed.

| Owner | Independent output | Integration boundary |
| --- | --- | --- |
| Coordinator | Native-test closeout, central source review, shared host/ARM checks and public receipts | Sole TV/build/integration owner; freezes each executable before any new native trial |
| Provider lane | Unconditional typed attempt trace and retained exchange-failure metadata | Provider and its tests only; no encoded image bytes in the trace, no Ticket recovery or changed settlement |
| Helper lane | Typed ReadAfter/Finish context-withdrawal candidate and explicit endpoint closure | Server/bootstrap and their tests only; strict readers, unchanged peer/pins and known child cleanup; no successful Finished fabricated |
| Independent audit lane | Saved native transport replay and adversarial review | New private audit files; no TV calls or shared source edits; Root replays the final auditor independently |

Provider and helper owners agree their small report contract before coding.
Root reviews and composes both foundations, then runs affected tests and ARM
compilation. A separate strict negative grader and current-Ticket settlement
remain future work under the [closure plan](artwork-context-closure-next.md).
This overlap shortens host work; it does not remove the later native acceptance
gate or make a reported closure sufficient authority for recovery.
The follow-on [0.52 composition](artwork-closed-negative-052.md) completed that
host grader/resolver slice with separate provider, grader and independent-review
owners. Root integrated the reviewed interfaces, repaired test fixtures, ran all
563 Rust/58 Python checks and Clippy, and froze both matching ARM builds. Native
ReadAfter/Finish qualification remains a separate wave; neither TV was updated
to this source.

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
   app-launch/return and LG Settings handoff
   regressions under actual use. Do not transfer G2 evidence to C2 or interpret
   an idle selected-source getter as live video proof.
4. **Close the visual and usability gaps.** Match the agreed Apple TV-inspired
   concepts with Luma branding, OLED-black space and graphite surfaces. Review
   long/localized labels, dynamic/new app icons, empty/stale/error states, focus
   hierarchy and remote legibility on real panels.
5. **Prove performance and reliability.** Measure matched idle/motion/input
   workloads, CPU/RSS/frame submission and settling on G2; panel latency
   and physical smoothness require separate evidence. Check repeated app and
   keyboard sessions, memory growth, service failure/recovery, durable favourites
   and controlled restart/reboot persistence without changing boot configuration.
   The [current measurement method](current-performance-method.md) and
   [G2 0.26.0 result](current-performance-milestone.json) now add one bounded
   isolated profile with process resource/raw timing evidence and removal.
   The later [scroll-rest fix and G2 delivery](apps-scroll-rest-milestone.json)
   adds a host reproduction and independent 0.28.0 profile. Timing uncertainty
   prevents a percentage-effect claim. Normal-identity resource comparisons,
   the recorded whole-session outlier and broader reliability remain open.
6. **Promote and deliver.** After the interactive acceptance gate, perform backed-up
   retained updates, verify favourites/metadata and normal behaviour, exercise
   clean removal, publish scoped release/test documentation and separate generic
   framework changes for possible upstream contribution. G2 retains 0.57.0 and
   C2 retains 0.48.0 after their own separate normal qualifications. Neither
   result qualifies C2 IME, normal Root artwork or full release acceptance.
   Component tests alone do not admit a retained update.

## How the parallel work shortens delivery

Run three background lanes alongside the coordinator, with small owned outputs
and explicit interfaces. Framework review and control-adapter development do not
wait for a TV. All G2 queries, launches and updates stay serial under one device
lease. While those tests run, host/review workers can
prepare the next independent change. This overlaps work that was previously
serial without making two sessions compete for a file, build target or TV.

The critical path remains central integration, a frozen source/binary pair,
affected host checks, independent per-device acceptance and retained rollout.
Do not publish or deploy a moving checkout. Merge module contracts and review
findings in small batches; request changes from a file's owner instead of editing
across assignments. More simultaneous sessions are useful only when there are
more independent outputs; shared shell/input integration remains one owner's work.

Use waves rather than keeping every agent on a permanent role. During device
validation, the coordinator owns G2, a reviewer audits evidence and host-only
workers prepare independent modules or adversarial fixtures. The coordinator
may also use C2 within its explicit dated window; agents never compete for a TV. During
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
| Coordinator: native integration and G2 | Integrate the artwork contracts, freeze matching source/build/package inputs, run serial checks and the isolated G2 test, and reconcile independent audits. | Owns shared live/shell/packaging integration, all builds, the device lease and central public records. Retained updates remain a separate qualification. |
| Background 1: bounded resource interface | Implement an explicitly assigned metadata/transport module with fixed operations, strict identities, byte limits and refusal semantics. After handoff, prepare a new private test driver while the coordinator integrates source. | Owns only named files until handoff. No builds, TV calls, policy edits or production startup changes. Reusable framework changes retain upstream authorship and licences. |
| Background 2: cache and integration review | Implement pure scheduling, ownership, invalidation and memory-capacity tests. After handoff, independently review the resource flow, context/refusal handling and candidate harness. | Owns a new isolated cache module during implementation; shared worker, GL and focus code stays read-only. Findings go to the integration owner. |
| Background 3: adversarial fixtures and saved-evidence audit | Prepare package-mode tests, synthetic transfer/decoder receipt regressions and an independent saved-result audit alongside the driver work. | Owns new private test/audit files. No TV calls, shared source edits or build-target competition. Distinguishes reported digests from independently rehashed saved bytes. |

The [0.35 artwork candidate](artwork-transfer-035.md) and
[0.36 C2 lifecycle](artwork-live-036.md) use this division. Cache,
transport and fixed catalogue readers were implemented independently, then handed
to one integration owner. Harness fixtures and the saved-evidence audit proceed
alongside the final ARM builds and package checks. Device qualification stays on
the critical path; increasing agent count does not admit a moving source tree or
concurrent access to the same TV. Later waves can reassign these slots to controls,
visual review or reliability work after the current slice is reconciled.

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
[guidance-read implementation](read-coverage-audit.md). The presentation policy
now supplies actual row wording in the next 0.27.0 candidate, with
[synthetic host previews](verification/control-row-presentation/README.md);
isolated G2 Sound/Accessibility evidence is recorded
[separately](control-row-presentation-milestone.json), and normal deployment
remains pending. At the earlier
guidance milestone it had no UI caller;
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

## Artwork departure qualification wave

For [0.53](artwork-context-qualification-053.md), one agent owned the protected
Installation/server hook and tests, a second owned the ordinary one-job UI/provider
recipe and tests, and a third independently reviewed their composition while
the coordinator owned package admission, integration, serial checks and builds.
Private handoffs were frozen before copying; initial failures and reviewed fixes
remain distinct. An independent saved-receipt validator can proceed alongside
the next carrier preparation. The native cases stay serial on the G2; this
parallel work creates no C2 Root authority or additional device window.

## Current artwork delivery direction

The [ordinary surface source/build](native-surface-public-source-milestone.json)
is now preserved and locally audited. The later [G2 scene experiment](native-surface-scene-g2.md)
passes its ordinary held-window, both-live sole Target CARD and complete cleanup
checks against retained 0.57; the independent saved audit and Root replay match.
The first saved-reader indexing failure remains recorded. Root owns all device
contact, integration and actual checks.
The next [responsive artwork boundary](artwork-responsive-cache-next.md) uses
an ordinary durable cache during normal interaction and separately qualified
privileged preparation. A source/architecture review identified the per-icon
session cost and early Unknown cases before any normal activation; the existing
broker and guard remain reusable, with no weakened closure or Ticket recovery.

The [ordinary cache foundation](durable-icon-cache-foundation.md) now passes the
coordinator's real-file/full Rust/Clippy checks and matching ARM build. Its owner
authored the isolated module/tests, independent review found the final-scan writer
race, and the coordinator repaired, integrated and executed the final checks.
The [0.58 integration](ordinary-artwork-cache-058.md) now combines reviewed private
consumer, diagnostic-publisher and fixed producer-profile proposals. Separate
agents reviewed the consumer, publisher, Main/Shell wiring and bounded capture
receipts; the coordinator integrated them and ran all host checks and matching
ARM builds. Independent local artifact auditing and native carrier preparation
continue in parallel. Main/UI integration, artifact admission, publication and
all TV contact remain coordinator-owned. No cache code has run on either TV.
