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
texture artwork, an opt-in OLED-black surface role and bounded-grid whole-tile
admission shared by painting and pointer stops. These generic changes are
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
package modes and removal. Version 0.19.0 is installed on both the OLED65C26LD C2
and OLED55G26LA G2. The [branding/status milestone](docs/branding-status-milestone.json)
records native branding and live sound/network reads. The
[Controls layout milestone](docs/controls-layout-milestone.json) records the
category/detail composition, native navigation, G2 Settings-return regression
and 0.12.0 updates with favourites preserved. The
[LG-managed sections milestone](docs/settings-sections-milestone.json) adds
verified Picture, Sound, Accessibility, General and OLED Care routes on both
TVs, plus a focused Network entry in LG General. Earlier native idle/wake and owned-app launch
checks are recorded in the [deployment milestone](docs/deployment-milestone.json).

The [Inputs detail milestone](docs/inputs-detail-milestone.json) adds a native
two-column source grid and graphite detail pane, matched to the concept. Both TVs
render the Luma mark/name and independently read source status. A separate G2
trial confirmed HDMI selection and restoration; C2 tuner restoration and live
video-format/playback remain unverified.

The [LG Home return milestone](docs/lg-home-return-milestone.json) adds the
concept's separate rail shortcut. It uses a fixed normal LG app launch, verifies
foreground and preserves the selected source. Isolated ordinary-app proofs on
both TVs checked clean removal, core hashes, picture/audio and saved favourites.

The [expanded control-readout milestone](docs/control-readouts-milestone.json)
adds 20 descriptor-validated settings across Picture, Accessibility, General and
Panel Care. Native values matched independent LG reads on both TVs. Range, enum,
profile and freshness checks protect the readouts; those readings remain LG-managed
unless a separate fixed control adapter has passed its own operation proof.

The [pointer-control milestone](docs/pointer-control-milestone.json) adds the first
editable native setting: Controls → General → Pointer speed. The shared choice
page commits only on OK and confirms through a separate read. Both TVs passed
independent focus-only, temporary change and original-value restoration checks;
physical pointer tracking remains unmeasured.

The [HDMI selection milestone](docs/hdmi-selection-milestone.json) enables all four
G2 source cards after separate ordinary-app selection/readback/restoration trials.
Runtime admission checks firmware, registered inputcommon metadata and uncovered
dashboard ownership. The C2 remains gated: its idle current-source baseline was
not exactly reproduced, although its recorded last physical preference, Home,
checked persistent settings and retained preview were recovered. HDMI video, HDR
and format remain unverified.

The preview remains isolated from LG Home, firmware, jail policy, media services,
network configuration and OLED protections. HDMI switching is enabled on the verified G2 and gated on the C2;
pointer speed has a verified native choice control; other adjustments remain
read-only or LG-managed. The [Settings handoff milestone](docs/settings-handoff-milestone.json)
adds a firmware-verified route to LG's Picture settings and native return after
closing its overlay. Version 0.13.0 validates both TVs and all six section entry points. Private TV details and native household captures
stay outside tracked source. The [responsiveness milestone](docs/responsiveness-milestone.json)
records native SDL input, bounded frame/CPU/RSS measurements and the shared
presentation gate. In the matched G2 workload, sampled foreground CPU fell from
8.2% to 1.7% of one core; physical remote/panel latency and long-run performance
remain unverified. A black OLED canvas with graphite tiles is the
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
input details, broader controls and long text. The original concepts remain
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
