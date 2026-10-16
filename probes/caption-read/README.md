# Ordinary native caption-preference probe

A separate, disposable native application makes one fixed anonymous LS2 request:

```json
{"category":"caption","keys":["captionEnable"],"subscribe":false}
```

The URI is `luna://com.webos.settingsservice/getSystemSettings`. LG's
[Settings Service reference](https://webostv.developer.lge.com/develop/references/settings-service)
documents a String value and an `off` example. This does not establish UK DVB
subtitle semantics, per-player behaviour, a complete enum or write permission.
The probe has no window, input handling, settings worker, retry or fallback.
Its application identity is separate from the retained Luma preview.

It checks its installed identity and ordinary credentials before creating new
private output directories and `0600` Ready/Result files. It requests an
eight-second pre-call hold and a twenty-second post-result hold for independent
process and namespace observations. Those reports do not establish process exit.
The process disables its own core dumps and sets its own umask to `077`.

The LS2 client is the explicitly attributed, pinned **PlxNative** framework's
`plx_platform::webos::ls2` implementation. Its callback returns a lossy UTF-8
String. The retained-reply limit applies after allocation; the reply-wait budget
excludes setup, cancellation and unregistration. Drop exposes no unregister
success result and can produce a separate framework event log. Read the source
and preserve these boundaries when interpreting a result.

The build also links PlxNative's unchanged, attributed ARM32 `getauxval`
compatibility source, as the dashboard does. Current Rust's standard library
needs it with the older TV libc. It reads only the process's own auxiliary vector;
it is not used to make security decisions.

This is an independent Cargo workspace. Its lock uses a subset of the main
project's registry versions/checksums. The framework must first be prepared and
verified using the main project's documented procedure. `build-arm.py` verifies
the reviewed source manifest and lock before and after an offline ARM build.
`package-caption-read-proof.py` emits exactly three ordinary app files, with no
permissions, services or hooks. Both take explicit hashes and fresh private
output directories. Building or packaging does not admit TV deployment.

A device trial additionally needs a fresh device lease, exact retained-preview
and preference baselines, live ordinary-process/file bindings, independently
observed closure and complete supported removal. No TV operator, host address,
credentials, household data or automatic deployment is included here. A result
for this identity does not grant the retained Preview identity any capability.
