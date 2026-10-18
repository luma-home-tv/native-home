# Luma Home implementation plan

The goal has resumed from the [retained deployment checkpoint](retained-preview-pause-checkpoint.md).
Both TVs retain normal0.61.0 from functional source
`d2215d5d8761fe385ae7ae3dd76ab173793c5bb8`, dashboard SHA-256
`3cefc9152fa0b0e051920763496274f211f24914085c83eed01d36b27812ecb3`.
That installed identity supersedes older f5/G2 and0.48/C2 baselines. The updates
verified complete installed payloads and preference preservation; they did not
launch or functionally test the new retained installations.

The subsequent [current retained G2 smoke](retained-current-g2-smoke.md) now
completes one no-install Home/Apps/Home/natural-Back session with original
operator 0 and verified preservation/closeout. Independent saved-data audit is
now clear with a byte-identical coordinator replay. This completes the bounded
gate-1 scope; it adds no current physical-input or C2 rendering result.

The matching earlier [ordinary component](current-ordinary-g2-native.md),
[navigation observation](passive-performance-observer-g2-native.md) and separate
[idle/wake observation](passive-performance-observer-g2-idle-wake.md) pass their
finite G2 scopes and independent saved replays. They supply useful evidence,
not current physical input, combined full-product or C2 runtime qualification.

## Current delivery sequence

The [combined Home beta candidate](home-beta-integration.md) now brings the
reviewed Home menu, focus restoration, Controls guides and offline icon tooling
into one 146-input tree. Its actual host checks, ARM build and separate local
artifact/direct-ABI review with coordinator replay pass. It is not deployed.
C2 is the current native-test target; human checks need fresh availability.

Current source work also addresses a concrete Home interaction gap: the
[shared app-options candidate](home-app-options-candidate.md) replaces Home's
immediate long-press unpin with the Apps drawer's explicit options menu. Source
review, targeted host checks and an actual ARM build pass; it remains undeployed.

The first separate headless icon-reader attempt
[stopped at initial SSH connectivity](public-icon-reader-first-connectivity-attempt.md)
before acquiring a device lease or executing anything on the TV. Preserve its
exit 1/HOLD. No native reader result follows and the closed scope must not be
restarted. Continue source work while the TV is unreachable.

| Gate | Concrete result required | Work owner and boundary |
| --- | --- | --- |
| 1. Current retained G2 smoke | Fresh actual3ce full bundle/state/cache/runtime baseline; normal entry, Home/Apps/Home and natural exit; preserved saved favourites and known closeout | Coordinator only, new lease and bounded reviewed scope. No old f5 rollback carrier. |
| 2. Favourites across restart | Disposable interactive Proof seeds its own store, one eligible unseeded app is pinned, natural exit, fresh second process loads exactly that committed state, then complete own removal | Carrier author plus independent package reviewer; coordinator alone runs it. Household Preview preferences stay unchanged. Real eligibility must come from fresh apps/launchpoint replies. |
| 3. Physical input and one keyboard | Current-build pointer click, wheel and hold/menu; separate typing/Done/reopen/Back with one LG keyboard and no unintended post-dismissal activation | Human window only when the test is ready. The favourites matcher requires zero keyboard cycles, so typing has a separate grade/session. SDK keys do not stand in for physical input. |
| 4. Normal daily handoffs | Current ordinary Preview opens an owned test app and returns, and completes one LG Settings return without changing a setting | Separate normal-capability scope; Proof launch suppression cannot prove this. No viewing-account writes. |
| 5. Usable beta and remaining coverage | Repeated ordinary entry/exit and honest Controls/Inputs coverage; identify artwork refresh/fallback behavior for changed apps | Coordinator integrates only qualified changes. Unverified controls stay read-only or LG-managed. Full native control parity and broad reliability remain unfinished. |

Gate1 is complete in its bounded G2 scope. The critical path now starts with
the fresh C2 preservation baseline, then gates2–4 on the current executable.
The existing Proof artifact has passed independent archive, profile and build-provenance review,
including five adverse checks and a coordinator input replay. This is local
artifact evidence; native execution and the two-session workflow remain open. It uses a synthetic Preview-only seed and a separate owning identity.
A fresh eligible unseeded catalogue ID can avoid an extra Target install for the
Pin-only operation; an owned Target requires its own artifact and cleanup proof.
No app/account action is needed for a local favourite write.

Parallel source work fixes and reviews optional bounded draw diagnostics. It
stays off the TVs and cannot delay the daily-use gates. The earlier partial
observer attempts and their coordination/timing limits remain preserved. Cache
maintenance [planning](https://github.com/luma-home-tv/native-home/pull/6) and
[same-cache worker ownership](https://github.com/luma-home-tv/native-home/pull/7)
are source foundations, with no mutation caller or native maintenance proof.
Automatic retirement/eviction and new diagnostic schema integration are later
work, not prerequisites for the first fixed-catalogue beta.

## Remaining product work

Passing the daily-use tests does not fill the following implementation gaps:

1. **Return and Home continuity.** Ordinary departures to another app currently
   end Luma after 500 ms hidden. Define and verify explicit user reentry without
   intercepting Home or extending an unknown background lifetime. Restore a
   stable last-deliberate favourite across a fresh process, with old-record,
   removed-app and empty-shelf fallbacks. The Settings cover/return state machine
   already exists; it needs current ordinary native proof rather than another
   implementation. The shared Home options candidate addresses a separate
   interaction inconsistency.
   The first continuity slice will remember only the last deliberate Home
   favourite in a separate bounded app-owned sidecar. This keeps the existing
   favourites record readable by older builds, which reject unknown fields.
   Hover, automatic reconciliation, actions, menus and search text are excluded.
   Reentry remains an explicit fresh launch through LG; no longer background
   lifetime or automatic relaunch is planned for this slice. Abrupt eviction can
   lose the latest unsaved selection.
   [Draft PR11](home-focus-bookmark-candidate.md) now implements this slice at
   source65868df: source/host review, matching ARM build and local direct-ABI
   review/coordinator replay pass. A new enabled ordinary native scope remains
   open. Both initial
   catalogue/fallback prototype failures are preserved; no TV received them.
2. **Input switching on both targets.** Ordinary C2 input changes currently fail
   closed. They need a dedicated operation-specific selection/readback and exact
   original-source restoration before enablement. G2 results, a Proof identity
   and host checks cannot unlock this. Signal presence, selected source and
   actual video format must remain distinct; retain Unavailable where no
   trustworthy format read exists.
3. **Settings tasks and artwork supply.** Named timers, app/storage,
   external-device/CEC/Bluetooth and tuner/channel tasks need honest LG-managed
   destinations or individually qualified native controls. Existing category
   overviews do not establish direct task destinations or comprehensive parity.
   Separately, implement a current-catalogue icon acquisition/refresh flow after
   qualifying the new reader. A one-icon CLI and fallback initials do not supply
   that flow. Keep normal Root artwork off until the complete path is qualified.

The [named task-guide candidate](controls-task-guides-candidate.md) now adds five
existing overview links and two read-only guides. Source/host review passes;
the combined Home/menu/focus/task tree passes 836 library and 16 UI host tests
and strict Clippy. These guides do not add native setters, direct task links,
storage capacity or actual TV destination/return qualification. The separate
offline icon-tool candidate now passes independent source review and is included
in the matching combined ARM build. This is local source/artifact evidence; the
reader and actual icon-supply path still need separate native qualification.

Source agents can design and test these changes in parallel. The coordinator
alone owns hardware tests and retained updates; new candidates must remain
separate from the frozen executable used by the current test workflow.

The initial resumed campaign used G2. The user has since explicitly redirected
manual testing to C2. Its [fresh permission observation](c2-current-permission-observation.md)
blocks reuse of the current G2 favourites carrier. A separately reviewed C2
preservation model may allow the disposable Proof's independent private store
to be tested without repairing the retained installation.
The metadata was observed through Root SSH; native effective permissions remain
unobserved. Qualify that distinction before selecting a metadata repair.
The earlier G2 code and baseline cannot be copied across. Human availability is
tracked privately. Every device scope needs a new actual
baseline, lease, finite cleanup reserve and known outcome. No old nonce, actor,
Ticket, snapshot freshness or closed-session authority transfers. C2 work needs
separate availability and authorization; G2 results never establish it.

The later C2 read-only profile now passes its finite observation and coordination
scope. It qualifies the opaque preservation reader, with sampled Core/source-map
agreement and four measured absent paths. Independent saved-data review and a
byte-identical coordinator replay pass, with all closed evidence unchanged.
It does not qualify normal C2 entry, private-store compatibility or Proof.
The observed screensaver/other-app context was ineligible for an app test.
Finish the C2-specific Proof baseline and known inert-install refusal cleanup,
then use fresh LG Home context for the two-process favourites and physical checks.

A separate [Controls label candidate](https://github.com/luma-home-tv/native-home/pull/8)
now fixes editor-readiness help, LG-opening status and partial General/Accessibility
headers. Its69 Shell tests, strict host Clippy, pinned-framework check and
independent source review pass. It also records the current source inventory
and remaining control-centre gaps. The actual ARM UI build now passes at
functional source `1f75701f7f8760ad839ae626f2b1119b5dabebe6`, producing ELF
`8f06c1a689667e7168066740a8ab70c3b2ad83081558aa9d311f2f3f6d5f419a`.
Independent local review and byte-identical coordinator replay bind all143
selected source inputs and find no new direct dependencies or imports against
the retained UI (11 libraries,223 complete symbol/version/type/binding tuples).
This is direct ABI evidence, not transitive loading or native rendering proof.
No ARM package, TV deployment or new setter is included; it does not change the
retained test executable.

The complete C2 disposable favourites carrier has passed independent source
review, including terminal-known mode refusal and separately guarded inert
removal. Native admission and physical favourites/restart remain open. The
separate no-install normal C2 smoke is still under review: its first proposal
inherited a G2-only SDK mount exception and must be corrected before admission.
Normal startup can initialize state before observation if its native view
allows it; this must not be described as an inert read-only permission probe.
Neither workflow authorizes permission repair.

Later independent review of the normal v2 source found a repeated-path-fragment
edge: looking only at the first canonical app-path occurrence can miss a later
valid same-app mount alias. The full disposable C2 Proof shares that algorithm.
Both future native admissions are held pending corrected source and fresh
review. The counterexample is synthetic source evidence; no native dispatch or
device effect was demonstrated. Preserve the earlier finite reviews and failed
counterexample rather than treating their passing fixtures as broad proof.

Corrected normal v3 (`74095515`) and disposable Proof v2 (`f91dc2fd`) now pass
independent source review. They enumerate every boundary-qualified app-path
occurrence, retain raw row order/duplicates and refuse overflow rather than
truncate: table1MiB, row8192B, target4096B,4096 occurrences and64 roots. The
previous actual-method counterexamples now refuse. Author suites pass49/48
groups, independent focused checks pass21/19, and two fresh coordinator replays
pass17 alias/limit groups each with343/436 original bindings unchanged.
These are source/host results. Only the new versions may be considered for a
fresh native scope; neither is dated or admitted. Normal selected-drift closure
remains distinct from cleanup/lease release, and the final-only coordination
companion now passes its separate source review. It can close coordination only
after an original normal workflow has already closed all owned resources; it
cannot repair mounts or adopt a selected/midphase failure. No matching original
normal run or native companion execution exists. No TV test follows automatically.

The separate [public icon reader foundation](public-icon-reader-foundation.md)
now has a frozen source commit and draft PR9, strict/sanitised host checks,
independent source/host review, coordinator replay and an actual ARM build.
This replaces the unsafe pathname-reopen approach only as a candidate primitive;
the combined candidate removes the legacy automatic shell file reads and makes
collection metadata-only by default, with explicit offline body/receipt ingestion.
Next qualify one known packaged-icon read in a separate headless device scope
before connecting a broader native collector. No normal Root artwork, cache writes or TV
deployment follows from the local milestone.

The current delivery target is a responsive native Home dashboard for both
authorised G2 and C2 TVs on webOS24. C2 is the current physical-test target.
The G2 is always available for coordinator-owned tests.
The C2 is available only from 00:00 to 06:00 Europe/London or when the user
explicitly releases it. Each run still needs fresh scene, device and lease
checks; refresh the C2 baseline before changes. Physical input requires current
human availability, and results stay device-specific.
It remains a separate application: LG Home, Home/power/volume handling, firmware,
system libraries, boot configuration, media services and OLED protections stay
under LG's control. Retained installations and each TV's saved favourites must
survive updates; temporary proofs use a separate identity or a backed-up,
verified update and rollback procedure for the retained identity.

The [concept renders](concepts/README.md) show the intended interface. They are
synthetic design illustrations, not screenshots of implemented functionality.
The [interface specification](../design.json) records the layout and interaction
requirements; the milestone records below identify what has actually been proved.
The [fidelity map](design-fidelity.md) compares the original concepts with the
native interface. The earlier [G2 0.61.0 campaign](g2-retained-preview061.md)
and [48-entry cache across two producer generations](ordinary-settings-validation-progress.md)
are historical component records; the current installed executable is identified
in the deployment checkpoint above. They followed the earlier
[0.57 caption-preference delivery](g2-retained-preview057.md). The historical
[0.58 cache observation](ordinary-artwork-cache-058-native.md) reached ordinary
Home/Apps cache associations and returned to exact 0.57 through separately
verified cleanup; its original operator failure remains explicit. The earlier
[0.47.0 milestone](g2-retained-preview047.md) recorded two fresh Home/Apps/Back
sessions, a natural idle-black/wake and full saved-state,
bundle and Core20 preservation. Its visual frames show the logo and unboxed
available icons; broader artwork and physical-input/performance qualification
remain separate. The historical C2 retained0.48 record is
[here](c2-retained-preview-048.md), following
its own ordinary-owner forward/reverse metadata roundtrip and exact old-version
restoration, then a fresh forward migration and two normal sessions. Original
preference bytes/owners, C2 resources, Core files and checked state are preserved.
This no-text trial does not qualify C2 IME, Root artwork or all-app coverage. The
[numeric-control record](numeric-control-milestone.json) covers the latest G2
native range path and the retained update; the
[Home navigation record](home-navigation-milestone.json) covers the shared Home/drawer layout;
the [HDMI selection record](hdmi-selection-milestone.json) covers source control and its C2 boundary;
the [pointer-control record](pointer-control-milestone.json) covers the first native editable setting;
the [expanded control-readout record](control-readouts-milestone.json) covers the twenty added readings;
the [LG Home return record](lg-home-return-milestone.json) covers the explicit rail shortcut;
the [Inputs detail record](inputs-detail-milestone.json) covers source layout/readback;
the [responsiveness record](responsiveness-milestone.json) retains the bounded
performance evidence from 0.14.0.

The [0.59 explicit preparation path](artwork-catalogue-preparation-059.md) now has
host checks, matching ARM builds and local package/ABI inspection. Its first
[native batch](artwork-catalogue-preparation-059-native.md) prepared 15 of 16
images and returned to exact 0.57 with independently audited known cleanup.
The original operator exit 1 remains explicit; the ordinary consumer was skipped.
The later [0.60 trial](bounded-icon-decoding-060-native.md) completes all sixteen
images in one selected batch and ordinary Home/Apps cache consumption, then
restores exact 0.57 with independently audited cleanup. Multiple-batch/current
per-key coverage and retained delivery remain open. This component does not
complete the dashboard or qualify a new normal release.

The [0.53 exclusive departure-test recipe](artwork-context-qualification-053.md)
now has host, strict-package, independent source-review and matching ARM-build
evidence. Its two real ReadAfter/Finish native cases and independently replayed
removal still precede normal artwork activation. This host work does not change
current device availability or admit a native run, and does not update either
retained app.

## Supporting evidence and earlier milestones

The current sequence and installed identities above govern new work. The records
below retain their original artifact and device boundaries; an older milestone
does not supply a current baseline or authorize replay of its closed test scope.

Preserve the retained normal app and ordinary cache while qualifying restart
and action flows against their actual current baseline.
Choose one coherent normal-release artifact and prioritise
normal Search, one-keyboard dismissal/reopen, remote focus/hold/wheel, an owned
app launch and LG Settings return, reversible favourites and repeated entry/idle.
The recorded normal sessions establish Home/Apps/Back and idle/wake within their
respective scopes. Both current retained installations are 0.61.0 with the
exact executable identified in the deployment checkpoint above; the C2 0.48
record below is historical. G2 has 48 retained icon entries across two
producer generations for the same supplied 24-key catalogue; older
cache-absent/057 carriers cannot be reused.
Earlier isolated results remain useful component evidence. Qualify the relevant
normal flows using the retained artifacts where possible, then change source only
for an observed gap. C2 keyboard and input results require their own evidence.

The [ordinary Settings validation progress](ordinary-settings-validation-progress.md)
records completed additive preparation, a normal attempt that stopped at the
input freshness guard before Settings, and separate successful old061 restoration.
Prioritise a focused normal Settings return and usable postreturn navigation,
followed by independently bounded fresh reentry. Preserve the full delivery goal:
these intermediate tests do not qualify a retained candidate or comprehensive
control coverage. Cache compatibility and interface work can proceed locally in
parallel while the coordinator alone admits and operates each TV test.

The bounded thumbnail fix now has matching ARM artifacts and a successful
reviewed batch/ordinary Root-free consumer trial, with per-attempt disposition
and known restoration.
The SDK mount comparison preserves all measured aliases; each trial needs a
fresh native baseline. The retained061 campaign now checks the supplied16+8
body set and its ordinary session. Prioritise restart/app/settings return,
invalidation/failure behavior and explicit cache producer/version transitions.
A cursor, index Ready or aggregate GL count is not all-app coverage. Measure
current-artifact cold preparation and ordinary warm navigation separately before
claiming a performance gain. Eviction/stale-writer recovery and C2 qualification
remain open; normal interaction must keep Root artwork off.

The [0.61 source](controls-and-icon-observations-061.md) moves each existing LG
category shortcut before long native readings and adds a bounded, session-only
per-key cached drawing ledger. Its [completed temporary G2 run](controls-and-icon-observations-061-native.md)
now qualifies both preparation batches, first-batch continuity and ordinary
drawing references for the exact supplied 24-key set, with natural Back exit,
independently audited restoration and cleanup. Preserve the two earlier failed
attempts. Next prioritise the marker-free retained artifact's practical daily
loop and backed-up delivery, then physical input/favourites and matching
performance. Do not repeat population solely to increase component counts.

The [first marker-free delivery attempt](g2-retained-preview061-first-attempt.md)
now records the intended package's 24-icon drawer and six-category tour, natural
exit and exact 0.57 restoration. It remains a partial campaign: keyboard/idle were
skipped to preserve the fixed cleanup budget. Run Search first on the next tour,
then return to the first app for one downward pass. Budget a fresh scope from
measured phase/cleanup durations; preserve every Root, scene, identity, input and
unknown-outcome guard. The later retained061 campaign completes this tour;
the remaining practical and physical checks still require their own evidence.

The [second marker-free attempt](g2-retained-preview061-second-attempt.md) now
records a safe second-batch preflight refusal and complete 0.57 restoration. A
native LS2 foreground read became unavailable after the inert check; the receipt
does not retain its typed failure. The later [retained061 campaign](g2-retained-preview061.md)
uses bounded typed diagnostics with unchanged budgets and dispatch/context guards,
then completes the normal tour and retention. Historical cause remains unknown.
Next use a fresh061/cache24 baseline for owned app launch/removal and LG Settings
return. Physical input/favourites, matched performance and cache maintenance remain
open; no old source/artifact or native-ledger association transfers to this build.

Split host work into independent streams, with one coordinator admitting frozen
artifacts and serialising each TV's tests:

| Stream | Useful next output | Dependency |
| --- | --- | --- |
| Interface and input | Empty/long/localised/stale states, focus/menu/dismissal acceptance matrix | Physical remote and IME outcomes need a current available TV window |
| Packaged icons and performance | Coverage by app identity, bounded startup/decode review, matched current-build workload | New/revised apps can lack snapshot artwork; a metadata path alone grants no byte access |
| Read-only controls and inputs | Missing-value inventory and consistent native/read-only/LG-managed labels | Each additional setter or C2 source change needs its own capability, context and restoration proof |
| Live artwork | Explicit finite preparation, per-key publication/body coverage and ordinary cache consumption | Actual Preview role, per-attempt helper/child/pending closure and known restoration; normal navigation keeps Root off |
| Integration and release | Known update/removal, fresh preference preservation and a scoped release record | Current source, package and native receipts must agree; unknown outcomes retain their barriers |

Release acceptance must also cover the practical daily loop on that same normal
artifact: Home/Apps/Inputs/Controls, Search and one LG keyboard, physical menu
hold/release, owned-app launch and explicit return, LG Settings handoff/return,
idle/wake and repeated entry. Use an owned Target for reversible favourites and
preserve the original preference semantics and bytes; inverse reordering alone
can materialise a previously absent order vector. Fresh SDK relaunch is not
automatic app return, and selected HDMI source is not actual video/format proof.

Complete the intended control inventory as verified native read/edit, explicit
LG handoff, protected or unavailable. Missing/stale/partial presentation and
long/localised labels can advance in parallel with cache work. New setters need
their own capability/context/effect/readback/restoration evidence. Measure the
selected normal artifact's total resource cost and warm navigation separately
from cold preparation and diagnostic capture overhead before claiming speedups.

The pre-0.54 packaged startup loader visited only the first 64 catalogue entries
and checked size/dimensions after reading/decoding. The current 0.54 source adds
a separate [bounded ordinary reader and demand cache](packaged-icon-demand.md);
its [isolated G2 result](packaged-icon-native-054.md) verifies three controlled
images, missing fallback, worker/texture shutdown and separate cleanup/preservation.
It does not update either retained installation or establish TV performance/all-app
coverage. The retained trials establish no over-64 failure
or measured memory fault. Measure normal navigation and return flows with the exact current artifact;
the [surface preparation record](native-surface-scene-prerequisite.md) explains
why old positive Root helper timings cannot qualify release responsiveness.
Keep normal Root artwork off while its independent feature work continues.

The separate [0.54 G2 performance-only workload](current-performance-054.md)
now completes Home/Apps/Controls, actual idle/wake and natural Back exit, followed
by verified removal/preservation and byte-identical independent audit replay.
Its earlier observer-limited attempt remains unqualified. The completed run
retains a 183.667 ms draw outlier already present in the earliest Home sample;
normal Preview cost, physical input/smoothness and C2 remain separate checks.

The [0.54 G2 keyboard record](keyboard-observer-054.md) now corroborates two
human/native Done/reopen/Back cycles with no extra keypad. Its final-exit reserve
and empty-file parser failures remain explicit; later known cleanup and full
preservation do not promote the failed original operator. The next
[0.55 source](drawer-empty-states.md) introduced distinct empty-favourites,
search-miss and unavailable-data copy. Its separate
[normal delivery](g2-retained-preview055.md) qualifies bounded sessions; every
individual empty/error layout remains unqualified on the TV.

## Direction

The agreed visual direction is the Apple TV / tvOS feel that PlxNative interprets
well: large cards, restrained chrome, smooth spatial focus and an obvious Back
hierarchy. Keep this native and remote-first. The concepts are a starting point;
remove unnecessary screen furniture as the interactive screens take shape.

Build the standalone Luma Home client on explicitly attributed PlxNative Rust UI, input/focus and
SDL/OpenGL ES libraries. Reuse its components instead of maintaining a second
widget system. Generic improvements can be proposed upstream separately after
verification; the Home client does not change the Plex application or player.
The shipping architecture uses native rendering, with no WebView.

The canvas is true black for OLED. Show available app icons directly on black,
preserving their aspect ratio, transparency and source shape. Add no graphite
plate or forced rounding; show a focus outline only on selection. Missing artwork
uses a graphite initials/name fallback. Graphite remains available for controls,
with bright focus/ink limited to useful areas. Idle behaviour must
minimise static bright elements without changing LG's panel protections. The
[branding masters](../assets/luma-home/README.md) record the icon, wordmark
and palette; a black background is not treated as proof of burn-in prevention.

| Area | Intended behaviour | Current evidence |
| --- | --- | --- |
| Home | Clock/status header, immediate access to favourites and HDMI inputs, control-centre entry and LG Home return | Native Luma identity, clock/calendar, status rail and larger favourites/HDMI shelves, explicit LG Home return and native View all shortcuts; Back restores the invoking local route |
| Apps | Complete actionable app drawer, search/filter, pinning and dashboard-only ordering | Registered visible apps joined to launch points; stubs/hidden/invalid targets excluded; live updates and identity/focus checks pass; normal app launch enabled and isolated native target proof passed; All/Favourites and native keyboard search are implemented and captured on G2; modal pin/order menu and two-launch persistence are verified on both |
| Favourites | Pin/unpin with held OK, save reliably, retain selection across changes | ARM pin/save/readback/relaunch/unpin verified; bounded background writer; reboot survival remains untested |
| Inputs | Four HDMI cards and a detail pane with distinct plug, signal and selected-source states; select through LG's normal source path | Two-column native cards/detail pane and independent EIM selected-source getter verified on both; separate G2 HDMI 4 → HDMI 1 trial confirmed source change and restoration; all four G2 ports now have independent selection/restoration trials and production selection in 0.19.0; C2 exact idle restoration and live format/playback remain unverified, so C2 selection stays gated |
| Controls | Native category/detail pages, correct context, validated choices/ranges and access to the rest through LG Settings | Six-category/detail composition and read-only picture, sound and network refresh verified on both; all six LG-managed entries and native return proved on both models in 0.13.0; 20 descriptor-validated readouts added in 0.17.0 across Picture, Accessibility, General and Panel Care; six more guidance/Sound readouts have isolated 0.25.0/0.26.0 ordinary-app/native proof on both; native pointer-speed choice/commit/restoration proved on both in 0.18.0; G2 native audio-description range entry/Apply/endpoints/restoration proved in 0.21.0, with description off; broader verified edits and setting coverage remain |
| Navigation | Remote focus, safe tap/hold, Back hierarchy, pointer and wheel | Shared-machine tests and scripted ARM navigation pass; injected Back reaches SDL and exits; physical remote/pointer/wheel proof remains |
| Performance | Smooth transitions, bounded memory and render only when needed | Native Mali-G52 rendering and app-local black idle/wake proved; Native injected SDL navigation and bounded submission/CPU/RSS/idle-paint measurements now exist on both; physical latency, GPU/panel timing and stress memory remain unverified |

The [input-batch candidate](input-batch-ordering-milestone.json) records the 0.29.0
queued-cancellation and duplicate-activation fixes. Its physical workflow remains
incomplete. The [0.30.0 language/region candidate](language-region-reads.md) adds
five protected General readings; [isolated ordinary-app reads and focused native layout](language-region-read-milestone.json)
are verified on G2. The [backed-up normal 0.30.0 delivery](retained-preview-030-milestone.json)
then verified Apps/Search/keyboard, General rows and preservation; broader release
checks remain open.
The [isolated 0.31.0 qualification](read-only-values-031-milestone.json) adds
legible current protected values, unchanged activation blocking and verified
build/package inputs. Its [retained promotion](retained-preview-031-milestone.json)
passed a separate backed-up normal workflow with favourites preserved.

After that delivery, prioritise live app artwork: the current live catalogue
preserves snapshot icons but cannot supply artwork for newly installed or
revised apps. Use the shared native texture loader, bounded work/cache ownership
and identity-preserving catalogue updates. First verify ordinary-app access to
webOS24's actual icon paths; metadata alone does not establish filesystem access.
The [first passive survey](artwork-access-032.md) read one developer icon but found
four sampled directories absent inside the ordinary jail. Qualify the missing
resource route before treating a direct-file worker as complete artwork support.
The [Identify-only candidate](artwork-broker-identify.md) now separates that
route's caller/transport qualification from icon reads and production startup.
The subsequent [fixed context reader](broker-context-reader-034.md) qualified one
restricted native G2 exchange and clean removal with checked state preserved.
The [0.35 isolated resource test](artwork-transfer-035.md) now qualifies a real
icon transfer and ordinary-process bounded decode from each of five observed
installation classes, with complete removal and preserved checked state. Its
pure cache state has separate ownership/invalidation tests. The
[0.36 isolated C2 lifecycle](artwork-live-036.md) now connects a bounded worker,
cache and main-thread textures for an owned fixture, covering initial artwork,
same-path replacement, package revision, removal and joined shutdown. Other apps
were deliberately excluded; initials in that test are not a C2 capability failure.
A direct RGB recheck withdraws the initial missing-text report: every pixel
outside the updated fixture was unchanged. The isolated 0.37 G2 capture now
qualifies synthetic icon-only alpha/aspect/fallback presentation with preserved
checked state. Its [backed-up normal G2 delivery](retained-preview-037.md) then
verified real packaged artwork and the normal loop across two launches.
The [0.38 read-only capability check](artwork-startup-capability-038.md) now
qualifies one ordinary native HBC status request with clean isolated removal;
the retained 0.37 installation and favourites remain untouched. It does not
qualify a Root child or production startup. The subsequent
[0.40 installed startup component](artwork-startup-040.md) qualifies two serial
protected Root Identify/Finish exchanges with fresh app/helper identities. The
first operator observation failed and needed separate recovery; the second
corrected driver passed with full cleanup/preservation. The isolated
[0.41 nonce-refusal results](artwork-startup-refusal-041.md) verify the two declared
G2 refusals, independently bound process/GID evidence and full cleanup, without
extending the helper's policy. Next, connect the existing resource route to the
actual worker/cache/native textures, then broader live artwork
coverage and a separately admitted retained integration. The
[0.42 provider candidate](artwork-installed-provider-042.md) adds the finite
installed transfer/provider and explicit worker budget in source; host checks
and both ARM builds pass. The [0.43 renderer result](artwork-installed-renderer-043.md)
adds isolated Home/Apps activation, raw captures and bounded joined-fetch
observations. Its G2 carrier now exits zero with six completed helpers/transfers,
96 known children, three shared live targets on both pages, complete temporary
removal and preserved normal 0.37 state. A 28,205-assertion saved-evidence audit
passes; actual Preview-role startup and a backed-up retained rollout remain
separate gates. The
[production startup design](artwork-startup-design.md) separates the
operator-started helper from an independently qualified bootstrap.
The [Preview admission foundation](preview-artwork-admission.md) now supplies
closed profile-aware identity/codecs and a negative-only pending-operation guard
in the Proof provider. Host checks pass; its new native guard regression,
remaining installed-profile integration and actual Preview-ID trial precede
normal activation. A new app ID cannot prove the retained app's LS2 permissions.
HDMI format/status and missing Sound/Accessibility readings remain subsequent
bounded, read-only inventory work. Existing verified source selection and LG
routes remain available throughout.

The earlier [OLED/branding milestone](oled-branding-milestone.json) records
true-black framebuffer evidence and clean removal on `2a53bfd`. The later [standalone device milestone](standalone-device-milestone.json) corrects
the earlier missing-rail interpretation: original cropped framebuffer bytes show
the complete rail on all six pages. All six complete PPM files also match the
prior captured states byte-for-byte. No renderer
change is supported by that claim. Physical input and broader UI work remain.


The [drawer milestone](drawer-milestone.json) records native All/Favourites
filtering and a shared-focus alphanumeric keyboard. The scripted G2 path typed
a query, filtered results, closed the keyboard and restored All without a TV
request. Physical input and full text/IME coverage remain to verify.

The [text-input record](text-input-milestone.json) adds a native bounded Unicode
field, grapheme-aware edits and isolated G2 LG-keyboard activation/reopening.
The user confirmed Magic Remote typing and Done/Back. LG consumes Back when
hiding its own panel; Luma now detects that dismissal, stops its request and
returns focus to Search without revealing its fallback keypad. Two corrected
native cycles and independent window/removal checks pass. This remains a proof
build; the retained versions above are unchanged.

The intended search flow uses **one keyboard at a time**. Prefer LG's keyboard
for its language and input integration, and offer Luma's keypad when unavailable
or explicitly selected. Keep the field, filtering and drawer natively rendered.
Done/Back retains the query and ends editing. Never switch to a second keyboard
merely because the first one closed. Normal request/fallback integration,
prediction and broader script/font qualification remain before deployment.

The [shared keyboard-controller record](keyboard-integration-milestone.json)
adds the candidate's Search/Keypad routing, admitted platform gate, corrected
native preedit ABI and bounded visible text uploads. The same frozen binary
passed isolated G2 and C2 trials: two guarded dismissals, synthetic preedit/clear,
explicit fallback and maximum-length text at three caret positions. Both were
removed with preserved-state checks. The normal interactive event loop, actual
composing languages and retained updates remain separate gates. The user saw no
word predictions in the available G2 keyboard mode; no prediction-replacement
contract is inferred. See [ABI notes](webos24-text-compatibility.md).

The newer [actual-main-loop record](main-loop-milestone.json) separates the
0.24.0 interactive scenarios from the earlier helper trials. Both models have
independent keyboard/fallback, real 120-second idle and programmatic LG Home
departure results, with preserved-state/removal checks. Both also completed idle
begun with LG editing active; adjacent recorded states show editor/SDK closure
at the idle transition, without establishing an exact SDK close cause. Network
Back and fixed synthetic text are not physical typing/Done evidence. The newer
[OK-wake record](ok-wake-milestone.json) independently qualifies a first OK that
only wakes, followed by a distinct second OK that opens Search, on both models.
Both host drivers were interrupted after native completion; reviewed same-run
closeout recovered removal and preservation without restarting either scenario.
The [G2 human record](manual-keyboard-milestone.json) adds current-build typing,
Done, same-process reopen and Back with `luma` retained and no fallback keypad.
One closing-key release was drained in the same SDL batch. Pointer, later-batch
and broader dismissal races remain unqualified, as do C2 physical typing,
explicit idle stop cause, performance and retained promotion.
One preceding G2 attempt exited on early focus loss before readiness; its cause
and cold-entry reliability remain unresolved. The
[startup investigation](startup-lifecycle-investigation.md) records the exact
event/departure timing and the next bounded observation. The other incomplete attempts and
the corrected host dismissal-counter mistake remain in the record.
The [0.26.0 startup/relaunch record](startup-relaunch-milestone.json) now qualifies
two predetermined G2 sessions through the actual loop: programmatic LG Home
departure with the LG editor open, then a distinct process loading the same
installed package and unchanged saved preferences. The second session closes
the keyboard and exits through network Back. Both native identities, startup
endpoints, independent window observations and removal/preservation checks pass.
One earlier observation stopped on LG's splash classification; that failed
attempt remains separate. No native departure-policy change was needed. The
original rare focus-loss cause and general reliability remain open, as does
this particular relaunch profile on C2.
The next delivery step is the [backed-up G2 preview rollout](preview-rollout-plan.md):
qualify installer rollback, preserve the retained installation/state and validate
the normal app identity. The 0.26.0 normal package is prepared, with deployment
still pending those checks.

The [parallel plan](parallel-development.md) now lists the six broad remaining
delivery stages, the immediate interactive-loop gate and the next parallel wave.
It assigns independent firmware/text,
controls and C2-validation lanes around one integration owner and one live owner
per TV. The [coverage audit](controls-coverage-next.md) preserves the comprehensive
control-centre backlog; existing category routes do not establish every-setting
coverage. Its next candidate is a separately admitted pointer-size control after
capability/semantics review, rather than a generic setter.
The [read-coverage audit](read-coverage-audit.md) identifies three guidance
parameter readouts and the Sound dimension boundary. The
[guidance milestone](guidance-read-milestone.json) now qualifies those readouts
and focused/scrolled native component frames independently on both models.
They add no setter; physical input, freshness transitions and retained promotion
remain separate.
The [optional global Sound group](sound-read-design.md) now has fixed GETs,
strict schemas and three protected rows integrated behind independent freshness.
Its [0.26.0 milestone](sound-read-milestone.json) independently qualifies ordinary
getter/descriptor access, typed readback matching and native focus/scroll/layout
on both TVs, with complete removal and retained/core/settings preservation.
Native stale/partial transitions, audible effects and retained updates remain open.
The [capability presentation model](control-capability-design.md) keeps reading
freshness, edit eligibility, pending operations and LG routes separate; it has
no normal UI caller yet.

## Next implementation stages

1. **Expand live data.** The typed normal-app provider now refreshes catalogue,
   launch points, HDMI and picture state in the native interface. Focus/favourites,
   unchanged gestures and stale/context failure handling have checks; four real
   cycles were applied on both TVs, including sound/network in 0.10.0. Complete
   broader coverage and newly installed icon loading. Do not treat subscription acknowledgement as delivery
   proof. Keep loading and queues bounded.
2. **Complete normal actions.** The restricted application adapter and live
   drawer/shared-OK path now launch an isolated native test target with both
   adapter and independent foreground readback. Normal application launching is
   enabled; personal viewing accounts were not exercised. Finish C2 tuner-source
   restoration and live HDMI proof; G2 source selection is now enabled after its per-port trials; both
   TVs now have verified LG-managed section routes. Keep
   unknown/hidden/self targets, unvalidated operations and extra parameters
   rejected; an acknowledged request is not execution proof.
3. **Expand controls safely.** Audit webOS24 descriptors and semantics for each
   operation. A descriptor alone does not grant write authority. Require valid
   range/enum, matching input/HDR/mode context, bounded effect, readback and an
   original-state restoration test. Ordinary-app audio getter aliases and network
   readings are now independently verified. Twenty more descriptor-validated
   picture/accessibility/general/panel-care readouts have ordinary-app proof on
   both TVs; signed ranges, enums, dimensions and expiry are checked. Descriptor
   success does not grant write authority. Pointer speed now has a separate fixed
   enum adapter with firmware/descriptor/foreground/expected-value checks and
   independent native change/restoration proof on both. An isolated
   [signed numeric adapter trial](numeric-control-experiment.json) now proves a
   one-step audio-description level change and restoration on G2, with
   description checked off. C2 was skipped before installation when the safe
   test context was unavailable. It is restricted to the proof identity and does not
   enable a production slider. The newer [combined numeric-control proof](numeric-control-milestone.json)
   connects the shared range editor, fresh fixed-setting reads and bounded worker
   on G2, including endpoints and native restoration. G2 0.21.0 enables that
   guarded entry; C2 numeric control stays gated. Other settings remain read-only
   or LG-managed. Use a restricted
   local helper only for operations that actually need privilege, with caller,
   executable and session validation and a fixed operation allowlist.
4. **Finish the interface and measure it.** The combined Home now uses two shared card shelves, with readable source
   status on every tile. Clock/status, app ordering and View all shortcuts are now implemented. The
   bounded drawer uses shared external grid geometry/springs, whole-tile
   admission and content-end clamping; native motion/settling proof is scoped in
   the current milestone. The [shared native range editor](range-editor-milestone.json)
   has synthetic staged-edit and removal proof on both TVs. Its fixed G2 numeric
   adapter now also has native entry, Apply, endpoint and restoration evidence.
   Extend this pattern only after the next operation passes its own capability,
   freshness/context, bounded completion and readback checks.
   Expand the verified choice pattern and complete broader read coverage. Verify physical
   remote, pointer, wheel and Back behaviour on both TVs.
   Measure the actual dashboard's frame pacing, input response, memory and idle
   behaviour; the earlier C renderer's results do not transfer to this client.

## Protected and unsupported operations

The [0.46 actual Preview diagnostic](preview-artwork-preflight-046.md) now has
G2 resource and exact restoration evidence. Keep normal artwork disabled until
the [transition lifecycle](artwork-transition-lifecycle.md) is qualified. The
[C2 installer metadata measurement](c2-installer-metadata-045.md) supports one
rollback mechanism component. The [C2 0.48 record](c2-retained-preview-048.md)
now adds actual ordinary-owner migration/reversal, same-ID restoration and a
separate current normal UI trial; these device-specific results do not transfer
Root artwork or keyboard authority.
The [0.48 SDK-argument candidate](preferences-sdk-arguments-048.md) accepts one
bounded opaque launcher JSON object without granting operation authority. Its
host/ARM checks are followed by the separately recorded C2 native roundtrip
and normal retention evidence; the original failed attempts remain distinct.
The [isolated C2 startup collection](c2-startup-collection.md) now confirms one
ordinary Target's launch ABI and narrow live runtime mapping, followed by
separately verified cleanup and unchanged retained 0.20 at that stage. Later
actual Preview migration is recorded separately; the earlier missing-report
cause is not established merely by later success.

Network reconfiguration, firmware, factory reset, service/calibration controls,
OLED protections and compensation scheduling remain protected/delegated. An
unsupported control routes to LG Settings or stays unavailable; the UI never
invents a successful native action. No arbitrary shell command, service URI or
file path is exposed through the interface. No Home interception or boot hook is
part of this plan.

## Verification and delivery

Every TV build uses its own isolated package and the repository's device lock.
The user has now authorised deployment and testing on both the OLED55G26LA G2
and OLED65C26LD C2. Keep LG Home and all core services untouched on both. Use
each device's own private inventory, firmware check, captures and cleanup receipts;
G2 results do not establish C2 behaviour. After removal proof, explicitly requested
interactive deployments may remain installed as a normal app with no boot hooks.
The earlier C2 hold has been explicitly lifted; both are authorised again. The [clock milestone](clock-milestone.json) records
the G2 0.8.0 update and preserved favourites, and separates the interrupted C2
component test from complete validation.
The [Settings handoff milestone](settings-handoff-milestone.json) then verifies
the LG Picture overlay, sustained underlying dashboard lifetime and return after
injected Back on the G2. Version 0.9.0 enables that fixed route with per-launch
model/firmware/SDK checks; native setting writes and HDMI switching remain gated.
Build and host-check before taking the lock. Keep the device lease through live
frame review and input, then release it after verified closeout before changing
source. Inspect native frames, record exact source/build revisions and keep
private device data outside Git. After each test, verify removal of the package,
launcher entry, process, runtime, persistent test data, jail and mounts; compare
checked core hashes and picture/audio readbacks, then verify LG Home and restore
the original foreground.

The current evidence is in the [native shell milestone](shell-milestone.json)
and [favourites/read-capability milestone](preferences-milestone.json), and
[live-reading milestone](live-milestone.json) and
[application-launch milestone](launch-milestone.json), plus the
[combined Home layout milestone](home-layout-milestone.json) and
[OLED/branding milestone](oled-branding-milestone.json). Completion
requires the intended functional interface and device evidence above, not only
concept renders, a successful build or component tests.


## Standalone dependency boundary

The Luma repository tracks its application, adapters, models, assets, plans and
tests. PlxNative is a pinned external dependency with its original authorship
and package names. Five framework crates are selected; Plex/media/session/data
application crates are excluded from the resolved graph. The integration patch
is kept separately and attributed. Root build/packaging targets build only Luma
binaries. Historical capture records remain historical until this standalone
tree receives new device verification.
