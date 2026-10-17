# Native startup context diagnostics

An unavailable foreground reading stopped the second G2 0.61 campaign before
resource preparation. The existing native reader projected both LS2 timeout and
setup failures to the same unavailable value, losing the distinction needed to
investigate that refusal.

This source adds separate `capability.context_diagnostics` observations. The
context Values remain unchanged, and admission never reads the diagnostic data.
The same three GETs run in the same order, with the same remaining-budget cap
of 500 ms each. The five-second foreground gate, eight-second operation budget,
single HBC check and all refusal/resource-dispatch conditions are unchanged.
No retry, fallback, request, permission or framework change is added.

## Recorded data

Each observed operation has its fixed label, requested budget and elapsed time
in nanoseconds, followed by one typed outcome:

| Outcome | Additional observation |
| --- | --- |
| `returned` | JSON/parse-error/oversize projection and SDK-delivered UTF-8 text length |
| `timeout` | The native SDK call returned its timeout variant |
| `setup` | SDK stage, optional code and bounded detail |
| `deadline_not_called` | The original deadline left no budget for a call |

Elapsed time covers the context operation, including payload conversion, SDK
setup/wait, reply projection and diagnostic work. It is not isolated service or
callback latency. SDK text length is not original wire size or an allocation
bound. Reply text and reply hashes are not retained by this addition.

Setup stage and detail are limited to 64 and 256 bytes with UTF-8-safe truncation.
History retains the first and seven most recent initial snapshots, plus the
post-request snapshot: at most 27 operation records. Missing, omitted or clipped
observations set `trace_truncated`. Neither complete nor truncated diagnostics
can supply missing context or grant resource permission. `context_first` and
`context_before` still refer to one copied initial snapshot.

## Validation and remaining work

Recorded host checks pass: 763 library tests, 10 UI tests and strict client
Clippy. Seven new synthetic cases cover fixed request/projection parity, typed
failure distinctions, the existing 65,536-byte projection threshold, no calls
after deadline, multi-byte detail limits, bounded history and unchanged positive/
refused capability grading. Independent source review and coordinator binding
readback pass. The isolated framework clone's initial mode-verification failure
and declared-mode correction are preserved; the framework revision/patch did not
change.

Matching ARM dashboard and helper builds now pass from functional commit
`167b4e4d33fc4f7b4aea885ef5eb6f9e20e19e3c`, with the same 136 selected inputs
before and after compilation. The dashboard is 4,528,128 bytes, SHA-256
`06cc22eca155b8c8e6b734069f1bb6db7b287b618785a16547c35b2f770134f8`;
the helper is 908,124 bytes, SHA-256
`981918ae4b73b1551253df86ee22bcd22e21aaee7416ab6c6adf78b7030795c7`.
Thirty actual SDK readelf commands and coordinator readback pass. Both retain
ARM32 EABI5 soft-float PIE and the same loader; direct library/GLIBC requirements
match their qualified dashboard/helper references. No RPATH/RUNPATH appeared.
The known unstable-NEON compiler warning is preserved.

These are static source/build/ABI results. No package, TV execution, permission,
transitive-loading, historical-cause or retained-delivery qualification follows.
Future artifacts must bind these new binaries, account for the diagnostic field
in receipt readers, and keep the original context/authority validation. This
change does not repair native LS2 behavior. Machine-readable evidence:
[milestone](native-context-diagnostics-milestone.json).
