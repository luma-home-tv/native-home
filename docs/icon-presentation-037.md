# Isolated icon presentation (0.37.0)

An isolated G2 run verified native icon presentation with three controlled image
shapes and one missing-artwork fallback. Available artwork appears directly on
OLED black, retaining its aspect ratio, alpha and source shape without an added
graphite plate or forced image rounding. Missing artwork retains the graphite
initials/name fallback. The shared renderer supplies a focus-only outline.

This is a static presentation result. It does not enable all-app live icons,
change either retained preview or qualify production resource startup.

## Shared rendering and isolated recipe

The new `Art::Icon` framework variant contains a caller-owned image with its full
UV range. It paints one ordinary texture quad and, on focus, a hollow outline.
Luma Home supplies that variant for valid application textures; the previous
manual overlay texture and duplicate icon focus ring are removed. Input/status
cards keep their existing presentation. Invalid or missing image dimensions use
the missing-artwork fallback.

This builds on the research and engineering of
[PlxNative](../README.md#plxnative-framework-dependency), by Gleb Linnik and its
contributors. Icon containment and shared painting are explicitly recorded in
the attributed framework integration patch. They are generic framework changes;
Luma Home remains a separate dashboard without Plex client or playback features.

A strict isolated recipe admits exactly four synthetic snapshot entries: square,
wide, tall and missing. The three packaged PNGs measure 128 × 128, 128 × 64 and
64 × 128 pixels. Their patterns include known colour landmarks, transparent
regions and a partial-alpha landmark. Only this recipe suppresses the native
live-catalogue and action workers. It uses fresh own Proof preferences with the
four fixtures as favourites; the retained preview's preferences are not loaded
or modified. There is no companion target installation, root broker, keyboard,
OK/hold action or fixture-app activation. The clock remains read-only.

## G2 result

The native component captured ten standard pages and six controlled icon views:
Home, the resting Apps grid and each of the four focused fixtures. All 16 raw
framebuffers were retained before external classification. The six icon views
passed independent saved-pixel checks for:

- Known pattern landmarks, full aspect and contained image placement.
- OLED-black transparent regions and unused contain margins, without a graphite
  plate behind available artwork.
- The graphite missing-artwork fallback and visible initials.
- Neutral ink in header, clock, footer and fixture-label bands.

Every fixture had complete coverage in all six icon views. Eight unique owned
texture names—branding, four navigation textures and three fixture images—were
reported valid at each stage, with zero checked GL errors. Those GL queries and
card geometry are bound native observations, not independently issued live
queries by the saved-result auditor.

Root also visually reviewed all six captures and confirmed the native Luma logo
and focus outlines. The automated oracle measures ink presence rather than
reading the full text, and does not independently qualify focus-ring pixels.
Neither capture review nor pixel analysis is a physical-panel camera observation.

All 24 positive cleanup/preservation gates passed across 20 selected state
domains and five selected Core hashes. G2 retained 0.31.0 with its exact preview
binary, saved favourites and checked metadata preserved. Proof registration,
launch point, ordinary UID processes, runtime, jail, state, mounts and upload
were verified absent. The device lease and private host agent were removed; LG
Home was current and no reboot was required. Retained final byte/metadata and
some absence checks rely partly on fresh equality/guard receipts from the
executed driver rather than separately saved final files. C2 was not contacted.

The saved-result audit passed 624 assertions. It rechecked the historical
85-input build context and frozen artifact/fixture bindings, six raw process
identity records, all 16 frame sizes and hashes, actual icon pixels, installer
terminal and cleanup evidence. Its result hash is
`def33911b6dd0669e11a1b9e7a7742f4f9e2f945b41389f2e83610414877fbe6`.
The [scoped milestone record](icon-presentation-037-milestone.json) keeps the
artifact and qualification boundaries explicit. Raw captures and household
state remain private.

## Correction to the earlier capture note

The earlier suggestion that text was missing from two
[0.36 captures](artwork-live-036.md) was incorrect. A separate standard-library
raw RGB comparison found changes confined to the synthetic fixture image; the
surrounding pixels were unchanged. This corrects the earlier visual observation,
not the original frame bytes. The correction receipt hash is
`6aaaff726e600d8709a011d514b13b57f3383c55a1d3a670252ed5256a6cf23c`.
There is no established renderer defect from that superseded observation.

## Validation and remaining scope

Source `24aee07b53036e5f7201eda7bc46ecdaafb37470` passed 339 library tests,
five dashboard binary tests, clippy with warnings denied, 27 combined
build/provenance and framework checks, 15 oracle tests and 17 carrier tests.
Forty-nine actual package cases passed; two optional control cases were skipped.
The ARM build receipt binds 85 selected inputs and the pinned framework to the
actual dashboard binary.

The result qualifies the fixed static recipe on G2. Dynamic texture refresh has
its separate [0.36 lifecycle evidence](artwork-live-036.md). This run establishes
neither access to every installed app's icon nor a production root-resource
launcher, retained rollout, physical input, sustained performance, idle power or
full release readiness. Broader icon integration and backed-up normal delivery
remain separate work.
