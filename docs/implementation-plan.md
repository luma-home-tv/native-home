# Luma Home implementation plan

The current delivery target is a responsive native Home dashboard for the
OLED55G26LA G2 on webOS24. The G2 is always available for coordinator-owned tests.
The C2 is available only from 00:00 to 06:00 Europe/London or when the user
explicitly releases it. Each run still needs fresh scene, device and lease
checks; refresh the C2 baseline before changes. Physical input requires current
human availability, and results stay device-specific.
It remains a separate application: LG Home, Home/power/volume handling, firmware,
system libraries, boot configuration, media services and OLED protections stay
under LG's control. Retained installations and each TV's saved favourites must
survive updates; temporary proofs use a separate identity or a backed-up,
verified update and rollback procedure for the retained identity.

The [concept renders](concepts/README.md) show the intended interface. They are
synthetic design illustrations, not screenshots of implemented functionality.
The [interface specification](../design.json) records the layout and interaction
requirements; the milestone records below identify what has actually been proved.
The [fidelity map](design-fidelity.md) compares the original concepts with the
current native interface. G2 now retains verified [0.57.0](g2-retained-preview057.md),
including the caption-preference readout. The later
[0.58 cache observation](ordinary-artwork-cache-058-native.md) reached ordinary
Home/Apps cache associations and returned to exact 0.57 through separately
verified cleanup; its original operator failure remains explicit. The earlier
[0.47.0 milestone](g2-retained-preview047.md) recorded two fresh Home/Apps/Back
sessions, a natural idle-black/wake and full saved-state,
bundle and Core20 preservation. Its visual frames show the logo and unboxed
available icons; broader artwork and physical-input/performance qualification
remain separate. The latest verified C2 retained version is
[0.48.0](c2-retained-preview-048.md), following
its own ordinary-owner forward/reverse metadata roundtrip and exact old-version
restoration, then a fresh forward migration and two normal sessions. Original
preference bytes/owners, C2 resources, Core files and checked state are preserved.
This no-text trial does not qualify C2 IME, Root artwork or all-app coverage. The
[numeric-control record](numeric-control-milestone.json) covers the latest G2
native range path and the retained update; the
[Home navigation record](home-navigation-milestone.json) covers the shared Home/drawer layout;
the [HDMI selection record](hdmi-selection-milestone.json) covers source control and its C2 boundary;
the [pointer-control record](pointer-control-milestone.json) covers the first native editable setting;
the [expanded control-readout record](control-readouts-milestone.json) covers the twenty added readings;
the [LG Home return record](lg-home-return-milestone.json) covers the explicit rail shortcut;
the [Inputs detail record](inputs-detail-milestone.json) covers source layout/readback;
the [responsiveness record](responsiveness-milestone.json) retains the bounded
performance evidence from 0.14.0.

The [0.59 explicit preparation path](artwork-catalogue-preparation-059.md) now has
host checks, matching ARM builds and local package/ABI inspection. Its first
[native batch](artwork-catalogue-preparation-059-native.md) prepared 15 of 16
images and returned to exact 0.57 with independently audited known cleanup.
The original operator exit 1 remains explicit; the ordinary consumer was skipped.
The later [0.60 trial](bounded-icon-decoding-060-native.md) completes all sixteen
images in one selected batch and ordinary Home/Apps cache consumption, then
restores exact 0.57 with independently audited cleanup. Multiple-batch/current
per-key coverage and retained delivery remain open. This component does not
complete the dashboard or qualify a new normal release.

The [0.53 exclusive departure-test recipe](artwork-context-qualification-053.md)
now has host, strict-package, independent source-review and matching ARM-build
evidence. Its two real ReadAfter/Finish native cases and independently replayed
removal still precede normal artwork activation. This host work does not change
current device availability or admit a native run, and does not update either
retained app.

## Near-term release and parallel work

Keep packaged icons usable while qualifying the separate ordinary cache path.
Choose one coherent normal-release artifact and prioritise
normal Search, one-keyboard dismissal/reopen, remote focus/hold/wheel, an owned
app launch and LG Settings return, reversible favourites and repeated entry/idle.
The recorded normal sessions establish Home/Apps/Back and idle/wake within their
respective scopes; latest retained G2 is 0.57 and latest verified C2 is 0.48.
Earlier isolated results remain useful component evidence. Qualify the relevant
normal flows using the retained artifacts where possible, then change source only
for an observed gap. C2 keyboard and input results require their own evidence.

The bounded thumbnail fix now has matching ARM artifacts and a successful
reviewed batch/ordinary Root-free consumer trial, with per-attempt disposition
and known restoration.
The SDK mount comparison preserves all measured aliases; each trial needs a
fresh native baseline. Follow with multiple batches where needed, final current
per-key body/decode coverage, invalidation/failure behavior and a backed-up normal
delivery. A cursor, index Ready or aggregate GL count is not all-app coverage. Measure
current-artifact cold preparation and ordinary warm navigation separately before
claiming a performance gain. Eviction/stale-writer recovery and C2 qualification
remain open; normal interaction must keep Root artwork off.

The [0.61 source](controls-and-icon-observations-061.md) moves each existing LG
category shortcut before long native readings and adds a bounded, session-only
per-key cached drawing ledger. Its [completed temporary G2 run](controls-and-icon-observations-061-native.md)
now qualifies both preparation batches, first-batch continuity and ordinary
drawing references for the exact supplied 24-key set, with natural Back exit,
independently audited restoration and cleanup. Preserve the two earlier failed
attempts. Next prioritise the marker-free retained artifact's practical daily
loop and backed-up delivery, then physical input/favourites and matching
performance. Do not repeat population solely to increase component counts.

Split host work into independent streams, with one coordinator admitting frozen
artifacts and serialising each TV's tests:

| Stream | Useful next output | Dependency |
| --- | --- | --- |
| Interface and input | Empty/long/localised/stale states, focus/menu/dismissal acceptance matrix | Physical remote and IME outcomes need a current available TV window |
| Packaged icons and performance | Coverage by app identity, bounded startup/decode review, matched current-build workload | New/revised apps can lack snapshot artwork; a metadata path alone grants no byte access |
| Read-only controls and inputs | Missing-value inventory and consistent native/read-only/LG-managed labels | Each additional setter or C2 source change needs its own capability, context and restoration proof |
| Live artwork | Explicit finite preparation, per-key publication/body coverage and ordinary cache consumption | Actual Preview role, per-attempt helper/child/pending closure and known restoration; normal navigation keeps Root off |
| Integration and release | Known update/removal, fresh preference preservation and a scoped release record | Current source, package and native receipts must agree; unknown outcomes retain their barriers |

Release acceptance must also cover the practical daily loop on that same normal
artifact: Home/Apps/Inputs/Controls, Search and one LG keyboard, physical menu
hold/release, owned-app launch and explicit return, LG Settings handoff/return,
idle/wake and repeated entry. Use an owned Target for reversible favourites and
preserve the original preference semantics and bytes; inverse reordering alone
can materialise a previously absent order vector. Fresh SDK relaunch is not
automatic app return, and selected HDMI source is not actual video/format proof.

Complete the intended control inventory as verified native read/edit, explicit
LG handoff, protected or unavailable. Missing/stale/partial presentation and
long/localised labels can advance in parallel with cache work. New setters need
their own capability/context/effect/readback/restoration evidence. Measure the
selected normal artifact's total resource cost and warm navigation separately
from cold preparation and diagnostic capture overhead before claiming speedups.

The pre-0.54 packaged startup loader visited only the first 64 catalogue entries
and checked size/dimensions after reading/decoding. The current 0.54 source adds
a separate [bounded ordinary reader and demand cache](packaged-icon-demand.md);
its [isolated G2 result](packaged-icon-native-054.md) verifies three controlled
images, missing fallback, worker/texture shutdown and separate cleanup/preservation.
It does not update either retained installation or establish TV performance/all-app
coverage. The retained trials establish no over-64 failure
or measured memory fault. Measure normal navigation and return flows with the exact current artifact;
the [surface preparation record](native-surface-scene-prerequisite.md) explains
why old positive Root helper timings cannot qualify release responsiveness.
Keep normal Root artwork off while its independent feature work continues.

The separate [0.54 G2 performance-only workload](current-performance-054.md)
now completes Home/Apps/Controls, actual idle/wake and natural Back exit, followed
by verified removal/preservation and byte-identical independent audit replay.
Its earlier observer-limited attempt remains unqualified. The completed run
retains a 183.667 ms draw outlier already present in the earliest Home sample;
normal Preview cost, physical input/smoothness and C2 remain separate checks.

The [0.54 G2 keyboard record](keyboard-observer-054.md) now corroborates two
human/native Done/reopen/Back cycles with no extra keypad. Its final-exit reserve
and empty-file parser failures remain explicit; later known cleanup and full
preservation do not promote the failed original operator. The next
[0.55 source](drawer-empty-states.md) introduced distinct empty-favourites,
search-miss and unavailable-data copy. Its separate
[normal delivery](g2-retained-preview055.md) qualifies bounded sessions; every
individual empty/error layout remains unqualified on the TV.

## Direction

The agreed visual direction is the Apple TV / tvOS feel that PlxNative interprets
well: large cards, restrained chrome, smooth spatial focus and an obvious Back
hierarchy. Keep this native and remote-first. The concepts are a starting point;
remove unnecessary screen furniture as the interactive screens take shape.

Build the standalone Luma Home client on explicitly attributed PlxNative Rust UI, input/focus and
SDL/OpenGL ES libraries. Reuse its components instead of maintaining a second
widget system. Generic improvements can be proposed upstream separately after
verification; the Home client does not change the Plex application or player.
The shipping architecture uses native rendering, with no WebView.

The canvas is true black for OLED. Show available app icons directly on black,
preserving their aspect ratio, transparency and source shape. Add no graphite
plate or forced rounding; show a focus outline only on selection. Missing artwork
uses a graphite initials/name fallback. Graphite remains available for controls,
with bright focus/ink limited to useful areas. Idle behaviour must
minimise static bright elements without changing LG's panel protections. The
[branding masters](../assets/luma-home/README.md) record the icon, wordmark
and palette; a black background is not treated as proof of burn-in prevention.

| Area | Intended behaviour | Current evidence |
| --- | --- | --- |
| Home | Clock/status header, immediate access to favourites and HDMI inputs, control-centre entry and LG Home return | Native Luma identity, clock/calendar, status rail and larger favourites/HDMI shelves, explicit LG Home return and native View all shortcuts; Back restores the invoking local route |
| Apps | Complete actionable app drawer, search/filter, pinning and dashboard-only ordering | Registered visible apps joined to launch points; stubs/hidden/invalid targets excluded; live updates and identity/focus checks pass; normal app launch enabled and isolated native target proof passed; All/Favourites and native keyboard search are implemented and captured on G2; modal pin/order menu and two-launch persistence are verified on both |
| Favourites | Pin/unpin with held OK, save reliably, retain selection across changes | ARM pin/save/readback/relaunch/unpin verified; bounded background writer; reboot survival remains untested |
| Inputs | Four HDMI cards and a detail pane with distinct plug, signal and selected-source states; select through LG's normal source path | Two-column native cards/detail pane and independent EIM selected-source getter verified on both; separate G2 HDMI 4 → HDMI 1 trial confirmed source change and restoration; all four G2 ports now have independent selection/restoration trials and production selection in 0.19.0; C2 exact idle restoration and live format/playback remain unverified, so C2 selection stays gated |
| Controls | Native category/detail pages, correct context, validated choices/ranges and access to the rest through LG Settings | Six-category/detail composition and read-only picture, sound and network refresh verified on both; all six LG-managed entries and native return proved on both models in 0.13.0; 20 descriptor-validated readouts added in 0.17.0 across Picture, Accessibility, General and Panel Care; six more guidance/Sound readouts have isolated 0.25.0/0.26.0 ordinary-app/native proof on both; native pointer-speed choice/commit/restoration proved on both in 0.18.0; G2 native audio-description range entry/Apply/endpoints/restoration proved in 0.21.0, with description off; broader verified edits and setting coverage remain |
| Navigation | Remote focus, safe tap/hold, Back hierarchy, pointer and wheel | Shared-machine tests and scripted ARM navigation pass; injected Back reaches SDL and exits; physical remote/pointer/wheel proof remains |
| Performance | Smooth transitions, bounded memory and render only when needed | Native Mali-G52 rendering and app-local black idle/wake proved; Native injected SDL navigation and bounded submission/CPU/RSS/idle-paint measurements now exist on both; physical latency, GPU/panel timing and stress memory remain unverified |

The [input-batch candidate](input-batch-ordering-milestone.json) records the 0.29.0
queued-cancellation and duplicate-activation fixes. Its physical workflow remains
incomplete. The [0.30.0 language/region candidate](language-region-reads.md) adds
five protected General readings; [isolated ordinary-app reads and focused native layout](language-region-read-milestone.json)
are verified on G2. The [backed-up normal 0.30.0 delivery](retained-preview-030-milestone.json)
then verified Apps/Search/keyboard, General rows and preservation; broader release
checks remain open.
The [isolated 0.31.0 qualification](read-only-values-031-milestone.json) adds
legible current protected values, unchanged activation blocking and verified
build/package inputs. Its [retained promotion](retained-preview-031-milestone.json)
passed a separate backed-up normal workflow with favourites preserved.

After that delivery, prioritise live app artwork: the current live catalogue
preserves snapshot icons but cannot supply artwork for newly installed or
revised apps. Use the shared native texture loader, bounded work/cache ownership
and identity-preserving catalogue updates. First verify ordinary-app access to
webOS24's actual icon paths; metadata alone does not establish filesystem access.
The [first passive survey](artwork-access-032.md) read one developer icon but found
four sampled directories absent inside the ordinary jail. Qualify the missing
resource route before treating a direct-file worker as complete artwork support.
The [Identify-only candidate](artwork-broker-identify.md) now separates that
route's caller/transport qualification from icon reads and production startup.
The subsequent [fixed context reader](broker-context-reader-034.md) qualified one
restricted native G2 exchange and clean removal with checked state preserved.
The [0.35 isolated resource test](artwork-transfer-035.md) now qualifies a real
icon transfer and ordinary-process bounded decode from each of five observed
installation classes, with complete removal and preserved checked state. Its
pure cache state has separate ownership/invalidation tests. The
[0.36 isolated C2 lifecycle](artwork-live-036.md) now connects a bounded worker,
cache and main-thread textures for an owned fixture, covering initial artwork,
same-path replacement, package revision, removal and joined shutdown. Other apps
were deliberately excluded; initials in that test are not a C2 capability failure.
A direct RGB recheck withdraws the initial missing-text report: every pixel
outside the updated fixture was unchanged. The isolated 0.37 G2 capture now
qualifies synthetic icon-only alpha/aspect/fallback presentation with preserved
checked state. Its [backed-up normal G2 delivery](retained-preview-037.md) then
verified real packaged artwork and the normal loop across two launches.
The [0.38 read-only capability check](artwork-startup-capability-038.md) now
qualifies one ordinary native HBC status request with clean isolated removal;
the retained 0.37 installation and favourites remain untouched. It does not
qualify a Root child or production startup. The subsequent
[0.40 installed startup component](artwork-startup-040.md) qualifies two serial
protected Root Identify/Finish exchanges with fresh app/helper identities. The
first operator observation failed and needed separate recovery; the second
corrected driver passed with full cleanup/preservation. The isolated
[0.41 nonce-refusal results](artwork-startup-refusal-041.md) verify the two declared
G2 refusals, independently bound process/GID evidence and full cleanup, without
extending the helper's policy. Next, connect the existing resource route to the
actual worker/cache/native textures, then broader live artwork
coverage and a separately admitted retained integration. The
[0.42 provider candidate](artwork-installed-provider-042.md) adds the finite
installed transfer/provider and explicit worker budget in source; host checks
and both ARM builds pass. The [0.43 renderer result](artwork-installed-renderer-043.md)
adds isolated Home/Apps activation, raw captures and bounded joined-fetch
observations. Its G2 carrier now exits zero with six completed helpers/transfers,
96 known children, three shared live targets on both pages, complete temporary
removal and preserved normal 0.37 state. A 28,205-assertion saved-evidence audit
passes; actual Preview-role startup and a backed-up retained rollout remain
separate gates. The
[production startup design](artwork-startup-design.md) separates the
operator-started helper from an independently qualified bootstrap.
The [Preview admission foundation](preview-artwork-admission.md) now supplies
closed profile-aware identity/codecs and a negative-only pending-operation guard
in the Proof provider. Host checks pass; its new native guard regression,
remaining installed-profile integration and actual Preview-ID trial precede
normal activation. A new app ID cannot prove the retained app's LS2 permissions.
HDMI format/status and missing Sound/Accessibility readings remain subsequent
bounded, read-only inventory work. Existing verified source selection and LG
routes remain available throughout.

The earlier [OLED/branding milestone](oled-branding-milestone.json) records
true-black framebuffer evidence and clean removal on `2a53bfd`. The later [standalone device milestone](standalone-device-milestone.json) corrects
the earlier missing-rail interpretation: original cropped framebuffer bytes show
the complete rail on all six pages. All six complete PPM files also match the
prior captured states byte-for-byte. No renderer
change is supported by that claim. Physical input and broader UI work remain.


The [drawer milestone](drawer-milestone.json) records native All/Favourites
filtering and a shared-focus alphanumeric keyboard. The scripted G2 path typed
a query, filtered results, closed the keyboard and restored All without a TV
request. Physical input and full text/IME coverage remain to verify.

The [text-input record](text-input-milestone.json) adds a native bounded Unicode
field, grapheme-aware edits and isolated G2 LG-keyboard activation/reopening.
The user confirmed Magic Remote typing and Done/Back. LG consumes Back when
hiding its own panel; Luma now detects that dismissal, stops its request and
returns focus to Search without revealing its fallback keypad. Two corrected
native cycles and independent window/removal checks pass. This remains a proof
build; the retained versions above are unchanged.

The intended search flow uses **one keyboard at a time**. Prefer LG's keyboard
for its language and input integration, and offer Luma's keypad when unavailable
or explicitly selected. Keep the field, filtering and drawer natively rendered.
Done/Back retains the query and ends editing. Never switch to a second keyboard
merely because the first one closed. Normal request/fallback integration,
prediction and broader script/font qualification remain before deployment.

The [shared keyboard-controller record](keyboard-integration-milestone.json)
adds the candidate's Search/Keypad routing, admitted platform gate, corrected
native preedit ABI and bounded visible text uploads. The same frozen binary
passed isolated G2 and C2 trials: two guarded dismissals, synthetic preedit/clear,
explicit fallback and maximum-length text at three caret positions. Both were
removed with preserved-state checks. The normal interactive event loop, actual
composing languages and retained updates remain separate gates. The user saw no
word predictions in the available G2 keyboard mode; no prediction-replacement
contract is inferred. See [ABI notes](webos24-text-compatibility.md).

The newer [actual-main-loop record](main-loop-milestone.json) separates the
0.24.0 interactive scenarios from the earlier helper trials. Both models have
independent keyboard/fallback, real 120-second idle and programmatic LG Home
departure results, with preserved-state/removal checks. Both also completed idle
begun with LG editing active; adjacent recorded states show editor/SDK closure
at the idle transition, without establishing an exact SDK close cause. Network
Back and fixed synthetic text are not physical typing/Done evidence. The newer
[OK-wake record](ok-wake-milestone.json) independently qualifies a first OK that
only wakes, followed by a distinct second OK that opens Search, on both models.
Both host drivers were interrupted after native completion; reviewed same-run
closeout recovered removal and preservation without restarting either scenario.
The [G2 human record](manual-keyboard-milestone.json) adds current-build typing,
Done, same-process reopen and Back with `luma` retained and no fallback keypad.
One closing-key release was drained in the same SDL batch. Pointer, later-batch
and broader dismissal races remain unqualified, as do C2 physical typing,
explicit idle stop cause, performance and retained promotion.
One preceding G2 attempt exited on early focus loss before readiness; its cause
and cold-entry reliability remain unresolved. The
[startup investigation](startup-lifecycle-investigation.md) records the exact
event/departure timing and the next bounded observation. The other incomplete attempts and
the corrected host dismissal-counter mistake remain in the record.
The [0.26.0 startup/relaunch record](startup-relaunch-milestone.json) now qualifies
two predetermined G2 sessions through the actual loop: programmatic LG Home
departure with the LG editor open, then a distinct process loading the same
installed package and unchanged saved preferences. The second session closes
the keyboard and exits through network Back. Both native identities, startup
endpoints, independent window observations and removal/preservation checks pass.
One earlier observation stopped on LG's splash classification; that failed
attempt remains separate. No native departure-policy change was needed. The
original rare focus-loss cause and general reliability remain open, as does
this particular relaunch profile on C2.
The next delivery step is the [backed-up G2 preview rollout](preview-rollout-plan.md):
qualify installer rollback, preserve the retained installation/state and validate
the normal app identity. The 0.26.0 normal package is prepared, with deployment
still pending those checks.

The [parallel plan](parallel-development.md) now lists the six broad remaining
delivery stages, the immediate interactive-loop gate and the next parallel wave.
It assigns independent firmware/text,
controls and C2-validation lanes around one integration owner and one live owner
per TV. The [coverage audit](controls-coverage-next.md) preserves the comprehensive
control-centre backlog; existing category routes do not establish every-setting
coverage. Its next candidate is a separately admitted pointer-size control after
capability/semantics review, rather than a generic setter.
The [read-coverage audit](read-coverage-audit.md) identifies three guidance
parameter readouts and the Sound dimension boundary. The
[guidance milestone](guidance-read-milestone.json) now qualifies those readouts
and focused/scrolled native component frames independently on both models.
They add no setter; physical input, freshness transitions and retained promotion
remain separate.
The [optional global Sound group](sound-read-design.md) now has fixed GETs,
strict schemas and three protected rows integrated behind independent freshness.
Its [0.26.0 milestone](sound-read-milestone.json) independently qualifies ordinary
getter/descriptor access, typed readback matching and native focus/scroll/layout
on both TVs, with complete removal and retained/core/settings preservation.
Native stale/partial transitions, audible effects and retained updates remain open.
The [capability presentation model](control-capability-design.md) keeps reading
freshness, edit eligibility, pending operations and LG routes separate; it has
no normal UI caller yet.

## Next implementation stages

1. **Expand live data.** The typed normal-app provider now refreshes catalogue,
   launch points, HDMI and picture state in the native interface. Focus/favourites,
   unchanged gestures and stale/context failure handling have checks; four real
   cycles were applied on both TVs, including sound/network in 0.10.0. Complete
   broader coverage and newly installed icon loading. Do not treat subscription acknowledgement as delivery
   proof. Keep loading and queues bounded.
2. **Complete normal actions.** The restricted application adapter and live
   drawer/shared-OK path now launch an isolated native test target with both
   adapter and independent foreground readback. Normal application launching is
   enabled; personal viewing accounts were not exercised. Finish C2 tuner-source
   restoration and live HDMI proof; G2 source selection is now enabled after its per-port trials; both
   TVs now have verified LG-managed section routes. Keep
   unknown/hidden/self targets, unvalidated operations and extra parameters
   rejected; an acknowledged request is not execution proof.
3. **Expand controls safely.** Audit webOS24 descriptors and semantics for each
   operation. A descriptor alone does not grant write authority. Require valid
   range/enum, matching input/HDR/mode context, bounded effect, readback and an
   original-state restoration test. Ordinary-app audio getter aliases and network
   readings are now independently verified. Twenty more descriptor-validated
   picture/accessibility/general/panel-care readouts have ordinary-app proof on
   both TVs; signed ranges, enums, dimensions and expiry are checked. Descriptor
   success does not grant write authority. Pointer speed now has a separate fixed
   enum adapter with firmware/descriptor/foreground/expected-value checks and
   independent native change/restoration proof on both. An isolated
   [signed numeric adapter trial](numeric-control-experiment.json) now proves a
   one-step audio-description level change and restoration on G2, with
   description checked off. C2 was skipped before installation when the safe
   test context was unavailable. It is restricted to the proof identity and does not
   enable a production slider. The newer [combined numeric-control proof](numeric-control-milestone.json)
   connects the shared range editor, fresh fixed-setting reads and bounded worker
   on G2, including endpoints and native restoration. G2 0.21.0 enables that
   guarded entry; C2 numeric control stays gated. Other settings remain read-only
   or LG-managed. Use a restricted
   local helper only for operations that actually need privilege, with caller,
   executable and session validation and a fixed operation allowlist.
4. **Finish the interface and measure it.** The combined Home now uses two shared card shelves, with readable source
   status on every tile. Clock/status, app ordering and View all shortcuts are now implemented. The
   bounded drawer uses shared external grid geometry/springs, whole-tile
   admission and content-end clamping; native motion/settling proof is scoped in
   the current milestone. The [shared native range editor](range-editor-milestone.json)
   has synthetic staged-edit and removal proof on both TVs. Its fixed G2 numeric
   adapter now also has native entry, Apply, endpoint and restoration evidence.
   Extend this pattern only after the next operation passes its own capability,
   freshness/context, bounded completion and readback checks.
   Expand the verified choice pattern and complete broader read coverage. Verify physical
   remote, pointer, wheel and Back behaviour on both TVs.
   Measure the actual dashboard's frame pacing, input response, memory and idle
   behaviour; the earlier C renderer's results do not transfer to this client.

## Protected and unsupported operations

The [0.46 actual Preview diagnostic](preview-artwork-preflight-046.md) now has
G2 resource and exact restoration evidence. Keep normal artwork disabled until
the [transition lifecycle](artwork-transition-lifecycle.md) is qualified. The
[C2 installer metadata measurement](c2-installer-metadata-045.md) supports one
rollback mechanism component. The [C2 0.48 record](c2-retained-preview-048.md)
now adds actual ordinary-owner migration/reversal, same-ID restoration and a
separate current normal UI trial; these device-specific results do not transfer
Root artwork or keyboard authority.
The [0.48 SDK-argument candidate](preferences-sdk-arguments-048.md) accepts one
bounded opaque launcher JSON object without granting operation authority. Its
host/ARM checks are followed by the separately recorded C2 native roundtrip
and normal retention evidence; the original failed attempts remain distinct.
The [isolated C2 startup collection](c2-startup-collection.md) now confirms one
ordinary Target's launch ABI and narrow live runtime mapping, followed by
separately verified cleanup and unchanged retained 0.20 at that stage. Later
actual Preview migration is recorded separately; the earlier missing-report
cause is not established merely by later success.

Network reconfiguration, firmware, factory reset, service/calibration controls,
OLED protections and compensation scheduling remain protected/delegated. An
unsupported control routes to LG Settings or stays unavailable; the UI never
invents a successful native action. No arbitrary shell command, service URI or
file path is exposed through the interface. No Home interception or boot hook is
part of this plan.

## Verification and delivery

Every TV build uses its own isolated package and the repository's device lock.
The user has now authorised deployment and testing on both the OLED55G26LA G2
and OLED65C26LD C2. Keep LG Home and all core services untouched on both. Use
each device's own private inventory, firmware check, captures and cleanup receipts;
G2 results do not establish C2 behaviour. After removal proof, explicitly requested
interactive deployments may remain installed as a normal app with no boot hooks.
The earlier C2 hold has been explicitly lifted; both are authorised again. The [clock milestone](clock-milestone.json) records
the G2 0.8.0 update and preserved favourites, and separates the interrupted C2
component test from complete validation.
The [Settings handoff milestone](settings-handoff-milestone.json) then verifies
the LG Picture overlay, sustained underlying dashboard lifetime and return after
injected Back on the G2. Version 0.9.0 enables that fixed route with per-launch
model/firmware/SDK checks; native setting writes and HDMI switching remain gated.
Build and host-check before taking the lock. Keep the device lease through live
frame review and input, then release it after verified closeout before changing
source. Inspect native frames, record exact source/build revisions and keep
private device data outside Git. After each test, verify removal of the package,
launcher entry, process, runtime, persistent test data, jail and mounts; compare
checked core hashes and picture/audio readbacks, then verify LG Home and restore
the original foreground.

The current evidence is in the [native shell milestone](shell-milestone.json)
and [favourites/read-capability milestone](preferences-milestone.json), and
[live-reading milestone](live-milestone.json) and
[application-launch milestone](launch-milestone.json), plus the
[combined Home layout milestone](home-layout-milestone.json) and
[OLED/branding milestone](oled-branding-milestone.json). Completion
requires the intended functional interface and device evidence above, not only
concept renders, a successful build or component tests.


## Standalone dependency boundary

The Luma repository tracks its application, adapters, models, assets, plans and
tests. PlxNative is a pinned external dependency with its original authorship
and package names. Five framework crates are selected; Plex/media/session/data
application crates are excluded from the resolved graph. The integration patch
is kept separately and attributed. Root build/packaging targets build only Luma
binaries. Historical capture records remain historical until this standalone
tree receives new device verification.
