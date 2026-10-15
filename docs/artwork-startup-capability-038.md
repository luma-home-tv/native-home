# Ordinary native HBC capability check, 0.38.0

On 2026-10-15, one isolated native app on the OLED55G26LA G2, firmware
23.25.55 / webOS24 SDK 9.2.2, received the exact HBC response
`{"returnValue":true}` through the attributed PlxNative LS2 transport.
The caller remained an ordinary jailed process. This qualifies this read-only
capability request, not privileged broker startup or automatic artwork delivery.
The retained dashboard remains **0.37.0** with its packaged icons and favourites.
The C2 was not contacted for this work.

## What ran

Functional source: `33ece612b51c3d6b9dcb20be1ae8b2da1d90a3b0`.
The actual ARM build receipt binds 87 selected inputs and the unchanged pinned
framework integration. Dashboard ELF SHA-256:
`637e884b122da5119ba25c53955c951255f39477dfadbbfc4b905e316f66141e`.
Isolated IPK SHA-256:
`8751f733af00d130b592546d89490c8166917b8073b3fb4c9e002c57fdf40649`.

The package contains only the native Proof app, synthetic artwork and one strict
`startup-capability-proof.json` marker, `{"schema":1}`. It adds no service,
permission, system overlay or boot hook. This mode suppresses live catalogue and
action workers, keyboard trials and the standard scripted navigation/captures.
Ordinary clock reads and the Proof app's own preference storage still exist.

One named worker owns its anonymous LS2 registration and makes one fixed
`luna://org.webosbrew.hbchannel.service/checkRoot` request with `{}`. Fixed
model/firmware, foreground and window GETs gate the request and its result.
The operator independently captured six raw `/proc` records binding the actual
ordinary UID in all four UID fields, PID, process start and executable hash.
The module's three self measurements agree with that binding. Both contexts
show the sole Proof CARD on the admitted G2 firmware.

The reply contains a bare boolean true, without error fields or extra members.
Duplicate keys, coercion, contradictory/error envelopes, malformed or oversized
replies cannot qualify as that result. Bare false remains a distinct legitimate
negative observation; transport/setup/registration failures remain distinct too.
Neither causes a privileged fallback.

The request has a one-second reply-wait budget inside an eight-second operation
budget. Initial hashing/registration, cancellation and SDK destruction are not
hard bounded by that wait budget; the SDK copies its response before the local
16 KiB parsing bound. No total lifetime or allocation ceiling is claimed.
The Home view stayed alive for the operator's process binding, then the main
thread actually joined the worker and saved one complete 1920×1080 Home PPM.
That frame was visually inspected through a lossless, decoded-RGB-identical PNG:
branding, labels and the synthetic icon/fallback presentation were present.
Live clock/catalogue presentation is deliberately absent from this narrow test.

## Preservation and checks

The actual run reached terminal exit 0. The full retained 0.37 bundle—32 entries,
30 files—and its entire saved-state tree match their before-test copies in
bytes, hashes, ownership and modes. The retained app was not launched or updated.
Five checked Core files and 20 read-only preservation domains match, including
the sampled picture, audio, input, accessibility, locale and panel-care state.

All 24 positive preservation/removal gates passed. The own Proof app, ordinary
UID processes, registration, launchpoint, jail, runtime, state, mounts and upload
were absent afterward; LG Home was foreground. The device lease and private SSH
agent/socket/directory were released and removed. No reboot was requested.
Raw inventories, source captures, household state and frame files remain private.

A separate stdlib saved-evidence audit passed **1,256 assertions** after Root
directly observed that terminal result. It independently parsed the raw SDK
reply and replayed the 87 committed source inputs, six process records, full
bundle/state copies, 20 preservation domains and 24 cleanup gates. Audit SHA-256:
`6fd53d68ac0e0bfb9f73b05c07ff97a3b1739e323626bf183a648ac74a7ca4fc`.
Its pure replay adds no fresh TV observation. Some absence and lease/removal
continuity remain bound to the executed carrier's checked receipts; contexts
are sequential observations, not an atomic system snapshot. The
[sanitized milestone](artwork-startup-capability-038-milestone.json) records the
artifact bindings and limits.

Host verification passed 352 library tests, five dashboard tests, Clippy with
warnings denied, 27 build/provenance/framework-helper tests, 13 private carrier
tests and 32 actual package-admission cases. The package's 11 regular files and
sole marker were also inspected directly. These do not establish performance,
physical input or panel behavior.

## Existing HBC trust boundary and next step

A separate read-only inspection found HBC metadata version 0.7.3 and inspected
the installed compiled handler and activation wrapper before this test. The
handler only returns its service's captured UID-derived boolean; exec, spawn,
autostart and elevation are separate operations and were not requested. LS2 may
activate the existing HBC daemon to answer. This is not a claim that no Root
process could start.

The installed bundle differs from current upstream beyond that handler. Its
existing service files were UID/GID 1001 and mode 0777; the captured development
activation descriptor was Root-owned 0644. These writable inputs and HBC's broad
command API remain an existing trust dependency. No hardening, elevation, repair,
role/permission change or startup update was performed.

Next, qualify a fixed installed broker with independent Root-owned enrollment,
actual child/process binding, repeated and duplicate startup, stale/unavailable
callers, interrupted/unknown outcomes and verified shutdown. Only then connect
that qualified route to the existing bounded artwork worker and admit a retained
integration. See the [startup design](artwork-startup-design.md). Successful
`checkRoot` alone establishes neither a restricted launcher nor resource access.
