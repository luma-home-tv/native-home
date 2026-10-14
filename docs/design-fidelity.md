# Concept fidelity

The [original concepts](concepts/README.md) remain the intended interface. Their
Apple TV-style composition is the visual target: quiet black canvas, graphite
cards, generous spacing and a clear focus cue. PlxNative supplies the attributed
native rendering, UI and input framework. Luma owns its dashboard composition,
branding, preferences and TV adapters; it contains no Plex client features.

Version 0.10.0 moves the running interface closer to those concepts. This is a
progress map, not a claim that the complete concept is implemented.

| Element | Native implementation | Remaining gap |
| --- | --- | --- |
| Identity | Luma icon and wordmark in the persistent header; own SVG rail icons cached as native textures. | Refine secondary page/status text as the information architecture settles. |
| Canvas and focus | OLED-black canvas, graphite surfaces, white card outline, separate quiet current-route fill. | Measure contrast and focus animation with physical remote/pointer use. |
| Home | Four larger favourite cards across the visible shelf, uncropped artwork, names below cards, four HDMI cards below. | View-all shortcuts, explicit LG Home route and long-title/empty-state polish. |
| Apps | Native All/Favourites filters and search keyboard; shared focus, app identities and favourite storage. | Concept management menu and favourite ordering; improve partially visible grid rows and long text. |
| Header and rail status | TV-local clock/calendar; live network availability and sound-output summary. | Complete stale/error-state and long-status visual review. |
| Inputs | Separate cable and signal readings, four source cards. | Reliable active-source/video-format detail pane and independently verified switching. |
| Controls | Native category navigation and read-only picture, sound and network rows. G2 Picture has a verified LG Settings handoff. | Concept category/detail layout, contextual widgets, broader LG-managed routes and C2 handoff proof. |

The sound output, TV volume/mute and sound profile are distinct readings. TV
volume is not a measurement of an external amplifier's volume. Wired and Wi-Fi
connection states remain separate; simultaneous connections do not establish
the active route. Missing, inconsistent or stale values become unavailable.
Settings setters remain absent; a concept slider never authorizes a TV write.

For this milestone, both commercial TVs rendered seven native proof pages on
webOS 24, using the same application binary under ordinary app identities.
Private Home, Apps, Sound and Network frames were inspected. Separate proof
packages were removed and checked for remaining launcher entries, state, paths
and mounts. The retained previews were updated with saved favourites preserved.
See the [branding/status record](branding-status-milestone.json) for scope.
Household frames and detailed device receipts remain private; the public concept
renders contain synthetic content and keep their watermark.

Next design work should close the management-menu and Controls layout gaps,
then review navigation, scrolling and text fit on both devices. HDMI switching,
native setting writes, physical input coverage, reboot persistence and sustained
performance each need separate evidence. LG Home, boot behaviour and core TV
services remain the working default throughout.
