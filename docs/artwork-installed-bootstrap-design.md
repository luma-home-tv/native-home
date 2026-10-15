# Installed broker startup candidate, 0.39

Status: implementation and host checks; native command execution is not yet
qualified. The retained G2 dashboard remains 0.37.0. This candidate adds no boot
hook, Home interception, service registration, LS2 policy or protected TV setter.

The first [0.39 device attempt](artwork-startup-039.md) refused at the legacy
context-reader helper-name guard and was fully recovered. The 0.40 correction
requires protected installed authority for the exact new helper and admits only
context GETs. It does not broaden the role to catalogue/resource operations.

The [0.38 status check](artwork-startup-capability-038.md) established ordinary
native access to HBC's read-only Root-status method. It did not qualify `/exec`,
a Root child, installed authorization or automatic resource access.

## Authority follows the actual filesystem

A subsequent read-only G2 inspection found `/media/developer` and the developer
app ancestors Root-owned but mode 0777. A private file inside that tree cannot
be treated as immutable Root execution policy merely because its leaf is 0700
or 0600. This rules out the initial app-bundle helper placement without changing
any global permissions.

The inspected `/var` and `/var/lib` are real Root-owned 0755 directories. The
candidate instead uses a fresh, exact `/var/lib/luma-home-proof` directory,
Root-owned 0700. A guarded Root installer must fail if that path already exists,
then create only the helper (0700) and `broker-installation.json` (0600). It
verifies actual bytes, owner/mode, link count, inventory and path identity before
the native app can invoke the fixed helper. Existing files and parent modes are
not modified. Complete removal of this separate installation artifact is part
of the isolated test, after every owned Root process is independently absent.

The closed policy contains a profile, rendezvous nonce, UI/helper executable
hashes and a pinned Core context-client hash. It accepts no PID, UID, executable
path, service URI, command or permission grant. Protected directory anchors and
helper/config names are opened relative to held file descriptors with no symlink
following, bounded reads, stable metadata/hash and current-name revalidation.

The UI's developer path is only a read-only lookup. Its writable ancestors are
explicitly not execution authority. Admission uses actual socket credentials,
four kernel UID fields, process start, exact jail/executable identity and the
running `/proc` executable hash against the independent protected UI pin. A UI
marker or a claimed PID/hash cannot enroll itself. Proof and Preview roles have
separate fixed paths; Preview is only a policy abstraction in this candidate,
not a qualified normal installation or deployable Root helper.

## Bounded operation, independently observed completion

The ordinary native app uses two joined workers. One makes a fresh inert HBC
status check, verifies its own identity and G2 foreground, then requests exactly
one literal command through existing HBC `/exec`:

```text
/var/lib/luma-home-proof/artwork-broker-startup-proof
```

There are no arguments, path substitutions, user strings, retries or fallback
privilege routes. The helper accepts no arguments. A second worker connects to
its installed abstract socket, retrying only expected not-yet-listening/backlog
conditions within the connection deadline. After connection it never reconnects
or repeats a failed protocol exchange.

The helper accepts one credential-bound client and independently creates its
session enrollment from the measured kernel peer and protected pins. It gates
Identify and Finish on fresh fixed model/firmware/sole-Proof context reads,
repeated installation and peer validation, and known child cleanup. It supplies
no artwork read or settings operation. The listener remains bound until Finish,
so a second helper cannot replace a live endpoint. Duplicate and independent
session behavior still require separate device evidence.

HBC buffers command stdout until process completion. Consequently, the UI saves
the kernel Root peer immediately after connection and again after Identify,
then holds the session for eight seconds so the operator can capture actual
Root UID/PID/start/executable evidence. Buffered stdout is not a live PID channel.
After matching Finished and clean EOF, the native app also reconciles the HBC
completion envelope and helper report with that exact Root peer/session. HBC's
`returnValue:true` alone is insufficient. Both workers are actually joined.

The inspected installed HBC returns stdout/stderr as String and base64 fields;
the candidate strictly cross-checks those representations and preserves refusal
output and child journals. Root helpers have a 20-second operation deadline and
a checked own-process alarm at 30 seconds. Local hashing, registration, SDK
cancellation/destruction and uninterruptible kernel waits are not hard bounded
by the reply wait. A timeout or lost output is an unknown outcome, not no start.
Cleanup must retain the protected installation and recovery evidence whenever
the exact helper or its context-child absence is uncertain. No unbound Root PID
may be signaled, and an incumbent socket is never removed.

HBC remains an existing broad, writable command carrier. Main-executable pins do
not authenticate its pre-main loader, shared libraries or ambient environment.
This candidate changes none of that existing trust boundary and must not be
described as eliminating it. The helper itself performs no Root filesystem
writes; the scoped installer does create/remove its own named artifacts.

## Evidence still required

Before retained integration: native `/exec` compatibility, actual ordinary and
Root process binding, successful Identify/Finish and command completion, exact
context-child exit/absence, complete Proof/Root-artifact removal, and preservation
of the full normal bundle/state and sampled LG state. Then qualify independent
sessions, duplicate start, stale/foreign callers, interruption, unavailable HBC
and unknown-outcome recovery. Only afterward connect resource transfers and the
bounded artwork worker. No production startup, live artwork, normal rollout,
performance or C2 compatibility follows from the current host checks.
