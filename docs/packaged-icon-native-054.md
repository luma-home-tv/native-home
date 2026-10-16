# Packaged icon rendering on G2 — 0.54

The isolated G2 test verified the ordinary asynchronous packaged-icon reader,
decoder and render-thread cache with three images and one missing-image fixture.
The [milestone](packaged-icon-native-054-milestone.json) binds the actual executable,
package and saved independent audit. Neither retained installation was updated.

Six Home/Apps framebuffers passed independent pixel checks for square, wide and
tall artwork, aspect-preserving placement, transparency on black, labels and the
initials fallback. The captures include each fixture selected; focus-outline
pixels were not independently qualified. Ten standard page captures were also
checked for complete framebuffer format, rather than ten additional pixel cases.

The native terminal reports the packaged worker joined without panic, three
uploads and three deletions, zero remaining packaged textures and zero packaged
backend errors. This covers the three image textures; it does not establish total
graphics memory or deletion of the separate branding/navigation textures.

The original operator timed out before obtaining a second stable terminal read.
A separate continuation read the existing captures and verified their rendering
and shutdown, then hit its final retained-runtime read deadline. A later finisher
encountered an SSH reset during a read-only batch. All three original exits remain
failures. The diagnostic was installed and launched only once.

The continuation also completed the supported app removal and individually
removed its known runtime/state files before its final read timed out. A later
successful scoped closeout verified those paths were absent, removed only the
known staged package and empty staging directory, and closed the coordination
lease and host agent. A subsequent fresh read-only audit checked the complete
retained bundle, preferences and runtime against the original backups: 32, 2 and
110 entries respectively, including all 104 file hashes and recorded metadata.
The 19 fixed getter reply groups and five protected-file hashes matched too. It
also verified test-artifact absence and closed its own new lease and agent.

Independent saved-evidence replay checked the raw pixels, process and namespace
bindings, original SDK outcomes, each recorded operator unlink, and the fresh preservation
readbacks. The coordinator repeated that audit and obtained a byte-identical
report. Preservation is a later sampled readback, rather than an atomic claim
about every instant between observations. Household data and captures stay private.

This result applies to the controlled G2 component. Native coverage beyond 64
apps, rapid scrolling/cancellation, live app-version artwork refresh, physical
input, performance, C2 and normal delivery remain separate checks. Normal Root
artwork stayed off; no artwork broker or HBC session ran. G2 retains 0.47 and C2
retains 0.48. The independently frozen 0.53 scene/Root experiments remain separate.

Luma's new worker/cache builds on PlxNative's explicitly attributed rendering,
focus/input framework and bounded image decoder. This test adds no Plex player,
account or server features.
