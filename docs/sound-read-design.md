# Optional global Sound read group

Status: [isolated 0.26.0 ordinary-app/native component qualification](sound-read-milestone.json)
on both TVs; the retained previews are unchanged. `src/sound_reads.rs` defines
the pure parser and two fixed GETs;
`native_probe` appends those GETs, `live` merges the three protected rows and the
existing Sound form renders them. No setter, writable UI action or write
capability is added. The separately guarded audio-description editor is
unchanged. Earlier frozen keyboard/lifecycle tests do not exercise this candidate.

## Evidence and scope

The [read audit](read-coverage-audit.md) identifies the saved G2 root-side audit
`g2-numeric-control-audit-v1/audio-description-{descriptors,values}.json` in the
private build workspace. It records category `sound`, empty global getter
dimensions, descriptor dimensions `[]`, `dbtype: G`, `account: false`,
`volatile: false`, `valueCheck: true` and `valueTypeCheck: false`. Descriptors are
visible and were active in that sample. Raw replies and selected household
values remain private.

| Fixed identity | Read schema | Display focus key |
| --- | --- | --- |
| `audioBalance` | Signed -50..50, step 1; integer or canonical decimal string | 30 |
| `audioDescription` | Exact string enum `on` / `off` | 31 |
| `audioDescriptionVolume` | Signed -15..15, step 1; integer or canonical decimal string | 32 |

The original saved audit contains decimal-string numeric readings. Historical
description-level trials also encountered integer readbacks. This parser
supports and retains either canonical representation for each numeric row; its
host tests do not establish that every representation occurs on both models.
Neither scale is claimed to be percent, dB or an external amplifier's volume.
Description enablement is read data here: an enabled description does not hide
its stored level, and no enablement or audible mixing change is requested.

These are global preferences. `soundMode` has an `audioInput` dimension in the
ordinary-app survey and is explicitly outside this group. No selected-input,
HDR, sound-route or active audio-pipeline inference is made from these values.
Root read success alone does not establish ordinary-app permission or C2 behaviour;
the later milestone records each model's separate ordinary-app trial.

## Integration interface

- `operations()` returns exactly two fixed `(reply_name, URI, payload)` tuples:
  `sound_extra_values` → `getSystemSettings` and `sound_extra_desc` →
  `getSystemSettingDesc`. Both payloads contain only category `sound` and the
  three named keys above. There is no caller-supplied URI, key, dimension,
  account, subscription, reset or other parameter.
- `parse(replies) -> Result<Vec<model::Setting>, GroupError>` consumes those two
  reply names and returns the three stable global-context rows. Every row has
  `protected: true`; unavailable rows have no constraint. Valid range/choice
  metadata is read validation, never editing authority.
- `parse_typed(replies) -> Result<[Readout; 3], GroupError>` retains exact numeric
  integer-versus-decimal-string encoding and explicit row failure reasons.
  `ReadValue::raw_value()` provides a read-side round trip for diagnostics, not
  a service operation or setter payload. Normal `Setting.value` is a display
  string and does not carry wire encoding; do not reconstruct write requests
  from that display value.
- `unavailable_rows()` returns only these same three protected, unavailable
  identities/keys with no constraint, for optional-group refusal or expiry. It
  never generates placeholders for the baseline Sound rows.
- Numeric strings must round-trip canonically. Leading zeros, `-0`, plus signs,
  whitespace, decimal/exponent strings, booleans, floats, overflow and values
  outside the fixed range are rejected. Enablement accepts only exact strings,
  never coerced booleans/numbers.
- Each descriptor must have the evidenced global category/storage/validation
  metadata. Range bounds/interval are exact; its values object contains only a
  range with the three named fields. Enablement requires exactly two unique,
  active, visible on/off choices and no mixed range/enum schema. Top-level UI
  `active: false` can retain a valid readout with an inactive constraint; a
  hidden or malformed descriptor cannot. This does not admit an editor.

Bad group envelopes fail closed: refusal, wrong category, nonempty or malformed
global dimensions, invalid settings/results containers, excessive fields,
unexpected keys or malformed descriptor rows return `Err`. An optional envelope
category/dimension, when supplied on the descriptor reply, must agree with the
global Sound scope too. Within an otherwise valid group, missing/duplicate
descriptors, missing values and invalid per-key metadata/types withdraw only that
row; unaffected requested rows remain available.

## Integrated wiring and remaining acceptance

The candidate uses a **separate optional namespace and freshness stamp**:

1. Collection appends the two fixed operations and preserves `sound_extra_*`
   replies separately from the existing Sound getters. `ControlSection::Sound`
   stores their freshness separately from `Freshness.sound`, which continues to
   qualify the core output/volume/mute/profile readings. Time is the conservative
   cycle-start observation stamp, not a per-call or per-frame measurement.
2. Only the three fixed IDs at keys 30..32 are replaced by an optional merge.
   Core success replaces only core rows at 20..24; optional denial creates only
   the three unavailable protected placeholders. Core failure retains its last
   values with `available: false`; the form displays `Unavailable`, not those
   old values. One group's failure/expiry cannot withdraw the other group.
3. `needs_expiry` and `expire` share one per-row freshness predicate. Once a stale
   group's rows are withdrawn, it cannot force continuous redraws merely because
   the other group stays available. Recovery replaces each group independently.
   The Sound status label reports partial availability only when at least one
   current displayed row is available; fresh empty envelopes cannot make wholly
   unavailable data appear partly live. Bad optional per-key metadata withdraws
   only that row.
4. The parser preserves explicit global context. Merge binds protected rows to
   the current snapshot's presentation context, matching the existing global
   groups and exact-context form model; it never reinterprets them as selected
   audio input or picture-profile settings. A later picture context change
   without valid replacement cannot promote old data to current.
5. Rows remain read-only/LG-managed, including the description-level reading.
   The separate Accessibility editor keeps its fixed G2 operation and all its
   gates. Neither balance nor description enablement gets a native action.
   Sound's existing validated LG Settings route remains available.

The [native milestone](sound-read-milestone.json) now records independent G2 and
C2 trials of an immutable isolated candidate. All three typed values and exact
schemas matched independent readbacks, and each protected row was focused,
disabled and fully inside the native viewport. Recorded positions establish
down/up scrolling and Back return. Core output/volume/mute/profile readings also
matched their independent baseline. No preference, audio route, source or
accessibility setting was changed. Complete removal and retained app/favourites,
settings/core/source/foreground preservation were verified on each model.
Native partial-error/stale transitions, audible effect and retained integration
remain pending. An unavailable model stays unavailable; root replies never
substitute for ordinary reads in the UI.

## Host verification boundary

The initial integrated host run passed 175 library tests, two shared-handler
regressions and all-target clippy with warnings denied, using a separate
agent-owned target. Independent review found a status-label error when valid
empty optional replies were fresh but every Sound row was unavailable. A new
regression reproduced the error before the fix. The final coordinator run passes
176 library tests, two shared-handler regressions and all-target clippy with
warnings denied. The diagnostic readout receipt now also includes Sound.
A separate ARM all-target `cargo check` also passed, including the ARM-only
collector chain (the pre-existing unstable `neon` target-feature warning
remains). The final coordinator ARM release build of `home-preview` also links
successfully with that existing warning. Neither check establishes deployment
or native runtime behaviour.
Nine parser tests plus integration tests cover fixed GET payloads, protected
identities, signed endpoints/encodings, rejected coercions, independent row
errors, descriptor/category/dimension drift, denial and malformed envelopes.
Integration regressions additionally cover core/optional partial failures,
profile preservation, independent expiry in both directions, expiry predicate
quiescence, recovery without duplicates, read-only focus/scroll, unchanged
preferences and the existing LG Sound route. Host checks do not establish
ordinary-app permission, physical/audio effect, device cleanup, performance or
native runtime layout. Current test results are recorded in the coordinator's
private build workspace before publication.

## Isolated native qualification profile

The 0.26.0 package adds an explicit `sound-read-proof.json` marker and
`native_sound_read_proof` receipt flag. It requires the native noninteractive
Proof identity and the existing read probe, and rejects other action, input,
performance and preferences trials, including their command-line forms. The
normal retained profile never enters this helper. Existing 0.25.0 artifacts and
receipts remain tied to their original binaries.

The Sound helper uses shared arrows to focus rows 5..7 at keys 30..32, captures
each fully contained disabled row, moves back up to balance and returns through
Back to the Sound category. Its separate `sound_layout_proof` records schema 1,
layout/graphics/presentation/readout snapshots and preference/request invariants.
It does not activate OK, call a setter or change LG audio/accessibility state.
The existing read probe supplies ordinary-app raw values/descriptors for an
independent typed comparison. Capture helpers do not establish per-stage service
freshness or failure transitions. Native execution and cleanup results belong in
a separate milestone; adding this instrumentation alone does not qualify them.

This is Luma-owned read-adapter code using the existing model and attributed
PlxNative framework. No Plex features or proprietary LG implementation code are
included.
