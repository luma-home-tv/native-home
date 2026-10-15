# Live app artwork: implementation and remaining integration

The retained preview still uses packaged artwork: `live::merge` restores those
paths by app ID and `Shell::load_icons` loads the first 64 initial applications.
The newer source adds a separate sealed artwork map, bounded worker/cache and
main-thread textures, deliberately enabled only in an isolated proof. It does
not yet provide all-app live artwork to the retained preview.

The [0.35 resource proof](artwork-transfer-035.md) transferred and decoded one
real icon from each of five observed installation classes. The
[0.36 lifecycle proof](artwork-live-036.md) then exercised one owned fixture on
C2 through same-path replacement, package update, removal and joined shutdown.
It did not use the root broker. Direct RGB comparison confirms unchanged header
and label pixels outside the updated fixture; the earlier missing-text reading
was incorrect. The [startup design](artwork-startup-design.md) records the
remaining production bootstrap boundary.

Available icons should preserve their aspect ratio, alpha and source shape on
black, without an added graphite plate. Only missing artwork uses the labelled
initials fallback. A focus-only outline communicates selection.

Existing G2 ordinary-app `listApps` evidence includes `folderPath`, relative PNG
`icon` and often `largeIcon`. Catalogue admission already joins registered launch
points and filters hidden/stub/own/invalid applications. Metadata access is
proved; reading other applications' image files inside the ordinary native jail
had not been measured at that planning point. No HTTP/icon service or permission
change was assumed.

The subsequent [0.32.0 passive survey](artwork-access-032.md) established one
developer-app read, while four sampled locations were absent inside the native
jail. Direct filesystem access is therefore insufficient as the demonstrated
complete solution. The separate broker proof addresses those reads; the owned-fixture worker
proof does not establish class-wide production access.

The next [Identify-only broker candidate](artwork-broker-identify.md) tests a
temporary operator-started native connection with kernel/process/context binding.
It has no resource operation or production startup mechanism. Its result must
precede any privileged artwork-read implementation.
The subsequent [fixed context reader candidate](broker-context-reader-034.md)
addresses the measured anonymous-registration refusal without changing the
existing context authorization or TV permission policy.

That candidate now has one native G2 Identify exchange with verified cleanup.
This admits further resource experiments; it supplies no icon access by itself.
The next implementation must derive sources from server-obtained app and launch
metadata rather than accept filesystem paths from the UI. Admit only registered
visible apps and the existing sealed root/app-ID/relative-component rules, with
descriptor-based regular-file reads and before/after identity checks. Send
bounded encoded data with declared length/digest and per-packet credential
binding; decode only in the ordinary worker using the prepared bounded decoder.
Keep the operator-started test path explicit until production startup is
separately qualified. Do not treat a cached development snapshot as live artwork.

## Admission experiment

After the retained 0.31 delivery, run a small separate Proof app on G2. Select one
already admitted application from each observed application-root class and attempt
bounded ordinary-UID file access. Record path-component/symlink behaviour,
permission errors, regular-file identity, encoded size, PNG dimensions and digest.
Keep IDs, raw paths, files and reports private. Use no root fallback, policy edit,
external request, app activation or account operation. Remove the Proof app and
verify normal preview, favourites, core/TV state and LG Home preservation.

The observed root classes are LG system, encrypted installed apps, developer
apps, system apps and the optional-content mount. They need separate evidence;
success in one class cannot authorize the others. Some valid icons use nested
relative directories, so a basename-only parser would omit real catalogue entries.

## Native implementation

Keep packaged `Application.icon` as a fallback. Put validated live artwork sources
in a separate typed map associated with `CatalogueReading`, keyed by stable app
ID, version and source identity. Accept only registered visible apps, a verified
root class, a folder belonging to that app ID and bounded relative components.
Reject absolute/escaping paths, URLs, control bytes and unexpected symlinks.
Use component-wise file access and verify regular-file identity around reads.
Unreadable artwork keeps its packaged image or initials.

The source now implements the worker in `artwork_worker.rs`, controller/cache
in `artwork_runtime.rs` and `artwork_cache.rs`, and main-thread ownership in
`artwork_textures.rs`/`artwork_gl.rs`. The attributed framework patch adds a
bounded decoder that enforces dimensions/pixel allocation before decoding.
The root provider and production startup still need separate integration.

Initial proposed limits are 1 MiB encoded bytes, 512px per axis, a small latest-work
queue and two pending results. Use a byte-limited artwork cache and visible/favourite
priority, with at most one upload per frame. Size the cache against the actual
visible grid and image resolution; configured limits are not measured RSS/GPU cost.
Prefer large icons when admitted; measure whether a bounded thumbnail is useful.
No persistent cache is needed for the first version.

Replace artwork without resetting focus, search/filter, order or favourites.
Keep an old valid texture while a replacement loads; avoid flushing all textures
on each catalogue refresh. Include file identity/content digest so unchanged paths
can change. Removed apps cancel pending work and evict their textures; stale
results cannot resurrect a removed card. Expired catalogues pause new reads and
retain their existing launch-admission withdrawal. Bound negative retries and
shutdown work. Current joining is cooperative: a blocked kernel read is not
hard-cancellable; do not claim a guaranteed shutdown bound.

## Parallel work and acceptance

- Coordinator: own G2 survey, source interfaces, live-catalogue/shell integration,
  main-thread texture updates and the final frozen candidate.
- Worker lane: typed source parsing, bounded file reads, work/cache state and
  meaningful host regressions in its owned module.
- Framework lane: bounded shared decoder API and generic integration patch,
  without touching Luma shell code.
- Review lane: path/allocation/stale-result/focus review and fixture analysis.

Agree the map/result interface first. Give each source file one owner and use
separate host build targets. Only the coordinator contacts TVs. The dated C2
window in `AGENTS.md` applies; it is not ongoing access after that cutoff.
Do not mutate code or patch pins while an admitted device candidate is running.

Host regressions must cover a newly installed app, same-ID version/path changes,
same-path file replacement, removal during decoding, malformed/oversized images,
path and symlink rejection, unreadable files, more than 64 catalogue entries,
unchanged polls without redundant decoding, and preserved focus/favourites.

Native qualification should use an isolated owned target with two icon versions,
installation/update/removal, actual frames, texture ownership/GL errors and bounded
memory/frame work. Household app launches are unnecessary. Then perform a separate
backed-up normal update. HDMI format detail, additional protected control readings,
physical input, current performance and full release acceptance remain open.
