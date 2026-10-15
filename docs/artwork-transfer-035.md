# Isolated encoded artwork candidate (0.35.0)

The ordinary app could read only one of the five sampled artwork locations in
the [0.32 survey](artwork-access-032.md). The [0.34 fixed context
reader](broker-context-reader-034.md) qualified a restricted native Identify
exchange. This candidate extends that temporary, operator-started helper with
bounded encoded icon reads. It does not enable a helper in the retained preview.

## Resource boundary

The ordinary Proof app requests an app ID and an icon variant. It cannot supply
a filesystem path, service URI, command or decoder options. The helper obtains
its own app and launch-point catalogues through two fixed, pinned system-client
GETs. Existing metadata admission constructs the permitted source: a registered,
visible app, an admitted installation root and bounded relative icon components.
The helper walks those components without following links and reads at most
1 MiB from a regular file. PNG/JPEG signatures are a preliminary gate; they do
not prove that the bytes are a valid image.

Each of at most five transactions sends a strict header, ordered chunks of at
most 4,000 encoded bytes and a completion message. Completion binds the complete
typed header and content digest. Before sending it, the helper rereads its own
catalogue and the file, requiring the same full source key, file identity and
content digest. It also rechecks the ordinary process, executable and current
root enrollment, and the exact model/firmware/foreground/sole-Proof-card context.
Changing or removing enrollment revokes further completion. An unqualified
catalogue reader aborts the session before another operation, including when
child cleanup is unknown.

Encoded bytes are transferred before the second read and final context check.
They remain unaccepted until completion; a late refusal discards the buffer.
The ordinary client checks kernel peer/message credentials, the session and
request IDs, source version/class/kind, chunk order, lengths and digests. It then
checks its own identity/context and decodes with a 512-pixel axis and 1 MiB RGBA
limit. Decoding runs in the ordinary process. Final session verification also
requires a matching Finished message, clean EOF and an ordinary context readback.
Context observations are sampled, not an atomic guarantee of foreground ownership.

The helper retains its existing 25-second I/O deadline and self-scheduled alarm.
The fixed client is executed through an admitted descriptor with private empty
stdin and bounded output. Owned child-group cleanup is reconciled separately.
There are no system policy changes, boot hooks, Home interception, setting writes
or caller-selected root commands.

## Cache groundwork

The separate pure cache module bounds work to one in-flight request, eight queued
requests and two completed results. Default reserved resident pixel capacity is
8 MiB, with at most 1 MiB per image. Entry generations reject stale results after
update/removal; unchanged entries survive catalogue refreshes. Visible and
favourite work is prioritised, and background work remains bounded for catalogues
larger than 64 apps. Fifteen regressions cover ordering, retry, removal/re-add,
refresh, stale results, allocation capacity, eviction and shutdown.

These are owned pixel-vector capacity limits, not a total heap, decoder, RSS or
GPU memory guarantee. This module does not yet run an I/O/decode worker or upload
textures. Main-thread texture ownership and a qualified normal startup path
remain integration work.

## Validation boundary

Source `8414fc640ea5a631bd1c0e2de319197cce376f41` passes 295 library tests and five
dashboard binary tests, host all-target clippy with warnings denied, 14
build/provenance checks and 13 framework-verifier checks. Both ARM release builds
have matching snapshots of 77 selected inputs and the pinned framework. The
existing ARM target-feature warning remains.

Dashboard ELF: `b4ed71455579cf67381976ad1e28f5a90f07d8cf9ee6c22eb8b03c8e2ad1a856`.
Helper ELF: `e4c46a245661fae76f2320e279350fd4d141794b895b9417967f779f29712d72`.
Sixteen actual local package cases pass, including exact marker separation,
invalid mode combinations and refusal to package the helper as an app. These
local checks do not qualify device transfer, normal delivery or rendering speed.

## Isolated G2 result

The G2 run completed five source-bound transfers and ordinary-process decodes,
one sample from each observed installation class. All five received completion;
the client then accepted Finished, clean EOF and its final context readback.

| Installation class | Decoded dimensions | Encoded bytes |
| --- | --- | --- |
| LG system | 72 × 72 | 7,911 |
| Encrypted apps | 80 × 80 | 3,938 |
| Developer apps | 160 × 160 | 3,518 |
| System apps | 115 × 115 | 5,251 |
| Optional content | 115 × 115 | 5,618 |

The helper recorded 22 context/catalogue snapshots, 56 successful fixed GETs and
156 matching Core-client measurements. Every reported child had known owned-group
cleanup before reap. An independent post-exit scan observed 330 processes and
found none of the 56 reported PIDs or groups. This is observed absence, not a
descendant sandbox or independent live measurement of every child executable.
Encoded/RGBA and Core-client output digests are producer reports; their raw bytes
were not separately saved for independent rehashing. Header commitments, saved
process records, artifacts and preservation evidence were independently replayed.

All 24 positive preservation gates passed, including retained-preview/favourites
bytes and metadata, five selected Core hashes, the pinned Core client and 20
explicit preservation domains, including input/last-input and checked audio
profiles. LG Home was current.
Proof processes/runtime/jail/state/catalogue/launchpoint, the helper/endpoint,
known child groups, two staged files and their empty directory, upload, device
lease and private SSH agent were verified absent. No reboot or host-issued signal
to a root process was used. The ordinary component reported 30.246 seconds elapsed;
the artwork probe reported 6,382 ms including context work. Process exit/absence
was checked separately. Neither timing is a UI latency/speedup claim.

The independent saved-evidence audit passed 1,088 assertions. Its result hash is
`a7b245563e78899a30e4411a78975467f3f8b489d9b9f6480d84f6834fcc1b6b`.
The [scoped record](artwork-transfer-035-milestone.json) keeps the artifact and
validation boundaries explicit. Five successful samples do not establish access
to every image in a class, negative kernel-case execution, live texture loading,
normal rollout, production startup or full release readiness.

The retained G2 preview remains 0.31.0; C2 was not contacted. Raw app inventories,
captures and process evidence stay private.
