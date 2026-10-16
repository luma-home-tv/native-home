# Drawer empty states — 0.55 source

An empty drawer previously always said “No matching apps,” including an empty
Favourites filter without a search. The next source version distinguishes the
reason and points to existing controls.

| Empty state | Heading | Recovery hint |
| --- | --- | --- |
| Favourites without a query | No favourites yet | Choose All apps, then hold OK on an app to add it. |
| Favourites with no search result | No matching favourites | Choose All apps to search the full list. |
| Ordinary search miss | No matching apps | Change your search or clear it with Keypad. |
| Empty catalogue without a failed/stale reading | No apps to show | Use the sidebar to return Home. |
| Failed/stale catalogue with no shown cards | Apps unavailable | Use the sidebar to open Home, Inputs or Controls. |

All apps changes the favourites filter while preserving the query. Back ends
editing and preserves the query. Failed readings retain cached cards for
inspection; a missing current result does not grant launch authority. This change
adds transparent native labels, with no new card, focus target, worker, action or
preference write. It uses PlxNative's attributed shared Label component.

Four actual Shell fixtures passed first in an isolated source/framework copy.
Integration passed 616 library and eight UI tests, 64 Python packaging tests and
Clippy. A final hint-only wording change passed the four focused fixtures and
Clippy again. The upstream framework remains byte-identical to its prior pinned
working tree after unrelated formatter changes were restored and verified.

This is host logic/source validation. ARM, native empty-state typography and
retained delivery remain separate; neither TV has received this source version.
The [0.54 performance](current-performance-054.md) and
[keyboard](keyboard-observer-054.md) observations stay bound to their own frozen
executable.
