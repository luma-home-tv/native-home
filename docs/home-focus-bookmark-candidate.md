# Home focus across explicit launches

[Draft PR11](https://github.com/luma-home-tv/native-home/pull/11), based on the
[shared Home options candidate](home-app-options-candidate.md), implements the
interface specification's last-deliberate-favourite focus policy. Functional
source is `65868df37f1488b3fad3fda12eb17a3a533d91fd`. It is not installed on a TV.

Deliberate Home navigation or card activation remembers a stable app ID. A fresh
launch restores focus from the packaged snapshot, then resolves it once against
the first fresh real app catalogue. If the saved app is no longer eligible or a
favourite, choose the first current eligible favourite, then View all apps for
an empty shelf. Stale, failed, future or simulated catalogue updates cannot
resolve this pending restore. Navigation, press, Back, click, page change or a
valid hover focus change cancels it; a later refresh cannot take focus away from
the user's choice.

Only navigation is remembered. Hover, programmatic focus, catalogue reconciliation
and fallback do not write it. No action, keypress, menu, search text, settings
state or capability is restored. Luma still exits after an ordinary departure;
reentry means explicitly launching it through LG. Settings lifetimes, Home keys,
boot behaviour and ordinary hidden-window expiry are unchanged.

## Storage and verification

The bounded `home-focus.json` sidecar shares the existing app-owned directory
and persistence worker. Favourites keep their existing strict schema, so older
builds can still read them. Sidecar writes use a private temporary file, atomic
rename and readback; the worker coalesces favourite and focus requests independently.
The final dirty selection is queued before normal worker shutdown. Corrupt,
unsupported, linked, foreign or oversized sidecars remain errors; there is no
permission repair. Failed focus storage does not discard healthy favourites.
Abrupt process termination can lose an unacknowledged selection.

The author ran 829 library and 16 UI tests plus strict all-target Clippy.
Independent source review passes 31 focused library cases and the actual main
persistence test, with nine additional independent functions. These cover late
catalogues, eligibility and fallbacks, input cancellation, private-file errors,
coalescing/readback, normal close and unchanged favourite bytes. The coordinator
rehashed all 180 review bindings. Framework and standalone dependencies remain
unchanged.

Two earlier prototypes failed independently reproduced startup cases: a saved
app absent from the packaged snapshot, and an eligible fallback appearing only
in the first fresh catalogue. Both failures and their original results remain
preserved; the corrected cases pass on this revision. The earlier ARM build is
kept separate and is not promoted.

An actual fresh ARM UI build passes against 143 matching source inputs, with
source/framework and five selected compiler fingerprints unchanged before and
after. Immutable source copies also match the compiled revision.

| Artifact | SHA-256 |
| --- | --- |
| Dashboard ELF | `4dda97a0dcbee385f807d0f5d883fc3264bd9382073a089b227834fd04fadf19` |
| Build receipt | `45d5b01b14dfa61725391291cb7d3e93f9e544bcc3d0ac293922e9d4a9cab00e` |

Separate local artifact/ABI review is pending. No package was built and no ARM
target executable ran. Native storage, focus appearance, physical input, performance and recovery
after eviction remain unqualified. Captured/observer sessions exclude bookmark
writes, so existing Proof results cannot qualify this feature. A new enabled
ordinary isolated test needs its own sidecar-aware baseline, matching artifacts,
fresh lease and complete known removal; retained installations remain separate.
