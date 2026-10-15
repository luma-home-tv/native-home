# Startup 0.39: context refusal and completed recovery

On 2026-10-15 the isolated G2 startup test remained **unqualified**. The ordinary
native app observed a Root socket peer, but the helper refused before Identify
because the shared context reader admitted only the historical operator-helper
basename. The operator's first live Root process read then found that process
already gone. No complete external Root identity binding was obtained.

The recovered HBC reply confirms `standalone_root_helper_required`, an unapproved
context reader, zero attempted Core GET operations and zero child journals. HBC
reported command failure. Both ordinary workers joined; no retry occurred. This
is a real compatibility failure, not a successful Root startup or resource proof.

The carrier correctly retained its private Root installation, Proof app, upload
and lease when that outcome was initially uncertain. Its immediate evidence
retrieval preceded the UI's minimum hold and therefore missed the final report.
A separate coordinator recovery fetched the completed native and HBC reports
without starting the helper again. The first recovery read also encountered a
missing host-side decoder alias after saving raw evidence; that failure was
preserved, and the continuation bound the alias explicitly.

Before deletion, recovery independently observed no exact helper process, no
Core `luna-send` executable process, no own socket namespace and no ordinary
Proof UID process. It then checked the protected directory's exact two-file
inventory, metadata and hashes before removing only those files and directory.
Supported Proof removal and guarded own-state/upload cleanup followed.

The original device driver exited 1; recovery exited 0. All 24 preservation
gates passed, including the full retained 0.37 bundle/state bytes, owners and
modes, five Core-file hashes and 20 sampled read-only domains. LG Home remained
foreground. The private agent/socket/directory and device lease were removed.
No external Root signal, new startup, Core-file replacement, policy/permission
change, boot hook, setting write or C2 contact occurred during recovery.

A separate saved-only audit passed while explicitly keeping startup unqualified
and recovery complete. Its result SHA-256 is
`81c78ddefd770f7203148cdd0c63837cf477cf38caa883ccd36ef8cff6d33d41`.
Process absence and removal continuity include source/receipt-bound observations;
this is not an exhaustive system audit or a substitute for the missing live
Root binding. Raw state, inventories, process data and captures remain private.

Functional source was `dff4c58f567f322b8e44782f3bf2932c7f05af42`, with 91
selected inputs shared by the actual UI and helper build receipts. UI ELF:
`c528e3bd5f339e5d5922192a3fb638ced048fffe58b640d6c482144a7ecf3723`.
Helper ELF:
`c1f8c9920ad94505d6bd160335330f629731d8db12889b0a1ac56689ebcf67a0`.
IPK:
`1f026e60946e7ad2fa54c781978d37dcc848d3d977c68302bbd32042acc7cb1a`.

The subsequent 0.40 correction admits the exact protected installed Proof helper
only after its independent installation policy and Core hash are verified, and
only for the three context GETs. Catalogue access remains denied to that role;
the historical operator role is unchanged. Those host checks did not establish
native compatibility. The separate [0.40 result](artwork-startup-040.md) now
records a completed native exchange, an operator observation failure and completed
recovery. The retained dashboard
remains 0.37.0. See the [installed startup design](artwork-installed-bootstrap-design.md).
