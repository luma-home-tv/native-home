# Concept fidelity

The [original concepts](concepts/README.md) remain the intended interface. Their
Apple TV-style composition is the visual target: quiet black canvas, generous
spacing and a clear focus cue. The latest direction shows available app icons
directly on black, preserving their shape and alpha; graphite is for missing-art
fallbacks and controls. PlxNative supplies the attributed
native rendering, UI and input framework. Luma owns its dashboard composition,
branding, preferences and TV adapters; it contains no Plex client features.

G2 0.21.0 and C2 0.20.0 move the running interface closer to those concepts. This is a
progress map, not a claim that the complete concept is implemented.

| Element | Native implementation | Remaining gap |
| --- | --- | --- |
| Identity | Luma icon and wordmark in the persistent header; own SVG rail icons cached as native textures. | Refine secondary page/status text as the information architecture settles. |
| Canvas and focus | OLED-black canvas, graphite control/fallback surfaces, selected-icon outline, separate quiet current-route fill. | Measure contrast and focus animation with physical remote/pointer use. |
| Home | Four larger favourite cards across the visible shelf, uncropped artwork, names below cards, four HDMI cards below; separate LG Home return in the rail; native View all apps/inputs with shortcut-origin Back. | Long-title and empty-state polish; source artwork. |
| Apps | Native filters/search, three-column drawer beside a modal options panel, pin/unpin and earlier/later favourite ordering; stable focus and save/relaunch proof on both TVs; bounded focus/caption viewport, menu anchoring and whole-tile clipping. | Isolated G2 system-keyboard typing/Done/Back and corrected dismissal now have evidence; normal integration, broader IME/script/font coverage, typography polish and physical hold/pointer/wheel proof remain. |
| Header and rail status | TV-local clock/calendar; live network availability and sound-output summary. | Complete stale/error-state and long-status visual review. |
| Inputs | Two-column HDMI cards beside a graphite detail pane; separate cable/signal and independently read selected-source state. Native frames reviewed on both TVs; all four G2 source selections have independent readback/restoration proof and are enabled. | Live video/format, exact C2 idle-state restoration and C2 production switching; source artwork. |
| Controls | Six categories beside a graphite detail panel, cached Picture context breadcrumb, independent scrolling and read-only Picture/Sound/Network values plus 20 descriptor-validated Picture/Accessibility/General/Panel Care readings, with six further guidance/Sound readings qualified in isolated builds; native Back/pointer and all six LG-managed section/entry routes proved on both TVs; G2 audio-description range entry/Apply/endpoints/restoration now proved with description off. | Broader verified edits/setting coverage and physical input proof. |

The sound output, TV volume/mute and sound profile are distinct readings. TV
volume is not a measurement of an external amplifier's volume. Wired and Wi-Fi
connection states remain separate; simultaneous connections do not establish
the active route. Missing, inconsistent or stale values become unavailable.
Pointer speed has its own verified native enum adapter. Descriptor metadata
alone never authorizes a write; the remaining settings stay read-only or LG-managed.

Earlier milestones rendered seven native proof pages on both commercial TVs on
webOS 24, using the same application binary under ordinary app identities.
Private Home, Apps, Sound and Network frames were inspected. Separate proof
packages were removed and checked for remaining launcher entries, state, paths
and mounts. The retained previews were updated with saved favourites preserved.
See the [branding/status record](branding-status-milestone.json) and
[app-menu record](app-menu-milestone.json) for scope. The
[Controls layout record](controls-layout-milestone.json) adds the current native
category/detail and G2 handoff regression evidence. The app-menu record verifies
order persistence through a second native launch on each TV.
Household frames and detailed device receipts remain private; the public concept
renders contain synthetic content and keep their watermark.

The [Inputs detail record](inputs-detail-milestone.json) adds the two-column native
composition and passive reads on both TVs. A separate G2 test changed from HDMI 4
to HDMI 1 and independently confirmed the selected source, then restored the
original source and foreground. It had no live HDMI signal. The C2 returned the
TV tuner as its selected source; Luma leaves that unsupported selection detail
unavailable. That record is historical; 0.19.0 enables G2 selection after the newer per-port trials.

The [LG Home return record](lg-home-return-milestone.json) adds the concept's
separate rail shortcut. It launches only the registered LG Home binary through
LG's application manager and verifies its foreground. No Home-button interception
or boot behaviour changes are involved.

The [expanded control-readout record](control-readouts-milestone.json) adds current
Accessibility, General and Panel Care pages and eight more picture-profile
readings. Their ranges/options are validated from LG descriptors. Known read-only
values use normal contrast, while shared form bindings prevent activation. An
inactive LG control stays subdued; stale or inconsistent state is unavailable.
Those added readouts remain LG-managed except the separately validated pointer-speed control.

The [pointer-control record](pointer-control-milestone.json) adds a shared native
choice page under General. Focus and the current-value checkmark remain distinct;
only OK commits. Both TVs passed independent focus-only, change and restoration
reads, native frame review and complete removal/state checks. Physical Magic
Remote tracking/acceleration remains unmeasured.

The [HDMI selection record](hdmi-selection-milestone.json) adds separate G2
selection/restoration trials for all four ports. C2 selection remains gated. Its
trial selected the requested port, but the default tuner return selected DTV
instead of the original idle ATV readout. The recorded last physical preference
and LG Home were recovered with the checked persistent state preserved; exact
idle-state reproduction is unproven. No service reset is part of this change.

The [Home navigation record](home-navigation-milestone.json) adds the concept's
View all actions and a bounded native drawer viewport. Focused captions remain
inside the panel, menu context stays visible, and closing the menu clamps to the
content end. Whole-tile admission is an opt-in attributed framework change, shared
by paint and pointer stops; it is a candidate for upstream contribution.

The [numeric adapter experiment](numeric-control-experiment.json) validates one
signed audio-description level transition and restoration under the isolated
ordinary-app identity on G2, with description checked off. C2 was skipped at
preflight; its existing preference is preserved. The experiment adds no UI or
production write capability. That adapter record did not include an editor; the
separate UI proof below covers staging and cancellation. Those component
records alone do not establish a combined native setting path.

The [native range editor record](range-editor-milestone.json) adds the shared
signed-range widget and modal detail composition. Both TVs rendered synthetic
initial, staged, endpoint and unconfirmed states; saved markers stay separate
from the staged thumb, and Back cancels. The native proof emits no TV writes.
The [host preview](range-editor-preview.png) shows the working widget with
synthetic values. That record precedes connection to a TV setting.

The [combined numeric-control record](numeric-control-milestone.json) now joins
that editor to a fixed G2-only operation. The native entry, staged-only cancel,
Apply, signed endpoints and original stored value/type restoration passed under
the isolated app identity. The normal G2 0.21.0 preview passed a no-write
readiness/update check. This does not prove audible mixing, enabled-description
behaviour or native numeric writes on C2.

Next design work should complete source/format detail and expand contextual native
controls and read coverage, then review long text and physical navigation on
both devices. HDMI switching,
native setting writes, physical input coverage, reboot persistence and sustained
long-run performance each need separate evidence. The
[responsiveness record](responsiveness-milestone.json) measures bounded app-thread
submission pacing, input handling, CPU/RSS and idle painting on both TVs. Those
measurements do not prove physical input latency, GPU completion or panel response. LG Home, boot behaviour and core TV
services remain the working default throughout.

The [text-input record](text-input-milestone.json) defines a single-keyboard flow:
LG system keyboard preferred, Luma keypad available as a fallback. The native
field and drawer remain Luma-rendered. Closing the LG panel ends editing and
returns to Search; it must not expose an active second keypad. Corrected G2
proof frames show that transition. The retained previews are unchanged.

The [shared-controller record](keyboard-integration-milestone.json) adds isolated
G2/C2 Search/Keypad routing, corrected legacy preedit decoding and maximum draft
rendering. It does not establish normal interactive performance or complete IME
support. [Control coverage](controls-coverage-next.md) and
[parallel ownership](parallel-development.md) make the remaining work explicit.

The [actual-main-loop record](main-loop-milestone.json) adds interactive
keyboard/fallback, plain idle/wake and programmatic departure observations on
both models. These are lifecycle results, not new concept-fidelity frames or
performance measurements. The [guidance milestone](guidance-read-milestone.json)
now adds focused/scrolled native review of its three readouts on both models.
Original framebuffer comparisons confirm static branding remains present;
preview omissions alone did not establish a renderer defect. The retained
previews remain at their recorded versions.

The [G2 human check](manual-keyboard-milestone.json) now confirms the intended
single-keyboard Done/reopen/Back flow on the 0.25.0 actual main loop. The
[OK-wake record](ok-wake-milestone.json) adds intentional wake suppression on
both models. Neither supplies new concept frames or performance measurements;
the [Sound milestone](sound-read-milestone.json) separately qualifies its three
read-only rows and native focus/scroll layout on both models. Retained previews
remain unchanged, and native stale/error-state review remains open.
