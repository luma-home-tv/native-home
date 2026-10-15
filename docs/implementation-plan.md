# Luma Home implementation plan

The target is a responsive native Home dashboard for the OLED55G26LA G2 and
OLED65C26LD C2 on webOS24. Both are authorised for development and independent testing.
It remains a separate application: LG Home, Home/power/volume handling, firmware,
system libraries, boot configuration, media services and OLED protections stay
under LG's control. Retained installations and each TV's saved favourites must
survive updates; temporary proof packages use a separate identity.

The [concept renders](concepts/README.md) show the intended interface. They are
synthetic design illustrations, not screenshots of implemented functionality.
The [interface specification](../design.json) records the layout and interaction
requirements; the milestone records below identify what has actually been proved.
The [fidelity map](design-fidelity.md) compares the original concepts with the
current native interface. G2 runs 0.21.0 and C2 remains on 0.20.0. The
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

The canvas is true black for OLED. Graphite remains available for tiles and
controls, with bright focus/ink limited to useful areas. Idle behaviour must
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
Build before taking the lock; release it before reviewing frames or changing
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
