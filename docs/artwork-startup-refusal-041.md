# Installed startup refusal candidate, 0.41

The next isolated candidate checks two existing session-nonce guards before
adding resource access to the installed artwork helper. It does not change the
helper's admission policy, add an operation or enable the normal dashboard's
live-artwork loader. G2's retained preview remains 0.37.0; C2 is outside this work.

The positive [0.40 result](artwork-startup-040.md) has two serial native exchanges
with fresh app/helper identities. This candidate asks a different question:
whether a correctly connected ordinary Proof client observes the exact expected
refusal when its Identify or Finish packet contains the wrong nonce.

## Two fixed cases

| Case | Sequence | Required refusal evidence |
| --- | --- | --- |
| Wrong Identify nonce | Connect to the real endpoint, hold for live process binding, then send one structurally valid Identify with a derived wrong nonce. | Exact helper nonce error, false HBC completion, no response or helper Core-child dispatch, and checked client EOF. |
| Wrong Finish nonce | Complete valid Identify, hold for live binding, then send the first Finish with a derived wrong nonce. | Exact helper Finish error, false HBC completion, three reconciled initial context children, no final context or Finished, and checked client EOF. |

Each case has one fixed HBC command, two joined ordinary workers and a separate
expected-refusal grade. The connection uses the actual endpoint nonce; changing
that endpoint would only test inability to connect. The diagnostic derives the
wrong packet nonce by flipping its first hexadecimal character. It accepts no
caller-selected command, path, URI or packet body.

The strict nested marker can select only these two cases in an ordinary,
noninteractive isolated Proof package. Other diagnostic markers cannot be
combined with it. Normal Preview cannot select this path. The Root installation
policy and protected helper remain separate from the app package, with the same
source/artifact binding and guarded installation/removal requirements as 0.40.

## Acceptance boundary

An expected refusal is never successful startup, resource authorization or proof
that no process started. The actual HBC output must include the bound final
helper report and its exact refusal, with zero or three known children as
appropriate. A timeout, missing receipt, unexpected packet, unknown cleanup or
lost completion remains unqualified and retains the recovery barrier.

The server's current guards reject the bad nonce before recording the rejected
packet's message credentials/request fields. Those fields must stay absent;
the client's sent bytes, connected kernel peer, helper error and source ordering
provide distinct evidence. The helper's `connection_closed` flag also remains
false on its refusal path. Client EOF, process exit and namespace absence must be
verified separately rather than rewriting that flag.

Before device admission, source, host checks, ARM builds and package isolation
must pass. Each native case then needs its own live ordinary/Root process
binding, complete preserved normal bundle/favourites and selected TV/Core
readbacks, exact protected-helper removal, supported Proof uninstall, and
upload/lease/private-agent cleanup. Duplicate/concurrent startup, stale/foreign
callers, broader protocol rejection, resource reads and retained integration
remain separate requirements. The dashboard continues to build on the
explicitly attributed [PlxNative framework](../README.md#plxnative-framework-dependency).

## Current validation

Host checks pass 374 library tests and five dashboard tests, Clippy with warnings
denied, and 35 build/provenance/framework/package tests. The new checks include
both synthetic refusal shapes, wrong-stage and identity drift, missing joins or
EOF, invented server fields, strict string-only scenario parsing, and real
package assembly with synthetic ARM/tool fixtures. Package tests verify both
markers and reject normal/interactive/mixed modes and mismatched helper context.
These are local source/fixture results, not observed G2 refusals.

Both ARM builds pass from functional source
`88aa058d0a9d957f9154f64ed95dac6d15e378bd`. Their actual receipts share all 92
selected inputs and the pinned framework. The UI ELF SHA-256 is
`a8d9247edfcc4c8cf77add41e82a8ad5be04a5c08b7f84e2aea3ad37bf6e0824`;
the helper ELF SHA-256 is
`4a9a7b1a242b8333f3a198a8ca6507b4388d325337709cb0489f6a612488f444`.
The existing ARM target-feature stability warning remains.

Two actual local IPKs were assembled with independent rendezvous policies, one
per declared case. The [candidate record](artwork-startup-refusal-041-candidate.json)
binds their hashes and the actual ARM receipts. The helper remains outside both
ordinary app archives. These builds/packages do not qualify runtime refusal.

The separate host validator passes 13 synthetic rejection/acceptance tests and
independently admits both actual ARM archives, including exact marker, resource
bytes and payload closure. Its artifact bindings are recorded in the candidate
record. It grades expected refusal separately from positive startup and requires
the ordinary effective GID from independent process records. A saved-only host
grade still supplies no native process or refusal evidence.

The independent device carrier still needs validation before either case can
run. No 0.41 package has been installed, and the Root helper/server policy is
unchanged. Full native preservation and clean removal remain required.
