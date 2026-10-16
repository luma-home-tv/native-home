# Controls access and icon observations: 0.61

The LG Settings shortcut now appears before the native readings in each Controls
category. It has a category-specific label and a short destination description.
The six existing fixed routes, stable focus keys, read-only bindings and the two
individually gated editors are preserved. Missing or stale readings do not hide
the LG shortcut. This adds no setting, service call, permission or setter.

An optional session-test ledger records the first observed drawing reference for
each full cached-icon SourceKey as Home and the app drawer are navigated. The
existing image worker still reads, validates and decodes cache bodies; the
existing controller still admits results and owns GL textures. This ledger adds
no worker, decoder, cache refresh or Root operation.

Main enables it only for an actual ordinary native Preview session with the
explicit session-proof marker. It samples owned, currently admitted textures
after drawing and before swap. Source freshness is aged again immediately before
sampling, and the final catalogue stamp is checked after image-worker join. The
existing first Home/Apps captures retain their original format. Normal release
sessions allocate no ledger or perform these extra catalogue serializations.

One ledger binds one full catalogue digest, preferred-source count and successful
cache adoption. After its first observation, unavailable/changed catalogue data,
adoption drift, malformed frames, conflicting source identities, nonmonotonic
observations or capacity limits permanently make its history incomplete.
Rejected frames cannot partially add records. First-observation history is
preserved; it cannot reset or heal into a success.

Limits are 64 rows and 128 KiB of compact JSON per supplied frame, 512 distinct
source keys and a 1 MiB compact report envelope. Parent pretty-printing adds
whitespace. Dimensions retain the existing 512-axis/1 MiB RGBA checks. These are
metadata/payload bounds, not total memory or time guarantees. Count equality is
only numerical: an independent verifier must compare the actual preferred-key
set and complete cache bodies/producer associations. History does not establish
simultaneous texture residency, original source-file freshness, image-content
identity, physical-panel quality, Root authority, cleanup or performance.

All 756 library tests, 10 UI tests, 84 Python packaging tests and strict Luma
Clippy pass against the same 136 selected source inputs. Ten ledger regressions
cover deduplication, later drawer observations, transactional rejection, sticky
withdrawal/drift, stale and malformed frames, geometry, monotonicity and bounds.
Two additional model tests cover all six first-accessible LG routes with failed
readings and preserve the existing editor/read-only gates. The first library run
preserves two failures in old positional fixtures; those assertions now identify
the intended reading or expect the deliberately moved first shortcut.

The [synthetic host previews](verification/controls-access-061/README.md) show
the updated Picture, General and Network layout. The actual scripted host
six-category navigation and Back/focus checks pass without setting or app actions.
These captures do not establish TV presentation or current TV values.

Both matching ARM release builds now pass against the same 136 inputs at
`76117505dd837ead35e16a550ae194185ef3acfd`. The helper is byte-identical to 0.60;
the dashboard contains the new UI and observations. A separately reviewed saved
ledger verifier passes ten synthetic guard cases, with actual native context
pins still unset. No TV was contacted by these builds or guards.

The [milestone](controls-and-icon-observations-061-milestone.json) binds the
checks and actual artifacts. This work does not qualify execution on the G2, a retained update,
new controls, full icon coverage or responsiveness. The current retained G2
remains 0.57. The next native workflow needs independently closed preparation
batches, cache continuity, actual full-key consumer observations and verified
restoration before retained delivery. LG Home, core services and OLED protections
remain untouched; normal interaction keeps Root artwork off. PlxNative's
attributed framework and integration patch are unchanged.
