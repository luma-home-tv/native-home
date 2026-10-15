# Artwork broker startup design

Status: installed-source review, isolated read-only capability result and proposed
startup qualification sequence, 2026-10-15.
No production startup route is qualified on the G2. This document adds no TV
operation, service, permission, boot hook or deployment instruction.

Two serial [0.40 isolated installed startups](artwork-startup-040.md) qualify the
protected Proof helper's Identify/Finish and HBC completion. The first operator
receipt observation failed and separate recovery completed removal/preservation;
the second corrected driver passed. This is installed enrollment, with no resource or retained
loader qualification. The [installed design](artwork-installed-bootstrap-design.md)
records the protected policy placement required by the actual writable app tree.

The subsequent [0.41 nonce refusals](artwork-startup-refusal-041.md) have their
own completed native records. The [0.42 provider](artwork-installed-provider-042.md)
and [0.43 renderer](artwork-installed-renderer-043.md) now have source, host and
ARM build evidence; their combined installed-resource/rendering device run is
still pending. These later candidates do not promote the earlier startup result
into resource or production-loader qualification.

The isolated 0.35.0 component demonstrated five bounded Root-side icon transfers,
ordinary-process decoding and Finish/EOF in an operator-started session. It did
not establish unattended startup, normal-preview enrollment, recurring refresh,
GL texture upload or production reliability. Its SSH operator remains part of
that proof, rather than the proposed product architecture.

## Recommended boundary

Keep resource requests on a restricted local broker interface: an admitted app
ID and icon variant, bounded encoded bytes, authenticated caller/session,
independent catalogue admission, fresh context and explicit completion. Do not
introduce caller-selected paths, service URIs or commands into that interface.
The ordinary native app continues to render and decode; it receives no general
Root filesystem grant.

Startup is a separate trust boundary. Existing elevated Homebrew Channel (HBC)
can potentially launch a fixed installed broker on demand without introducing a
new Core, jail, Home, boot or LS2-policy change. However, its existing launcher
is a generic shell execution service. A literal fixed command reduces Luma's
exposed input surface; it does not turn HBC into a restricted launcher or remove
HBC from the trusted computing base.

## Verified source primitives

The following references were inspected upstream; they do not identify the
installed HBC version or prove that this G2's ordinary native caller may use them.
The HBC references below are pinned to `1a2b969e992617c9ba4b8a891542cb93a9e07b0a`.
A separate read-only G2 inspection found installed metadata version 0.7.3 and the
same boolean-only handler in its compiled bundle. The installed entry uses an
older platform activity-manager path; this is not full-source/build equivalence.
Its service bundle SHA-256 is
`f5a68b70d7a883e675f19d80ef07e276b336b74ffa48adab548f4a6252a1d011`.
Existing service files were UID/GID 1001 and mode 0777, with a Root-owned 0644
activation descriptor. These existing writable inputs remain part of the broad
HBC trust boundary; this work changes no ownership, modes or activation policy.

| Primitive | Verified source behavior | Consequence |
| --- | --- | --- |
| `luna://org.webosbrew.hbchannel.service/checkRoot`, payload `{}` | The handler responds with `returnValue` equal to the service's `runningAsRoot` boolean. | A concrete inert first capability check; no command execution is needed. Bare false reports that the service was not running as Root; it supplies no permission or missing-service diagnosis and authorizes no fallback. |
| HBC `/exec` | Accepts a `command` string and uses `child_process.exec`, returning stdout/stderr representations. | Existing bootstrap candidate, with a broad trusted shell carrier. |
| HBC `/spawn` | Accepts a `command` string and launches `/bin/sh -c`, streaming output and exit/close events. | Existing on-demand process carrier; no narrow broker policy or explicit cancellation handler is supplied here. |
| HBC `elevateService` | Calls its elevation tool for a supplied service ID. | Not a permission-free way to add a privileged service. |

Sources: HBC [Root status handler](https://github.com/webosbrew/webos-homebrew-channel/blob/1a2b969e992617c9ba4b8a891542cb93a9e07b0a/services/service.ts#L595-L598),
[execution handlers](https://github.com/webosbrew/webos-homebrew-channel/blob/1a2b969e992617c9ba4b8a891542cb93a9e07b0a/services/service.ts#L719-L791),
and [elevation endpoint](https://github.com/webosbrew/webos-homebrew-channel/blob/1a2b969e992617c9ba4b8a891542cb93a9e07b0a/services/service.ts).
The HBC [interface documentation](https://github.com/webosbrew/webos-homebrew-channel/blob/1a2b969e992617c9ba4b8a891542cb93a9e07b0a/README.md#luna-service)
also states that execution may be unprivileged when the device is not rooted.

The stock elevation tool rewrites service execution and LS2 roles, client/API
permissions and manifests; its grants include wildcard names and `all`. Those
files can be named for one app while still changing global LS2 authorization.
Do not use it under this design's unchanged-policy boundary. See
[service execution changes](https://github.com/webosbrew/webos-homebrew-channel/blob/1a2b969e992617c9ba4b8a891542cb93a9e07b0a/services/elevate-service.ts#L29-L57),
[role changes](https://github.com/webosbrew/webos-homebrew-channel/blob/1a2b969e992617c9ba4b8a891542cb93a9e07b0a/services/elevate-service.ts#L86-L146),
and [permission/manifests and service rescan](https://github.com/webosbrew/webos-homebrew-channel/blob/1a2b969e992617c9ba4b8a891542cb93a9e07b0a/services/elevate-service.ts#L217-L303).
Likewise, HBC's documented `init.d` mechanism is a boot hook and is excluded:
[HBC startup hooks](https://github.com/webosbrew/webos-homebrew-channel/blob/1a2b969e992617c9ba4b8a891542cb93a9e07b0a/README.md#features).

## Feasible choices

1. **Existing HBC, fixed launcher.** Qualify one literal startup command selecting
   only the admitted installed broker. Do not interpolate a path, PID, nonce or
   user text into shell input. The broker must establish its own authorization
   from kernel peer credentials, Root-owned installation pins and fresh context;
   an unprivileged caller's claimed hash or PID cannot enroll itself. Startup
   failure leaves ordinary/package artwork available and must not trigger SSH,
   another service identity, a policy edit or an alternate privilege route.
2. **Restricted HBC endpoint contributed upstream.** If the startup operation
   itself must avoid a generic command carrier, add a fixed operation such as a
   broker launcher with no command/path argument, Root-owned admission and
   authenticated caller/session binding. This is a proposed upstream change,
   not an endpoint found in current source. It would need its own compatibility,
   installation and rollback review.
3. **Ordinary packaged service.** Use package-scoped activation as worker
   lifecycle prior art. It does not supply Root icon access by itself. A new
   elevated service requiring LS2 configuration changes is outside this design.

No existing restricted HBC broker-start endpoint was found. If neither the
generic HBC bootstrap dependency nor an upstream HBC change is acceptable,
production privileged startup remains unavailable within these constraints.
Do not substitute a boot hook or broad elevation workaround.

LG documents service names prefixed by the app ID and commands private by
default in [services.json](https://webostv.developer.lge.com/develop/references/services-json).
webOSBrew documents native services as unofficial in its
[native development guide](https://www.webosbrew.org/pages/native-development.html#native-services).
These are packaging/lifecycle references, not a Root capability guarantee.

## PlxNative prior art and attribution

Luma's pinned framework is PlxNative revision
`d507c4f9ed5b4ecd88ce9b2e998c50da506e6271`, recorded in
[dependency-lock.json](../dependency-lock.json). Its
[HBC call adapter](../.dependencies/plx-native/rust-modules/platform/src/webos/jail_repair.rs)
uses anonymous LS2 registration and a reply-wait budget. Setup, cancellation
and unregister are outside that wait budget; the callback copies the full reply
before Luma applies its post-call JSON bound. This is not a hard lifetime or
allocation ceiling. The upstream file is
[jail_repair.rs at that revision](https://github.com/GLinnik21/plx-native/blob/d507c4f9ed5b4ecd88ce9b2e998c50da506e6271/rust-modules/platform/src/webos/jail_repair.rs#L13-L41).
That adapter is useful research into transport failure handling. Its jail-repair
operation is excluded here; it is not an icon launcher or qualified G2 startup.

A separate historical local checkout, inspected at
`4f9269ae584f7cbcdbceb35586ffc0704a878527`, contains package-scoped native storage
service work in
`/Users/ashhopkins/Projects/webos-native-home/ci/mkipk.py:84` and
`/Users/ashhopkins/Projects/webos-native-home/rust-modules/platform/src/storage_service/`.
Its descriptor is activation-only; `service.rs:26` explicitly refuses a
Root-launched process. It provides useful lifecycle/authentication ideas, not a
privileged resource implementation, and is separate from Luma's pinned
framework dependency. Do not import Plex/application features or silently treat
this different checkout as the pinned dependency.

PlxNative remains attributed under GPL-3.0-or-later as recorded in the dependency
lock and [Luma README](../README.md#plxnative-framework-dependency). HBC uses the
[MIT license](https://github.com/webosbrew/webos-homebrew-channel/blob/1a2b969e992617c9ba4b8a891542cb93a9e07b0a/LICENSE);
reuse of substantial code must preserve its copyright and permission notice.
Prefer a small, separately reviewed upstream contribution where useful.

## Next isolated qualification

1. From an ordinary isolated native app, call only HBC `checkRoot` with `{}`.
   Bind the actual caller UID/PID/start/ELF and model/firmware/context before and
   after. Preserve bare true/false, explicit refusal fields, registration/setup
   failure, timeout and malformed replies separately. Do not infer a particular
   missing-service or permission cause from an unrecognized error. The isolated
   0.38 source implements this single capability request on a joined worker;
   its [native G2 result](artwork-startup-capability-038.md) received a stable bare
   true reply with complete isolated cleanup and preservation. This establishes
   this status-method access, not a broker startup. No shell command
   is requested. LS2 may activate the existing HBC daemon to answer; this does
   not promise that no Root process can start.
2. Admit a fixed installed launcher/broker using Root-owned regular-file
   ownership, modes and build/image pins. UI-provided values are challenges or
   comparison data, never authorization. Create enrollment under Root authority;
   bind each accepted process and packet to the installed app and current
   session. The 0.35 Proof-only enrollment cannot simply be relabeled as normal.
3. Exercise a fresh on-demand start, a second independent session, duplicate
   start, stale caller, unavailable HBC and interrupted startup. Prove bounded
   ordinary responsiveness and clean idle exit. A successful HBC reply alone
   does not establish the expected Root child or a functioning broker.
4. Treat timeout or lost startup output as an unknown outcome. A process can
   outlive its initiating LS2 call. Reconcile known broker/child identities and
   independently verify absence before declaring cleanup. Never guess that a
   missing reply means no process started, signal an unbound Root PID, or delete
   uncertain running state. Preserve explicit recovery evidence when uncertain.
5. Only then connect a native refresh worker: bounded transfers/decodes,
   identity-based cache invalidation, generation checks rejecting stale results,
   fallback artwork, no blocking render-thread startup and separately qualified
   GL texture ownership/lifecycle. Preserve retained preferences and LG state.

Reuse isolated package/removal and preservation checks; keep normal retained
installations untouched. The earlier
[startup investigation](startup-lifecycle-investigation.md) only establishes
two bounded ordinary sessions and leaves the rare focus-loss cause unresolved.
It does not prove service startup, continuous focus or this broker architecture.
G2 evidence never establishes C2 compatibility; any separately authorized C2
work remains subject to the dated window in [AGENTS.md](../AGENTS.md).
