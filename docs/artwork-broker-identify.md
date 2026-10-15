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
inputs and the same verified framework. Ten actual local packaging cases pass:
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
67 private host harness checks also pass, including independent policy review,
preservation-path invariance and an actual local child verifying the startup
token and EOF. The coordinator reran all 67 and executed the real candidate's
local admission prefix with every process except two pinned framework-verifier
calls forbidden. Both ARM receipts, current source/framework and the exact
two-marker package passed before any device access.

Two initial G2 attempts declined at the idle-window gate before deployment. The
user then corroborated that Home can coexist with a built-in HDMI background
card. A separately reviewed private harness predicate admits only Home with one
explicitly declared built-in HDMI card and an optional exact screensaver. Ten
new host regressions check that state, reject other cards/overlays/duplicates,
and verify that all other driver code is unchanged. This prelaunch calibration
does not alter the broker's sole-Proof-card authorization.

The subsequent isolated run installed and launched Proof. It independently bound
the ordinary producer and the root helper's actual proc/executable records before
releasing the fixed startup barrier. Raw root status records corroborate that
`SIGALRM` was unblocked and neither ignored nor caught. The helper then failed its
own anonymous message-bus registration with **-1027, Invalid permissions for
(null)**. It never reached its context getters or listener binding. The ordinary
client's own exact model/foreground/sole-Proof-card context passed, but connecting
timed out after 15,097 ms. No Identify request, reply or icon read occurred. This
qualifies the registration refusal, not the cross-jail exchange or its socket
authentication.

The native collector completed its bounded run, saved 13 frames, and was removed.
All 24 positive preservation checks passed: the checked retained preview files
and favourites bytes/metadata, selected core hashes, input/last-input and checked
settings/descriptors/audio profiles were unchanged. Proof UID processes,
runtime/jail/state/catalogue/launchpoint were absent. The bound helper, abstract
endpoint, exact two staged files, empty directory, upload, lease and private SSH
agent were also removed and verified; no external root PID signal was sent.
LG Home was current. Normal preview remains 0.31.0; C2 was not contacted.
An independent saved-evidence audit passed 976 assertions across the pinned
source/artifacts, raw process records, signal-mask observations, 20 explicit
preservation domains and removal/cleanup. Its result hash is
`f549aa0c36425dc3c8ad5bd9fdd48dbf1f8488365db0221e7856255984aa5bd0`.
It confirms the refusal and cleanup; it does not qualify an Identify exchange.

Raw household observations remain private; [the scoped record](artwork-broker-033-milestone.json)
contains sanitized admission, refusal and preservation/removal facts. These
checks do not establish playback/DRM/HDMI-format behaviour, a normal update or
full release readiness.

The host is macOS: Linux transport tests are compiled conditionally and have not
been executed there. The attempted native client did not establish a connection,
so packet credentials and cross-jail communication remain unqualified. The
30-second own-process alarm was configured; a stalled-process termination test
was not performed.

## Next qualification

Qualify an independent context reader for the temporary helper using the installed
system client's three fixed read-only getter invocations. Keep immutable arguments,
bounded output/deadlines and owned-child cleanup, then retain the same strict
model/foreground/window checks. The normal dashboard still uses its qualified
ordinary native bus registration. The proposed experiment preserves the existing
permission policy; no resource operation is admitted by this plan.
