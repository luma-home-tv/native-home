# Luma Home

<img src="assets/luma-home/banner.svg" alt="Luma Home — Native TV dashboard" width="800">

An experimental native home dashboard for LG webOS TVs. This standalone project
owns the dashboard's app catalogue, favourites, navigation composition, TV
adapters, controls, persistence, branding and tests.

This is the independent [Luma Home repository](https://github.com/luma-home-tv/native-home).
The earlier combined prototype is preserved in
[native-home-prototype-history](https://github.com/luma-home-tv/native-home-prototype-history)
as an attributed PlxNative fork. It is historical reference; development continues here.

## PlxNative framework dependency

Luma Home builds on the research and engineering of
[PlxNative](https://github.com/GLinnik21/plx-native), by Gleb Linnik and its
contributors. Their rendering/UI/input framework remains explicitly named and
attributed as PlxNative. It is fetched into an ignored external checkout, pinned
by [dependency-lock.json](dependency-lock.json), rather than copied into Luma
Home's tracked application source.

The application consumes `plx_ui`, `plx_gfx`, `plx_machine`, `plx_base` and
`plx_platform`. It does not depend on PlxNative's Plex client, player, media,
session, data or application-screen crates. A small, separately identified
[integration patch](patches/plxnative-framework-integration.patch) adds local
texture artwork and an opt-in OLED-black surface role. These generic changes are
candidates for upstream contribution; their presence does not turn the upstream
framework into Luma Home-authored code.

The current upstream build scripts also need their SVG rasterizer, vector-icon
assets, locale catalogue and install-identity generator. The external checkout
keeps that support material intact. Its full repository cache contains upstream
application files, but those files are neither tracked here nor application
dependencies. A future upstream framework release could remove this checkout
and build-script coupling.

## Layout

| Path | Ownership and purpose |
| --- | --- |
| `src/` | Luma Home application, models, adapters and tests |
| `docs/`, `design.json` | Plans, synthetic concepts and scoped proof records |
| `assets/luma-home/` | Independent imagegen-derived branding and vector masters |
| `resources/fonts/` | Third-party OFL Inter font resources and licence |
| `patches/` | Explicit modifications to the externally attributed framework |
| `.dependencies/plx-native/` | Ignored upstream dependency checkout |

## Development

Run `python3 prepare-framework.py`, then `python3 build-client.py test`.
`build-client.py host --bin home-preview` builds the desktop renderer.
The ARM path uses the LG SDK through `NATIVE_HOME_WEBOS_SDK` and a separate
`NATIVE_HOME_BUILD_DIR`; it builds only the dashboard binaries. Packaging includes
the Luma client, fonts, its own icon and explicitly supplied development assets.
No Plex application package, server sign-in or playback feature is built.

`NATIVE_HOME_INTERACTIVE_SESSION=1` selects the installable interactive preview
when packaging `home-preview`. It opens directly on Home, runs while foreground,
and exits on Back from Home or after losing foreground. The ordinary proof build
keeps its scripted capture sequence and 30-second bound. Interactive packages
reject scripted action probes. Their canvas blanks to opaque black after two
minutes without app input; the first input wakes it without activating a hidden
card. This is app-local idle behaviour, not a replacement for LG's OLED protections.
Normal packages make no household screenshots; session capture requires a
separate, explicit development proof flag. Host interactive mode uses `--session`.
The [interactive preview guide](docs/interactive-preview.md) describes navigation,
package modes and removal. Version 0.11.0 is installed on both the OLED65C26LD C2
and OLED55G26LA G2. The [branding/status milestone](docs/branding-status-milestone.json)
records native branding, live sound/network reads, isolated removal and retained
updates with favourites preserved. Earlier native idle/wake and owned-app launch
checks are recorded in the [deployment milestone](docs/deployment-milestone.json).

The preview remains isolated from LG Home, firmware, jail policy, media services,
network configuration and OLED protections. HDMI switching remains gated and
native settings remain read-only. The [Settings handoff milestone](docs/settings-handoff-milestone.json)
adds a firmware-verified route to LG's Picture settings and native return after
closing its overlay on the G2; the C2 handoff is still gated pending separate proof. Private TV details and native household captures
stay outside tracked source. A black OLED canvas with graphite tiles is the
design direction; sustained performance, physical input and complete controls
still require device verification. The [drawer milestone](docs/drawer-milestone.json)
adds native All/Favourites filtering and on-screen search, tested through
scripted shared focus/OK on the G2. Keyboard input is currently Latin/digits.
The [app-menu milestone](docs/app-menu-milestone.json) adds native app options,
pinning and durable favourite ordering on both devices; full text entry remains ongoing. The
[clock milestone](docs/clock-milestone.json) adds a cached native TV-time header,
validated and installed as version 0.8.0 on the G2 with saved favourites preserved.
The [concept fidelity map](docs/design-fidelity.md) records the implemented Luma
logo/rail, larger Home shelves and status readings alongside the remaining
Controls layout, input details and long text. The original concepts remain
the target for the finished interface.

Historical [milestone records](docs/implementation-plan.md) describe the earlier
combined prototype. The [standalone device milestone](docs/standalone-device-milestone.json) now
records native G2 capture, live reads, injected Back and clean removal on
`88a037e`. Physical input, complete controls and performance remain unverified.

The [standalone boundary record](docs/standalone-boundary.json) records 35 passed
client tests, Clippy, host/ARM builds, the resolved dependency graph and the
absence of Plex application/media crates. The Git-tracked tree contains no
framework source or Plex application tree.

## Licensing

Luma Home's derivative application is GPL-3.0-or-later. PlxNative remains
copyright Gleb Linnik and contributors under its own GPL-3.0-or-later grant.
Preserve `LICENSE`, the upstream notices and third-party licences. See
[DEPENDENCIES.md](DEPENDENCIES.md) for the attribution boundary.
