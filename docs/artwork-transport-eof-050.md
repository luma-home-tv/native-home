# Transport foundation in 0.50

This source candidate distinguishes a packet from EOF at one bounded receive.
It adds no closed-negative acceptance, provider recovery or ordinary Root
artwork activation. G2 retains 0.47 and C2 retains 0.48; no 0.50 package has been
installed. The [0.49 native component](artwork-transition-049.md) remains bound
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
Linux socket tests are present for real packets/EOF, an empty credentialed
message and a write-half-close; they have not run. The local container daemon
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
The [next closure plan](artwork-context-closure-next.md) still requires an
unconditional typed attempt trace, explicit bootstrap closure, joined HBC
evidence, a separate strict negative grader and successful resolution of only
the current Ticket. No such acceptance path is implemented here.
