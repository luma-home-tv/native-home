# Installed startup nonce refusals, 0.41

Both isolated G2 nonce-refusal cases are now observed and independently audited.
Each device carrier exited **0** while the Root helper deliberately reported
**refused** and HBC reported a failed command, exactly as required for that case.
These are expected-refusal results, not positive startup or resource access.
G2's retained preview remains **0.37.0**; C2 was untouched.

The positive [0.40 result](artwork-startup-040.md) remains separate: it has two
serial native exchanges with fresh app/helper identities. Version 0.41 changes
neither the helper admission policy nor its operations and does not enable the
normal dashboard's live-artwork loader. The
[device milestone](artwork-startup-refusal-041-milestone.json) records the actual
case results. The earlier [local candidate record](artwork-startup-refusal-041-candidate.json)
remains unchanged as a snapshot from before native execution.

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
upload/lease/private-agent cleanup. These cases do not cover duplicate/concurrent
startup, stale/foreign callers or broader protocol rejection. Resource reads and
retained integration remain subsequent work. The dashboard continues to build on the
explicitly attributed [PlxNative framework](../README.md#plxnative-framework-dependency).

## Local-only candidate stage (historical)

The following describes the completed pre-device stage. Its statements about
no native execution apply to that historical snapshot, not the results below.

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

At that stage the independent device carrier still needed validation, no 0.41
package had been installed, and full native preservation/removal remained to be
shown. The candidate JSON keeps those original local-only fields unchanged.


## Actual native results

| Case | Operator exit | Helper refusal | Helper context / children / Core measurements | Independent audit |
| --- | --- | --- | --- | --- |
| Wrong Identify nonce | 0 | `Identify nonce mismatch` | 0 / 0 / 0 | 29,324 assertions |
| Wrong Finish nonce | 0 | `Only first Finish request is admitted after Identify` | 1 / 3 / 8 | 29,316 assertions |

Both sessions bound six raw ordinary-process records, including effective GID,
and seven raw Root-process records. The actual running executables and process
identities matched the protected installation and the connected kernel peers.
Each session made one fixed HBC exec request, joined both ordinary workers,
received the exact complete refusal report and observed clean client EOF without
Finished. Wrong Finish also completed and bound its preceding valid Identify.
Wrong Identify reached no helper Core GET; Wrong Finish reconciled all three
prior context children and independently confirmed their PID/process-group
absence before removal.

The helper's refusal-path `connection_closed` field stayed false in both cases.
Independent client EOF, complete process/namespace scans and guarded removal
established cleanup separately. No rejected-packet SCM or server request record
was invented. Sent packet bytes/digests remain client/source-bound, distinct
from the actual helper error and live process evidence.

All **24 preservation/removal gates** passed for each case. The full retained
0.37 bundle and saved state preserved bytes, ownership and modes; five Core file
hashes and 20 sampled read-only domains matched. The protected helper/configuration
directory, Proof registration/launchpoint, runtime, jail, state, mounts, upload,
lease and private agent were removed, and LG Home was current. No external Root
PID was signaled. The normal dashboard was not updated.

The saved-only audits independently replayed both actual ARM receipts, all 92
committed inputs, package/policy bindings, raw process records, HBC String/base64
and exact refusal stages, case-specific child accounting, full retained bytes
and cleanup/preservation. Their SHA-256 results are:

- Wrong Identify: `62c7236af53c8eaf27732ae1a9519fcf1f200137a9a551662c332ecc74d77275`.
- Wrong Finish: `88c48ffc694a7f353b21296c79c71c829cdd4ec488462e0d11df7eea2baec5dc`.

Both saved Home frames are byte-identical to the previously inspected frame;
no raw capture or household artifact is published. Exact sanitized artifact
bindings and audit-code hashes are in the
[milestone](artwork-startup-refusal-041-milestone.json).

These two cases qualify only the declared nonce refusals and their cleanup.
They do not establish exhaustive malformed-packet, foreign/stale caller,
duplicate/concurrent, interruption or denial-of-service coverage. Core child
stdout digests and some guarded removal facts remain source/receipt-bound;
samples are sequential and HBC's existing loader/environment remain trusted.
No resource transfer, normal loader, production startup, physical-input,
performance or C2 compatibility is qualified by these results. The next delivery
slice connects the existing bounded resource protocol to the artwork worker and
native texture pipeline.
