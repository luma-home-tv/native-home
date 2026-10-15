# Installed artwork provider candidate, 0.42

The source candidate connects the existing bounded artwork transfer operation to
a new provider for the ordinary artwork worker. Its installed server is limited
to Proof: valid Identify, one registered-app icon read, then Finish and EOF.
The normal Preview loader remains disabled. Native renderer integration and a
combined isolated device run are the next admission stage.

The [0.41 nonce-refusal results](artwork-startup-refusal-041.md) remain historical
evidence for their frozen source. This candidate extends the installed helper's
operations and therefore requires its own qualification. The retained G2 stays
on 0.37.0; no 0.42 package has been installed and C2 is outside this work.

## Reused operation and authority

The server reuses the [0.35 transfer operation](artwork-transfer-035.md), including
its server-owned app/launch-point catalogue, sealed source paths, bounded
nofollow reads, ordered chunks, digest binding and file/catalogue readback before
Complete. Historical operator authority and protected installed authority are
distinct internal choices. The operator path keeps its five-read limit; the
installed Proof path permits one read. The caller supplies an app ID and icon
variant, with no path, command, URI or decoder configuration.

Installed operations revalidate their protected installation and actual peer
before and after context/resource work. A separate installed reader revalidates
Proof authority around the shared file-read engine. Ordinary reads retain their
Root refusal. The fixed Core reader now admits its two catalogue GETs for the
exact protected Proof helper and matching Core pin, alongside the three existing
context GETs. Preview is excluded from this candidate.

## Worker provider

`InstalledProvider::start_worker` constructs the provider on its worker thread
and propagates initialization failure as a normal provider error. Each fetch
owns one fixed-command HBC completion thread and one socket exchange. The HBC
worker is joined on success and error paths. There is no resident Root daemon or
diagnostic timing hold in the provider.

Bytes are returned to the existing decoder only after Complete, matching
Finished/EOF, matching HBC completion and the Root receipt reconciliation. The
wire header lacks the folder and relative path components of the complete source
identity; the provider additionally matches the Root final record's before and
after SourceKeys against its requested job. It checks file identity and digest,
context/catalogue child reports and the actual session credentials. Unresolved
startup or child completion quarantines that provider instance rather than
immediately starting another helper. Completed typed resource refusals retain
the existing packaged-image/initials fallback.

`Runtime::with_fetch_budget` permits an explicit nonzero budget up to 30 seconds.
The ordinary direct-reader default remains two seconds. This changes the job
deadline supplied to the worker; it cannot interrupt blocked SDK/kernel/codec
work or guarantee a total join time, heap use or Root lifetime. The existing
worker/cache and main-thread texture ownership stay in place.

## Remaining admission

Host checks pass 387 library tests, five dashboard tests, warnings-denied Clippy
and 35 build/provenance/framework/package tests. New controlled provider checks
cover withheld Complete, absent Finished/EOF, full SourceKey drift, late
cancellation/expiry, typed and contradictory refusals, explicit/Drop worker
joins, unknown-child quarantine and parser isolation. Runtime tests exercise
actual worker deadlines and delayed/cancelled results with the bounded decoder.
These checks do not call HBC or establish device rendering through this provider.

ARM checks must pass before the combined Proof mode can be admitted.
That mode must exercise real live catalogue sources through this provider into
visible native textures, preserve focus and favourites, reconcile helper/child
completion, and verify full temporary removal. The shipping Preview profile,
hide/resume/relaunch behaviour and actual icon-arrival performance require a
separate backed-up delivery. A compiled provider alone does not qualify normal
icon display, all-app coverage or production startup.

Rendering, decoding, input and shared UI build on the explicitly attributed
[PlxNative framework](../README.md#plxnative-framework-dependency). This candidate
adds Luma dashboard code and preserves the framework dependency and licences;
it introduces no Plex client or playback features.
