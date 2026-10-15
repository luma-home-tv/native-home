# Ordinary-app artwork access on G2

The isolated 0.32.0 probe tested bounded image-file access under an ordinary
native app UID on webOS24. It attempted one selected source per observed root
class, using real registered/visible app metadata already returned by LG's app
and launch-point APIs. It added no service request, setting write, root fallback,
policy edit or production loader.

| Sample class | Ordinary-jail result |
| --- | --- |
| LG system | Directory not found (`ENOENT`) |
| Encrypted installed apps | Directory not found (`ENOENT`) |
| Developer apps | Read succeeded: 3,518 bytes; 160×160 PNG header |
| System app updates | Directory not found (`ENOENT`) |
| Optional content | Directory not found (`ENOENT`) |

The successful sample's reported digest, length and header match the known icon
in the saved normal 0.31 package. This corroborates that sample without a new
root-side file read. The other four observations concern those paths in the
ordinary jail; they are not class-wide permission denials or proof that every
possible resource route is absent. No image was decoded by this survey, and
header recognition does not establish valid image content or rendering.

This result changes the implementation plan: direct filesystem reads alone do
not establish a complete live-artwork solution. Do not promote a developer-only
loader as the finished drawer. Keep packaged artwork/initials available while
investigating a route for the missing resource locations.

## Implemented boundaries

`app_artwork.rs` validates catalogue admission, exact app-owned folders under
five known roots, versions and bounded relative icon components. Source values
cannot be constructed from arbitrary caller paths. The reader uses component-wise
no-follow opens, rejects special/empty/oversized files, limits encoded data to
1 MiB and checks descriptor/path identity around the read. The one-shot survey
lives only in `native_probe::collect`; the normal recurring collector does not
perform it. Byte bounds do not guarantee a filesystem wall-clock deadline.

The attributed PlxNative integration patch also adds an opt-in bounded image
decoder. Eight host tests cover real PNG/JPEG decoding, exact 512×512/1 MiB output,
overflow, high-bit-depth native-buffer bounds and refusal before pixel decoding.
Its default decoder is unchanged. Strict dimensions/per-image buffer checks and
codec limits are not a hard total-heap/OOM guarantee. This API is prepared for
the future worker; this native survey did not exercise it.

Checks passed: 220 application tests, five binary tests, 27 host harness tests,
all-target host clippy with warnings denied, the eight dependency decoder tests
and ARM release compilation. The existing target-feature warning remains. The
independent saved audit verified 68 committed build inputs, ten framework patch
pins, six raw process records and all 24 preservation/removal checks. Normal G2
remains on 0.31.0 with its appinfo, executable, favourites and metadata preserved.
LG Home and checked core/settings/input/audio state are unchanged. The Proof app,
temporary upload, lease and private SSH agent were removed; C2 was not contacted.
The [milestone](artwork-access-032-milestone.json) binds exact artifacts. Raw paths,
inventories, images and device details remain private.

## Resource-route research

No qualifying cross-root image-byte accessor has been established in the inspected
interfaces. Homebrew Channel's [`getAppInfo` handler](https://github.com/webosbrew/webos-homebrew-channel/blob/1a2b969e992617c9ba4b8a891542cb93a9e07b0a/services/service.ts#L695-L710)
returns application metadata. It does not return installed icon bytes. LG's
[ordinary JS-service guide](https://webostv.developer.lge.com/develop/guides/js-service-usage)
also describes filesystem restrictions; merely adding a standard service does
not demonstrate access to these resources.

The open-source [OSE application manager](https://github.com/webosose/sam/blob/master/src/bus/service/ApplicationManager.cpp#L459-L509)
returns metadata for `getAppInfo` and restricts `getAppBasePath` to the caller's
own app. These are research references, not retail webOS24 capability proof.
LG's [Connect SDK launch-point API](https://github.com/ConnectSDK/Connect-SDK-Android-Core/blob/master/src/com/connectsdk/service/WebOSTVService.java#L2163-L2185)
provides another lead through SSAP. Its local resource access, authorization and
retail behaviour remain unqualified; do not invent resource URLs, reuse unrelated
credentials, pair automatically or change mobile/network settings.

Next, qualify an existing bounded resource route if available. If a separate
privileged icon provider proves necessary, first design a restricted app-ID-based
read interface and validate its capability, context, byte limits and cleanup in
isolation. The dashboard must never expose arbitrary commands, paths or URIs.
The live worker, texture cache, new/replaced-app rendering and complete artwork
coverage remain unfinished, alongside the broader dashboard acceptance work.
