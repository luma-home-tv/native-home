# Backed-up G2 preview rollout

Historical scope: this plan delivered G2 normal Preview 0.26.0. G2 now retains
0.28.0 after the [later backed-up update](apps-scroll-rest-milestone.json). The original
[update record](preview-update-milestone.json) covers the installer rehearsal,
fresh backups, complete installed bundle/state verification, normal keyboard
smoke and cleanup. C2 remains on 0.20.0. The [startup/relaunch](startup-relaunch-milestone.json),
[Sound](sound-read-milestone.json) and earlier input records establish their
listed isolated results. This rollout delivers those changes to the installed
preview while preserving LG Home and the user's saved dashboard state. Full
release qualification remains in the [interactive contract](interactive-keyboard-acceptance.md)
and [delivery plan](parallel-development.md).

## Changed paths and evidence

The candidate is functional source
`7f29884be4c5a0c48794e71110744676c78bb371`, binary SHA256
`d63472f5264f6a2c696b1ef6bdddc2ea811771bfb9174582befd74dad22c71bf`.
The normal package contains the Preview identity and interactive marker, with
no Proof diagnostics, observer, text recipe, captures, services, permissions or
boot hooks. Preparing it does not admit deployment.

Relative to the 0.25.0 G2 [human check](manual-keyboard-milestone.json), keyboard,
drawer, SDL event decoding/runtime, session lifetime, preferences, action adapters,
Cargo lock and attributed framework integration are byte-unchanged. Main-loop
text dispatch and SDK start/stop/drain handling are unchanged too. The additions
are the optional Sound GET/parser/merge, per-row expiry/status and isolated Sound
diagnostics. The background collector has two extra bounded calls; their cost
and the new registry have not received a current matched performance benchmark.

The earlier human result remains evidence for that recorded binary and relevant
unchanged code paths. It is not a new 0.26.0 physical test. Current G2 0.26.0
native evidence covers ordinary reads, focused/scrolled rows, keyboard opening,
network Back, programmatic departure and same-install relaunch. Normal Preview
has existing action capabilities that Proof suppresses, so its own smoke and
preservation checks are required.

## Finite rollout sequence

1. Rehearse the installer version rollback on the isolated Proof identity:
   install 0.26.0, install the known 0.25.0 package without launching either,
   verify terminal statuses and actual installed metadata/binary, then remove
   it normally. Preserve retained/core/settings state. An unlaunched package
   grants no live-PID authority for recursive residual deletion.
2. Under a fresh G2 lease, require quiet LG Home with only the known optional
   LG screensaver, and verify the current Preview version and binary. Save a
   fresh complete bundle/metadata manifest and favourites/order bytes, ownership
   and modes. Verify the existing 0.21.0 rollback package against its expected
   identity and binary. Do not clear state or uninstall Preview.
3. Install the frozen normal 0.26.0 package through the ordinary installer and
   require its terminal result. Read back installed bytes/version and saved
   preference metadata before launch. A changed identity or unexpected state
   stops admission.
4. Bind the actual ordinary Preview process and executable. Check stable native
   entry, saved-state loading and one empty Search → LG keyboard → Back/exit
   flow through the normal UI. Keep proof-only observation gates intact; use
   independent foreground/window/frame evidence where necessary. Restrict test
   actions to the owned dashboard and keyboard; do not launch viewing accounts
   or exercise settings/source writes. Compare retained favourites, relevant
   settings/core/source and ownership/modes after the smoke.
5. On failure, close only the owned Preview and reinstall the verified old
   package without clearing its state. Verify the restored version/binary and
   saved state. Preserve unexpected new preference bytes for inspection rather
   than silently overwriting them. Do not force Home over unrelated use or
   restore settings/source through guessed setters.

The isolated installer rehearsal establishes service downgrade admission, not
every normal-app rollback condition. The real update therefore retains its
verified rollback artifact and fresh state backup throughout. Numeric editing
may legitimately be unavailable under the current accessibility context; it is
not a required smoke action.

## Remaining qualifications

Physical hold/pointer/wheel, later-batch dismissal races, broader composing
languages, repeated-entry reliability, matched performance, full HDMI/media
behaviour and reboot persistence remain explicit work. A successful backed-up
preview update does not complete those requirements or the whole dashboard goal.
C2 promotion requires its own candidate admission and device evidence.
