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

The latest verified retained G2 installation is 0.56.0, with dashboard ELF
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
Use the actual056 archive and a fresh baseline for any future backup/rollback;
older055/047 update operators must not run against current056. Normal runtime
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
performance and C2 remain unqualified. Retained G2 remains 0.56; this probe is removed.

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
