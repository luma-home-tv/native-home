# Live app artwork: next implementation slice

The current live catalogue deliberately discards incoming icon paths in
`live::parse`. `live::merge` restores only previously packaged paths by app ID,
and `Shell::load_icons` loads the first 64 initial applications once. New apps
therefore lack artwork; updated apps can keep an old image. This falls short of
the intended complete native drawer and concept fidelity.

Existing G2 ordinary-app `listApps` evidence includes `folderPath`, relative PNG
`icon` and often `largeIcon`. Catalogue admission already joins registered launch
points and filters hidden/stub/own/invalid applications. Metadata access is
proved; reading other applications' image files inside the ordinary native jail
had not been measured at that planning point. No HTTP/icon service or permission
change was assumed.

The subsequent [0.32.0 passive survey](artwork-access-032.md) established one
developer-app read, while four sampled locations were absent inside the native
jail. Direct filesystem access is therefore insufficient as the demonstrated
complete solution. Resource-route qualification comes before the worker/texture
integration below; no developer-only substitute is admitted as completion.

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

Add one bounded I/O/decode worker in `app_artwork.rs`; keep every GL upload,
replacement and deletion on the main thread. The existing shared decoder can
allocate beyond the shell's post-decode 512px check. Add a generic bounded decoder
entry point to the attributed framework patch, enforcing dimensions/allocation
before decoding while leaving its existing default unchanged.

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
shutdown work; never make the UI wait indefinitely for the worker.

## Parallel work and acceptance

- Coordinator: own G2 survey, source interfaces, live-catalogue/shell integration,
  main-thread texture updates and the final frozen candidate.
- Worker lane: typed source parsing, bounded file reads, work/cache state and
  meaningful host regressions in its owned module.
- Framework lane: bounded shared decoder API and generic integration patch,
  without touching Luma shell code.
- Review lane: path/allocation/stale-result/focus review and fixture analysis.

Agree the map/result interface first. Give each source file one owner and use
separate host build targets. Only the coordinator contacts G2; C2 stays excluded.
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
