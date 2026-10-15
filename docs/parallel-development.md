# Parallel development plan

Use one coordinator and up to three background agents. Parallelise independent
research, review, host work and per-device validation; reconcile shared rendering
and application changes through one integration owner. This extends the
[implementation plan](implementation-plan.md) and uses the
[controls coverage audit](controls-coverage-next.md) as the control-centre backlog.
It does not change device authorisation or the protections in `AGENTS.md`.

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

- The ABI/text reviewer can audit source, recorded device-derived layout facts
  and fixtures without operating a TV or changing the active renderer.
- The controls agent can extend the coverage matrix and write pointer-size
  admission/restore cases while keyboard integration continues. Pointer speed
  and G2 audio-description level are the only current fixed setting writers;
  pointer-size labels are not proof of its complete enum or write capability.
- The C2 agent can inspect compatibility gates and prepare read-only preflight,
  independent keyboard-window observation, retained-state backups and cleanup
  checks. It must wait before running an unqualified keyboard candidate.
- The coordinator can integrate and run G2 validation independently of those
  host/review tasks. G2 and C2 device jobs may overlap only with separate leases,
  SSH agents, package/output directories and teardown ownership.

The reconciled candidate source is `760db0eb11217e3de7766bc7d3223d995d1fc634`. It is a candidate, not a release receipt.
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
