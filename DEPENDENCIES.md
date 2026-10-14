# Dependency ownership and attribution

PlxNative provides the SDL/OpenGL ES rendering path, text and artwork support,
shared cards/forms, identity-based focus/input machinery and the framework's
native compatibility and verification work. Luma Home provides the dashboard
application that composes and extends those capabilities.

The upstream repository, pinned revision, crate selection and local patch are
recorded in `dependency-lock.json`. Package names and source notices retain
PlxNative attribution. The external checkout is excluded from this repository.
The source of the five framework crates remains available from that pinned
upstream revision plus the included integration patch.

This application does not depend on `plx_plex`, `plx_media`, `plx_session`,
`plx_data`, `plx_appkit`, `plx_net`, `plx_telemetry` or `plxnative-modules`.
The initial proof must inspect Cargo's resolved graph, not only the manifest.

The current `plx_platform` crate contains some legacy application-oriented
storage/catalogue vocabulary. Luma Home does not use it for Plex features.
Further upstream separation of platform identity/localisation and generic
rendering support would reduce this coupling. It must preserve upstream credit
and pass actual builds before adoption.

Inter font files are third-party OFL resources; their licence is supplied in
`resources/fonts/OFL.txt`. The external framework supplies the nanosvg C
rasterizer (zlib), and LGPL/platform library notices remain upstream. No LG SDK
or TV system library is redistributed by this source tree.
