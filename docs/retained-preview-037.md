# Retained G2 Preview 0.37.0

G2 now retains Luma Home Preview 0.37.0. Available packaged application icons
render directly on OLED black with their aspect ratio, alpha and source shape
preserved, without an added graphite plate. Missing artwork keeps the
initials/name fallback. This delivers the shared icon presentation tested in the
[isolated 0.37 result](icon-presentation-037.md); it does not enable dynamic
all-app artwork or a production root-resource launcher. LG Home remains intact.
C2 was not contacted.

## Backed-up update

The normal package uses the same ARM executable as the isolated result, built
from source `24aee07b53036e5f7201eda7bc46ecdaafb37470`. Its receipt binds 85
selected inputs and the pinned framework to the actual binary. Historical
source inputs were replayed from the committed revision; this is not a
hermetic-build or deterministic-reproduction claim.

The old 0.31 and new 0.37 normal bundles each have 32 entries and 30 files. Only
the executable and app-version metadata change. All 28 resource files remain
byte-identical, including the real normal snapshot, 22 packaged icons, fonts,
branding and interactive marker; no synthetic presentation assets were deployed.
File modes are unchanged. No proof, observer, live-artwork, broker or icon-recipe
marker is included in the normal package.

Before installation, the full installed 0.31 bundle and complete saved-state tree
were backed up with bytes, owners, groups and modes. The app was updated without
uninstalling or resetting favourites. The upgraded bundle, both completed
sessions and final bundle/state were checked against their exact expectations.
The actual 0.31 package remains the rollback artifact. Rollback was not needed,
so this run does not qualify execution of a 0.37-to-0.31 downgrade.

## Two normal sessions

The first ordinary session exercised Home and Apps, including the unboxed icon
presentation, then Search and LG's native keyboard. At the checked keyboard
view, one LG keyboard was visible without a fallback keypad. Back closed it;
the terminal recorded one open, stop and dismissal, with no failures. No text
was injected, and this is not a physical Magic Remote typing result.

Root visually matched five protected General values against independent LG
GETs. The read-only Inputs view distinguished the selected source from a
different focused card. No input row was activated and the selected/last-input
state remained unchanged. General review used a local checkpoint that sent no
TV input; Inputs review was bound to its private compositor frame.

A 125-second local hold sent no input while sampling the same verified process
and sole Preview card in short intervals. The captured idle compositor image
had a dark-pixel fraction of 100% and mean RGB channel value zero. One fixed
arrow woke the canvas; the first session recorded exactly one idle presentation
and one wake. Back then exited naturally to LG Home. These observations do not
measure draw cessation, CPU use, GPU inactivity, panel power or OLED lifetime.

The same already-installed app was launched once more from quiet LG Home. A new
PID and process-start identity retained the same ordinary UID and executable.
Home → Apps → Home → Back completed with no keyboard opens, idle presentations
or wakes. The saved preferences stayed unchanged and LG Home returned naturally
again. Two successful sessions are a bounded relaunch observation, not general
cold-start, reboot or startup-reliability qualification.

## Verification and evidence limits

The run retained 41 private compositor frames and 39 fixed network-key
acknowledgements, plus the local General checkpoint and natural-idle hold. Root
reviewed meaningful visual checkpoints, including the icon presentation,
keyboard, General, Inputs and wake; not every one of the 41 frames was visually
inspected. There is no full normal-view pixel oracle or OCR claim. The isolated
static pixel checks remain separate evidence.

Twelve raw process records bind the two ordinary sessions' UID fields, PIDs,
start identities, executable hashes and jail paths. Full bundle/state backups
rehash correctly, with favourites and checked metadata preserved. All 20 typed
TV-state domains and five selected Core hashes are unchanged. There were no
setting writes or household app launches. Owned temporary uploads and
compositor files, the device lease and private host agent were removed with no
closeout errors. Normal runtime receipt directories were preserved; the
retained app was not removed. Both session processes were checked absent.

The saved-result audit passed 1,886 assertions. Its result hash is
`997e1d7ea7de536b4dcf0eebe7ad3bdaee7a38845e13d7920be1e87829323063`.
Eighteen new host regressions cover the update guards. The
[delivery record](retained-preview-037-milestone.json) binds the artifacts and
scope. Process/window continuity is sampled; fixed key payload construction,
some absence observations and guarded temporary-file deletion are supported by
the pinned executed carrier and its receipts. No live requery is performed by
the saved-result audit. Raw values, inventories, household frames and backups
remain private.

Rendering and input build on the research and engineering of
[PlxNative](../README.md#plxnative-framework-dependency), by Gleb Linnik and its
contributors. Shared icon containment/painting is an explicitly attributed
[integration patch](../patches/plxnative-framework-integration.patch), separate
from Luma Home's dashboard code and without Plex client features.

Normal live-artwork runtime remains disabled. The delivered package still
supplies 22 static icons; artwork for every installed app, protected-resource
startup and dynamic refresh in the normal app remain separate work. Physical
input/IME, comprehensive HDMI/media/DRM checks and current performance or idle
power are also outside this delivery's qualification. C2's dated testing window
has closed; this G2 result does not change C2 or extend access to it.
