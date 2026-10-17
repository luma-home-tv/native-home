# Settings handoff lifecycle candidate

The retained G2 0.61 Settings trial confirmed launch but ended with
`window_hidden` before a usable return was verified. Its SDL focus-return count
was not proof that LG Settings had closed. The precise native event sequence is
unknown; the source changes here address the lifetime risks found during review.
The installed preview and its artwork cache remain unchanged.

SDL focus gain is now a hint. A successful return requires a current-generation,
fresh exact Settings-overlay/own-card observation followed by a fresh sole-own
card. Own-before-cover, unavailable observations and the known LG opening
spinner cannot establish a return. Observation freshness starts before the
existing window GET, so delivery delay cannot make an old read fresh.

An accepted cover or spinner suspends Luma's own drawing and visible input even
when SDL sends no hide event. A focus hint cannot resume them. Spinner suspension
does not renew the opening deadline or supply cover evidence. A confirmed scene
return resumes the existing UI and artwork lifecycle.

The original 20-second opening and two-second cover-freshness bounds remain.
Expiry is consumed before replies or window observations can mutate the handoff,
and before the main loop or action dispatcher continues. A late cover or refusal
cannot revive an expired lifetime. Foreign departure and expiry end the current
Luma session without dispatching another request. A bounded 64-record typed
timeline distinguishes these outcomes from the legacy SDL counter; it retains no
raw service replies or app inventory and grants no service-call authority.

Host validation passed 772 library tests, 15 UI tests and strict Clippy. Focused
cases include delayed cover renewal followed by a same-batch own scene, late
success/refusal replies, actual dispatcher refusal after expiry, and the main
hidden-state helper without SDL hide. Two independent finite source reviews
passed; matching native qualification remains open. No new service calls, settings
setters, permissions, dependency changes or timeout increases are introduced.

## Isolated native qualification

Use the existing separate Proof identity and fixed `settings` route with an empty
synthetic application snapshot. Do not replace retained Preview or use its saved
preferences, artwork cache or producer association. This profile excludes the
packaged app-icon worker and Root artwork; embedded branding, navigation SVGs and
ordinary read-only live/clock queries still run.

The existing noninteractive loop limit is 30 seconds after the capture prelude
and initial dispatch; it is not a whole-process deadline or a guaranteed overlay
window. The profile emits standard page captures rather than interactive
readiness files. A carrier must inventory those exact outputs, establish the
actual Proof process/window mapping, and admit any Back only against a fresh
exact own-card/Settings-overlay pair. Spinner permits bounded read-only waiting.
Insufficient time is inconclusive and does not extend the lifetime.

Require matching ARM/package provenance, a fresh device lease and retained-state
baseline, observed usable return, and supported Proof removal with known process,
state, runtime, staging and lease cleanup. Native rendering, physical input,
performance, reliability and clean removal are not established by host checks.
LG Home and all core components stay untouched. Earlier rejected candidate
artifacts must not be substituted for this source.
