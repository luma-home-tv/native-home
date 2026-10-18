# Named LG task guides

[Draft PR12](https://github.com/luma-home-tv/native-home/pull/12) adds task-level
guidance after the existing General and Sound readouts at functional source
`d73e9dc10cae948ebb7127c22a8b97640a77bcf0`.

| Entry | Luma action |
| --- | --- |
| Time & timers | Existing General overview; row describes the remaining manual steps |
| HDMI-CEC (SIMPLINK) | Existing General overview |
| Bluetooth input devices | Existing General overview |
| Programmes & tuning | Existing General overview |
| Bluetooth audio | Existing Sound overview |
| App management | Read-only directions to LG Home's app-list editor |
| Memory Optimiser | Read-only directions to LG Settings; no optimiser operation |

The five links use the existing fixed actions and capability/context gates.
They do not jump directly to a task subpage. The two guides remain focusable
and legible without activating an operation. Existing rows, keys, getters,
editors, preference schema and service parameters stay unchanged. There is no
pairing, tuning, app deletion, memory optimisation or new native setter.
Storage capacity and storage management remain unfinished.

Directions draw on LG's generic webOS24 documentation, including
[Time & Timer](https://kr.eguide.lgappstv.com/manual/w24_mr2/global/Contents/settings/system/time_b_e_c_a_t/eng/w24__settings__system__time_b_e_c_a_t__eng.html),
[Bluetooth controllers](https://kr.eguide.lgappstv.com/manual/w24/global/Contents/settings/externaldevice/bluetoothcontroller_k_u_b_e_c_a_t_j/enga/w24__settings__externaldevice__bluetoothcontroller_k_u_b_e_c_a_t_j__enga.html),
[wireless audio](https://kr.eguide.lgappstv.com/manual/w24_mr2/global/Contents/settings/sound/wirelessspeaker_k_u_e_c_a_t_j_01/eng/w24__settings__sound__wirelessspeaker_k_u_e_c_a_t_j_01__eng.html),
[app-list editing](https://kr.eguide.lgappstv.com/manual/w24_mr2/global/Contents/smarttv/edithome_k_u_b_e_c_a_t_j/enga/w24__smarttv__edithome_k_u_b_e_c_a_t_j__enga.html)
and [Memory Optimiser](https://kr.eguide.lgappstv.com/manual/w24_mr2/global/Contents/settings/support/memoryoptimizer_k_u_b_e_c_a_t_j/eng/w24__settings__support__memoryoptimizer_k_u_b_e_c_a_t_j__eng.html).
Research used indexed primary excerpts; direct full-page opens failed. These
references do not establish the current menus, support or effects on either TV.
The original broader memory/storage label was corrected before integration.

The author ran 809 library and 15 UI tests plus strict Clippy. The title-only
correction passed four focused tests and strict Clippy. Independent source review
and 12 focused tests pass, including real-arrow access to read-only rows,
OK/Right/hold/refresh/page-change refusal, unchanged preceding rows/editors and
capability/context rejection before platform I/O. No standalone ARM build,
package or TV test is included.

## Combined UI source

The coordinator merged this source with Home menu and focus persistence in the
separate `codex/home-beta-integration` branch at
`3a8f5f4c7f9d9d8d3e66215a34f56856e88b5233`. Independent composition review
reproduces the exact Controls wrapper and preserves all 89 Home tests plus the
three new Guide tests. The initial merge-resolution assertion was too strict;
its failure and temporary uncommitted marker staging remain recorded. No
conflict markers were committed.

Actual combined checks pass 836 library and 16 UI tests, strict all-target
Clippy, the unchanged framework verifier and standalone dependency graph
(78 nodes, five framework crates, no excluded application crates). All 144
selected inputs and framework bytes match before and after. The first isolated
host compile failed to link SDL because its environment omitted HOME; a
read-only differential check confirmed Homebrew's requirement. A fresh host
run inherited the real HOME without changing source or dependencies and passed.
The original compile failure remains separate.

This is combined source/host evidence. A later merge of icon tooling, matching
ARM/package checks and device-specific ordinary validation remain separate.
Neither TV received this candidate.
