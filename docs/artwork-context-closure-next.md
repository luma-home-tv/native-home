# Next artwork closure slice

This is an implementation plan, not deployed behavior. The existing positive
image checks, successful Finished response, helper roles and durable pending
records remain unchanged. Ordinary interactive Root artwork remains disabled.

The first supported closed-negative cases should be a context change after a
prepared image read or while checking Finish, following a valid authenticated
Identify. ReadBefore and pre-Identify failures remain unknown in this slice.
They need separate support and qualification; apparently complete receipts alone
must not silently expand the accepted stages.

A closed negative returns no encoded image. Even if Complete arrived before a
Finish rejection, discard its bytes before decoding, caching or GL upload. Only
the current fetch's Ticket can be resolved, after all required evidence binds.
An older pending record cannot be cleared by restart, elapsed time or a new nonce.

## Implementation boundaries

`broker_transport.rs` needs a same-receive distinction between a valid packet and
clean kernel EOF. Its current receive error combines EOF, empty messages and
truncation. Do not infer EOF from that error or consume a malformed packet and
then accept a later EOF. The new typed receive API must keep existing packet
bounds, credentials, rights disposal and legacy receive behavior unchanged.
Transport foundation work alone must not enable negative acceptance.

`broker_server.rs` should emit a typed context-withdrawal reason only after a
successful fixed reader, unchanged admitted system identity and complete known
child cleanup. Malformed context, reader failure, changed pins, changed peer or
deadline expiry remains unknown. The existing sole foreground CARD policy must
still fail in the rejected context; it gains no overlay exception or resource
authority. Stop all subsequent resource operations and retain partial records.

`broker_bootstrap.rs` must explicitly close its owned accepted connection and
listener before recording closure. Preserve fresh protected-installation, Root
self and ordinary peer bindings. An error or missing final report cannot be
replaced by a flag claiming closure. Report a closed negative separately from
successful Finished; do not manufacture Finished after policy withdrawal.

`artwork_installed_provider.rs` needs an unconditional typed attempt trace. The
optional Proof observation JSON is diagnostic and cannot be its authority.
Preserve authenticated Identify, actual kernel credentials, request sequence,
validated packet progress and exact EOF through a failed exchange. For that
narrow candidate, join the single already-started HBC observer within its finite
budget rather than stopping observation immediately. No retry or new helper.

A separate negative grader must reparse the bounded raw HBC envelope and bind
the exact role, fixed command, caller, helper, protected pins, Core executable,
final peer and every actual reader/journal entry. Derive child coverage from the
attempted stages. HBC false, EOF, process death or a timer alone proves nothing.
Any mismatch retains the Ticket and quarantine. Successful negative grading
still requires the current Ticket's clear/sync operation to succeed before
exporting known settlement. The positive lifecycle and byte-commit path stay
strict and unchanged.

## Required host cases

Use the existing source-shaped receipt fixtures and regenerate raw HBC output
after each mutation, so a negative test reaches its intended check.

1. ReadBefore remains unknown, with no image and its Ticket retained.
2. ReadAfter withdrawal accounts for prior Header/chunks and discards them.
3. Finish withdrawal discards even a valid Complete and never returns image bytes.
4. Pre-Identify failure remains unknown across a fresh provider and nonce.
5. A typed foreign CARD or keyboard snapshot can establish policy withdrawal,
   while never authorizing another read or release.
6. Changed system identity, malformed GETs or reader failures remain unknown.
7. Mutated role, nonce, helper/UI/Core pins or Root identity remains unknown.
8. Mutated ordinary identity, credentials or final peer remains unknown.
9. Missing, extra, reordered or repeated child journals remain unknown.
10. Unreaped children, unknown group cleanup or incomplete EOF evidence remains
    unknown.
11. Contradictory Finished, qualified or session-completed flags are rejected.
12. A complete image followed by Finish withdrawal never reaches decode/cache.
13. A zero-byte credentialed message or truncation followed by EOF remains an
    error; a later clean read cannot erase it.
14. Delayed complete HBC evidence is joined once without premature observation
    cancellation.
15. HBC timeout, panic, missing/truncated report and unclassified cancellation
    retain pending state and cannot automatically start another helper.
16. Ticket unlink or directory-sync failure prevents known settlement; unrelated
    preferences remain unchanged and older pending records remain blocked.
17. Existing positive transfers, typed resource refusals, no-start behavior and
    default Proof/explicit Preview role boundaries retain their current checks.

Pure classifier tests do not establish Linux socket behavior. Controlled workers
should prove result/join ordering without substituting sleeps for evidence.

## Native qualification remains separate

After complete host review, use separately admitted isolated Proof cases at the
declared ReadAfter and Finish stages. Bind actual ordinary and Root processes,
the changed context, complete helper/HBC evidence and every owned child outcome.
Require fresh external helper/child/namespace absence before artifact cleanup,
while preserving the working retained installation and LG Core state.

The outcome qualifies only scoped closure and byte discard. It does not enable
production artwork, recover old unknown sessions, qualify real keyboard or
external-app handoffs, establish physical-input reliability or performance, grant
another TV model Root access, or promise a hard lifetime for blocked calls.
