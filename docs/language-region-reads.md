# Language and region readings (0.30.0)

General now includes five fixed, protected readings: menu language, preferred
and secondary audio languages, TV country, and LG services country. Language
and country codes are displayed as returned, after bounded string/type checks.
They are not a complete locale registry or a list of supported choices. The
existing LG General overview remains the route for changes.

The value requests use LG's documented retail-TV API:
[Settings Service](https://webostv.developer.lge.com/develop/references/settings-service).
`localeInfo` must be requested without a category. `country` and
`smartServiceCountryCode2` require the `option` category. These are four fixed,
non-subscribing GET operations: two public value calls and two optional descriptor
calls through the previously used service alias. No arbitrary key/URI API or
setter is added.

A G2 root-side survey on webOS 24 accepted both value aliases. Its descriptors
contained an empty choice list or a country-file reference, not an admitted enum.
Luma neither fetches that file nor infers editing authority. Valid current values
remain visible if optional descriptors are absent, refused or unsuitable.
The later isolated native check independently accepted the public values under
an ordinary app UID; root-side reads alone do not establish that permission.

Each refresh supplies all five rows. Failed groups and invalid individual leaves
become unavailable without hiding valid siblings. Existing General groups cannot
make old locale values fresh. Expiry withdraws availability; every row keeps its
fixed focus identity, protected policy and disabled activation. No clock/timezone,
network, account, LG Home or OLED setting is changed.

Host checks: 207 library tests and 5 home-preview binary tests pass, including
partial-failure/expiry transitions and real FormTable disabled/route bindings.
All-target host clippy with warnings denied and the ARM release build pass (the
existing target-feature warning remains). These checks do not establish TV
rendering, ordinary UID access, physical navigation or comprehensive settings
coverage. The original isolated phase left normal G2 on 0.28.0. The subsequent
[backed-up normal delivery](retained-preview-030-milestone.json) installed 0.30.0,
visually matched all five rows to independent GETs, and preserved favourites.

The isolated diagnostic adds seven General captures using the existing shared
arrows and Back: each of the five new rows, the enabled LG General route without
activating it, and return to the first new row. It requires visible focused rows,
read-only bindings, unchanged preferences and zero TV action requests. The [native milestone](language-region-read-milestone.json) records successful
ordinary-app reads matching independent full locale/region objects, seven
Root-reviewed frames and all 24 preservation/removal checks. An independent
saved-artifact audit passed 277 assertions. The driver binds live UID/start/ELF
identity, but raw process reads were not retained for independent replay. These
are component read/layout checks; native expiry transitions, physical General
navigation remain open. Retained G2 delivery is recorded separately above.
The comprehensive control centre
remains incomplete.
