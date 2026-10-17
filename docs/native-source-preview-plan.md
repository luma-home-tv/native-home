# Native input previews: feasibility and intended interface

The intended Inputs view can show one live preview for the focused source, with
device icons and connection status for the remaining sources. Keep the surrounding
canvas OLED-black and preserve the existing idle-to-black behavior. This is a
design target; native HDMI or broadcast embedding is not implemented or qualified.
Previewing each supported source in turn and displaying several simultaneously
are separate capability questions.

## Existing engineering to build upon

The attributed PlxNative framework contains useful composition research:
`src/starfish.c` dynamically resolves `SDL_webOSCreateExportedWindow`,
`SDL_webOSSetExportedWindow` and `SDL_webOSDestroyExportedWindow`; its platform
window and sink interfaces describe placement, opaque regions and plane lifetime.
These references are present in the pinned framework source. They establish a
place to investigate native composition, not an HDMI/tuner adapter or permission
on webOS24.

Luma currently links the framework's UI, graphics, base, platform and machine
crates. It does not link `plx_media` or import the Plex player, authentication or
server features. Existing input actions hand off to LG input applications.
Any reusable generic adapter should retain attribution and licences, remain
separate from the dashboard application, and be proposed upstream separately.

The [webOS Homebrew web-app guide](https://www.webosbrew.org/develop/guides/web-app/#inputtv-embedding)
documents external-input and broadcast embedding for web apps, with an apparent
single-external-input limit that may depend on hardware. That is a useful lead;
it does not establish the native API, caller permissions or behavior on our target
software. It does not change Luma's native-renderer requirement.

## Work required before product integration

1. Identify the native external-source routing ABI and its ordinary-app permission
   model from headers, source and read-only evidence. An exported window alone is
   insufficient.
2. Design a tiny, separately identified native adapter and a bounded lifecycle for
   one source. Specify exact preservation and release checks before a device test.
3. In an isolated test, establish actual composition and reliable teardown while
   preserving the selected input, viewing app, audio, HDR/HDCP handling and OLED
   protections. Do not alter system media services or use a full player dependency
   merely to obtain a video rectangle.
4. Verify source changes, no-signal/error states, foreground departure, idle and
   normal removal before integrating the adapter into Inputs.
5. Investigate simultaneous sources only after single-source behavior is proven.
   Plane count, arbitration and hardware routing may impose a limit.

No runtime media call, TV change or live-preview qualification follows from this
source review. Input switching, native embedding and simultaneous previews retain
separate acceptance evidence.
