# Pointer-size candidate

Status: unconnected, proof-only implementation and mock-platform validation.
No pointer-size setter has been exercised on a TV for this work. No normal
control, worker, package marker or retained-preview capability is enabled.
The coordinator owns admission to an isolated device trial and any later UI
integration. See the [coverage audit](controls-coverage-next.md) and
[parallel development plan](parallel-development.md).

## Recorded schema

The private G2 control-read audit and ordinary-app read proof, and the C2 range
proof's ordinary-app read receipt, contain matching pointer-size descriptors.
The precise historical evidence locations are
`g2-control-read-audit-v1/general-option-descriptors.json`,
`g2-control-read-proof-v2/run-1/native-api-probe.json` and
`c2-range-layout-proof-v1/run-1/native-api-probe.json` in the private build
workspace. These were inspected from disk; neither TV was contacted. Raw replies
and the household's selected size remain private.

| Field | Recorded value / meaning |
| --- | --- |
| Category and key | `option`, `pointerSize` |
| Value encoding | String: `small`, `medium` or `large` |
| Descriptor type | `ArrayExt`, with exactly those three unique choices |
| Choice availability | Every recorded choice is active and visible |
| Storage scope | `dbtype: G`, `account: false`, `volatile: false` |
| Validation flags | `valueCheck: true`, `valueTypeCheck: false` |
| Dimensions | Descriptor `[]`; getter has no dimension field in the inspected receipts |
| UI metadata | Active and visible; recorded widget `None` and display name `pointerSize` |

These facts establish a bounded parser and candidate payload, not permission to
write, observed cursor-size effect, persistence across reboot or equivalence
between the models. The descriptor's `valueTypeCheck: false` is not a reason to
accept arbitrary types: Luma accepts only its closed string enum. Display name
and widget metadata are not used as write-authority signals.

## Module contract

`src/pointer_size.rs` is Luma-owned application adapter code, separate from the
attributed PlxNative framework. It introduces:

- `Size::{Small, Medium, Large}` and a closed `Change { expected, value }` wire
  type which rejects unknown fields. No setting/category/service string can be
  supplied by a caller.
- `inspect(platform, own)` for candidate read/descriptor inspection. It requires
  the isolated proof identity, G2 model, firmware `23.25.55`, SDK `9.2.2`, the
  owned foreground app and a nonempty foreground-window list containing only
  its own CARD rows, exact known descriptor metadata and a
  string value in the admitted enum. C2 remains refused despite its matching
  recorded read schema.
- `apply(platform, own, change, enabled, queued_at_ms)`, for a separately admitted
  proof harness only. The explicit enable flag defaults to no invocation because
  no caller exists. Its outcome separates attempted, admitted and verified.
- A mockable `Platform` with bounded calls, waits and a single monotonic clock;
  `queued_at_ms` must come from that platform's `now_ms()` clock.
- An ARM native transport which refuses construction outside the actual proof
  installation identity. Its fixed setter payload contains only
  `category: option` and `settings: { pointerSize: <closed enum> }`, with no
  account, dimension, reset, subscription or extra flags. Calls have a 700 ms
  timeout and replies are limited to 64 KiB.

The candidate expires a queued gesture after two seconds, rejects a future queue
timestamp, rechecks expected value and foreground ownership before commit, and
expires pre-write validation after four seconds. A no-op still validates current
ownership/schema/value but sends no setter. After an acknowledgement, at most
four readbacks within a bounded readback interval may confirm the exact string;
foreground ownership must still hold. Every ownership check rejects foreign
CARD rows, LG KEYBOARD windows, overlays, missing fields and unknown window
types. This strict proof-only rule requires rechecking the actual trial's window
context; it does not claim that historical settings receipts established this
complete allowlist. Stored-value confirmation makes no claim about the physical
pointer.

Restoration is a fresh reverse `Change` through the same gates. If the current
value no longer equals the test's expected changed value, the adapter refuses;
it does not force the original over another writer. There is no root fallback,
generic recovery setter or compare-and-swap guarantee. A failed readback is not
treated as success, and an automatic retry must not blindly repeat a write.

An attempted but unverified operation is ambiguous: the TV may have applied the
change before the setter timed out, or acknowledged it before a later getter
failed. `admitted: false` means no accepted reply was observed, not that nothing
changed. The trial harness must independently reread the actual size and relevant
context before deciding what recovery is appropriate. A fresh guarded reverse
change is allowed only when the actual state unambiguously matches the trial's
changed value and ownership/schema/context are freshly admitted. If already
original, verify it without another write. If a different value, unknown state or
external writer is observed, stop and report the unresolved state; do not restore
blindly or treat package removal as setting restoration.

Even `verified: true` is a sampled readback claim. A separate writer can change
the size after the matching getter and during the following ownership check;
those calls are not atomic. The adapter cannot guarantee the value is still
current when it returns or exclude a concurrent writer. Trial closeout therefore
needs fresh independent readback, and no UI or receipt may present this as CAS.

This API is an internal trusted Rust interface, not a privileged broker or
security boundary against other code in its own process. Only an isolated test
harness should call it until native evidence supports a reviewed rollout.

## Required native proof before promotion

1. Freeze source/framework/binary/package identities and build before acquiring
   the G2 lease. Use only the proof identity, preserving the retained preview,
   favourites bytes/metadata, source/last-input, picture/audio, expanded settings
   and checked core files. Confirm the TV is available for a physical pointer
   observation and that no accessibility need would be disrupted.
2. Under the ordinary proof-app identity, re-read the exact getter and unique
   descriptor rather than replaying historical metadata. Independently observe
   the owned foreground card and own-CARD-only foreground-window list. Log only the scoped
   capability facts publicly. Actual setter admission is still unknown.
3. Choose the smallest reversible change from the captured original, using a
   fresh expected value. Confirm the normal service admits that one fixed write;
   independently read the stored string and observe a size change of LG's Magic
   Remote pointer. Keep pointer speed and all other settings unchanged. A stored
   value alone does not prove cursor size or which UI surfaces it affects.
4. Restore the exact original string with a fresh reverse change and independent
   readback. Confirm the physical pointer returns to its original size. If an
   external change intervenes, stop and report the conflict; do not overwrite it
   using broader authority.
5. Remove the proof app and verify process, package paths, launcher/catalogue,
   runtime, state, jail and mounts are absent. Compare the preserved settings,
   retained app and favourites, verify LG Home and restore original foreground.
   Keep raw replies and captures private. No reboot, boot/Home change, network
   reconfiguration, media-service intervention or OLED action is part of this
   proof.

After that proof, the coordinator can separately assign shared choice UI,
one-pending-request worker, freshness/expiry withdrawal and failure presentation.
Normal-identity write admission and physical remote/pointer behaviour must be
qualified at their actual scope before calling the retained integration proved.
If C2 is later included, give it its own candidate gate, lease, physical effect
and restoration proof; the matching descriptor is insufficient.

## Host verification scope

Nine focused mock tests pass, and hostsim library/test clippy passes with warnings
denied, using a separate agent-owned Cargo target. No shared parent build target
was used. An ARM softfp library `cargo check` also passes in a separate target,
including the native transport; this is type checking, not executable linking or
device execution. The existing unstable `neon` target-feature warning remains.
Mock tests exercise change plus reverse restoration, refusal to restore over an
external change, descriptor/storage/dimension drift, ownership loss, LG keyboard
appearance before the final ownership check, foreign cards and unknown covers,
queue/validation expiry using a fake clock, acknowledgement without mutation,
setter rejection, mutation followed by setter timeout, acknowledged mutation
followed by a transient read error, and external changes both before readback
and during the post-read ownership call. The last case deliberately demonstrates
that sampled verification is possible while the actual value has changed.
Other cases cover ownership loss after readback, no-op behaviour, closed wire
types and the exact setter payload. They do not establish service admission,
physical pointer behaviour, actual native ABI execution, device cleanup or
retained-preview readiness. No TV test is claimed by this document.
