# Luma Home project rules

Keep Luma Home's application separate from the explicitly attributed PlxNative
framework dependency. Do not add Plex client, playback, authentication or server
features. Retain upstream package names, copyright and licences. Record generic
framework changes in the integration patch and propose them upstream separately.

Use shared UI/focus components. Preserve an OLED-black canvas with graphite
surfaces. Build and verify the standalone dependency graph; host checks do not
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

The user has now placed the C2 on hold for device work. Do not read, test, deploy
or update that TV until the user reopens it. Continue device work on the G2 only.

Do not rewrite public history or change the existing public repository's identity
until the standalone result has been reviewed with the user.
