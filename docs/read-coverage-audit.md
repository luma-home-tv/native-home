# Sound and Accessibility read-coverage audit

Audit date: 2026-10-15. Source audit baseline `8121333`, including
`src/control_reads.rs`, `src/native_probe.rs`, `src/live.rs` and the fixed
audio-description adapter. A later host-only candidate in `control_reads.rs`
adds the three guidance rows described below; it is not part of the frozen
`8121333` native lifecycle tests and is not a deployment receipt.
Saved receipts were inspected locally; no TV was contacted for this work.
Historical evidence is not a current installation or settings-state query.

## Evidence scope

The following paths identify existing private evidence under the build workspace;
raw replies and household values remain private:

| Evidence | What it establishes |
| --- | --- |
| `g2-control-read-audit-v1/accessibility-{descriptors,values}.json` | G2 root-side schema/value audit of the existing four Accessibility keys |
| `g2-control-read-proof-v2/run-1/native-api-probe.json` | G2 ordinary-app registration, no writes, accepted existing Accessibility descriptors/values and baseline Sound getters |
| `c2-range-layout-proof-v1/run-1/native-api-probe.json` | Independent C2 ordinary-app accepted existing Accessibility descriptors/values and baseline Sound getters |
| `g2-reading-survey-proof-31f502b/run-1/native-api-probe.json` and `c2-controls-proof-v2/run-1/native-api-probe.json`, `survey` section | Ordinary-app Sound API aliases and descriptor dimensions on each model; these records identify ordinary UIDs and no writes |
| `g2-guidance-range-audit-v1/guidance-{descriptors,values}.json` | G2 root-side schema/value audit for guidance enablement, volume, speed and pitch |
| `g2-numeric-control-audit-v1/audio-description-{descriptors,values}.json` | G2 root-side schema/value audit for description enablement, level and balance |
| [Numeric milestone](numeric-control-milestone.json) and current `description_level.rs` | Narrow G2 ordinary-app description enable/level checks supporting the fixed level operation with description off; not a broad Sound read registry |

The guidance/numeric audit scripts use the root SSH `luna-send` helper. Their
success cannot establish ordinary-app access to a newly added key. Independent
root readbacks used by later cleanup drivers likewise are not ordinary-app
capability receipts.

## Already covered

| Domain / exact field | Recorded type and scope | Current use and qualification |
| --- | --- | --- |
| Audio `volume` | JSON integer, parser bounds 0..100, no settings dimensions | Native TV-volume reading via `com.webos.audio/getVolume`; ordinary-app acceptance recorded on both models. Not external-amplifier volume. |
| Audio `muteStatus` | JSON boolean, same getter | Native mute reading on both; string booleans are rejected. |
| Audio `scenario` | Bounded string, same getter | Required by parser as reply-shape evidence; not presented as volume-routing authority. |
| Audio/settings `soundOutput` | String; settings category `sound`, descriptor dimensions `[]`, `ArrayExt` | Native output summary on both. Output is sampled before/after the volume call; changed output withdraws that Sound cycle. Settings output must agree before optional profile fields are shown. |
| Settings `soundMode` | String; category `sound`, descriptor `ArrayExt`, dimensions `[audioInput]`; getter supplies `dimension.audioInput` | Native optional sound-profile reading on both, alongside its bounded-string profile-input label. Existing live parsing does not consume this descriptor or prove the active HDMI/audio pipeline. |
| Option `audioGuidance`, `highContrast`, `menuTransparency` | String `ArrayExt` choices `on`/`off`; global descriptor dimensions `[]`; getter omits dimensions | Descriptor-validated native readouts on both. Each recorded descriptor is `G`, non-account, nonvolatile, `valueCheck: true`, `valueTypeCheck: false`, UI active/visible. No native toggle. |
| Option `screenMagnification` | Same global on/off string schema, but recorded `volatile: true` | Existing descriptor-validated readout on both. It must not inherit a setter's nonvolatile-storage assumption. No native toggle. |
| Sound `audioDescriptionVolume` with `audioDescription` | Global signed level -15..15 step 1, with on/off enablement | Separate G2 admitted worker/editor path when description is off. The fixed adapter accepts an integer or canonical decimal string and retains representation for its own writes. No general read row for description enablement and no C2 level-control qualification. |

Ordinary surveys accepted `com.webos.audio/getVolume` and
`com.webos.service.audio/getSoundOutput` on both models, while
`com.webos.service.audio/getVolume` returned a refusal in the inspected records.
The latter is not an interchangeable fallback. The survey also accepted the
legacy `com.webos.audio/getSoundOut`; normal collection uses the qualified modern
output getter. Do not replace aliases based on naming consistency.

The Sound descriptors have `dbtype: G`, `account: false`, `volatile: false`.
The recorded `soundMode` descriptor is visible but UI-inactive on both samples;
the global `soundOutput` descriptor is active/visible. Their choice sets contain
inactive/hidden members. No full enum is admitted here, and none is write
authority. A profile may remain a valid reading when changing it is inactive.

## Exact additional keys evidenced on G2

All rows below have accepted G2 root-side descriptor and value receipts. Their
descriptor dimensions are `[]`; getter scope is global (no nonempty dimension).
All have `dbtype: G`, `account: false`, `volatile: false`,
`valueCheck: true`, `valueTypeCheck: false`, and UI active/visible. Every recorded
enum member below is active/visible. No selected household value is reproduced.

| Category / key | Exact descriptor schema | Recorded value encoding | Gap |
| --- | --- | --- | --- |
| `option/audioGuidanceVolume` | `ArrayExt`: `veryLow`, `low`, `medium`, `high`, `veryHigh` | String enum | Implemented in the host candidate; ordinary-app, C2 and native layout admission unproved |
| `option/audioGuidancePitch` | `ArrayExt`: `veryLow`, `low`, `medium`, `high`, `veryHigh` | String enum | Same |
| `option/audioGuidanceSpeed` | `ArrayExt`: `veryFast`, `fast`, `medium`, `slow`, `verySlow` | String enum; preserve descriptor ordering | Same |
| `sound/audioBalance` | `Range`: min -50, max 50, interval 1 | Signed decimal string in inspected audit/closeout values | Missing normal Sound row and descriptor-backed collection; ordinary-app admission unproved; no decibel/percentage or audible-effect claim |
| `sound/audioDescription` | `ArrayExt`: `on`, `off` | String enum | Fixed G2 adapter reads it as a gate, but no normal enablement read row; no toggle proposed |
| `sound/audioDescriptionVolume` | `Range`: min -15, max 15, interval 1 | Decimal string in original audit; integer also occurred in historical mutation/readback records | Fixed G2 path exists; a general read-only row usable when enabled or on C2 is unqualified |

The guidance fields are discrete preferences, not sliders. Their names and the
historical audit directory name do not imply numeric ranges. Audio balance and
description level are LG scales with different ranges; neither may be labelled
master volume, percent or dB without separate evidence.

## Smallest next integration slice

The host candidate now implements **only the three guidance parameter readouts**:

1. Appended stable identities/specs for `audioGuidanceVolume`,
   `audioGuidancePitch` and `audioGuidanceSpeed` to the existing Accessibility
   option registry at keys 104, 105 and 106, following existing keys 100..103.
   Registry-wide uniqueness is checked. Its fixed values/descriptors batch grows from four to seven
   keys, within the existing eight-key bound, with no additional service calls.
   All rows remain protected/read-only; operation count remains ten fixed
   getters/descriptors and no setter was added. Do not toggle guidance to test them.
2. The existing read parser now has an identity-specific veto requiring each
   guidance field's exact five-member string `ArrayExt`, global option scope,
   active/visible unique members and no mixed range payload. Unknown/numeric
   types, malformed options and unexpected dimensions cannot infer a slider.
   Synthetic regressions cover partial missing values/descriptors, duplicate
   descriptors, wrong enums, mixed schemas and denied/malformed group replies.
   A valid group can retain its unaffected rows when one key fails; an invalid
   group envelope is rejected as a whole. Section expiry and real focus/visual
   behaviour remain integration/device acceptance gates.
3. In a later isolated ordinary-app proof, require accepted named getters and
   descriptors on each target model, current-value membership and independent
   readback matching without changing any setting. A root audit is insufficient.
   Inspect the native long-label/scroll/availability states and verify complete
   removal plus retained/core/settings/source/foreground preservation. Do not
   transfer G2 qualification to C2 or change the user's accessibility settings.

This candidate has no ordinary-app admission proof for the new keys, no C2
qualification and no native frame review. It does not promote write authority
or change the established fixed audio-description adapter.

Host verification uses a separate agent-owned Cargo target: nine focused
control-read tests and hostsim library/test clippy with warnings denied pass.
After integration with the shared live fixtures and display labels, all 162
library tests and two actual keyboard-handler regressions pass; all-target
hostsim clippy with warnings denied and the ARM release build also pass.
Those checks do not establish TV access or native layout behaviour.
The cases also distinguish a valid reading with an inactive top-level UI
descriptor from malformed/inactive individual enum members. No shared parent
build target or running device candidate is changed by these checks.

Then consider a **separate global Sound read group** for `audioBalance`,
`audioDescription` and `audioDescriptionVolume`, using two fixed read-only calls.
Do not add it to the existing profile-dimensional getter and infer global scope
from the combined response. Validate dimensions and signed range/string types;
preserve optional-group failure independently of valid output/volume/mute rows.
Give this group its own conservative freshness receipt and independent
ordinary-app matching proof before claiming native coverage. Reading a level
must not require changing description enablement or promoting a numeric editor.

## Rejected shortcuts and remaining unknowns

- No saved descriptor/getter pair was established in this audit for additional
  caption/subtitle, equaliser, delay, eARC/format, device-route or other missing
  settings. Their exact keys, types, dimensions and ordinary-app access remain
  unknown here; this is not evidence the TV lacks those functions. Keep their
  verified category entry routes LG-managed rather than inventing identifiers.
- `control_reads::read` currently understands global dimensions or Picture
  `input`/`pictureMode` dimensions. It cannot validate Sound's `[audioInput]`
  descriptor by passing a Picture context or dropping dimensions. Any future
  descriptor-backed sound-mode path needs its own exact audio-context validator.
- Current optional sound-profile parsing proves only bounded strings, category
  and output agreement. It does not detect every possible profile/context
  change between separate calls or establish atomicity. Broader validation
  needs deliberately bounded before/after snapshots and an explicit scope.
- Do not accept a new field merely because its current string fits the generic
  scalar parser. Match the exact enum/range and relevant dimensions; missing,
  duplicate, malformed or stale evidence must not become a live value.
- Read metadata may be volatile or UI-inactive. That is distinct from missing
  read data and cannot be translated into write eligibility. Only a separately
  verified fixed adapter can supply that admission.
- No write, listening test, physical effect, reboot persistence or performance
  claim follows from this read audit. Existing performance receipts do not
  measure a larger future registry. Luma remains an independent dashboard using
  the attributed PlxNative framework, with no Plex features or copied LG code.
