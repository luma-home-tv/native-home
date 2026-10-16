# Native TV information — 0.56 source

General now includes three native read-only rows: **TV model**, **Software
version** and **webOS SDK version**. They display the existing collector's
`modelName`, `firmwareVersion` and `sdkVersion` fields. The SDK version is not
converted into a marketing webOS version, and no CPU, GPU or memory specification
is inferred.

The change adds no service request, permission, setter or persistence operation.
Each field requires a successful existing reply and a bounded printable value.
A missing or invalid field becomes unavailable independently. Failed TV-info
readings cannot inherit another General group's freshness; valid TV-info readings
cannot revive failed pointer or language settings. Complete General failure
withdraws its freshness and disables cached values until a new valid reading.

All three rows use the attributed shared native FormTable. They remain read-only
with no editor or activation, including when the existing pointer-speed editor
is available. Displayable model/version text grants no keyboard or settings
capability. LG's General entry remains available through its existing route.

Six parser tests and five integrated tests cover malformed/partial replies,
mixed failures, expiry/recovery, stable row identities, presentation context and
actual form behavior. Integration passed **627 library tests, eight UI tests,
64 Python packaging tests and Clippy**. The framework is unchanged.

The SDK ARM dashboard build also passed from source
`87945205e21393f5ef93d40a780b0c7e23d9acd8`, with all 114 selected source inputs
matched to that revision. Its SHA-256 is
`e556658ae1603f51218d8d87d9e30cdb2becab260d50199f2985f4f02f7aeac9`.
The existing unstable `neon` configuration warning is unchanged. The helper was
neither rebuilt nor run for this display-only change.

Native presentation remains unqualified. Neither TV has received this source.
The [G2 retained delivery](g2-retained-preview055.md)
remains 0.55, with its own frozen executable and evidence.
