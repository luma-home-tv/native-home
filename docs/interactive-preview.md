# Interactive preview

The installed app is **Luma Home Preview**, package
`com.ash.nativehome.preview`, version **0.30.0 on G2 and recorded 0.20.0 on C2**.
The [0.30.0 G2 delivery](retained-preview-030-milestone.json) verified the normal
keyboard and five protected General readings with favourites preserved. C2 was
not contacted or modified by that delivery.
The [latest G2 update record](apps-scroll-rest-milestone.json) verifies the complete normal
bundle, preserved favourites and a guarded Apps/Search/LG-keyboard/Back smoke.
Both devices are authorised for development again. It is a normal native app in LG's
Home app list. It does not become the Home-button destination or start at boot.

Open it from LG Home. The separate **LG Home** entry below Controls returns to
LG's normal homescreen. It uses a fixed registered target, requires the tested
model/firmware/SDK and LG Home metadata, and confirms foreground after launch.
It cannot be activated through a stale hit behind an app menu or search keyboard.
The [return record](lg-home-return-milestone.json) describes the isolated native
proof and state checks. The remote's Home button keeps its original behaviour.

Home has **View all apps** and **View all inputs** buttons beside the shelf
headings. Up from favourites reaches View all apps; Up from the source shelf
reaches View all inputs. View all apps opens the complete drawer and clears any
previous local search/filter. Back closes a keyboard or app menu first, then
returns to the invoking Home button. These routes never select a TV source or
open a household app by themselves.

The drawer keeps the focused card/caption inside its actual viewport. Its menu
preserves the selected app as context across three/four-column layouts, and Back
returns to that app. Whole-tile clipping keeps partial unfocused cards out of both
the picture and pointer targets. Long focused labels retain the shared marquee.
The [navigation record](home-navigation-milestone.json) separates host, native
layout and bounded motion/settling evidence.

Directions move between the rail, favourites and app drawer. OK opens a validated app. Holding OK on Home adds/removes a favourite;
in Apps it opens the native options panel. Choose Pin/Unpin, Move earlier/later
or Open app. Back closes the panel and restores the app; an app removed from
the filtered view lands on a valid neighbour. The panel traps focus and pointer
hit targets until closed. Apps has
All/Favourites filters and native search. G2 uses LG's keyboard for Search;
the separate Keypad button selects Luma's fallback explicitly. C2 0.20.0 still
uses the earlier Luma keypad. Only one keyboard is active at a time. Closing LG's
keyboard ends editing and returns focus to Search without revealing a second
keypad. Back closes the keyboard, returns toward Home, then exits from Home. Favourites persist across
Luma launches. Explicit ordering affects Home and the Favourites filter; All
Apps retains its alphabetical order. Existing records load without a preference
rewrite; order is saved after an explicit change. Older clients cannot read the
new order field, so preserve a pre-ordering backup before considering a downgrade.
Opening another app hides/exits this preview; relaunch Luma from
LG's list. LG Home, power and volume keep their existing ownership.

Interactive sessions now use PlxNative's shared motion/damage presentation gate.
Focus/gesture motion, changed data, clock/status changes and foreground return
request repaint; unchanged screens skip redraws with the framework keepalive
backstop. Input polling and live workers keep running.

After two minutes without app input, Luma presents opaque black and stops
repainting its UI. The first app input wakes it without activating a hidden tile.
This is an app-local measure: physical panel power, burn-in prevention and LG's
own protections have not been validated by a framebuffer capture.

The Inputs page now uses two columns of HDMI cards beside a graphite detail pane. Cable,
signal and selected source are separate readings. The selected source comes from
EIM's independent getter and can persist while another app is foreground. Missing,
stale or unregistered source IDs are unavailable. Video format remains unavailable.
The [Inputs detail record](inputs-detail-milestone.json) records native screens on
both TVs and an earlier restored G2 switch trial. The newer
[HDMI selection record](hdmi-selection-milestone.json) enables source selection
on the G2: focus a source card on Home or Inputs and press OK. LG handles the
source window; relaunch Luma from LG Home afterward. The C2 remains read-only
for source selection while exact idle-state restoration is unproven. Its last
physical preference and LG Home were recovered after the experiment. Source
selection proof does not establish live HDMI video/HDR, CEC behaviour or format. Pointer speed and the G2 audio-description level now have native controls; other adjustments remain
read-only or LG-managed. Controls now shows a persistent six-category column beside the
selected detail panel. Arrows preview categories; Right or OK enters the rows.
Back returns through the category column to Home. Accessibility, General and
Panel Care now show verified current readings, alongside eight more picture-profile
values such as colour depth, sharpness, tint, colour temperature, gamut and gamma.
Bounds/options come from LG descriptors; numeric offsets are not presented as
physical colour temperature or measured picture output. Known read-only values
remain legible; the verified pointer-speed row opens a native choice page. Panel protections stay
LG-managed. The [readout record](control-readouts-milestone.json) documents
ordinary-app proof and independently matched values on both TVs. The cached Picture breadcrumb
uses the TV settings profile context and becomes unavailable when stale.
Scrolling keeps the detail heading fixed, including when reading the final
LG-managed row. Each category now has its own verified LG-managed route on the two tested
firmware/model combinations. Network uses **Open LG Network entry** to focus
Network in LG General; LG handles further navigation and configuration.
Picture, Sound, Accessibility, General and Panel care open their corresponding
LG screens. From **Controls → Picture → Open LG Settings**,
LG's Picture interface opens above Luma. Back through LG's pages closes the overlay and returns
to Luma. Luma keeps only its own underlying window alive behind a verified
Settings overlay and stops painting while covered; LG continues to own its controls.
Other departures retain the preview's existing teardown behaviour. The
[handoff record](settings-handoff-milestone.json) describes native/injected-Back
proof and runtime firmware checks. Pointer speed and the G2 audio-description level each use a separate fixed adapter; no generic settings-write action is available.
The dashboard is still a development
preview; complete controls, physical input, performance and reboot persistence
remain unfinished. The [deployment record](deployment-milestone.json) identifies
the precise source and native proof on the C2 and G2.

The header displays the Luma logo/name and the TV's own calendar, time and timezone label. It reads
LG's system-time service every 30 seconds on a worker, extrapolates briefly with
monotonic time, caches the displayed text and expires unavailable/invalid/stale
readings after 65 seconds. It never changes the TV's time, timezone or panel
settings. The [clock milestone](clock-milestone.json) records the earlier G2
proof; the [branding/status milestone](branding-status-milestone.json) records
the branding checks on both devices. The [app-menu milestone](app-menu-milestone.json)
records two native management runs per TV, exact order reload and the 0.11.0
retained updates. The [Controls layout milestone](controls-layout-milestone.json) records the
latest six-category, pointer, stale-click, scrolling and Back tests plus a G2
Settings-return regression on 0.12.0. These are scripted native tests, not
physical remote/pointer proof.
Sound and Network now show read-only ordinary-app readings. LG Settings handoff is gated by model, firmware, SDK and Settings app UI version.
The [sections milestone](settings-sections-milestone.json) records all twelve
section/model trials, rendered-screen checks and native return. Nested LG
screens took up to three Back presses in the tests. No raw target, item, path or
update flags can be supplied; the payloads come from a fixed route registry. The [fidelity map](design-fidelity.md) tracks the
remaining differences from the concepts. API research used the
[official webOS OSE LS2 reference](https://www.webosose.org/docs/reference/ls2-api/com-webos-service-systemservice/);
commercial-TV compatibility is based on the recorded ordinary-app device tests.

## Audio-description level on G2

Open **Controls → Accessibility → Audio-description level**. This signed
−15…15 level is available on the validated G2 firmware only when audio description
is off. It does not change description enablement, master volume or sound routing.
If the capability is unavailable, the row stays LG-managed.

Left/Right or a track click stages a value. Move Down to **Apply** and press OK;
only verified value/type readback advances **Saved**. Back or Cancel discards a
staged change. Freshness or an external value change closes the editor so it
cannot commit an old selection. Input is blocked while one request is pending.
The [numeric-control record](numeric-control-milestone.json) separates the isolated
native write/restoration proof from the normal identity's no-write readiness check.
C2 numeric edits remain gated; its existing accessibility settings are preserved.

## Pointer speed

Open **Controls → General → Pointer speed**. The checkmark shows the current
setting. Move focus between Slow, Normal and Fast, then press OK to apply.
Back or Cancel returns without changing it. Luma waits for a separate value
readback before showing success; it does not change the checkmark optimistically.
The control is unavailable when firmware, descriptor, foreground/window ownership,
expected old value or freshness cannot be validated. Failure leaves the rest of
LG Settings available. A bounded worker runs the native calls outside the draw
thread, and older background reads cannot rewind a confirmed change.

The [pointer-control record](pointer-control-milestone.json) covers scripted native
focus/press, independent service change/restoration reads and clean removal on
both TVs. This proves the stored setting path, not measured physical Magic Remote
tracking or acceleration. Boot/Home behaviour, OLED protections, network,
firmware and calibration stay LG-managed.

## Package modes

Set `NATIVE_HOME_INTERACTIVE_SESSION=1` when using `package-preview.py` to create
the interactive package. Supply a fresh staging directory, the ARM binary, SDK
and the selected device's private snapshot/icons through the documented
`NATIVE_HOME_*` environment variables. Never commit those private assets or
distribute a household package as a generic release.

Normal interactive packages have no capture markers or scripted navigation,
hold, launch, capability, viewport, numeric-write or pointer-write proof actions. `NATIVE_HOME_SESSION_CAPTURE_PROOF=1` adds
explicit development captures of initial Home, idle black and wake. It must be
used only in a scoped device test and removed afterward. Without interactive
mode, the existing diagnostic package keeps its captures and 30-second lifetime.

Use `NATIVE_HOME_TEST_IDENTITY=1` for a separate `com.ash.nativehome.proof` package,
shown as **Luma Home Test**. It has its own runtime and favourites directory, and
passive tests cannot launch household apps. Named proof markers admit only their explicit owned target or fixed LG route. Keep capture/probe markers out of the retained normal app.

`NATIVE_HOME_DESCRIPTION_LEVEL_PROOF=1` is an isolated backend experiment only.
It cannot be packaged under the normal identity, in interactive mode or alongside
another scripted trial. It requires description to be off, preserves the observed
numeric representation, permits one adjacent step, and restores the original.
Independent readback and complete removal checks are required. That earlier experiment alone added no slider
to the retained app; see the [experiment record](numeric-control-experiment.json)
for the G2 result and C2 preflight skip.

`NATIVE_HOME_RANGE_LAYOUT_PROOF=1` packages the synthetic staged range editor
under the separate test identity. It refuses interactive mode and other action
trials. The host equivalent is `home-preview ASSETS NEW_OUTPUT --range-layout-proof`.
The proof captures five states, checks adjustment/cancel/pointer/proposal rules,
and returns an unconfirmed local reply; it has no TV transport. That proof did not update the retained previews. See the [range editor record](range-editor-milestone.json).

`NATIVE_HOME_DESCRIPTION_UI_PROOF=1` runs the fixed live numeric G2 trial under
the separate identity. It verifies staging/cancel without writes, native Apply,
both signed endpoints and original value/type restoration. Host observers must
independently read every stage and check complete test removal. It cannot be
mixed with another action trial or an interactive package. Keep it out of the
retained normal app.

## Removal and future tests

For each tested TV, the scoped proof removed the package through LG's installer,
then checked its launcher entry, process, jail, runtime and mounts were absent.
An additional guarded cleanup removed only the preview's own unmounted residual
paths and `/media/developer/.com.ash.nativehome.preview-state`. LG's package
removal alone is not proof that the separate favourites directory is gone.
The test compared four checked core-file hashes and picture/audio readbacks,
verified LG Home and restored the original foreground before the final install.

Both final installations are intentionally retained. Future tests must preserve
their saved favourites and installed state; do not use the former test driver's
uninstall/cleanup phase against a user's retained app. Prefer a separate test
identity or a backed-up, verified update procedure. Complete removal must refuse
symlinked roots, live processes, registered packages and descendant mounts
before cleaning any residual directory. Never remove a mounted jail recursively.

For a Settings handoff test, refuse to start if LG Settings is already open.
Observe its overlay independently from before launch, and close only the new
test surface. On this commercial TV, app-manager `close` with an `id` was refused;
surface-manager `closeByAppId` with **`id`** was accepted, while the earlier
`appId` parameter was not. The
[OSE API reference](https://www.webosose.org/docs/reference/ls2-api/com-webos-surfacemanager/)
helped locate the operation; the commercial-TV result and native return are the
compatibility evidence. The client itself relies on LG Back, not this root test cleanup call.
