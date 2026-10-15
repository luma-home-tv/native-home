# Interactive preview

The installed app is **Luma Home Preview**, package
`com.ash.nativehome.preview`, version **0.14.0 on both the G2 and C2**.
Both devices are authorised for development again. It is a normal native app in LG's
Home app list. It does not become the Home-button destination or start at boot.

Open it from LG Home. Directions move between the rail, favourites and app
drawer. OK opens a validated app. Holding OK on Home adds/removes a favourite;
in Apps it opens the native options panel. Choose Pin/Unpin, Move earlier/later
or Open app. Back closes the panel and restores the app; an app removed from
the filtered view lands on a valid neighbour. The panel traps focus and pointer
hit targets until closed. Apps has
All/Favourites filters and a native on-screen search keyboard. Back closes the
keyboard, returns toward Home, then exits from Home. Favourites persist across
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

Inputs show status. HDMI switching remains gated, and native controls are read-only
or LG-managed. Controls now shows a persistent six-category column beside the
selected detail panel. Arrows preview categories; Right or OK enters the rows.
Back returns through the category column to Home. The cached Picture breadcrumb
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
proof and runtime firmware checks. No native setting-write action is available.
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

## Package modes

Set `NATIVE_HOME_INTERACTIVE_SESSION=1` when using `package-preview.py` to create
the interactive package. Supply a fresh staging directory, the ARM binary, SDK
and the selected device's private snapshot/icons through the documented
`NATIVE_HOME_*` environment variables. Never commit those private assets or
distribute a household package as a generic release.

Normal interactive packages have no capture markers and no scripted navigation,
hold, launch or capability probes. `NATIVE_HOME_SESSION_CAPTURE_PROOF=1` adds
explicit development captures of initial Home, idle black and wake. It must be
used only in a scoped device test and removed afterward. Without interactive
mode, the existing diagnostic package keeps its captures and 30-second lifetime.

Use `NATIVE_HOME_TEST_IDENTITY=1` for a separate `com.ash.nativehome.proof` package,
shown as **Luma Home Test**. It has its own runtime and favourites directory, and
passive tests cannot launch household apps. An explicit owned-target launch proof
is the only exception. Keep capture/probe markers out of the retained normal app.

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
