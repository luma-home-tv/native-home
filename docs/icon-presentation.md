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
checks do not measure native framebuffer alpha or physical appearance.

The [0.36 C2 proof](artwork-live-036.md) predates this presentation change. Its
other cards deliberately showed initials because only the owned fixture was
allowed through the live loader and its synthetic snapshot had no app artwork.
That is not a C2-specific icon failure. The proof also exposed missing text in
two refresh captures; complete-view correctness remains open.

The new presentation still needs an isolated, looked-at native capture with
transparent and non-square fixtures before normal promotion. Neither retained
preview has been updated by this change. Production resource startup and
all-app live artwork remain separate work; this style change does not supply
otherwise inaccessible icons.
