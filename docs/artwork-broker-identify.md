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
can outlast it. A subsequent safety refinement schedules default `SIGALRM`
termination of the helper's own process after 30 seconds. Signal delivery can
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

The initial ARM release builds pass from source
`f137bd5bdd643f91e7392c9922d20f172b53e41d`, with equal snapshots of 73 selected
inputs and the same verified framework. Ten actual local packaging cases pass:
Identify has exactly its two diagnostic markers; ordinary passive and normal
interactive packages omit the broker marker; retained identity, interactive
Identify, mixed trials, invalid nonce/hash, stray marker values and packaging
the helper itself are rejected. These used a synthetic snapshot and made no TV
contact. The existing ARM target-feature warning remains.

| Initial artifact (before own-process alarm refinement) | SHA-256 |
| --- | --- |
| Dashboard ELF | `7ada786be729bbc4cd17a270f3ea88fa9f97b4eafa083742ad10ef31cd06206a` |
| Separate helper ELF | `be944ea507f760a808880e231e34a9fde84b6b13595e7e156a7c3420a1f41ad5` |

The host is macOS: Linux transport tests are compiled conditionally and have not
been executed there. Rebuilt alarm-refinement artifacts, isolated G2 communication and complete removal remain
pending. No device capability, artwork, normal-preview promotion or full-release
claim follows from these tests/builds.
