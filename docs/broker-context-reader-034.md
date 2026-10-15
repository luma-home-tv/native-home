# Fixed context reader candidate (0.34.0)

The [.33 isolated test](artwork-broker-identify.md) proved that the temporary root
helper could not register anonymously on this webOS 24 system. It also verified
removal and preservation. This candidate keeps its Identify-only protocol and
exact model/foreground/sole-Proof-card checks, and changes how the helper obtains
its three context replies.

## Fixed reader

The standalone helper executes the installed `/usr/bin/luna-send` for exactly
three GETs: system model/firmware/SDK, foreground application, and foreground
windows. URI, payload, executable and environment are fixed in source. The caller
still sends only `identify` and its session nonce. There is no command, URI,
path or setting-write selector.

Root-owned enrollment now has a required ninth field,
`context_reader_sha256`, supplied from the operator's independent measurement of
the installed client. It is absent from the client marker and wire requests.
The helper walks the fixed path without following symlinks, verifies root
ownership, safe permissions, regular-file/ARM ELF shape, link count, size,
identity and that exact digest. It retains the admitted descriptor and executes
that descriptor, rechecking the current name and bytes around operations.
There is no pathname-execution fallback.

The [OSE command guide](https://www.webosose.org/docs/tools/commands/luna-send/)
documents interactive mode and reply-count options. Retail behaviour is measured
separately: with closed stdin, the tested invocations exited without a reply.
Keeping a private empty input pipe open allowed the fixed `-i -n 1` invocation to
receive one reply and exit. The reader holds this pipe, sends zero bytes, clears
the environment to its fixed minimal values and bounds stdout/stderr. No caller
input is forwarded. Three fixed GETs succeeded in a locked read-only G2
inspection with the client's bytes and metadata unchanged; no app was installed
or launched by that inspection, and its lease/private agent were cleaned up.
This does not yet prove native descriptor execution.

## Child ownership and failure behaviour

Each client runs in its own process group. Checked parent-death protection is
installed before descriptor execution. The helper observes child exit without
reaping it, keeping the leader PID allocated until owned-group cleanup finishes.
Output, execution and reap observation are bounded; only that owned group can be
signalled. The host driver never signals an external root PID.

Unknown capture, exit, group cleanup or reaping stops further GETs and withdraws
reader qualification. Strict parsing rejects duplicate JSON keys, extra JSON,
oversized data and nonliteral success values. The server also requires the
reader's approved/qualified/cleanup flags and pinned digest, then applies the
original exact context policy before and after Identify.

Per-child records and startup events allow the isolated driver to reconcile all
reported groups and independently check their absence after helper exit. Missing
final evidence, including a failure during spawn before any child event, must
retain recovery/ownership evidence. These controls are not an arbitrary
descendant-process sandbox or a hard bound on uninterruptible kernel waits.

## Validation boundary

260 library tests and five dashboard binary tests pass, along with host all-target
clippy with warnings denied. Thirteen reader regressions include actual host
children, strict reply classification, bounded output, deadlines, held-empty
stdin and descendant-held pipes. On macOS, signalling a zombie-only process
group returned `EPERM`; the implementation correctly declines that cleanup
result. Host tests preserve this observed refusal. Positive Linux cleanup and
native execution remain device qualifications, not conclusions from macOS tests.

Both ARM release builds pass from source
`b1bebb59f5e55338fbe142e927ad544a319b4998`, with matching snapshots of 74 selected
inputs and the same verified framework. The existing ARM target-feature warning
remains. Dashboard ELF:
`eb6785cc9ab4d711cc18af2e225ab7a89e5758ad5e10a0c8b7df69ecc88913fa`;
helper ELF:
`d63195053b74a6b17e7850cf9b05810fcaac33bd1df5dd6ccc11ccbf45efd4af`.
Ten actual local package cases pass, including marker separation and refusal to
package the helper as an application. Fourteen build/provenance guard and
13 framework-verifier tests pass. The isolated driver adds 22 child/context
checks to 67 unchanged legacy checks; the coordinator independently reran all 89.
Its actual frozen-candidate admission prefix passed with all processes except
two pinned read-only framework verifiers forbidden and no agent/device lane
created. Those are local admission checks, not a TV run.

## Isolated G2 result

The native run completed with one verified Identify request/reply across the
jail boundary. Both sides bound connection and per-packet kernel credentials,
nonce, process/start identity and executable hashes. The helper's three context
snapshots used nine successful fixed client invocations, with known cleanup,
owned-group cleanup before leader reap, and the same pinned Core executable
across 24 reader measurements. The ordinary client also checked its own context
before and after the exchange. These remain sampled context checks.

The driver's independent post-exit scan found none of the nine reported child
PIDs or groups. This is observed absence, not independent live child executable
measurement or a descendant sandbox. Per-child output digests are computed by
the bound reader; raw child stdout was not separately saved for recomputation.
The client probe took 3,112 ms including its own context and readback work. That
is not a UI performance or physical-input latency result.

The ordinary native collector exited at its bounded timeout after 30.14 seconds,
saved 13 frames, and was removed. All 24 positive preservation checks passed,
including checked retained-preview/favourites bytes and metadata, five selected
Core hashes, Core-client metadata, 20 explicit preservation domains, input/last-input
and checked audio profiles. LG Home was current. Proof processes/runtime/jail/
state/catalogue/launchpoint, helper/endpoint, known child groups, two staged files,
empty directory, upload, lease and private SSH agent were verified absent.
Normal G2 stays on 0.31.0; C2 was not contacted.
An independent saved-evidence audit passed 1,006 assertions across the pinned
inputs/artifacts, raw process records, reader and wire/credential observations,
20 preservation domains and cleanup. Its result hash is
`825350be68335a3247b3362a0c69c4ef1deef89a229348b2a6456b26573473fe`.
The raw response packet was saved and replayed; the raw request packet was not
separately saved. Request admission is observed through the bound server and
client receipts.

The [scoped milestone](broker-context-034-milestone.json) records these facts.
One valid native exchange does not execute every negative transport case or
prove alarm delivery, image reads, a production startup path, normal delivery,
playback/DRM/HDMI-format behaviour or full release readiness. Live artwork
worker/cache integration follows server-owned resource qualification.
