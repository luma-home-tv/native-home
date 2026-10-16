# G2 retained native preview, 0.55

The G2 now retains the ordinary **0.55.0 preview**. Its backed-up update from
0.47 completed three fresh normal sessions, exact preference preservation,
checked TV-state preservation and temporary-file/lease cleanup. LG Home remains
the system home. Normal Root artwork remains off.

The [sanitized milestone](g2-retained-preview055-milestone.json) records the
frozen artifacts and bounded result. The independent saved-evidence audit passed;
a separate coordinator replay produced the same report bytes.

## Delivered and observed

The executable was built from functional source
`46f431cd3f601810a41d76c929b9d7392d223ae1`, with 111 selected inputs. Its SHA-256
is `9acd524e84a3f5c2781fb472d4766016516fca4d287e3054c9185934da31617a`.
The supported same-ID update preserved all 28 packaged snapshot, icon, font and
branding resources. Only the executable and appinfo version changed among the
30 installed files. The exact 0.47 rollback was verified and staged but unused;
the retained app was never uninstalled.

1. Home → Apps → Home, then a 125.516-second no-input hold, black idle capture,
   one Up wake and natural Back exit.
2. A fresh process repeated Home → Apps → Home and natural Back exit.
3. A third fresh process opened empty Search and LG's system keyboard. One
   injected Back closed it, with no second keypad visible, followed by Home and
   natural Back exit. The native counters recorded one open, stop and dismissal,
   zero failures, no text recipe and no observer mode.

Root reviewed captured Home/Apps frames with Luma branding and available icons
directly on black. The idle compositor image had dark fraction 1.0 and mean RGB
0.0. This is framebuffer evidence, not panel power or physical-input measurement.
Each session joined its packaged-icon worker, deleted its seven uploaded textures,
left zero handles and recorded zero backend errors. These counts exclude other
renderer/branding resources and do not establish all-app artwork coverage.

There were 27 saved compositor frames, 21 SDK key requests and four accepted OK
requests. One proposed OK was withheld when the fresh whole image differed; Root
reviewed the replacement frame before continuing. Accepted OK requests required
an identical whole BMP less than five seconds old plus current process/scene
checks. These are sampled guards, not atomic focus/key-delivery guarantees.

## Preservation and closeout

Fresh before/after reads matched the full 32-entry installed bundle against its
expected version and all original preference bytes and private ownership/modes
(`0700/0600`). No preference content write was queued. Five Core-file digests and
19 reply domains matched. This is a sampled preservation check, not proof about
every TV setting or service. Normal runtime logs and new session receipts remain
intentionally; preexisting runtime file bytes were not rehashed or claimed equal.

All owned capture/staging files were removed, the original coordination token
was released and the private host agent exited. No Root helper, boot hook,
system/jail-permission change, viewing-account action or TV-setting write was requested.
The attributed PlxNative framework remains unchanged.

The earlier operator remains **exit 1**: it refused Home with LG's screensaver
before any installation, launch, input or staging. Its unused owned lease was
closed separately with exit 0. A new run allowed exactly Home plus the optional
LG screensaver; no screensaver configuration changed. The first saved auditor
also remains failed: its suffix glob included similarly named cleanup records.
The corrected reader anchors the exact label and retains all raw checks; no TV
rerun was needed for that saved-evidence correction.

## Remaining scope

This qualifies the bounded normal delivery and empty Search lifecycle. It does
not qualify physical typing on this executable, app-launch/return, settings
handoff, favourite mutations, performance, C2 behavior or Root artwork. The
[0.54 human typing result](keyboard-observer-054.md) remains bound to its own
executable. The [new empty-state copy](drawer-empty-states.md) was not exercised
with every empty/error state on the TV. Broader controls, HDMI/video context,
reliability and concept fidelity remain in the [delivery plan](parallel-development.md).
