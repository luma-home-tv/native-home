# App icon presentation

Available app artwork sits directly on the OLED-black canvas. Preserve the
source aspect ratio, alpha and corner shape; do not add a graphite backing
plate, crop it into the card aspect or force rounded corners. Selection gets a
hollow outline around the contained artwork. Names stay below the icons.
Only missing or invalid artwork uses the graphite initials/name fallback.
HDMI status and control surfaces keep their existing presentation.

The attributed PlxNative integration patch adds a reusable `Art::Icon` mode.
Luma's shared card adapter uses it for valid app textures, replacing the former
empty graphite card plus manually overlaid, inset icon. The original
`Art::Texture` and media artwork paths retain their behaviour. No Plex client
features are included.

Host verification passes 336 application tests, five dashboard tests, ten focused
framework artwork tests, warnings-denied Clippy and 23 provenance/framework
verifier tests. The artwork tests check square/wide/tall containment, invalid
geometry, stable content identity, no server artwork requests, and recorded draw
commands: one plain texture at rest, with one hollow outline on focus. These
checks do not measure native framebuffer alpha or physical appearance. The ARM
release build also passes for source `6c75306b749f3fa1261123ad224f92228bf73498`;
its ELF SHA-256 is
`636d10cbc4ad5aef7026fc6de2fd865f9c2a8202fc80c113a12695cf6ad7183a`.
Four build-entrypoint checks also pass.

The [0.36 C2 proof](artwork-live-036.md) predates this presentation change. Its
other cards deliberately showed initials because only the owned fixture was
allowed through the live loader and its synthetic snapshot had no app artwork.
That is not a C2-specific icon failure. The initial missing-text report was
incorrect: a [direct RGB recheck](artwork-live-036.md#correction-to-the-initial-visual-reading)
confirms every pixel outside the updated fixture stayed unchanged.

The [isolated 0.37 G2 capture](icon-presentation-037.md) now qualifies synthetic
transparent and non-square fixtures with looked-at native frames and clean
removal. The subsequent [normal G2 delivery](retained-preview-037.md) installs
this styling with real packaged artwork, preserves favourites and verifies two
normal sessions. C2 is unchanged. Production resource startup and all-app live
artwork remain separate work; this style change does not supply otherwise
inaccessible icons.


## Isolated 0.37 capture candidate

A separate fixed presentation recipe now keeps the real native Window, shared
Shell, fonts and GL renderer, while suppressing native catalogue and action
workers only for this diagnostic. It requires an ordinary non-interactive Proof
identity, refuses other trial markers, validates four fixed synthetic app
identities and packaged paths, and refuses previously loaded Proof preferences.
The synthetic favourites use only that Proof identity's fresh persistence path.
Normal catalogue and action behaviour is unchanged.

The candidate emits the usual ten page captures plus six presentation captures:
Home with all four favourites, Apps at rest, and Apps focused on each of the
square, wide, tall and missing-art fixtures. It records actual shared drawn
card geometry, resource validity and read-only GL program/unit/binding state.
Input or departure interrupts the test; it never sends OK or activates a fixture.
The host checks cover the fixed recipe/snapshot refusals and non-mutating
placement receipts with integer pixel dimensions. Packaged snapshots, icons,
branding and fonts are explicitly readable by the ordinary app even when their
host source files are private.

The subsequent [0.37 result](icon-presentation-037.md) records the matching
frozen ARM package, independent pixels, looked-at frames and cleanup evidence.
Its static fixtures do not qualify production live artwork or catalogue work.
Normal Home/Apps and a bounded idle/relaunch now have their
[separate retained evidence](retained-preview-037.md). Exhaustive icon shapes,
physical-panel appearance and production artwork startup remain separate work.
