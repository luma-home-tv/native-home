# G2 retained native preview, 0.56

The G2 now retains **0.56.0**, with three native General readings: **TV model**,
**Software version** and **webOS SDK version**. The backed-up same-ID update from
0.55 completed two normal sessions and full known cleanup. The independent
saved-evidence audit and separate coordinator replay produced identical reports.
LG Home remains the system home and normal Root artwork stays off.

The [sanitized milestone](g2-retained-preview056-milestone.json) records the
artifacts and result. Functional source is
`87945205e21393f5ef93d40a780b0c7e23d9acd8`, with 114 selected build inputs;
the ARM executable SHA-256 is
`e556658ae1603f51218d8d87d9e30cdb2becab260d50199f2985f4f02f7aeac9`.
Only the executable and appinfo version changed. All 28 packaged snapshot,
icon, font and branding resources remained byte-identical. The exact current
0.55 rollback was verified and staged but unused; Preview was never uninstalled.

## Native observation

The first fresh process opened Controls, selected General with direction keys,
entered its form and scrolled to the new rows. The coordinator reviewed all
three complete labels and values together in a captured compositor frame. They
matched successful system-info reads before and after the frame and again at
confirmation. Each row displayed “View only”; no form row was activated.
Back returned to the category list, then Home, then exited normally.

The external reads establish agreement with the captured values. They do not
identify the exact asynchronous ordinary-app reply that populated each row.
A native terminal's live-update count alone was not accepted as visibility proof.
The [source record](device-information-056.md) describes parser, freshness and
shared FormTable checks; no new TV getter or setter was added for these rows.

A second fresh process completed Home → Apps → Home → Back exit. It did not
repeat the General, keyboard or idle workloads. Both sessions joined their
packaged-icon workers, balanced uploads/deletions (four then seven), left zero
textures and recorded zero icon backend errors. These counts exclude other
renderer resources and do not establish all-app artwork coverage.

There were 32 compositor frames, 28 SDK key requests and two accepted rail OKs.
One proposed OK was withheld when the fresh whole image changed; the coordinator
reviewed its replacement before proceeding. Accepted OKs required identical
whole BMPs less than five seconds old plus current actor/scene checks. These
sampled checks are not atomic input-targeting or physical-panel measurements.

## Preservation and limits

The full 32-entry/30-file installed bundle matched the expected 0.56 archive.
Original preference bytes, owners and `0700/0600` modes remained exact, with
zero native preference-write attempts. Five Core-file digests and 19 reply
domains matched before and after. These sampled checks do not prove every TV
setting or service. All staged packages and captures were removed, the owned
lease was released and the private host agent exited. Normal runtime logs and
session receipts remain; preexisting runtime bytes were not rehashed.

The first saved auditor remains exit 1: its operation window included the next
tree's inventory after the preceding tree had closed. The corrected reader
selects the actual surrounding inventories and retains complete member/hash
checks. No TV rerun or raw-evidence change was needed.

No settings, favourites, LG Settings, external app, keyboard or Root artwork
action was requested. C2 was not contacted. Physical input, native failure-state
presentation, normal performance, broader reliability and comprehensive control
coverage remain open. Earlier [0.55 app-launch evidence](g2-normal055-app-launch-return.md)
remains bound to its own executable.
