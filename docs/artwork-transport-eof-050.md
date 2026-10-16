# Transport foundation in 0.50

This source candidate distinguishes a packet from EOF at one bounded receive.
It adds no closed-negative acceptance, provider recovery or ordinary Root
artwork activation. G2 retains 0.47 and C2 retains 0.48; no 0.50 dashboard or
helper package has been installed. A separate ordinary G2 probe now qualifies
four local transport cases, as described below. The
[0.49 native component](artwork-transition-049.md) remains bound
to its separately frozen source and artifacts.

`recv_or_eof_until` classifies the same `recvmsg` result after disposing of any
delivered file descriptors. A zero-byte result is EOF only with no ancillary
data and neither truncation flag. An empty credentialed packet, malformed
credentials, truncation and inconsistent bounds remain errors. Nonempty packets
retain their existing size and credential checks.

The public `recv_until` path maps typed EOF back to its original error kind and
text. Successful Finished checks, `expect_eof_until`, send behavior, roles and
pending-record rules are unchanged. Unsupported platforms still return
Unsupported. EOF is not authority to declare helper or child closure: a peer
may close only its write side. A future consumer must retain an earlier malformed
packet error even if another receive would return EOF.

Root ran 512 host Rust tests (505 library and seven dashboard), all 58 Python
checks and Clippy with warnings denied. The new host cases cover exact legacy
EOF errors, credentialed empty messages, packet bounds, ancillary errors,
descriptor disposal and the unsupported-platform boundary. Three additional
Linux Cargo socket tests are present for real packets/EOF, an empty credentialed
message and a write-half-close; those tests have not run. The local container daemon
was unavailable and was left stopped. Pure host classification does not qualify
Linux kernel behavior.

Both ARM executables compiled from frozen functional source
`b5b49503d16ecac45168785758d1d31bd6a23e51`, with matching 102-input build
snapshots. Dashboard SHA-256 is
`fa4de63950a40d520269ef15f6813f28878ed7d295972b0ad7b02212acbd1290`;
the installed Proof helper is
`8582867f66acbe57e76a3025b3a1067217a044e262df9c87d624e8e4373961ac`.
Compilation does not qualify native execution, which remains unverified for
these executables. No 0.50 UI package or helper has been deployed.

## Separate ordinary G2 transport probe

A new headless native test package exercised the byte-identical committed
transport module on webOS24. It opened no window and made no LS2/HBC, Root-helper,
artwork or settings call. All four fixed cases passed:

- A real packet retained its bytes and kernel credentials, followed by typed EOF;
  the legacy receive API retained its previous EOF error.
- A credentialed zero-byte packet was rejected rather than classified as EOF.
- Write-half-close produced EOF while the peer could still receive. EOF supplies
  no proof of process exit or child cleanup.
- An unwanted SCM_RIGHTS descriptor was rejected and closed: references to the
  probe's own pipe inode remained two before and after receiving it.

External raw process records bind the same ordinary actor, start identity and
exact executable before and after report collection. Matching live host/proc-root
file and directory identities, owners, modes and bytes bind the report to that
actor. These are source-bound native observations, not a separate syscall trace
or general permission to reuse a temporary-path mapping.

The carrier exited zero after natural process/UID absence, supported removal and
scoped cleanup. The new identity's registrations, bundle, jail, state, runtime,
mounts and upload were absent. The retained G2 bundle, private preferences and
twenty preservation domains/five Core files remained unchanged. Its owned lease
and private host agent closed. Existing normal runtime membership and parent
metadata were preserved; old runtime contents were not deeply rehashed. No native
process exit status was measured directly.

The [milestone](artwork-transport-eof-050-milestone.json) binds the frozen
standalone source, executable, package, carrier and native report separately
from the unexecuted dashboard/helper builds.
An independent saved-evidence audit passed, and Root ran the unchanged auditor
again with a byte-identical result. It replays the actual lease release/remote
absence and host-agent birth, identity, stop and process absence. Guard-ordered
deletion, SDK invocation and agent socket-directory removal retain their stated
source-bound limits. Its 18,500 checks and 3,260 file bindings are bookkeeping,
not coverage or performance scores. An earlier audit filter falsely matched a
read-only icon filename; that failed audit and the later narrower replay remain
preserved without changing the successful device result.

This qualifies those four local socket observations on G2. It does not qualify
the new dashboard/helper executables, an artwork session, authenticated
closed-negative acceptance, C2 kernel behavior, physical input, performance or
production startup. Finished semantics and durable pending rules remain strict;
ordinary Root artwork stays off.

## Next integration

The [next closure plan](artwork-context-closure-next.md) still requires an
unconditional typed attempt trace, explicit bootstrap closure, joined HBC
evidence, a separate strict negative grader and successful resolution of only
the current Ticket. No such acceptance path is implemented here.
