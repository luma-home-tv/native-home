# Identify-only artwork broker candidate (0.33.0)

The [ordinary-app artwork survey](artwork-access-032.md) could read one sampled
developer-app icon, while four other sampled locations were absent inside its
jail. A complete live drawer needs a qualified resource route before its artwork
worker and texture cache can be finished.

This candidate tests only communication and caller identity. It adds a separate
temporary native `artwork-broker-proof` executable, started by the test operator
through the existing SSH lane. The dashboard neither starts it nor invokes a
general command executor. It is excluded from application packaging. There is
no installed service, boot hook, jail-policy change or production startup path.

## Restricted exchange

The only wire operation is `identify`. Strict JSON messages reject unknown or
duplicate fields, other operations, unsupported schemas and malformed bindings.
The marker and enrollment bind both executable hashes and one session nonce.
The enrollment also binds an independently observed Proof PID, all four UIDs,
process start ticks and an expiry no more than 90 seconds away.

Transport uses a Linux abstract UNIX sequenced-packet socket. The abstract name
and nonce are rendezvous data, not authentication. Both connection credentials
and exactly one kernel credential record on the received packet are required;
the server checks them against its root-owned enrollment and fresh process
evidence. Unknown ancillary data, received file descriptors, truncation, empty
messages and oversized messages are rejected. Received descriptors are closed.
Other platforms refuse this transport rather than substituting a stream.

The server checks the exact Proof executable and jail-root paths, streaming ELF
digests, UID tuple and start identity repeatedly. Its own session directory,
enrollment and executable must have the prescribed root ownership, permissions
and file identities. Both sides independently sample the exact admitted G2
model/firmware/SDK, foreground application and sole Proof card before and after
the exchange. These are sampled observations, not atomic foreground continuity.

One accepted connection and one packet are allowed. Any failure ends the proof.
The client checks the response against its own identity and both server kernel
credential records. An echoed helper hash is not independent measurement of the
server executable: the operator must obtain that separately from procfs.

## Lifecycle and boundaries

The helper prints and flushes a startup record, then requires the exact fixed
operator-stdin token and EOF within eight seconds. The operator releases this
barrier only after binding the actual helper PID, UIDs, start identity, root,
executable path and digest. This input cannot select a command or operation.
The app never supplies it.

Socket operations have monotonic deadlines. The server's 25-second I/O deadline
does not guarantee process lifetime: synchronous bus registration and file reads
can outlast it. The helper resets `SIGALRM` to its default disposition, explicitly
unblocks that signal, checks its mask, and schedules its own termination after
30 seconds. Signal delivery can
still be deferred by a kernel uninterruptible wait. The driver never signals an
external root PID based on sampled procfs identity. It waits, verifies actual
process/endpoint absence, and removes only its verified executable and enrollment,
then the empty owned directory. Failure to bind or verify exit/cleanup requires
a recovery record with ownership evidence retained. Operator observation and
cleanup also include bounded SSH-call overhead; no hard wall-time claim follows.

The marker is accepted only as a bounded root-owned regular package file, with
symlink, FIFO, mode, link-count and mutation checks. The handshake is called only
from the isolated native collector when explicitly packaged with that marker;
the normal catalogue refresh path never calls it. Normal preview packaging has
no marker or helper. The retained G2 preview remains 0.31.0.

No icon/resource operation exists in this protocol. It grants no filesystem,
decoder, setting-write or app-launch authority. Even a successful exchange would
qualify only this temporary operator-started communication path. Production
enrollment/startup, server-owned artwork admission, bounded reads, worker/cache
integration and end-to-end artwork replacement remain separate work.

## Validation status

245 host library tests and five dashboard binary tests pass. They include strict
message and identity rejection, credential ancillary parsing, malformed marker
file types, bounded hashing, and actual host-stream startup-token/EOF/deadline
checks. Host all-target clippy with warnings denied and 13 framework-verifier
tests and 14 build/provenance guard tests pass. The helper has no binary-specific
unit tests.

Both final ARM release builds pass from source
`e098ac2d51ff1837ae5b75e8ef3beef0e193c113`, with equal snapshots of 73 selected
inputs and the same verified framework. The ARM-only signal configuration is
cross-compiled; it has not yet run on G2. Ten actual local packaging cases pass:
Identify has exactly its two diagnostic markers; ordinary passive and normal
interactive packages omit the broker marker; retained identity, interactive
Identify, mixed trials, invalid nonce/hash, stray marker values and packaging
the helper itself are rejected. These used a synthetic snapshot and made no TV
contact. The existing ARM target-feature warning remains.

| Final artifact | SHA-256 |
| --- | --- |
| Dashboard ELF | `7ada786be729bbc4cd17a270f3ea88fa9f97b4eafa083742ad10ef31cd06206a` |
| Separate helper ELF | `b6b8245c7a0d07aec79d7022f06635c2f088465c24490fce0a9af5c6bbe3f6cd` |

A fresh separate Proof IPK binds that helper hash and the new build receipt with
a new session nonce. It remains a private test package, separate from normal G2.
57 private host harness checks also pass, including independent policy review,
preservation-path invariance and an actual local child verifying the startup
token and EOF. The coordinator reran all 57 and executed the real candidate's
local admission prefix with every process except two pinned framework-verifier
calls forbidden. Both ARM receipts, current source/framework and the exact
two-marker package passed before any device access.

The first G2 attempt then declined at the initial idle-window gate, before
upload, install or app/helper launch. No TV setting, input, household app or
retained-preview update was attempted. The helper directory and abstract endpoint
were absent, and the own lease plus host agent/process/socket/directory cleanup
were verified. This is a predeployment decline, not installed-app removal or a
successful broker exchange. Retained/core bundles were not rehashed because the
lane exited before collecting those baselines. Raw household observations remain
private; [the scoped record](artwork-broker-033-milestone.json) contains only
sanitized admission and cleanup facts.

The host is macOS: Linux transport tests are compiled conditionally and have not
been executed there. Isolated G2 communication and complete removal remain
pending. No device capability, artwork, normal-preview promotion or full-release
claim follows from these tests/builds.
