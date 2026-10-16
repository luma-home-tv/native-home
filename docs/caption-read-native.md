# Ordinary native caption-preference read

A separate ordinary native G2 application successfully made one fixed public
caption GET and was completely removed. The retained **0.56 preview**, original
preferences and sampled TV state were preserved. Independent saved-evidence
audit and coordinator replay produced identical reports. This is API-access
evidence for the new test identity, not a new retained-preview capability or UI.

The source is the independent [caption probe](../probes/caption-read/README.md),
frozen at `a9dbb9535474f88277fb994d3e5f4b66d25fab67`. It directly uses the
attributed PlxNative LS2 client. Its request is
`com.webos.settingsservice/getSystemSettings` with category `caption`, key
`captionEnable` and `subscribe:false`. LG documents this stored closed-caption
preference as a String with `on`/`off` values in its
[Settings Service reference](https://webostv.developer.lge.com/develop/references/settings-service).

## What was measured

Fresh kernel observations bound the new ordinary caller's PID, start, credentials,
ELF and jail. Live host and `/proc/PID/root` views identified the same private
output directories. Two stable reads through both views corroborated each
immutable `0600` Ready/Result file beneath its own `0700` parent and PID directory.
The native directory identities and link counts matched the external metadata.

Plain anonymous registration succeeded. The source invoked exactly one fixed
GET and returned a successful, duplicate-free caption String envelope, independently
reparsed from the client's retained text. It did not attach an application ID,
retry, use a fallback alias or invoke a Root broker, HBC, setting write, input
handler or preference operation. No separate framework log directory appeared.

The app created no SDL window. Paired system observations showed LG Home with
its screensaver; one result sample also contained LG's launch splash. Final state
returned to LG Home with the screensaver. This does not establish zero visual
effects or a caption-rendering result.

## Removal and preservation

The original SDK installation reached its installed terminal. After the measured
process and complete UID were absent, one SDK removal was requested. Its
acknowledgement was followed by typed catalogue/launch-point, bundle, jail, state,
mount, PID and UID absence checks. Only the positively mapped reports and empty
directories, plus the original staged package, were removed. Final full
32-entry/30-file preview hashes, private preferences/metadata, normal runtime
membership, five Core-file digests and 19 reply domains matched. Old preview
runtime file bytes were not rehashed. The lease and private host agent closed.

The initial SDK build failed to link `getauxval`; the successful successor
links PlxNative's unchanged, attributed ARM32 compatibility source. The first
saved native auditor remains failed: a suffix glob also matched earlier polling
records. Its successor selects the exact decisive closure record and preserves
the polling history. Neither correction required repeating the TV run.

## Boundaries

The LS2 wrapper supplies a lossy UTF-8 String, not original wire bytes. Its
retained-reply limit is applied after allocation, and its wait budget excludes
setup, cancellation and Drop. Successful unregistration and the OS exit status
were not measured; external process/UID absence was measured separately.

This result does not map the key to the updated UK TV's Subtitles control or
prove DVB, content-app or currently rendered captions. No setting was toggled.
An ordinary caller result for this test identity does not admit Preview's
identity. C2, physical input, performance and a normal dashboard caption row
remain unqualified. The [sanitized milestone](caption-read-native-milestone.json)
records artifact hashes and the independently replayed scope.
