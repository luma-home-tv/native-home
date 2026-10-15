# Luma Home project rules

Keep Luma Home's application separate from the explicitly attributed PlxNative
framework dependency. Do not add Plex client, playback, authentication or server
features. Retain upstream package names, copyright and licences. Record generic
framework changes in the integration patch and propose them upstream separately.

Use shared UI/focus components. Preserve an OLED-black canvas. Available app icons keep their own shape and
transparency without a graphite backing plate; missing-art fallbacks and controls
may use graphite surfaces. Build and verify the standalone dependency graph; host checks do not
prove TV rendering, physical input or performance.

TV details, credentials, actual app inventories/icons and household captures
must stay outside tracked source. Device work requires the existing device lock,
an isolated package and complete removal/state restoration. LG Home, boot hooks,
firmware, system libraries, jail policy, media services, networking and OLED
protections must remain untouched. Protected/unverified controls remain read-only
or LG-managed. Never exercise personal viewing-account writes in tests.

The interactive preview is now intentionally retained on both authorised TVs.
Preserve its saved favourites and installed state. Do not apply the former
diagnostic driver's unconditional uninstall/cleanup to a retained installation;
use a separate test identity or a backed-up, verified update procedure.

The latest verified retained G2 installation is 0.37.0, with dashboard ELF
SHA-256 `3310d1a5c0e7d7910dc2052673e6f508efaa25b9e83f2602204c5f72e5ae2c21`.
Its two normal sessions, full bundle/state preservation and temporary cleanup
are recorded in `docs/retained-preview-037.md`. Historical drivers pinned to
0.31.0 must remain historical; prepare a fresh guarded candidate for later tests
and preserve the current installation and favourites.

The G2 is the primary device. The user explicitly reopened isolated C2 testing on
2026-10-15 until 20:30 Europe/London (19:30 UTC). Within that dated window the
coordinator may run isolated tests under the existing C2 lease, preserving its
retained preview and favourites. Do not start a C2 test after 20:15 local time;
finish cleanup before the user's 20:30 cutoff. After that window, do not contact
or modify C2 without a later explicit instruction. Earlier C2 records remain
historical evidence; G2 results never establish C2 behaviour.

Do not rewrite public history or change the existing public repository's identity
until the standalone result has been reviewed with the user.
