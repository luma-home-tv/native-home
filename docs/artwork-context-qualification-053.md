# Isolated artwork context-withdrawal qualification

The 0.53 test recipe is implemented at `324d016a248d8c5d84b0b4fe89766d79bf3b54c5`.
It has no native execution qualification. The retained G2 0.47 and C2 0.48 installations are unchanged;
normal Root artwork loading remains off. The [0.52 host result](artwork-closed-negative-052.md)
qualifies its host cases, not either future TV experiment.

The remaining question is whether the real installed provider can complete a
known failed request after another application takes foreground, without
accepting image bytes or leaving an unresolved current operation. This test uses
Luma's own isolated native Target, actual LG foreground/window replies, and one
artwork request. It does not replace LG Home or exercise another viewing account.

## Two separate cases

| Case | Synchronisation | Required real outcome |
| --- | --- | --- |
| ReadAfter | A fixed five-second protected Proof-only hold after successful body delivery and resource readback, before the existing final context reads | The Target owns the sole CARD; all validated body bytes are discarded after authenticated EOF; the helper records the context-withdrawal refusal and its complete child journal |
| Finish | The client validates actual Complete, publishes progress, then waits five seconds before its unchanged Finish request | The real Finish context reads show the Target's sole CARD, followed by the strict joined refusal and authenticated EOF |

Progress is an observation, not proof of the helper's program counter or process
closure. A slow Target launch may miss the interval. Such a run is unqualified;
a known positive completion may be closed under its existing rules, but cannot
be counted as a successful negative case. There is no automatic retry.

The recipe submits exactly one current Target source from the actual catalogue.
It uses no normal artwork scheduler, preference writer, keyboard or setting
action. Every observed departure immediately latches off drawing and buffer swaps while
the already started worker is drained and joined. SDL focus loss alone does not
identify the foreground app: the device operator must independently bind the
actual Target launch, foreground and window records.

## Closed test configuration

The ordinary ARM Proof package has one strict five-field marker:
`artwork-closed-negative-proof.json`, containing schema, nonce, actual UI/helper
digests and the fixed `read_after` or `finish` case. Its packager rejects other
modes, interactive/normal Preview identities and all unrecognised nonempty
`NATIVE_HOME_` parameters. The existing six protected installation pins and
same-source build-receipt checks stay mandatory. The packaged catalogue must be
empty: the Target source comes only from the successful fresh live catalogue.
No Root helper or protected
plan is shipped in the ordinary application bundle.

Only ReadAfter admits a separate fixed protected file,
`/var/lib/luma-home-proof/context-qualification.json`, with exactly
`{"schema":1,"phase":"read_after"}`. It is owned by Root, regular 0600,
single-link, bounded and opened without following links. Presence, complete
bytes, digest and identity join the immutable installation snapshot; malformed
or changed state refuses the attempt. Preview never consults it. Finish requires
its absence. The plan adds a fixed delay at one Proof resource-read boundary;
it grants no new operation, caller, path, command or privilege.

The helper rechecks its actual peer, installation and remaining deadline before
and after the hold. The context reads remain real, fixed GETs. No reply is
substituted and no acceptance predicate is relaxed. The existing logical
budgets and joins do not establish a hard shutdown bound for blocked system
calls.

## Native acceptance and removal

A future run needs a frozen reviewed source/UI/helper/package set, a current
per-device lease, a quiet baseline and enough time for complete cleanup. It must
preserve the retained application's complete bundle, saved preferences and
metadata, and the protected Core measurements. Neither the old overnight
operator's dates nor an older binary's result establishes the new run.

The operator must bind actual ordinary/Root process identities and namespace
mapping, save the unmodified HBC response and all fixed reader records, and
independently show the exact owned Target scene. ReadAfter accounts for its
13 known reader children; Finish accounts for 16. The joined negative grader,
fresh current-attempt seal, successful current Ticket removal/sync and typed
EOF must all agree. No failed result reaches image decoding, cache acceptance
or GL upload.

Current-Ticket closure is separate from device cleanup. Before removing the
owned protected files or either test package, independently verify the helper,
every journalled child/group, namespace and test processes have gone. Remove
only positively identified test artifacts, then verify package/catalogue/state/
runtime/jail/mount absence and preserved retained/Core state. An unknown outcome
keeps the recovery barrier; timers, restart or a later EOF cannot clear an old
pending record.

The native cases, independent saved-evidence audit and any retained promotion
remain pending. These tests establish no C2 Root authority, physical keyboard
behaviour, arbitrary external-app coverage, crash/power-loss recovery,
performance or OLED protection guarantee.

## Verified host and ARM boundary

The [milestone](artwork-context-qualification-053-milestone.json) binds 583 passing
Rust tests (576 library and seven UI), 64 Python checks, Clippy with warnings
denied, and successful dashboard/helper ARM builds with 105 identical source
inputs. Twenty new Rust methods cover the protected plan, injected-clock hold,
marker/render gate, byte withholding, progress and cross-case classification;
six new package methods exercise admission and the real generated archive.
The ARM builds retain the existing SDK target-feature warning.

Independent source review found and resolved private-state initialization, a
missing final input poll, an asset-contract mismatch and a selected-case/actual-
stage classification gap. The final source matches the reviewed handoffs, with
only mechanical timeout-error conversions added during Root's checks. The first
compile and lint failures remain recorded, together with the successful repairs.
A final count-print error in the initial freeze wrapper is also preserved; a
separate readback verified every frozen file.

No 0.53 dashboard/helper IPK has been created or run on a TV. Its host/build
results do not transfer previous native qualification to either new executable.
The separate [ordinary surface prerequisite](native-surface-scene-prerequisite.md)
now has audited ARM packages and passing host checks, with device admission still
blocked. It establishes no artwork authority. The independent negative-case
native carrier and saved-receipt audit remain separate work.
