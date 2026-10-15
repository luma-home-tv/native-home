# Control capability presentation model

Status: host-only pure module, registered in the library for shared checks and
future integration. It is not used by the normal interface. No app behaviour,
capability gate, setting adapter or TV installation changes with this slice.

The existing application already has useful but separate concepts:

- `model::Setting` carries read availability, exact `Context` and optional
  descriptor constraints. Its `protected` field also marks the expanded
  read-only registry; it is not exclusively the policy for genuinely protected
  functions. Current pointer-speed presentation explicitly overrides that flag
  only through a separate fixed adapter.
- `live::Freshness` expires ordinary reads using the existing 15-second rule.
  The audio-description worker uses its own narrower eight-second admission
  sample. Those rules remain with their owners.
- `shell::pointer_value` and the audio-description path independently gate
  editors. `model::controls_form_with_numeric` provides verified LG entry routes
  independently of native value rows.

The new module consolidates presentation decisions without duplicating their
parsers, timers, settings values, rendering or service operations. It cannot
establish a capability or perform a write.

## Interface

`evaluate(Control) -> Presentation` is deterministic, allocation-free and
standard-library-only. Inputs are explicit summaries of existing validated facts:

| Input | Contract |
| --- | --- |
| `setting_id` | Stable setting identity used to bind write evidence to this row |
| `Policy::ViewOnly` | A native reading with no native editing |
| `Policy::LgManaged` | Delegated setting; a write result cannot promote this policy |
| `Policy::NativeCandidate` | A specifically implemented and reviewed operation; still requires every admission below |
| `Policy::Protected` | Never native editable, regardless of descriptor, write result or route |
| `reading: EvidenceState` | Current, unavailable, stale or wrong-context, computed from existing availability/freshness/context logic |
| `descriptor: DescriptorState` | Active, inactive or unknown. Descriptor state may veto an editor; it cannot supply write authority |
| `write: Option<WriteCapability>` | Setting-ID-bound admission from a separate fixed adapter. Its state must include that operation's actual platform/identity, metadata, ownership, expiry and context gates |
| `lg_route_verified` | A separately qualified route; independent of reading/native-edit availability |
| `busy` | A matching operation is already pending; blocks another edit |

`Presentation` returns a display state, precise `EditEligibility` reason,
`value_is_current`, `operation_pending`, `lg_settings_available` and `can_edit()`. It does not contain
an action, service URI, raw setting value, string label, timer or rendering token.
The normal string/component system remains the place for user-facing wording.

A current read can remain legible while its write admission expires. Conversely,
a stale/wrong-context read never presents its value as current just because the
write adapter was recently admitted. A protected row can retain a validated
readout, but its protected policy is always visible and its editor always denied.
An inactive descriptor is distinguished from a failed read. While admission is
still current, a pending native operation has `edit: Busy`. If admission expires
or its context changes, the presentation reports that cause and independently
retains `operation_pending`. That operation still needs a worker outcome and
independent closeout; losing edit eligibility does not cancel it.

`value_is_current: false` means integration must hide the value or clearly label
it last-known; it must never display it as a current/live reading. LG route
availability survives all reading/editing failures, but the model never invents
a route when its input is false.

## Integration boundary

The coordinator owns later wiring in `model.rs`, `shell.rs` and `live.rs`.
That work should:

1. Derive reading status from the current `Setting.available`,
   `Setting::still_current` and existing domain timestamp, with the appropriate
   clock and expiry for that specific reading. Do not add a competing timeout.
2. Assign explicit policy from a fixed operation registry. Do not mechanically
   convert `Setting.protected` into `Policy::Protected`: the former currently
   also means an unverified, LG-managed registry row. OLED/firmware/network/boot
   protections stay protected, while a separately validated pointer-speed path
   can be an explicit native candidate.
3. Build `WriteCapability` only from the matching fixed adapter's real current
   admission. A valid descriptor, available value, host fixture, readable service
   or generic setter permission is insufficient. The unconnected pointer-size
   proof candidate does not acquire normal write authority through this model.
4. Preserve exact setting identity and invalidate both read and write admission
   on relevant context transitions. A fresh new-context reading cannot revive
   the old-context write admission. Context generations/values and proof of
   invalidation remain the integration owner's responsibility.
5. Re-evaluate when readings, context, descriptors, admission, ownership or busy
   state change. Withdraw unsubmitted edits when admission is lost. Keep an
   already queued/in-flight operation tracked until its worker outcome and
   closeout; `can_edit() == false` blocks new edits and does not cancel or discard
   existing work. This pure model cannot close a real UI or cancel a worker.
6. Keep final expected-value/context/ownership/readback validation inside the
   fixed setter. `can_edit()` is UI eligibility only, never a cached action token
   or proof that a queued gesture will still be admitted.

`WriteCapability` is caller-constructed data inside a trusted Rust process, not
a security capability. Identity matching prevents accidental evidence reuse
between rows; it cannot verify provenance or stop an incorrect caller from
inventing facts. Protect that boundary with explicit adapter wiring and review.

## Verification

The initial module compiled directly with `rustc --edition 2021 --test -D warnings`
in a separate agent-owned output path. After library registration and independent
review, seven focused tests pass as part of the shared all-target host suite
(157 library tests and two actual keyboard-handler regressions before the
guidance registry addition). Its
negative/transition cases cover active descriptors
without write evidence, mismatched/empty identities, protected and delegated
policy despite write evidence, reading expiry/context changes, independently
expired write admission, unknown/inactive descriptors, pending operations and
unavailable LG routes and tracking an in-flight operation after admission expires.
This verifies pure presentation decisions only; native
integration, actual edit withdrawal, layout, device behaviour and state/removal
proof remain later acceptance gates.
