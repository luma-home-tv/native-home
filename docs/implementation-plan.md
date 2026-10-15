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
current native interface. Version 0.16.0 is installed on both devices; the
[LG Home return record](lg-home-return-milestone.json) is the latest scoped proof;
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
| Home | Clock/status header, immediate access to favourites and HDMI inputs, control-centre entry and LG Home return | Native Luma identity, clock/calendar, status rail and larger favourites/HDMI shelves and an explicit LG Home return verified on both TVs; view-all shortcuts remain |
| Apps | Complete actionable app drawer, search/filter, pinning and dashboard-only ordering | Registered visible apps joined to launch points; stubs/hidden/invalid targets excluded; live updates and identity/focus checks pass; normal app launch enabled and isolated native target proof passed; All/Favourites and native keyboard search are implemented and captured on G2; modal pin/order menu and two-launch persistence are verified on both |
| Favourites | Pin/unpin with held OK, save reliably, retain selection across changes | ARM pin/save/readback/relaunch/unpin verified; bounded background writer; reboot survival remains untested |
| Inputs | Four HDMI cards and a detail pane with distinct plug, signal and selected-source states; select through LG's normal source path | Two-column native cards/detail pane and independent EIM selected-source getter verified on both; separate G2 HDMI 4 → HDMI 1 trial confirmed source change and restoration; C2 tuner restoration and live format/playback remain unverified, so production switching stays disabled |
| Controls | Native category/detail pages, correct context, validated choices/ranges and access to the rest through LG Settings | Six-category/detail composition and read-only picture, sound and network refresh verified on both; all six LG-managed entries and native return proved on both models in 0.13.0; broader native controls/read coverage remain |
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
   restoration and live HDMI proof before enabling production switching; both
   TVs now have verified LG-managed section routes. Keep
   unknown/hidden/self targets, unvalidated operations and extra parameters
   rejected; an acknowledged request is not execution proof.
3. **Expand controls safely.** Audit webOS24 descriptors and semantics for each
   operation. A descriptor alone does not grant write authority. Require valid
   range/enum, matching input/HDR/mode context, bounded effect, readback and an
   original-state restoration test. Ordinary-app audio getter aliases and network
   readings are now independently verified; native setters remain absent. Use a restricted
   local helper only for operations that actually need privilege, with caller,
   executable and session validation and a fixed operation allowlist.
4. **Finish the interface and measure it.** The combined Home now uses two shared card shelves, with readable source
   status on every tile. Clock/status and app ordering are now implemented. Complete
   native choice/range widgets and broader read coverage. Verify physical
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
