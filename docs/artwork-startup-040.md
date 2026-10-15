# Installed startup 0.40: two independent serial sessions

Two independent serial G2 startup exchanges are now qualified. In each, the
ordinary app requested one fixed helper command through existing HBC and
completed Identify/Finish, clean EOF and matching HBC completion with the
protected Root helper. Admission came from protected policy and the measured
kernel peer. Two Root context snapshots qualified all six fixed Core GET
children per session. Resources, repeated-session reliability and the normal
loader remain unqualified. The retained G2 dashboard remains **0.37.0**. C2 was
untouched.

## Evidence and execution boundary

Functional source: `e54ff4814b17c27e33499d6a88a19a71c5065383`. The actual UI and
helper ARM receipts share all 91 selected inputs and the same pinned framework.
UI ELF SHA-256:
`dc22f42b2b88d168b15d48cabea7fd9eda2686411d158d44797e9f3d38a45e1e`.
Helper ELF SHA-256:
`3dc076513da4c3073d4dccc415494cc0e27d70a5f9ec94d56e298d11bbc59315`.
IPK SHA-256:
`74d00030ff527e2e2c8d8e884b589978522b94cbc0441ac1d8047ff77a05266b`.

Each session bound six raw ordinary-process records and seven raw Root-process
records, including all four UID fields, stable PID/start, actual executable
hash/path and Root namespace. Root socket/message credentials and the helper's
own measurements agree with those independent observations. The protected
policy generated enrollment from that measured kernel peer; UI-supplied identity
claims were not installation authority. Its UI lookup remains explicitly
untrusted for Root execution because the developer ancestors are writable.

The helper remained alive through the eight-second binding hold. It served one
Identify and one matching Finish, closed its socket, and exited successfully.
The ordinary app observed matching Finished, clean EOF, exact HBC String/base64
output correspondence, the same helper/session in the final report, known
context-child cleanup, and both actual worker joins. There was one HBC exec
attempt per session; no automatic startup retry or alternative privilege route
was used.

Root code was installed only in a freshly created, separately protected product
directory, after actual ancestor, free-space, ownership/mode, file/hash/inventory
and namespace/process checks. The helper accepts no arguments and exposes no
artwork or setting operation. The corrected context-reader role requires the
exact protected helper and installation/Core pins; it admits only the three
context GETs. Catalogue access remains denied to this role. See the
[installed startup design](artwork-installed-bootstrap-design.md).

## First session: operator observation failed, recovery completed

The original **device driver exited 1**. It checked for the terminal receipt
before that file appeared, then observed the producer had exited and declared
the receipt missing. Its immediate failure retrieval nevertheless saved the
complete native receipt, completed startup proof and framebuffer. The native
proof itself was qualified, with no error and successful Root/HBC completion.
This was not evidence of a native crash. The original failure history remains
unchanged and must not be described as a passing driver run.

The harness conservatively retained its artifacts and lease. A separate
coordinator recovery made **no new startup request**. It fetched the final
evidence again, independently replayed the complete live bindings and protocol,
observed helper/socket absence and absence of all six owned child PIDs/process
groups, then removed the exact protected helper/configuration and directory.
Supported Proof removal and guarded own-state/upload cleanup followed. Recovery
exited **0**.

All 24 positive preservation/removal gates passed. The full normal 0.37 bundle
and saved state match their original bytes, owner/group and modes; five checked
Core-file hashes and 20 sampled read-only domains match. LG Home was foreground;
the Proof registration, launchpoint, jail, runtime, state, mounts, upload and
protected Root artifacts were absent. Both the device lease and private recovery
agent/socket/directory were removed. No external Root PID was signaled.

A corrected harness performs one final bounded read of the same receipt after
observing producer absence. Five host regression cases cover that race and true
missing-receipt cases. It was used for the separate second session below, without
rerunning or rewriting the first session.

A separate saved-only audit independently replayed both real ARM receipts,
committed inputs, raw process records, HBC/base64 and Identify/Finish, both context
snapshots, child PID/process-group absence, protected factory, full retained
bundle/state and preservation/cleanup receipts. Result SHA-256:
`703f834abc7ce6885b8ec26c147fbae31ce1daa5c405b84d5cb5f09c6ccde07e`.
It grades native startup qualified, original operator run failed, and recovery
complete. Raw evidence and household state remain private; the
[sanitized milestone](artwork-startup-040-milestone.json) binds the artifacts.

## Second session: independent startup and complete operator run

The same frozen UI, helper, package and protected policy completed a second
serial session. Its corrected **device driver exited 0**. Independent raw process
records established fresh ordinary and Root PIDs and start times relative to the
first session. This was a separate startup, not a retry within either session.

The second session completed its one fixed HBC exec, Identify/Finish, clean EOF,
matching HBC completion and both worker joins. Two successful Root context
snapshots accounted for six distinct Core GET children and 16 executable
measurements. The saved complete process/group and namespace scans independently
confirmed absence before guarded Root-artifact removal.

All 24 preservation/removal gates passed again. The complete retained 0.37 bundle
and state preserved bytes, ownership and modes; five checked Core file hashes
and all 20 sampled domains matched. Proof registration, launchpoint, runtime,
jail, state, mounts, upload,
the protected helper/configuration directory, lease and private agent were
removed. LG Home was current, and no recovery or external Root signal was needed.

A separate saved-only audit passed **29,526 assertions**, binding both ARM
receipts and 91 committed inputs, the fresh 13 raw process records, complete
session/child evidence and preservation/cleanup. Result SHA-256:
`8e63dc6ce031367844aa0697b6dcea6a1600f52f5f7bf6cf84076c0d7af07e85`.
It pins the first audit and explicitly retains the first operator's exit 1 and
recovery exit 0. Only sanitized hashes and counts are published here; raw process
identities and household data remain private.

Host checks passed 369 library tests, five dashboard tests, Clippy with warnings
denied, 27 build/provenance/framework helper tests, 14 carrier tests and 41 actual
package-admission cases. Both ARM binaries compiled. The first saved Home frame
was visually inspected through an RGB-identical lossless conversion; the second
session's frame is byte-identical. This is not a physical-input, performance,
power or panel qualification.

## Remaining limits

Two independent serial Proof sessions are qualified. This is not a reliability
study. Duplicate/concurrent startup, foreign/stale callers, interruption,
unavailable HBC and further unknown-outcome cases still need their own device
evidence. Core stdout digests and some removal/process-continuity facts remain
source/receipt-bound; observations
are sequential, not atomic snapshots or an exhaustive system audit.

HBC's existing broad, writable dependencies and pre-main loader/environment
remain trusted. The SDK's post-call reply bounds and helper alarm do not create
a hard total allocation/lifetime guarantee. This result changes no HBC policy,
boot hook, jail, firmware, networking, media or OLED configuration. Automatic
resource transfers and retained live artwork remain subsequent work.
