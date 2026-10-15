# Luma Home project rules

Keep Luma Home's application separate from the explicitly attributed PlxNative
framework dependency. Do not add Plex client, playback, authentication or server
features. Retain upstream package names, copyright and licences. Record generic
framework changes in the integration patch and propose them upstream separately.

Use shared UI/focus components. Preserve an OLED-black canvas. Available app icons keep their own shape and
transparency without a graphite backing plate; missing-art fallbacks and controls
may use graphite surfaces. Build and verify the standalone dependency graph; host checks do not
prove TV rendering, physical input or performance.

TV details, credentials, actual app inventories/icons and household captures
must stay outside tracked source. Device work requires the existing device lock,
an isolated package and complete removal/state restoration. LG Home, boot hooks,
firmware, system libraries, jail policy, media services, networking and OLED
protections must remain untouched. Protected/unverified controls remain read-only
or LG-managed. Never exercise personal viewing-account writes in tests.

The interactive preview is now intentionally retained on both authorised TVs.
Preserve its saved favourites and installed state. Do not apply the former
diagnostic driver's unconditional uninstall/cleanup to a retained installation;
use a separate test identity or a backed-up, verified update procedure.

The latest verified retained G2 installation is 0.37.0, with dashboard ELF
SHA-256 `3310d1a5c0e7d7910dc2052673e6f508efaa25b9e83f2602204c5f72e5ae2c21`.
Its two normal sessions, full bundle/state preservation and temporary cleanup
are recorded in `docs/retained-preview-037.md`. Historical drivers pinned to
0.31.0 must remain historical; prepare a fresh guarded candidate for later tests
and preserve the current installation and favourites.

The isolated 0.38.0 check in `docs/artwork-startup-capability-038.md` qualifies one
ordinary native HBC `checkRoot` request and complete temporary cleanup. It did
not update the retained app, launch an artwork broker or qualify production
startup. Keep that boundary explicit when planning subsequent artwork work.

The isolated 0.40 result in `docs/artwork-startup-040.md` qualifies two serial
protected installed Root helper sessions and Identify/Finish. The first device
driver exited 1 on a terminal-receipt observation race; native evidence completed
and separate recovery exited 0. The second used the same frozen binaries with
fresh app/helper PID and start identities; its corrected driver exited 0 with
full temporary removal and preservation. Keep the original failure/recovery
history distinct. Normal 0.37 is unchanged; resources, concurrent/duplicate
startup, normal enrollment and production delivery remain unqualified.

The 0.41 result in `docs/artwork-startup-refusal-041.md` verifies the declared
wrong-Identify and wrong-Finish nonce refusals on G2. Both carriers exited 0;
each helper/HBC result remained an actual refusal. Independent saved evidence
replays the zero/three known Core children, raw process/GID bindings, full normal
0.37 preservation and all 24 cleanup gates per case. Keep this separate from
positive startup and other rejection/reliability coverage. Historical 0.40/0.41
carriers and artifacts remain pinned to their source and package. Resource and
normal loader integration still need their own native qualification.

The 0.42 source in `docs/artwork-installed-provider-042.md` adds one installed
Proof resource read per session and a worker Provider with joined completion,
full SourceKey/readback checks and per-instance quarantine. It extends the
protected Proof Core reader to the two fixed catalogue GETs. Host checks and both
ARM builds pass; renderer activation and a combined native provider run remain
pending. No 0.42 package is installed and Preview remains 0.37. Do not promote the
historical 0.41 guards into proof of the new resource/renderer path.

The 0.43 isolated renderer result in `docs/artwork-installed-renderer-043.md`
connects that provider to Home and Apps with bounded per-fetch observations and
joined shutdown. The G2 carrier exited zero: six completed transfers/helpers,
96 known children, three shared live targets on both pages and all 24 cleanup
gates. Independent replay passes 28,205 assertions. Only the first helper has
external live raw process binding; later helpers bind kernel/self/HBC records.
Retained 0.37 and production startup are unchanged; normal-profile admission,
all-app coverage and a backed-up retained promotion remain pending.

The G2 is the primary device. The user explicitly reopened isolated C2 testing on
2026-10-15 until 20:30 Europe/London (19:30 UTC). Within that dated window the
coordinator may run isolated tests under the existing C2 lease, preserving its
retained preview and favourites. Do not start a C2 test after 20:15 local time;
finish cleanup before the user's 20:30 cutoff. After that window, do not contact
or modify C2 without a later explicit instruction. Earlier C2 records remain
historical evidence; G2 results never establish C2 behaviour.

Do not rewrite public history or change the existing public repository's identity
until the standalone result has been reviewed with the user.
