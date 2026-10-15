# Sound and Accessibility read-coverage audit

Audit date: 2026-10-15. Source audit baseline `8121333`, including
`src/control_reads.rs`, `src/native_probe.rs`, `src/live.rs` and the fixed
audio-description adapter. Later implementation and the separate
[0.25.0 guidance milestone](guidance-read-milestone.json) add three readouts with
independent ordinary-app/native component proof on both models. The initial
schema audit used saved receipts; the later milestone identifies the actual
device work. Neither expands the earlier `8121333` lifecycle receipt or updates
the retained previews.

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
| `option/audioGuidanceVolume` | `ArrayExt`: `veryLow`, `low`, `medium`, `high`, `veryHigh` | String enum | Isolated ordinary-app reads and focused native layout qualified independently on both models; retained update pending |
| `option/audioGuidancePitch` | `ArrayExt`: `veryLow`, `low`, `medium`, `high`, `veryHigh` | String enum | Same |
| `option/audioGuidanceSpeed` | `ArrayExt`: `veryFast`, `fast`, `medium`, `slow`, `verySlow` | String enum; preserve descriptor ordering | Same |
| `sound/audioBalance` | `Range`: min -50, max 50, interval 1 | Signed decimal string in inspected audit/closeout values | Missing normal Sound row and descriptor-backed collection; ordinary-app admission unproved; no decibel/percentage or audible-effect claim |
| `sound/audioDescription` | `ArrayExt`: `on`, `off` | String enum | Fixed G2 adapter reads it as a gate, but no normal enablement read row; no toggle proposed |
| `sound/audioDescriptionVolume` | `Range`: min -15, max 15, interval 1 | Decimal string in original audit; integer also occurred in historical mutation/readback records | Fixed G2 path exists; a general read-only row usable when enabled or on C2 is unqualified |

The guidance fields are discrete preferences, not sliders. Their names and the
historical audit directory name do not imply numeric ranges. Audio balance and
description level are LG scales with different ranges; neither may be labelled
master volume, percent or dB without separate evidence.

## Guidance implementation and qualification

The implementation adds **only the three guidance parameter readouts**:

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
3. Separate isolated ordinary-app 0.25.0 trials now verify accepted getters and
   descriptors, all seven typed values matching independent reads, and each new
   row focused, disabled and fully inside the native viewport. Captures and
   recorded positions establish actual down/up scrolling and Back return.
   Each trial removed its proof package and preserved retained/core/settings/
   source/foreground state. No accessibility preference was changed.

The [native milestone](guidance-read-milestone.json) qualifies these component
results, not physical remote input, per-frame service freshness, performance or
normal-identity retained promotion. Stage readouts are captured snapshots.
No write authority or change to the fixed audio-description adapter follows.

Host verification uses a separate agent-owned Cargo target: nine focused
control-read tests and hostsim library/test clippy with warnings denied pass.
After integration with the shared live fixtures and display labels, all 162
library tests and two actual keyboard-handler regressions pass; all-target
hostsim clippy with warnings denied and the ARM release build also pass.
Those checks do not establish TV access or native layout behaviour.
The earlier frozen host/ARM candidate was source
`3a54428245769df6cb4cbef79427ba32e4a5e3df`, binary SHA256
`73b6d190e636abb9e6b7112a7fcd2e6440054aa92a6c61b49126236f54727953`.
The 0.24.0 lifecycle milestone uses the earlier `8121333` binary and does not
qualify these added readings.
The cases also distinguish a valid reading with an inactive top-level UI
descriptor from malformed/inactive individual enum members. No shared parent
build target or running device candidate is changed by these checks.
The later layout-proof source is `a46ddcf9be8d3bf5d06eb6af72ddc531cb547fca`,
with 163 library tests plus two handler regressions, clippy and ARM build passing.
Its source/package guards admit the read/layout marker only under the isolated,
non-interactive native dashboard identity.

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
