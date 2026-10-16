# Preference metadata proof and HOLD contract, 0.47

This is a source and host-test design for a proposed ordinary, actual-Preview
trial. Native migration, reversal and restoration are not qualified. The earlier
[C2 installer metadata measurement](c2-installer-metadata-045.md) established
requested modes on a new inert package; it did not establish same-identity
rollback or changing an existing preference directory.

The proposed operation concerns only the application's existing saved-preference
directory and its fixed preference file. It is user-authorized own-application
work, not a system permission operation. LG Home, Core, media, networking, boot
and OLED protection remain outside the mutation scope.

## Bounded source operation

Forward migration admits exactly directory/file modes `0777/0777` and changes
them to `0700/0600`. Explicit reverse admits exactly `0700/0600` and restores
`0777/0777`. Neither direction creates, replaces or rewrites preference content,
changes ownership, repairs mixed modes or automatically reverses a failure.

The implementation opens the existing directory and fixed child without
following their final symlinks, holds both descriptors, rejects hardlinked or
special files, and bounds the preference read to 64 KiB. It verifies ordinary
ownership/group, inode identity, valid preference encoding and the captured
original byte digest. Both objects are synced before the first chmod. Each
subsequent descriptor-based chmod, sync and name/content readback is checked.
These are sequential checks, not an atomic two-object transaction or an
exclusive lock against another writer with the same UID.

The generic forward API may leave a valid already-private record unchanged.
The controlled proof does not count that no-op as migration: it requires the
exact direction, original digest, initial/final modes and both recorded steps.

The proof's early main branch requires native ordinary Preview, no arguments
and an exclusive four-file metadata package. It returns before snapshot loading,
Window/GL, SDK clients, action/resource workers or preference persistence. The
packager binds a protected six-field marker to the actual ARM build receipt and
rejects every other nonempty trial/asset parameter. The exact payload has no
fonts, snapshot, helper, service or permission grant. Installed marker ownership
and modes still require their own readback before native admission.

The marker binds direction, expected preference/UI digests and ordinary owner
and group. Native self checks require consistent UID4/GID4, process start and
executable hash/link identity. A private, newly created runtime leaf holds the
receipt after an observation hold. The native root link describes its own
namespace; the coordinator must independently bind the actual host-visible jail.
System/jail ancestors are trusted: final-component no-follow and held-descriptor
checks do not establish a component-by-component trusted ancestor chain.

## Failure contract: HOLD

A first successful chmod followed by failure can leave `0700/0777`. A later
failure can also leave the intended final modes without a qualified completion.
Neither app exit, elapsed time nor an observed mode pair resolves that ambiguity.
Restoring the old application package does not restore preference metadata.

On **PartialUnknown, a missing receipt, unknown installer completion or an
unknown process outcome**, the coordinator must stop all further TV mutations.
That includes reverse, package restoration/uninstall, process close/signals,
runtime or upload deletion, and device-lease release. It must not launch the
normal preview against an uncertain state or use a backup to overwrite content.

While the current lease and dated authorization remain valid, the coordinator
may save read-only process/jail identity, exact state bytes and modes, process
inventory and Core observations. Preserve the original preference-byte backup,
known package inputs, lease identity and a private barrier/recovery receipt.
The dated window does not recur automatically. Lease expiry supplies no recovery
authority and must not be treated as a fresh grant or evidence of closure.

The ordinary preview can remain temporarily unavailable under HOLD. This does
not itself break LG Home or TV Core, but recovering the application may require
a separately reviewed, explicit future operation by the correct owner. The
current APIs intentionally provide no generic partial-state recovery shortcut.

A known refusal **before mutation** may permit package restoration only when all
of the following independently hold: authoritative installer state, no remaining
application process, the exact original preference bytes and metadata, and a
bound report showing `migration_attempted: false` with no API receipt or failure.
That is a source-bound pre-call refusal: no permission-step flags exist because
the metadata API was never invoked. An API error with a step receipt remains
HOLD. The pre-call flag alone, or a mode/digest readback alone, is insufficient.

## Qualification work still required

Host coverage targets full forward/reverse byte preservation, no-op proof
rejection, wrong digests and owners, links, special/mixed modes, descriptor/name
replacement and injected first-step failures. It also targets strict markers,
ordinary UID4/GID4 and private runtime ownership. Host validation passed 458
library tests, five dashboard tests, warnings-denied Clippy and 70 Python checks.
The 13 new packaging cases use synthetic build receipts and inspect actual
archives; they do not compile or install a TV package. The matching ARM dashboard
also compiled. The [compiled candidate](preferences-migration-047-candidate.json)
preserves those build-stage facts and keeps all native qualification flags false.

Before a native trial, independently admit the exact four-file package and its
installed modes, refresh the retained bundle/state backup, bind the real
ordinary process and jail, and verify no conflicting process or writer. A full
forward/reverse success must preserve every preference byte and restore the
original metadata before normal package restoration. Any uncertain stage instead
exercises HOLD, not guessed cleanup. Keep runtime recovery, same-identity
rollback, production activation, physical input and performance unqualified
until their respective evidence exists.

The subsequent [C2 native trial and separate restoration](c2-native-preferences-047.md)
records a failed native attempt and an independently replayed return to the
complete old installation. Native forward/reverse migration remains unqualified.
