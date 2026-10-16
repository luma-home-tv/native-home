# Artwork attempt and shutdown foundation 0.51

This source adds typed failure evidence without accepting a closed negative or
enabling ordinary Root artwork. Both retained installations remain unchanged:
G2 0.47 and C2 0.48. No 0.51 executable has been deployed or run on either TV.

The provider retains one bounded, in-memory attempt trace even without an
optional Proof observation writer. It distinguishes received packets from
Receiver-validated progress, records authenticated Identify separately from a
parsed response, and retains the first failure. A rejected packet cannot consume
a later EOF and erase that failure. The trace contains metadata, not encoded
image bytes. A new fetch clears the previous trace before its preconditions;
an early refusal leaves no new trace and grants no settlement authority.

Unexpected EOF at Identify or Read is distinguished diagnostically while still
returning the previous Unavailable result. Finish uses the existing strict
Finished receiver: its failures remain generic and do not claim an observed EOF.
The stop, join, positive settlement, current-Ticket resolution and durable
pending rules are unchanged. Final diagnostic JSON is projected only for its
optional writer after join and settlement; typed retention adds no JSON work
before cancellation. Existing live connected observations remain separate.

The installed helper can now return a private typed context-withdrawal error at
ReadAfter request 1 after successful image preparation, streaming and readback,
or Finish request 2 after one successfully completed image. It requires the
qualified fixed reader, unchanged Core identity, admitted G2 system identity,
unchanged ordinary peer/protected installation and a live deadline. The supported
rejected contexts are narrowly a matching sole foreign CARD or the own CARD plus
the exact LG keyboard window. Empty, inconsistent, duplicate or unfamiliar
snapshots, reader contradictions and changed identities remain unknown. These
snapshots still fail resource policy; none authorizes another read.

Bootstrap revalidates the typed candidate, then moves and drops its owned
connection and listener before recording closure. It reports
`context_changed_closed` separately, with `qualified: false`, no Finished and no
successful session. Partial read records are preserved. The provider continues
to quarantine this result as Unknown; there is no trace-to-Ticket recovery path.
Operator and ReadBefore refusals keep their existing strict behavior.

Root ran 528 Rust tests (521 library and seven dashboard), all 58 Python checks
and Clippy with warnings denied. Sixteen new behavioral tests cover trace
retention/validation, malformed-packet stickiness, payload exclusion, actual
worker cancellation, context matrices, drop-before-receipt ordering and
contradictory-success rejection. The first host build failed because a new test
used the wrong cache type name; its log is preserved and the corrected build
passes. ARM compilation and any native qualification remain separate.

The [0.50 ordinary G2 socket probe](artwork-transport-eof-050.md) remains bound
to its own frozen source and executable. It does not qualify this new provider
or helper. The [closure plan](artwork-context-closure-next.md) still requires
shared typed Finish/EOF handling, a separately strict raw-HBC/journal grader,
finite observation joining, current-Ticket-only settlement, adversarial tests
and isolated native ReadAfter/Finish qualification before any negative recovery
or normal activation.

Luma Home uses the explicitly attributed PlxNative framework and its separately
recorded integration patch; this foundation adds no Plex application feature.
