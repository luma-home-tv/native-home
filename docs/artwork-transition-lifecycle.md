# Artwork across keyboard and foreground transitions

The app-controlled pause/drain slice is now implemented in the
[0.49 component](artwork-transition-049.md), with host checks and one isolated G2
queued-work/local-intent/Back result. Real LG keyboard/external-app handoffs and
the authenticated closed-negative slice below remain open.
The [next closure implementation plan](artwork-context-closure-next.md) specifies
the transport, typed attempt and adversarial-test boundaries without enabling
negative acceptance.
Ordinary interactive artwork activation remains disabled. The admission experiment does not establish
keyboard/departure recovery, physical input, performance or C2 Root access.

The resource policy requires the selected application's sole foreground CARD.
LG's keyboard or an external departure can invalidate that policy while an
artwork job is in flight, including immediately before Finish. Keep the policy
strict: an overlay must never authorize another image read or release.

## Why the current failure must remain unknown

Three source paths currently prevent a failed session from proving closure:

- `broker_bootstrap::run` propagates an error from `serve_installed_artwork`
  before recording `connection_closed`. Scope exit drops the connection and
  listener, but the final report still says closure is false.
- The provider's exchange returns an error without its typed `Exchange`, losing
  the authenticated peer/Identify evidence needed by a separate failure grader.
- `native::fetch` signals its exec observer to stop on every exchange error,
  before joining HBC. That can discard the final helper evidence that would
  distinguish a closed refusal from an unknown outcome.

The existing positive `session::lifecycle` also combines reader cleanup with
foreground admission. Loosening that function would weaken successful image
acceptance. A timer, HBC false, EOF or caller death alone cannot clear a pending
record. Existing unknown outcomes must retain the latch.

## First slice: pause and drain before our transitions

Add a small transition controller around `home-preview` keyboard/action
dispatch and Shell's artwork pump. Compose pause reasons: keyboard intent or
activity, deferred external action, foreground loss, idle and quarantine.
FocusGained must not override a remaining keyboard or action pause.

Pause dispatch before the current input frame reaches its tick/artwork pump.
Pausing only after `input_frame` returns can submit another job in that frame.
Keep input, rendering and result polling responsive while the existing job
finishes. Defer only keyboard Start or the typed external action until there is
no in-flight job and the current ticket has a known resolution. Do not block the
main thread or cancel a live transfer merely to accelerate the transition.

Keep at most one deferred action in memory; cancel or revalidate it on Back or
changed intent. Give keyboard startup an explicit readiness gate rather than
pretending its window is hidden, which could cancel the editor. Resume only
when every pause reason is cleared and normal fresh admission succeeds.
External departures cannot wait for this barrier and need the second slice.

## Second slice: authenticated, closed negative completion

Preserve a typed attempt trace even on exchange failure. Initially support only
context changes after a valid Identify response. Keep pre-Identify failures
unknown, except the existing joined, zero-attempt pre-dispatch no-start case.

The Root server must distinguish a typed context-policy rejection from a reader,
peer, pin, transport or deadline failure. A context-change rejection requires
complete successful pinned Core GETs with the admitted system identity; it does
not make the new foreground acceptable. Stop resource operations immediately.
Explicitly close the owned connection/listener on error and report a closed
negative outcome without marking the session qualified. Preserve all actual
release, request and child records, including bytes sent before a later refusal.

For an authenticated EOF candidate, retain the HBC observer through its remaining
finite budget instead of immediately signalling it to stop. A separate
`grade_closed_negative` may discard bytes and resolve **only this job's newly
created Ticket** when all of the following bind:

- The exact source-selected role/command, protected configuration, helper/UI/Core
  pins, fresh own and Root PID/start/hash identities, kernel peer and Identify
  message credentials, and final self/peer revalidation.
- The exact request/session sequence and typed failure stage, with every release
  accounted for and no image eligible for cache, decoding or GL upload.
- Complete untruncated final helper/HBC evidence, actual joined HBC completion,
  explicit connection/listener closure and observed EOF.
- Every actual child-start journal matched to its reader outcome, executable
  identity, stdout/stderr EOF, reap and known owned-group cleanup. Derive the
  required children from the attempted stages; do not assume a fixed count.

Any missing or inconsistent binding leaves the outcome unknown and the durable
latch intact. HBC false is compatible with a separately proved closed refusal;
it supplies no closure authority itself. The positive lifecycle and byte-commit
checks remain unchanged. There is no API to clear an older latch by age, restart,
app death or a generic retry, and no new command, path, URI or overlay permission.

## Acceptance before activation

Host tests must hold a real controlled Worker in flight and show responsive
Back/navigation, no new dispatch, and no early keyboard/action start. Exercise
same-frame intent ordering and FocusGained while another pause reason remains.

Negative lifecycle tests must cover before-read, after-read and Finish context
changes, discard all bytes and clear only the current matching ticket. Mutating
any identity, pin, child, EOF, report or truncation condition must retain the
pending record across a new provider. Include pre-Identify and incomplete-reader
cases that remain unknown.

Then use isolated native trials at declared transition stages, with actual
process bindings, joined negative HBC reports and complete scoped cleanup. Keep
the existing retained installation and LG Core/OLED behaviour preserved. These
are scoped lifecycle observations, not a hard blocked-call lifetime, generic
process-tree recovery, production, physical-input or performance guarantee.
