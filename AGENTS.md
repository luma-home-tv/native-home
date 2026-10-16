# Luma Home project rules

Keep Luma Home's application separate from the explicitly attributed PlxNative
framework dependency. Do not add Plex client, playback, authentication or server
features. Retain upstream package names, copyright and licences. Record generic
framework changes in the integration patch and propose them upstream separately.

Use shared UI/focus components. Preserve an OLED-black canvas. Available app icons keep their own shape and
transparency without a graphite backing plate; missing-art fallbacks and controls
may use graphite surfaces. Build and verify the standalone dependency graph; host checks do not
prove TV rendering, physical input or performance.

Format only changed client files, with Rustfmt child-module traversal disabled.
`cargo fmt --all` also traverses the pinned framework path dependencies; preserve
their verified bytes and integration patch instead of reformatting them.

TV details, credentials, actual app inventories/icons and household captures
must stay outside tracked source. Device work requires the existing device lock,
an isolated package and complete removal/state restoration. LG Home, boot hooks,
firmware, system libraries, jail policy, media services, networking and OLED
protections must remain untouched. Protected/unverified controls remain read-only
or LG-managed. Never exercise personal viewing-account writes in tests.

The interactive preview is now intentionally retained on both authorised TVs.
Preserve its saved favourites and installed state. Do not apply the former
diagnostic driver's unconditional uninstall/cleanup to a retained installation;
use a separate test identity or a backed-up, verified update procedure.

The G2 is always available for coordinator-owned tests. The C2 is available only
from 00:00 to 06:00 Europe/London or when the user explicitly releases it. This
supersedes earlier availability windows; fresh device leases and baseline checks
still apply, and no G2 evidence establishes C2 behavior.

The latest verified retained G2 installation is 0.57.0, with dashboard ELF
SHA-256 `a85b302d6bb371f2e82a93b071c0ab7fd8c2789f93f85053b96690e5e1475590`
from functional source `da2d9683b8c7df8a0bf4064f83c2ee4b1915f9d0`.
`docs/g2-retained-preview057.md` records the backed-up actual056→057 update,
captured protected caption-preference row against fresh external fixed GETs,
source-correct Back and separate fresh Home/Apps/Back session. Original operator,
independent saved audit and Root replay pass. Full bundle/private preferences,
all 28 resources and sampled Core state are preserved, all owned remote captures,
staging, lease and host agent removed. Normal runtime logs/session receipts remain;
old bytes were not rehashed. Normal Root artwork stays off. This is stored caption
preference, not active captions/UK DVB/app subtitles or write authority. Native
failure/expiry, physical input, broad controls/performance/reliability and C2 remain
open. Future backup/rollback must use actual057 and fresh state; older056-baseline
carriers must not run against057.

`probes/native-surface` now preserves the ordinary held-window prerequisite,
with two render-gate tests, 28 environment cases, strict Clippy, a matching ARM
build and independent SDK archive/ELF/source audit. Its new six-file font-bearing
package is `f63060a2480923abbd7ca462c187ca137a90e5a21d7ab642a85dae30647bd736`;
ELF2195 is reproduced byte-for-byte. That source wave made no TV contact. The
later `docs/native-surface-scene-g2.md` qualifies the ordinary held-window scene,
both original actors alive around sole Target CARD, stopped drawing, full 60-second
lifetime and complete known cleanup against retained057. Full bundle/preferences,
old runtime bytes and sampled Core were preserved. Saved audit and Root replay
match; preserve the first saved-reader indexing failure. No Root broker/HBC,
normal artwork, physical input, performance or C2 qualification follows. Future
carriers need fresh actual057 state and dates; older047/V7 dates and owner records
cannot be reused. Natural Home/screensaver admission never broadens sole-Proof
readiness or both-live sole-Target qualification. Component nonqualification and
known ordinary cleanup remain distinct; unknown ownership/transport retains barriers.

`docs/artwork-responsive-cache-next.md` chooses ordinary durable icon reads for
normal interaction, with separately qualified privileged preparation. Preserve
the current broker, source checks and pending guard. Do not merely remove its
input wait, clear old Tickets or infer Root authority from app-writable cache
metadata. The 0.58 source now integrates ordinary reads and an explicit
diagnostic producer; matching current executable/positive-transfer qualification
remains necessary. Normal Root artwork stays off.

`docs/durable-icon-cache-foundation.md` records the unintegrated ordinary cache
module: 22 real-file cases, all 665 Rust host tests, strict Clippy and a matching
ARM build pass. Preserve the initial lint failure and independent final-scan race
repair. The exclusive writer slot covers the final capacity scan. Cache data and
producer hashes grant no Root authority; publication requires a future matching
positive/joined/current-Ticket-closed producer. No UI/provider/worker callsite or
native cache execution is qualified. Consumer integration must preserve complete
SourceKey, expected producer and generation/selection through late-result admission,
withdraw on independent catalogue failure/expiry, and keep all I/O/codec work off
the render thread. Automatic eviction/old-version retirement after restart and
stale-slot recovery remain open. Retained057 and normal Root-off state are unchanged.

`docs/ordinary-artwork-cache-058.md` records the new source integration, passing
711 Rust/72 Python checks, strict Clippy and matching ARM dashboard/helper builds.
Ordinary cache/profile I/O and decoding stay on the existing single image worker;
fresh source withdrawal applies even without RootRuntime. Only the separately
admitted Preview diagnostic may publish after positive/joined/current-Ticket
closure. Producer metadata grants no Root authority. Session-only cache-frame
receipts are bounded observations, not foreground/pixel/provenance/performance
proof. That source milestone did not qualify native execution; preserve retained057
and require matching native population, ordinary hit evidence, known cleanup and
exact restoration before
promotion. Keep eviction and stale-slot recovery explicitly unfinished.

`docs/ordinary-artwork-cache-058-prerequisite.md` records the later read-only G2
check and independent saved audit/byte-identical Root replay. Current057 full
bundle, fresh unchanged private preferences, metadata-only Home eligibility,
fixed Core reader hash/stat format and exact lease/agent closeout pass. Preserve
the first saved-reader TTL-units failure; correction required no TV rerun. No058
code, ordinary cache IO/permissions/live namespace, pixels or full runtime/Core
preservation is qualified. Future native trials need fresh state/new leases;
this catalogue and dated prerequisite window cannot grant later freshness.

`docs/ordinary-artwork-cache-058-first-attempt.md` preserves the first native
trial's exit1 after a known completed diagnostic install. It stopped before
explicit app launch or Root staging because the installer rejected a known PTY
close diagnostic. A separate restore-only continuation exited0 under the same
original lease and conservative original deadline, restoring exact057 and
preserving fresh private preferences, complete measured Host runtime and sampled
Core5+19. Only original journal-bound staging was removed; lease and agent cleanup
passed. Preserve the failed trial independently.
Independent saved audit and byte-identical coordinator replay also pass; preserve
the first two reader-adapter failures separately. No cache population, ordinary
hit, native image/performance or launched-producer restoration is qualified.
Future carriers need fresh state/leases/dates and strict matching completion
before accepting known installer PTY diagnostics; never infer success from ACK
or connection close alone. Normal Root artwork remains off; C2 was not contacted.

`docs/ordinary-artwork-cache-058-native.md` records the later second attempt:
six positive transfers/96 Core children and ordinary Home/Apps cache associations,
with four cache plus seven packaged uploads and matched GL cleanup. The original
operator still exited1 at exact mount-row equality after verified057 rollback.
An independently reviewed cleanup-only continuation exited0 within the same
original lease/deadline, removed positively mapped runtime/cache/staging, preserved
full057/preferences/state/sampledCore, all 92 immutable old runtime files and the
original event-log prefix with 262 appended bytes. Lease and agent cleanup pass.
Original234 mount rows remain an exact prefix of238: four duplicates of existing
read-only SDK bind rows were observed, with no mount repair or unchanged-mount
claim. Do not reuse those counts as a future baseline; capture fresh state and
review the mount comparison before another trial. The composite saved audit,
byte-identical coordinator replay and narrow raw cleanup assessment pass. No
normal058 promotion,
all-app/pixel/physical/performance qualification or C2 contact follows. Raw BMP,
PNG and internal PPM byte comparisons withdrew an apparent missing-UI impression;
verify raw pixels before proposing renderer fixes from preview appearance.

`docs/ordinary-artwork-cache-058-pixels.md` adds independently decoded saved
content evidence for four opaque unfocused integer Apps quads: 183,988 constant
pixels match exactly, whole-quad residuals and wrong-image/flip/shift controls
are recorded, three native CPU digests match and one is unobserved. Coordinator
replay is byte-identical. This retrospective analysis preserves its schema and
arbitrary-coverage-floor failures, reports the fourth image's sub-500 nonmodal
count, and grants no broad pixel/alpha/focused/physical/performance authority.
`tools/sdk_mount_observation.py` preserves all four measured same-App mount roots
with an exact bounded read-only canonical SDK-pair exception. Nine public
synthetic tests and private 35+15 host fixtures/offline checks pass. Preserve the
failed canonical-only parser; do not shrink scope by dropping mirrored mounts.
The carrier remains undated/unadmitted and no new TV execution follows.

`docs/artwork-catalogue-planning-059.md` records the pure 512-source/16-per-batch
planner and independent Shell source-clock view. All 715 library/10 UI host
tests and strict Clippy pass; preserve the first test-name lint failure. Every
operation rechecks explicit NOW in the SAME live_origin wrapping clock domain;
observed stale/failed/simulated/incoherent/changed maps permanently withdraw the
plan. Cursors and manifest hashes prove enumeration only, not execution,
publication, generation continuity, capacity or Root authority. No worker/IO/GL,
ARM build, package or TV execution is added by this foundation. Retained057 and
normal Root-off state remain unchanged. The future explicit runner must match
actual current-key publication and known closure, handle partial capacity,
and retain Unknown barriers; never promote cursor progress into completion.
The follow-on preflight correction joins/saves capability unconditionally but
guards draw/capture/normalization/swap after observed input/departure. Source
review, the existing admission-refusal unit case and strict Clippy pass; a native
interrupted-window test and observation during synchronous capture remain open.

`docs/artwork-catalogue-preparation-059.md` records the integrated explicit
preparation recipe/strict package selection. All 737 library/10 UI and 84 Python
host checks plus strict Clippy pass; source/packaging reviews are clear. Main
keeps Shell RootRuntime/actions/persistence/setter workers off; ordinary sidecars
do not select preparation. Require exact Work/fullkey/sequence/actual index,
both KnownClosed/currentTicket and actual publication+receipt+decoded fingerprint
before local image coverage. Empty batches, cursors, generic decode or join never
grant readiness/Root authority. Preserve the original180s budget and stop new
work at120s for job30+drain30; never cancel/drop active Work as closure. Stop
presentation on observed input/departure, drain/join before report I/O, and poll
actual live again after join. Local reports leave external closure/cache body/
all-catalogue/native pixels false. No059 ARM/native/retained promotion is qualified
by these checks. Next work needs new matching artifacts/packages, fresh G2
baseline/lease and independent native closure/restoration. C2 remains untouched;
preserve normal Root-off and actual retained057. Product preparation flow,
cross-process completion/capacity, same-key refresh/eviction/stale slots remain open.
The later059 local artifact milestone now passes both actual matching ARM builds,
normal/preparation packages and an independently authored/source-reviewed SDK
archive/ELF/current-Git audit with 12 guard cases. Preserve the first audit's
receipt0644 refusal; only two own local receipts became0600 with unchanged bytes,
then fresh audit/readback pass. UI ABI compares actual057; helper compares the
specifically bound native046 helper, never the UI or a maximum GLIBC version.
The132 functional inputs remain dd2671da4e9288b1bef668ad9257a5bae1870d59;
later documentation HEADs grant no new compile or native evidence. Exact057
rollback matches its saved complete bundle. These local results do not qualify
new059 native permissions/lifecycle/transitive loading or Root admission. Fresh
reader hash/stat, lease, scene and full retained baseline remain mandatory before
any TV trial; the new carrier proposal remains undated/unadmitted.

The later `docs/artwork-catalogue-preparation-059-native.md` records the actual
temporary G2 trial: 16 positive transfers/256 known children, 15 prepared images
and one oversized-image decode failure. Preserve its operator exit1 and skipped
ordinary consumer. The corrected saved auditor and byte-identical coordinator
replay pass with exact057/private state/preferences, sampled Core5+19 and old
runtime preservation; known trial cleanup and original lease/agent closeout pass.
Preserve the first single-snapshot saved-reader refusal. Original238 mount rows
remain an exact prefix of240 with one existing read-only SDK duplicate pair;
no mount operation or restored-mount claim. The bounded-thumbnail candidate has
isolated host codec evidence only and is not integrated or ARM-qualified.
Normal Root artwork stays off, C2 was not contacted, and the product goal remains
incomplete. Future native work requires fresh state, leases and matching artifacts.

`docs/bounded-icon-decoding-060.md` records the shared three-path thumbnail policy
and generic attributed PlxNative API. All744 library/10UI/84Python, strict Luma
Clippy and17 actual framework decoder tests pass;134 selected inputs match.
Preserve the first old513-axis fixture failure and separate framework-wide strict
lint failures; no blanket upstream lint-clean claim. Encoded1MiB/inputaxis2048/
native+sourceRGBA each8MiB are independent from unchanged output512/1MiB.
Original encoded provenance, generations, cancellation, worker/GL ownership and
Root protocol remain unchanged. Bounds are not total-heap/OOM/time guarantees;
native memory/latency and matching ARM/package/native evidence remain pending.
No060 retained promotion or C2 contact follows from these source checks.
The later060 local artifact milestone passes both actual ARM builds and both
packages at functional9f387c862d29c029a7a6b3fbe9070c623766c835/134inputs.
Independent source review/12guards/actual local audit and coordinator178input+
507output rehash pass; UI direct ABI matches actual057, helper matches bound046,
and exact057 rollback matches the saved complete bundle. Preserve mode0600 new
receipts and unchanged other9 framework files. ARM release panic=abort means
catch_unwind grants no native panic-recovery guarantee. No target ELF/TV/helper
ran in that audit; fresh reader identity, scene, lease, complete retained baseline,
native memory/latency and known cleanup remain required before promotion.
The first060 native trial in `docs/bounded-icon-decoding-060-first-native.md`
is closed: inert checkRoot returnedtrue, after-foreground unavailable, no resource
worker/Rootexec/image jobs, consumer skipped, operatorexit1. Preserve strict
context refusal; no actual foreground change or decoder failure is inferred.
Exact057/privateprefs/Core5+19/oldruntime preservation and known cleanup+original
lease/agent closeout pass independent saved audit/byte-identical Root replay.
Original240 mount rows remain exactprefix242 with one existing read-only SDKpair;
no mount operation/restored-mount/cause claim. Five saved startupRSS/HWM samples
29480KiB do not qualify larger decoding/totalmemory/performance. Current060V1
scope is closed, never re-adopt dates/nonce/lease. A new trial needs new private
pins/package inspection/admission/fresh baseline; retain every context gate.

The second060 scope in `docs/bounded-icon-decoding-060-native.md` is closed with
actual operator0, sixteen positive publications/local preparations and ordinary
Home/Apps cache associations. Independent saved audit, seven guards and
byte-identical Root replay pass;20169 bound inputs rehashed. Ordinary8cache+4packaged
textures all deleted, joined/no recorded GLerrors. Exact057/privateprefs/Core5+19,
old140runtime entries/92immutablefiles/eventlogprefix+262B and known scoped
cleanup/originallease+agent closeout pass. Original242mountrows remain exactprefix246
with two existing readonly SDKpairs across four measured aliases; no mount
operation/restored-mount/cause claim. Only firsthelper has external live raw7;
later authenticated identities have final external absence checks. Counts and
frame associations are not allcatalogue/body-to-pixels/physical/performance
proof. RetainedG2 remains057, normalRoot off, C2 untouched. Never reuse either
closed060 scope's dates/nonce/lease; fresh native work needs matching current
artifacts, fresh state and unchanged context/cleanup guards.
Its saved-memory reader/fourguards/byte-identical replay also pass,5162bindings
rehashed.118diagnostic samples maxRSS44408KiB/HWM51344KiB;36ordinary samples
42308/49380KiB. One literal exit-race group supplies no memory. These are retained
complete raw7 reads, not terminal/after-final-decode/GPU/helper/totalmemory,
96MiBfit/OOM/leak/performance proof. Do not reuse these as matched release metrics.

`docs/controls-and-icon-observations-061.md` records the 0.61 source: existing
LG category handoffs precede native readouts, with the same keys/actions/editor
gates. A session-marker-only bounded per-full-key cached drawing ledger extends
observation beyond the first Home/Apps captures without changing their format.
Normal release sessions create no ledger/new worker/Root runtime. Preserve sticky
catalogue/adoption withdrawal, transactional whole-frame validation and final
live-clock ageing after image join. Historical observations/count equality grant
no all-catalogue, source-file freshness, residency, pixels, authority or performance
proof. Limits512keys/64rows/128KiB compactframe/1MiB compactreport; parent pretty
printing adds whitespace. All756library+10UI+84Python and strict Luma Clippy pass
against136 inputs; synthetic host Controls captures/navigation also pass. Preserve
the two initial old-position fixture failures. New matching ARM/package/native
evidence still precedes promotion; retainedG2 remains057 and normalRoot off.
Both actual061 ARM builds later pass at76117505dd837ead35e16a550ae194185ef3acfd/
136inputs; helper bytes match060. A separately reviewed saved-ledger verifier
passes10syntheticguards withnativecontext pinsunset. NoTV/package/native authority
follows. Preserve actual local SDK/package audit and fresh native admission gates.
The later061 two-preparation local audit/composition now passes17guards and both
actual SDK runs with independent source review. Root rehashed186inputs/1026SDK
outputs/4frontends; directUI ABI exact057/helperexactbound046/full057rollback
match. This remains offline artifact evidence, not native/Root admission. New
three-launch carrier needs its own frozen review/guards and fresh dated scope.

The earlier verified retained G2 installation was 0.56.0, with dashboard ELF
SHA-256 `e556658ae1603f51218d8d87d9e30cdb2becab260d50199f2985f4f02f7aeac9`
from frozen functional source `87945205e21393f5ef93d40a780b0c7e23d9acd8`.
`docs/g2-retained-preview056.md` records the backed-up same-ID 055→056 update,
three complete General info rows visually matched to fresh external reads,
source-correct form/category/Home Back, a separate fresh Home/Apps session and
complete staging/lease/agent cleanup. Exact original preferences/metadata, all
28 packaged resources and sampled Core state were preserved. The actual operator
and saved audit/Root replay pass; one saved-reader inventory-window failure is
preserved separately. No form row was activated; synchronous native-reply binding
was not established. Normal Root artwork stays off. Physical input, native failure
states, normal performance, all-app artwork, broad controls and C2 remain open.
That historical update used the actual055 archive for rollback. The current057
baseline above supersedes its future-update instructions; do not reuse its
055→056 carrier against057. Normal runtime
logs and immutable session receipts remain; old runtime bytes were not rehashed.

`docs/caption-read-native.md` records a separate ordinary headless one-GET probe
from `probes/caption-read` at source `a9dbb9535474f88277fb994d3e5f4b66d25fab67`.
The original native operator passes with complete known removal, exact retained 0.56
bundle/preferences, sampled Core preservation and closed lease/agent. Independent
saved audit and Root replay agree byte-for-byte. Preserve the local getauxval
link failure and first saved-reader suffix-glob failure; neither was a failed
native run or required a TV retry. Use only the plain anonymous fixed public
caption GET; no call_as_app, fallback, setter or normal integration is admitted
by this result. Native source uses a separate `0700` report parent/PID directory and
`0600` reports, plus possible independently mapped framework `1777`/log `0640` paths. The
wrapper's String is lossy/postallocation and Drop exposes no unregister status.
OS exit status, UK/DVB/app subtitle semantics, Preview permission, normal UI,
performance and C2 were unqualified by that probe. It closed on retained0.56 and
was removed; the separate normal057 delivery above provides later row evidence.

The 0.57 source in `docs/caption-read-057.md` adds one protected TV caption
preference row and one fixed public GET in the existing ordinary read worker.
It has no setter/editor/activation. All eight Accessibility rows are replaced
per cycle; failed peers stay unavailable and all-failed withdraws freshness.
Keep stored-preference meaning distinct from UK/DVB/app subtitle behavior.
Host checks and source review do not transfer the isolated probe's permission
to Preview. The separate retained057 delivery above now qualifies actual normal
row presentation. Native failure/expiry and regional semantics remain open.

The earlier verified retained G2 installation was 0.55.0, with dashboard ELF
SHA-256 `9acd524e84a3f5c2781fb472d4766016516fca4d287e3054c9185934da31617a`
from frozen functional source `46f431cd3f601810a41d76c929b9d7392d223ae1`.
`docs/g2-retained-preview055.md` records the backed-up same-ID update, two fresh
base sessions and a third empty Search open/Back cycle, black idle/wake, exact
private preferences, full bundle and sampled Core preservation, and complete
owned staging/lease/agent cleanup. The first pre-mutation refusal stays exit 1,
with a separate known unused-lease closeout. The final operator and saved audit
pass. Runtime logs/session receipts remain; old runtime bytes were not rehashed.
Normal Root artwork stays off. No new physical typing, Target/settings/favourite
mutation, all-app artwork, C2 or normal performance qualification is implied.

The separate `docs/g2-normal055-app-launch-return.md` now qualifies one owned
native Target drawer launch and a later explicit fresh normal Preview return,
with complete known Target/staging removal, current055/prefs/Core preservation
and released original lease/host agent. Independent saved audit and Root replay
agree byte-for-byte. Preserve the original exit1/full-workflow false: its single
acknowledged remove had not completed at the immediate registry check; the
separate exit0 continuation observed completion without retry. This is neither
automatic nor same-process return, arbitrary-app coverage, OS Target exit status,
physical input, performance, Root artwork or 056 deployment. The initial saved
reader failure remains recorded; correcting its nested-record comparison required
no TV rerun.

The earlier retained G2 installation was 0.47.0, with dashboard ELF
SHA-256 `72436fbe8717d7a65fa915ca1569ba1b0337cfec944ff348b1cf65e3742f1feb`
from frozen functional source `be6c06a341d72be1d06e92819a1be6c71ed52048`.
Its two fresh normal sessions, original private preferences, full bundle/Core20
preservation and six scoped staging-directory removals are recorded in
`docs/g2-retained-preview047.md`, with an independently replayed saved audit.
The six earlier failed operators remain failures; runtime logs are intentionally
retained. Normal Root artwork remains off, with no new physical-input, IME,
performance or all-app artwork qualification. The older 0.37 record and drivers
pinned to 0.31 remain historical; preserve the current installation/favourites.

The latest verified retained C2 installation is 0.48.0, with dashboard ELF
SHA-256 `b8e0ac9b3918c354a3fc1a7068c404a4a61d155318807f30e027b0aa130962d3`
from frozen source `cba8196ba66763f959bb50b5b1193d114de7d880`.
`docs/c2-retained-preview-048.md` records its own ordinary-owner migration
roundtrip, separate old-version restoration, fresh forward migration and two
normal sessions. Preserve the original preference bytes/owners in private
0700/0600 state and all C2-specific packaged resources. Do not reuse older
0.20 rollback/mode assumptions as the current baseline. The original failed
operators remain failures; later continuation and independent saved-audit
success do not rewrite them. Root artwork, C2 IME, physical input, performance
and all-app coverage remain unqualified.
The isolated C2 startup collection in `docs/c2-startup-collection.md` now has
independently replayed observation and separate cleanup evidence. Its new Target
identity confirms the observed opaque SDK argument and only its own measured
runtime mapping. Both earlier host operators remain failures. This grants no
Preview credential/permission authority or general temporary-path mapping; the
retained C2 installation at that historical observation remained 0.20.0.
Current C2 retention is the separately qualified 0.48 result above.

The 0.49 source in `docs/artwork-transition-049.md` implements app-controlled
pause/drain ordering and typed initialization/settlement. Its isolated G2 native
component now qualifies queued/unpolled Work, local intent/Back and poll-only
known completion, followed by seven helper sessions and complete cleanup with
normal 0.47 preserved. The independent saved audit and Root semantic replay agree
on all facts and bindings; diagnostic-check ordering differs. Real LG keyboard
or external-app handoff and authenticated closed-negative outcomes remain open;
keep normal interactive Root artwork off. Frozen 0.48 C2 artifacts remain bound to their own
source and receipts; never substitute a new 0.49 executable into those carriers.

The 0.50 transport foundation in `docs/artwork-transport-eof-050.md` preserves
legacy receive errors and adds no provider/server/bootstrap/Ticket acceptance.
Host tests do not qualify its Linux socket cases or native deployment. Do not
transfer 0.49 native evidence to a new executable; normal Root artwork stays off.
A separate ordinary headless G2 probe now qualifies four fixed local socket
cases from the byte-identical transport module, with scoped removal and retained
0.47 preservation. It does not qualify either new 0.50 dashboard/helper executable,
C2 behavior, an artwork session or closed-negative acceptance.

The 0.51 source in `docs/artwork-attempt-foundation-051.md` adds typed attempt
metadata and a narrow helper context-withdrawal/endpoint-closure candidate.
Host checks pass; no 0.51 native execution is qualified. The provider still
settles these failures as Unknown, retains pending state and leaves normal Root
artwork off. A reported closed candidate is not authority to clear a Ticket.
Finish failure still has no typed observed-EOF evidence. Retained G2/C2 baselines
and frozen earlier carriers/artifacts remain unchanged.

The 0.52 source in `docs/artwork-closed-negative-052.md` implements a strict
ReadAfter/Finish negative grader, consuming byte discard, finite joined
observation and a sealed current-Ticket-only resolver. All 17 host groups pass;
there is no 0.52 native execution qualification. Normal Root artwork stays off.
Do not transfer previous native evidence or recover an old pending record.
Post-unlink sync failure remains Unknown and may leave no marker; it grants no
power-loss or crash-recovery claim. Native cases require a separately reviewed
isolated design and a fresh permitted device window.

The 0.53 source in `docs/artwork-context-qualification-053.md` adds an exclusive
one-job Proof recipe, fixed protected ReadAfter plan and client Finish hold.
Host/Python/Clippy and matching ARM builds pass; neither native case is qualified.
Keep its package catalogue empty and use only the fresh actual owned Target
source. Ordinary Proof creates only its own private state directory without
saving preferences; never initialize it through Root or clear an old pending
record. Stop drawing/swapping on departure and drain the existing worker.
Positive timing misses release no image; selected-case classification must match
the actual failure stage. Preserve G2 0.47/C2 0.48 and keep normal Root artwork off.
No 0.53 package has been installed or run; the next native carrier needs its own
reviewed artifacts, external Target/process/child/namespace evidence and cleanup.

The 0.54 source in `docs/packaged-icon-demand.md` adds an ordinary packaged-icon
reader/worker/cache and Shell integration. It does not use Root sources, Tickets
or transition waits. Keep initial packaged bindings distinct from fresh live
app-version authority, drawing references borrowed, and all GL ownership on the
render thread. Pause dispatch/uploads immediately for departure/idle/keyboard/
external intent, including window close before tick. Capture diagnostics require
bounded readiness; frame counts alone do not qualify asynchronous image arrival.
The isolated G2 0.54 four-fixture test in `docs/packaged-icon-native-054.md` now
qualifies controlled framebuffer pixels and three-texture worker shutdown,
with separate known cleanup and a fresh full preservation readback independently
replayed. Three failed operators remain failures; no normal input/performance,
all-app, C2 or retained promotion is qualified. Preserve G2 0.47/C2 0.48 and the
independent frozen 0.53 artifacts. The separate ordinary scene packages
were installed then removed without launch requests; their limited cleanup is
recorded in `docs/native-surface-prepared-cleanup-milestone.json`, not native scene
or test-UID execution evidence.

The separate 0.54 performance-only G2 profile in
`docs/current-performance-054.md` now qualifies one isolated navigation/idle/wake
workload, natural exit and full known cleanup/preservation, independently replayed
by both reviewer and coordinator. Its earlier observer-limited attempt remains
unqualified. Retain the early 183.667 ms draw outlier and metric boundaries;
normal Preview total cost, physical latency/smoothness, C2 and retained promotion
remain open. Neither retained installation changed; normal Root artwork stays off.

`docs/keyboard-observer-054.md` records two observed human/native keyboard cycles
and separate cleanup. Preserve original exit1/full-workflow nonqualification:
the manual exit reserve expired, then the metadata parser rejected the source's
zero-byte owner-file spelling. Known later cleanup used the same original lease
and a size-zero-only parser adapter; it is not generic HOLD recovery. The two
cycle observations do not qualify normal Preview, C2 or general IME behavior.

The 0.55 source in `docs/drawer-empty-states.md` adds drawer empty-state copy and
existing-control recovery hints. Host checks and matching SDK ARM builds pass;
the separate normal G2 delivery above qualifies its bounded sessions. The
individual empty/error-state typography cases remain unqualified on the TV.
Keep its package version distinct from frozen 0.54 artifacts and preserve the
attributed framework unchanged.

The 0.56 source in `docs/device-information-056.md` adds three read-only General
rows from the already-collected system_info reply. Host checks and the matching
SDK ARM dashboard build pass; no new TV service call,
setter, permission or display-to-capability admission is added. Preserve mixed
failure withdrawal and the existing pointer/keyboard gates. The separate retained
G2 delivery above qualifies bounded native presentation; native failure/expiry
presentation and C2 remain unqualified.

The following earlier component records describe their historical baselines;
they do not replace the current retained versions above.

The isolated 0.38.0 check in `docs/artwork-startup-capability-038.md` qualifies one
ordinary native HBC `checkRoot` request and complete temporary cleanup. It did
not update the retained app, launch an artwork broker or qualify production
startup. Keep that boundary explicit when planning subsequent artwork work.

The isolated 0.40 result in `docs/artwork-startup-040.md` qualifies two serial
protected installed Root helper sessions and Identify/Finish. The first device
driver exited 1 on a terminal-receipt observation race; native evidence completed
and separate recovery exited 0. The second used the same frozen binaries with
fresh app/helper PID and start identities; its corrected driver exited 0 with
full temporary removal and preservation. Keep the original failure/recovery
history distinct. Normal 0.37 is unchanged; resources, concurrent/duplicate
startup, normal enrollment and production delivery remain unqualified.

The 0.41 result in `docs/artwork-startup-refusal-041.md` verifies the declared
wrong-Identify and wrong-Finish nonce refusals on G2. Both carriers exited 0;
each helper/HBC result remained an actual refusal. Independent saved evidence
replays the zero/three known Core children, raw process/GID bindings, full normal
0.37 preservation and all 24 cleanup gates per case. Keep this separate from
positive startup and other rejection/reliability coverage. Historical 0.40/0.41
carriers and artifacts remain pinned to their source and package. Resource and
normal loader integration still need their own native qualification.

The 0.42 source in `docs/artwork-installed-provider-042.md` adds one installed
Proof resource read per session and a worker Provider with joined completion,
full SourceKey/readback checks and per-instance quarantine. It extends the
protected Proof Core reader to the two fixed catalogue GETs. Host checks and both
ARM builds pass; renderer activation and a combined native provider run remain
pending. No 0.42 package is installed and Preview remains 0.37. Do not promote the
historical 0.41 guards into proof of the new resource/renderer path.

The 0.43 isolated renderer result in `docs/artwork-installed-renderer-043.md`
connects that provider to Home and Apps with bounded per-fetch observations and
joined shutdown. The G2 carrier exited zero: six completed transfers/helpers,
96 known children, three shared live targets on both pages and all 24 cleanup
gates. Independent replay passes 28,205 assertions. Only the first helper has
external live raw process binding; later helpers bind kernel/self/HBC records.
Retained 0.37 and production startup are unchanged; normal-profile admission,
all-app coverage and a backed-up retained promotion remain pending.

The source foundation in `docs/preview-artwork-admission.md` adds explicit closed
profile protocol/self/capability APIs while default APIs remain Proof-only. The
Proof provider now uses an own-state pending latch before HBC dispatch, retained
on unknown completion across new instances. Host checks and both ARM builds pass;
the new guard's
native regression, Preview Root/helper/provider wiring and actual Preview-ID
qualification remain pending. Do not clear old pending records on age, restart,
caller death or HBC false, or treat a parsed Preview response as Root authority.

The 0.44 result in `docs/artwork-installed-profiles-044.md` qualifies the controlled
G2 no-dispatch restart guard, a fresh six-transfer resource regression and C2's
separate static icon presentation. All three carriers exited zero with full
retained app/state preservation and 24 cleanup gates each. Independent saved
replays passed. The guard's exact own-record cleanup follows a no-helper-ever-
staged experiment; it is not recovery from an unknown real Root outcome. Only
first positive helper has independent live external process binding. Normal G2
0.37 and C2 0.20 remain unchanged; no C2 Root artwork permission is established.

The 0.45 candidate adds an explicit actual-Preview diagnostic, not normal artwork
activation. It requires existing initialized private preferences and does not
create state, start a persistence/action/setter worker or queue preference writes.
A joined inert Preview capability check and eight-second no-input hold precede
any resource worker. A declined preflight records zero Root exec/submitted jobs;
a created worker requires full known Root/child/Finished/HBC and pending closure.
The coordinator must back up and verify the actual G2 normal 0.37 installation,
preserve preferences, then restore and verify it through the supported installer
after a qualified diagnostic. Unknown installer or Root closure retains barriers;
never uninstall/reset the retained Preview or clear an old pending record. Actual
Preview permissions, retained promotion and keyboard/departure recovery remain
unqualified until their separate evidence is complete.

The G2 is primary and the user has now explicitly made it always available.
Do not repeatedly ask for G2 availability. C2 is available only between 00:00
and 06:00 Europe/London, or when the user explicitly makes it available. This
supersedes older all-day and overnight windows. Check fresh foreground and the
actual per-device baseline before a new native run; availability never replaces
process/scene/input guards. Old dated carriers and expired leases do not become
valid again.
Only the coordinator contacts TVs, with distinct per-device leases and private
agents and preserved retained installations/favourites. Refresh C2's actual
model, firmware, app version and complete bundle/state baseline before changes;
G2 results never establish C2 behavior. Physical-input checks require current
user availability. This authorization does not broaden protected operations or permit
LG Home, Core, boot, permission, media or OLED changes.

Do not rewrite public history or change the existing public repository's identity
until the standalone result has been reviewed with the user.

The actual 0.45 Preview experiment in `docs/preview-artwork-admission-045.md`
completed a known-no-worker decline and exact normal 0.37 restoration. Its context
expired before checkRoot: no HBC request, resource worker or Root command ran.
This proves the controlled restoration path, not Preview permission/resource
access. The 0.46 result in `docs/preview-artwork-preflight-046.md` renders while
collecting the same inert preflight and retains its first context for diagnosis;
all admission gates remain strict. Its actual G2 Preview-ID diagnostic completed
six resource/helper sessions and restored exact normal 0.37, with independent
saved-evidence replay. Normal interactive artwork stays off; transition/departure
closure and retained promotion remain separate gates. C2 Root access is not
established by this G2 result.

The inert C2 installer result in `docs/c2-installer-metadata-045.md` verifies
requested metadata on a new never-launched Proof package and preserves exact
normal 0.20. Its reference archive has different modes from the installed app;
do not use it as an exact rollback without reconciling actual metadata. Any
preference migration must run as the ordinary owning Preview identity, preserve
the original bytes/owners, change only its own directory and preferences file,
and prove reversal separately. No Root chmod/chown, state reset, parent-directory
permission change or retained-app uninstall is authorized by that procedure.
