# TV caption preference — 0.57 source

Accessibility adds a protected **TV caption preference** row using the shared
native FormTable. It has no activation, editor, write constraint or setter.
The existing ordinary read worker makes one additional fixed public
`com.webos.settingsservice/getSystemSettings` request per collection:
`{"category":"caption","keys":["captionEnable"],"subscribe":false}`.
No additional worker, subscription, privileged helper or fallback API is added.
The additional sequential wait can increase collection time; no performance
improvement or fixed end-to-end timeout is claimed.

The successful typed caption envelope must contain only the expected fields,
the single requested setting and a documented `on` or `off` string. An optional
empty dimension is accepted. Missing, refused, malformed, unknown or stale
readings show **Unavailable**, never an inferred Off. The collector's existing
JSON Value parser cannot preserve duplicate-key information; its reply cap is
postallocation, not a transport memory limit.

Every refresh replaces all eight Accessibility readings. A successful caption
read cannot revive failed legacy readings, and the reverse is also true. Complete
failure withdraws the group's freshness. Existing 15-second expiry and current
presentation-context checks still apply. Presentation context does not establish
settings-database scope, active programme, UK/DVB subtitles, app subtitles or
caption rendering. LG Settings remains the managed route.

The [isolated ordinary GET](caption-read-native.md) establishes compatibility for
its separate probe identity only. Normal Preview permission, actual row
presentation, native failure/expiry behavior and C2 remain separate qualification
work. The retained G2 is still [0.56](g2-retained-preview056.md).

Eight new tests cover the fixed request, typed values, malformed/mixed failures,
independent withdrawal, first all-failed presentation, expiry and the actual
shared FormTable's value/context/no-activation behavior. Root ran the focused
eight, all **635 library tests**, **eight UI tests**, **64 Python build/packaging
tests** and Clippy with warnings denied. The pinned, explicitly attributed PlxNative framework is unchanged.

The [LG Settings Service reference](https://webostv.developer.lge.com/develop/references/settings-service)
documents the caption category and `captionEnable` string values. That reference
alone does not map this preference to UK broadcast or application subtitles.

The SDK ARM dashboard build passed from functional source
`da2d9683b8c7df8a0bf4064f83c2ee4b1915f9d0`, with all 117 selected inputs
matched to that Git revision. Its ELF SHA-256 is
`a85b302d6bb371f2e82a93b071c0ab7fd8c2789f93f85053b96690e5e1475590`.
The existing unstable `neon` warning remains. The normal 0.57 package preserves
all 28 packaged resources from actual 0.56; only the dashboard ELF and appinfo
version change. It requests no additional permissions or services. Local build
and archive checks do not establish TV access or native presentation. An
independently authored archive reader, executed by the coordinator, matched the
complete package and all 117 inputs at the frozen source revision; its
[scoped record](caption-read-057-milestone.json) preserves the hashes.
