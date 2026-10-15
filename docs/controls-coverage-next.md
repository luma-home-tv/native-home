# Control-centre coverage and next work

Current source review: `14edb5ca08e3d91f5ebcc5097613446c3df25da7` (2026-10-15).
The [retained-update record](preview-update-milestone.json) now establishes G2
0.26.0 after a backed-up normal-package upgrade and guarded network UI smoke;
C2 remains at its separately recorded 0.20.0. This document update reads source
and saved evidence, not fresh TV state. Normal G2 Home/Apps/Search/keyboard smoke
does not establish every control-page interaction, physical input or performance.

The original coverage audit used `f3e58d6495c859dae02e6773ea9b41d788ab69c2`
and excluded keyboard integration that was then concurrent and uncommitted.
Retain that historical boundary for its original conclusions. Later isolated
[guidance](guidance-read-milestone.json) and [Sound](sound-read-milestone.json)
trials add six protected read-only rows, with independent ordinary-app/native
component qualification on both models. The matrix includes these additions.
Their source is included in retained G2 0.26.0; their isolated qualification does
not imply installation in retained C2 0.20.0 or admit any new writes.

The target remains one native settings entry point with meaningful native controls
where individually verified, and explicit LG-managed routes for the rest. Six
category pages and working LG handoffs do not establish coverage of every LG
setting. Descriptor membership establishes a valid reading, never permission or
safety to write it.

## Coverage matrix

| Area | Native editable now | Validated native readings | LG-managed, protected or missing |
| --- | --- | --- | --- |
| Picture | None | Picture mode, OLED pixel brightness, contrast, black level; descriptor-validated colour depth, sharpness, tint, colour temperature on LG's scale, gamut, gamma, dynamic contrast, peak brightness | Picture overview handoff verified on both models. All edits remain LG-managed. Active video/HDR pipeline and per-input applicability are not established by a default settings profile. Motion, aspect ratio, noise reduction, game-specific and other picture controls lack registry coverage. Calibration/service controls remain protected. |
| Sound | G2-only signed audio-description level, -15 to 15, when description is off and the fixed operation's gates pass | TV output, TV volume, mute; optional sound mode/profile input when output context agrees. Later isolated proof adds global balance, description enablement and description level as protected read-only rows on both | Sound overview handoff verified on both. Master volume/mute, sound mode, output/routing, balance, description enablement, equalisation, delay and eARC/format edits have no Luma adapter. Volume does not measure an external amplifier. Audible mixing behaviour is unverified; C2 numeric writes remain gated. |
| Network | None | Internet availability, separate wired/Wi-Fi connection states, valid addresses and connected Wi-Fi name when supplied | Network entry via General is verified on both. Route priority and throughput are not measured. Reconfiguration, credentials, link enablement, DNS and addressing remain protected/LG-managed; no Luma network write is proposed. |
| Accessibility | Audio-description level as above, presented here | Audio guidance, high contrast, screen magnification, menu transparency; later isolated proof adds guidance volume, pitch and speed | Accessibility overview handoff verified on both. These seven readings remain read-only. Captions/subtitles and other accessibility options lack native coverage. Do not disable an accessibility aid to create a convenient test context. |
| General | Pointer speed: Slow/Normal/Fast, individually guarded on both models | Quick Start+, pointer speed, pointer size, LG Home auto launch, LG Home promotions | General overview handoff verified on both. Pointer size is read-only. Quick Start+ and Home auto launch affect preserved startup/Home behaviour and stay LG-managed. Promotions have no write adapter. Language, region, time/timers, storage/app management, external-device/CEC/Bluetooth and tuner/channel settings lack a native inventory or adapter. |
| Panel Care | None | Screen Move, logo brightness adjustment, energy saving | OLED Care overview handoff verified on both. OLED protections and compensation scheduling remain protected; no native edit, pixel-cleaning action or service shortcut is proposed. Firmware, reset and service/calibration functions remain protected regardless of where LG places them. |
| Inputs, adjacent to Controls | G2 HDMI selection through LG's application/source path; C2 production selection remains gated | Four HDMI identities, plug/signal flags and independently read selected source | G2 per-port selection/restoration is recorded. Live video/format/HDR and C2 exact idle tuner restoration remain open. No decoder, HDMI, audio or DRM service replacement. |

“Native editable” means an operation-specific path exists; it is not a general
setter. In `model.rs`, ordinary Picture/Sound/Network rows can have
`protected: false`, but their activation is `Action::Inspect`, not a write.
The expanded registry deliberately marks its rows protected. Only the explicit
pointer-speed and audio-description adapters add writes.

The two current setters use ordinary application identities. No privileged
helper is installed or needed for these proofs. The generic range widget's
existence does not enable other numeric settings.

## Evidence and limits

| Claim | Authoritative implementation / scoped record | Boundary |
| --- | --- | --- |
| Fixed read registry and context validation | `src/live.rs`, `src/control_reads.rs`; [readout record](control-readouts-milestone.json) | Twenty expanded readings independently matched on both models. Picture extras bind to input/picture-mode dimensions; before/after profile reads detect change. Missing, invalid or stale data becomes unavailable. Existing base-picture numeric validation is not descriptor-backed write validation. |
| Category navigation and LG routes | `src/model.rs`, `src/actions.rs`, `src/shell.rs`; [section record](settings-sections-milestone.json) | Six exact overview/entry routes were inspected and return/removal checked on both. The Network payload is General plus `item: network`; OLED Care is an overview. Exact subpage coverage cannot be inferred from a generic Settings window acknowledgement. |
| Pointer-speed edits | `src/pointer_speed.rs`; [pointer record](pointer-control-milestone.json) | Scripted native choice/change/readback/restoration on both; physical tracking/acceleration remains unmeasured. The retained updates themselves were no-write checks. |
| Audio-description numeric edit | `src/description_level.rs`, range paths in `src/shell.rs`; [numeric record](numeric-control-milestone.json) | Isolated ordinary-app G2 entry, Apply, endpoints, representation-preserving readback and restoration. Retained G2 readiness was checked without a write. No audible-effect, enabled-description or C2 write claim. |
| Overall design coverage | [design specification](../design.json), [plan](implementation-plan.md), [fidelity map](design-fidelity.md) | The comprehensive control centre is incomplete. Older milestone `remaining` arrays describe their historical revision; later records and current fixed action paths supersede them. |

## Next implementation sequence

1. **Make coverage legible before adding writes.** Give each visible setting a
   consistent state: native control, native reading, LG-managed, unavailable in
   this context, or protected. Distinguish an inactive descriptor from a failed
   read, and show source/context freshness without suggesting an inactive value
   is editable. Preserve existing shared FormTable, choice and range components,
   black canvas and graphite surfaces. Keep category entry points available when
   individual reads fail. Verify long names, unavailable rows and independent
   scrolling with synthetic host data, then isolated native frames.
   The next 0.27.0 candidate now connects the pure policy to actual form-row
   wording, with [synthetic host previews](verification/control-row-presentation/README.md),
   behaviour checks and an ARM build. Existing actions/disabled bindings are
   preserved. It does not infer an expiry cause from `available: false`.
   [Isolated G2 Sound/Accessibility read/layout qualification](control-row-presentation-milestone.json)
   now adds scoped native wording/scroll evidence; normal deployment and broader
   native conditional-state coverage remain pending.
2. **Finish validation of existing controls.** Prioritise physical Magic Remote
   choice/range navigation, staged-versus-saved clarity, Back/Cancel, repeat
   suppression and pointer/wheel behaviour. Prove expiry/external-change
   withdrawal and no late write after ownership loss. The audio-description
   test must keep description off; skip if the user's actual context cannot
   support a safe isolated test. Do not transfer G2 evidence to C2.
3. **Audit pointer size as the next small write candidate.** It already has a
   descriptor-validated read row and does not require changing input, playback,
   networking, startup or panel protections. First establish its exact enum,
   storage scope, permission and physical cursor semantics on this firmware.
   The display labels Small/Medium/Large in `model.rs` are presentation mappings,
   not evidence that these are the complete admitted choices. Only after the
   gate below passes should a separate fixed typed adapter reuse the choice UI.
4. **Expand read coverage in bounded groups.** Audit missing Sound and
   Accessibility settings first; add named getters/descriptors with explicit
   context and unavailable states. Then inventory General/external-device and
   input-format readings. Record which settings exist on this exact webOS24
   build and which only have an LG route. Do not copy proprietary LG Settings
   source or expose an arbitrary service/key inspector in the application.
5. **Consider further edits only after that audit.** Menu transparency is a
   possible later reversible UI preference, with a human present to confirm its
   actual effect and accessibility needs. High contrast, guidance and
   magnification can affect the user's ability to operate the TV and should not
   be selected as convenience test writes. Picture choice/range edits require a
   known real input/dynamic-range/profile context and separate visual validation;
   the current no-signal/default-profile receipts are insufficient. Keep sound
   routing, Home/boot behaviour, networking and panel protections delegated.

## Admission and proof required for each new edit

Each candidate needs its own evidence; do not parameterise either existing
setter into a generic category/key endpoint.

- **Capability:** exact model, firmware, SDK and application identity; a fixed
  ordinary-app API surface actually admitted under that identity. A readable
  descriptor, root access or installation permission alone proves no write.
  If ordinary-app access is unavailable, retain LG management until a separately
  reviewed fixed-operation local helper has caller/executable/session checks.
- **Descriptor and semantics:** unique key/category, known type and wire
  representation, exact allowed enum or bounded range/step, visible/active
  metadata, expected database/account/volatility scope and dimensions. Reject
  schema drift. Establish what the control changes and what nearby state it
  could affect; do not infer physical units from a numeric range.
- **Context and ownership:** fresh expected value, fresh actual input/HDR/mode
  where relevant, owned foreground card with no covering overlay, bounded queue
  age and validation duration. Recheck value/context immediately before commit.
  A settings change elsewhere invalidates a staged edit; no compare-and-swap
  guarantee is claimed where the service supplies none.
- **Bounded action and readback:** one typed request off the render/input thread,
  bounded calls/replies/retries, no automatic optimistic Saved marker. Record
  attempted, admitted and independently verified separately. Confirm exact
  stored value and representation, and independently observe the physical/UI
  effect needed by that operation's claim. Reject stale read cycles begun before
  a confirmed change.
- **Restoration:** save original value/type and relevant surrounding state;
  perform the smallest reversible test change and restore/read back the original
  using the same fixed validated path. Read back adjacent controls and preserved
  source/picture/audio/core state. Do not force an old value over a later external
  change or widen to arbitrary root recovery. Failure leaves the editor gated
  and the outcome explicit.
- **Device closeout:** isolated identity and exclusive device lease, preserved
  retained app/favourites, independent foreground/window observations, then
  package/process/catalogue/launchpoint/runtime/state/jail/mount removal and
  original foreground/state verification. Keep raw settings and household frames
  private; publish source revisions and scoped boolean/count evidence.

Luma owns the dashboard registry, adapters, state and composition. Continue to
attribute PlxNative's pinned rendering/UI/input framework and preserve its
licences. Generic widget improvements belong in the attributed integration
patch and can be proposed upstream separately; Plex client features remain
outside this application.
