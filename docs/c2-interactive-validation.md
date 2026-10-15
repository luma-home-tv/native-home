# C2 interactive validation lane

Four isolated C2 **0.24.0 actual-main-loop profiles passed**: keyboard/fallback,
programmatic departure, idle, and idle begun with LG editing active. They used
source `81213337bee7dde2d4808f058c9973957d56ab12`, binary SHA256
`17d975d2f156a1d87e31e41f2938b182342b1da8452ec7bc51092dfd7685fc86`.
The retained C2 preview remains **0.20.0**. These are separately bounded
profiles; they do not complete the
[interactive keyboard acceptance contract](interactive-keyboard-acceptance.md).
The earlier [0.23.0 component trial](keyboard-integration-milestone.json)
retains its original scope.

## Keyboard/fallback v2

The proof used the ordinary interactive loop, SDL dispatcher, live workers,
keyboard reconciliation, presentation gate, persistence and normal exit path.
Source and package gates excluded the direct component/action helpers. The
external harness checked the live ordinary-UID producer and its actual running
executable SHA against the immutable candidate. Observer source/binary and
execution labels remain constructor/package declarations; the independent
binding and reviewed hooks establish their provenance.

All 31 scenario checks passed:

- Ordinary remote navigation reached Home → Apps → Search. Two SDK sessions
  opened, stopped and dismissed, with zero keyboard failures. Independent LG
  window observations confirmed both appearances and absences. Closed states
  had no requested/owned session, SDK text activity or SDK panel flag; Search
  was focused and fallback targets remained absent.
- The explicitly enabled native recipe queued `lu`, then `ma`, after two real
  Search openings. The approved draft checks observed `lu` → `luma`. Both LG
  panel closes used guarded network Back. This is separate from physical
  Magic Remote typing and LG Done.
- Explicit Keypad selection exercised the fixed native grid through A,
  Delete, Clear, Done and Back. Approved draft checks observed `lumaA` →
  `luma` → empty. Effective fallback targets were present during fallback;
  system-session opens remained at two.
- The observer recorded 63 received adapter events, including two text commits,
  all with origin `native_unclassified`. Two recipe enqueue acknowledgements
  were counted separately. The bounded change log contained 76 records, with
  no overflow. There were 26 real live-read cycles and zero external-action
  attempts. One initial proof-preference bootstrap write preceded the tested
  stages; those stages made no additional preference writes.
- Ordinary Back exited with a final observer summary and storage receipt.
  Editing, requested/owned keyboard state and SDK text input were closed.
  Proof favourites were unchanged. No hidden submissions were recorded in
  this profile; window-departure behaviour was not exercised.

Independent cleanup confirmed the proof process and UID workers, package,
catalogue entry, launchpoint, runtime, persistent state, jail and mounts were
absent. Retained preview/favourites bytes and file/directory ownership/modes,
protected core hashes, source/last input, picture/audio/audio-description
values and types, and the 20 expanded settings were unchanged. LG Home returned
automatically; teardown issued no baseline Home launch or setting/source
recovery write. The device lease was released.

The first wrapper stopped on the host before any TV call because its agent
socket path exceeded the macOS length limit. The v2 wrapper used a short private
agent directory. Its immediate shutdown check raced asynchronous agent exit;
a later independent check confirmed the process/socket absent, then removed
only the empty owned directory. No stale socket was unlinked. Raw receipts,
backups and timing records remain private. Screenshots were disabled.

## Observer-only programmatic departure

A separate `window_departure` profile passed on the same source/binary with the
recipe disabled and a separately hashed immutable package. All 18 profile checks
passed. An active, independently visible LG keyboard preceded one guarded normal
LG Home application launch. Independent foreground/window observations confirmed
Home and keyboard absence. The app finished through `window_hidden` with one
open/stop, zero failures, and editing/requested/owned/SDK activity/panel flags
closed. Draw and swap counts stayed unchanged after the first native hidden
state; hidden submissions remained zero. The final native summary followed that
state by 506 ms. This interval describes the recorded native lifecycle, not
physical Home-key or panel latency.

No recipe events were queued. The bounded log held 22 records without overflow;
seven live-read cycles completed and external-action attempts remained zero.
Full removal, typed state/metadata/hash preservation, automatic Home return,
lease release and delayed agent-directory closeout passed independently again.
The retained preview remains unchanged. This result covers programmatic
departure; physical Home-key handling and same-install clean relaunch remain
unverified.

## Real idle and wake

The observer-only, recipe-off `idle` profile passed 17 harness checks plus an
independent fresh-hold analysis. The app crossed its real 120-second idle
boundary. Native elapsed/write timestamps advanced 3,010 ms across the short
hold, controller time advanced 3,000 ms and 60 poll batches advanced with no
new input events. Draws remained 144, swaps 145 and the black-idle presentation
count one; live-read cycles advanced as well. Equal counters were therefore
corroborated by a progressing producer.

A separate recipe-off `idle_with_keyboard` profile passed 21 harness checks.
Its corresponding fresh hold advanced 3,008 ms in native elapsed/write time,
3,000 ms in controller time and 60 poll batches, again with no input events,
unchanged draws/swaps and one idle presentation. In the bounded change log, the
state immediately preceding the first idle transition retained editing,
requested/owned keyboard state and active/panel SDK flags; the first idle state
had those closed. This is an adjacent observed-state result. It does not prove
the exact SDK closure cause or continuous compositor keyboard visibility.

Both runs woke on network LEFT, retained the same generic focus role and showed
no observed activation; subsequent deliberate navigation reached Apps and
ordinary Back exited. These observations do not establish intentional OK
suppression or exact focused-card identity. The short holds establish neither
a 60-second performance workload nor CPU/RSS, GPU completion or physical pixel
behaviour. Both profiles independently passed complete removal, typed state and
metadata/hash preservation, automatic Home return, lease release and delayed
agent cleanup. No retained-preview update or setting/source recovery write
occurred.

## Next independent C2 checks

Use the same immutable artifact or a newly frozen candidate with its exact
source, binary, package, adapter and host-module hashes. Keep a separate lease,
run label and preservation/cleanup receipt for each bounded profile.

1. Extend wake validation to intentional OK and exact focus/activation evidence.
   The observed LEFT wake and generic focus role above retain their narrower
   scope; the active-before-idle result remains an adjacent-state observation.
2. Follow the observed programmatic departure with physical Home-key handling
   and a subsequent same-install clean relaunch. The programmatic profile above
   establishes its scoped stop/hidden-exit path; it does not satisfy these
   remaining input/relaunch requirements.
3. Obtain physical C2 typing and LG Done/Back observations. Qualify prediction,
   hardware keyboard, preedit/caret and composing-language semantics at their
   actual scope.
4. Observe native dismissal-drain ordering inside the same SDL batch and across
   later batches. This trial recorded zero drain events, so settled navigation
   cannot satisfy that requirement.
5. Run the acceptance contract's matched performance workloads after lifecycle
   acceptance. Submission counters from this trial are not panel FPS, GPU
   completion, physical latency or a CPU/RSS comparison.

Preserve retained backups and typed baselines before every run. Refuse unrelated
foreground use or old proof artifacts. If external activity changes foreground
or settings, close/remove only the proof, preserve the new context and report
incomplete restoration; do not force LG Home or overwrite retained state.
Writes to protected settings or accessibility aids, firmware changes, Home/boot
changes, service interventions and OLED actions remain outside these trials. A retained-preview
update requires its own backed-up promotion procedure after acceptance.
