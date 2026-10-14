# Interactive preview

The installed app is **Luma Home Preview**, package
`com.ash.nativehome.preview`, version **0.7.0**. It is a normal native app in LG's
Home app list. It does not become the Home-button destination or start at boot.

Open it from LG Home. Directions move between the rail, favourites and app
drawer. OK opens a validated app; holding OK adds/removes a favourite. Apps has
All/Favourites filters and a native on-screen search keyboard. Back closes the
keyboard, returns toward Home, then exits from Home. Favourites persist across
Luma launches. Opening another app hides/exits this preview; relaunch Luma from
LG's list. LG Home, power and volume keep their existing ownership.

After two minutes without app input, Luma presents opaque black and stops
repainting its UI. The first app input wakes it without activating a hidden tile.
This is an app-local measure: physical panel power, burn-in prevention and LG's
own protections have not been validated by a framebuffer capture.

Inputs show status. HDMI switching and LG Settings execution remain gated, and
controls are read-only or LG-managed. The dashboard is still a development
preview; complete controls, physical input, performance and reboot persistence
remain unfinished. The [deployment record](deployment-milestone.json) identifies
the precise source and native proof on the C2 and G2.

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
