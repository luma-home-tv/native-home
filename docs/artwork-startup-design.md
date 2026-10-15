# Artwork broker startup design

Status: source research and proposed qualification sequence, 2026-10-15.
No production startup route is qualified on the G2. This document adds no TV
operation, service, permission, boot hook or deployment instruction.

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
The upstream `main` URLs can change and must be pinned before implementation.

| Primitive | Verified source behavior | Consequence |
| --- | --- | --- |
| `luna://org.webosbrew.hbchannel.service/checkRoot`, payload `{}` | The handler responds with `returnValue` equal to the service's `runningAsRoot` boolean. | A concrete inert first capability check; no command execution is needed. A false result means Root capability is unavailable, not permission to escalate or fall back. |
| HBC `/exec` | Accepts a `command` string and uses `child_process.exec`, returning stdout/stderr representations. | Existing bootstrap candidate, with a broad trusted shell carrier. |
| HBC `/spawn` | Accepts a `command` string and launches `/bin/sh -c`, streaming output and exit/close events. | Existing on-demand process carrier; no narrow broker policy or explicit cancellation handler is supplied here. |
| HBC `elevateService` | Calls its elevation tool for a supplied service ID. | Not a permission-free way to add a privileged service. |

Sources: HBC [Root status handler](https://github.com/webosbrew/webos-homebrew-channel/blob/main/services/service.ts#L550-L553),
[execution handlers](https://github.com/webosbrew/webos-homebrew-channel/blob/main/services/service.ts#L658-L732),
and [elevation endpoint](https://github.com/webosbrew/webos-homebrew-channel/blob/main/services/service.ts#L792-L820).
The HBC [interface documentation](https://github.com/webosbrew/webos-homebrew-channel/blob/main/README.md#luna-service)
also states that execution may be unprivileged when the device is not rooted.

The stock elevation tool rewrites service execution and LS2 roles, client/API
permissions and manifests; its grants include wildcard names and `all`. Those
files can be named for one app while still changing global LS2 authorization.
Do not use it under this design's unchanged-policy boundary. See
[service execution changes](https://github.com/webosbrew/webos-homebrew-channel/blob/main/services/elevate-service.ts#L29-L57),
[role changes](https://github.com/webosbrew/webos-homebrew-channel/blob/main/services/elevate-service.ts#L86-L146),
and [permission/manifests and service rescan](https://github.com/webosbrew/webos-homebrew-channel/blob/main/services/elevate-service.ts#L217-L303).
Likewise, HBC's documented `init.d` mechanism is a boot hook and is excluded:
[HBC startup hooks](https://github.com/webosbrew/webos-homebrew-channel/blob/main/README.md#features).

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
uses anonymous LS2 registration and a bounded call. The upstream file is
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
[MIT license](https://github.com/webosbrew/webos-homebrew-channel/blob/main/LICENSE);
reuse of substantial code must preserve its copyright and permission notice.
Prefer a small, separately reviewed upstream contribution where useful.

## Next isolated qualification

1. From an ordinary isolated native app, call only HBC `checkRoot` with `{}`.
   Bind the actual caller UID/PID/start/ELF and model/firmware/context before and
   after. Preserve true, false, permission refusal, missing service and timeout
   as distinct observations. No shell command is required for this first step.
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
