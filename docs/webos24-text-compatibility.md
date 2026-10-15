# webOS 24 text-input compatibility

This records the SDL ABI and text-event findings behind Luma Home's native search
field. The audited platform is webOS 24 / SDK 9.2.2, firmware 23.25.55, on the
OLED55G26LA. The OLED65C26LD independently returned the same SDL library size and
SHA-256 as the G2. That establishes matching library content for this ABI audit;
it does not establish matching keyboard visibility, physical input, dismissal,
application lifecycle or performance on the C2.

The implementation snapshot is `760db0e`. Its combined ARM build and isolated G2/C2 shared-controller trials passed; the
[scoped integration record](keyboard-integration-milestone.json) contains the
counts and closeout boundaries. This does not claim a normal-preview deployment
or a complete composing-language test. The earlier
[text-input milestone](text-input-milestone.json) records the separately scoped
G2 system-keyboard lifecycle and human typing observations, along with its
retained-preview boundary and cleanup evidence.

## Two event types, two native layouts

The running library reports SDL **2.0.14**. Its text-input and preedit records do
not share the layout implied by the newer build SDK's headers. Offsets below
are byte offsets within the corresponding event, not addresses in a library.

| Native event | Audited layout | Luma decoding |
| --- | --- | --- |
| `SDL_TEXTINPUT` | Opaque `inputSource` at +12; inline 32-byte text buffer at **+16** | SDK-typed text-input field, with a compile-time offset assertion |
| `SDL_TEXTEDITING` | Inline 32-byte text buffer at **+12**; signed `start` at **+44** and `length` at **+48**; no source field | Explicit 52-byte legacy record, static size/offset assertions and `memcpy` into aligned storage |

The distinction was checked against the actual G2 runtime library's event
writers, rather than inferred from an exported symbol or SDL version string.
The newer SDK's editing record includes an extra source field and would shift
this runtime's preedit text and ranges. The
[C adapter](../sdl-events.c) therefore accepts native preedit only when the
runtime reports exactly 2.0.14. Unfamiliar native versions reject that event;
they do not fall back to the build SDK's editing layout. This is a narrow gate
for the audited deployment, not a claim that every vendor build identifying
itself as 2.0.14 has this ABI.

Desktop decoding continues to use the actual host SDL fields. Where supported,
its extended editing event copies a bounded payload and releases SDL's owned
pointer. The native adapter does not dereference an unqualified extended-event
pointer. C requires an in-buffer NUL terminator; the
[Rust boundary](../src/runtime.rs) validates UTF-8 before exposing text to the
draft. Raw text is excluded from event serialization.

## What the component tests establish

Queue-free raw fixtures reproduced the old SDK-shaped interpretation turning
`Écran` into `an`. After the correction, five focused decoder/injection tests
passed. They cover complete UTF-8 text, empty preedit, signed ranges, an unaligned
minimum-size record, the 31-byte payload boundary, unknown versions, short
records, missing termination and invalid UTF-8. The C adapter also compiled for
the native target with warnings denied. These are ABI/component results; they
do not prove a composing language works on a television.

The isolated proof can inject synthetic native text and preedit through SDL's
queue. Preedit injection constructs the audited legacy record, requires runtime
SDL 2.0.14, and accepts at most 31 UTF-8 bytes without an embedded NUL. Its
[window method](../src/runtime.rs) also requires the proof identity and an owned
text-input session. The host injection branch always returns false without
calling SDL's queue. No synthetic host text events are pushed through
`sdl2-compat`; fixtures exercise decoding directly instead.

A successful synthetic preedit/clear trial would establish delivery, decoding
and separate rendering for those injected events. It would not establish what
LG's physical IME actually emits or how prediction, composition and deletion
interact. Combined native trial results belong in their own dated receipt.

## `inputSource` is opaque metadata

In the audited text-model commit callback, SDL forwards the protocol argument
named `serial` unchanged into `SDL_TEXTINPUT.inputSource`. The callback does not
hard-code 35. An older PlxNative trace observed a value of 35 on another firmware;
that observation is not a portable keyboard-source enum or a prediction marker.
The audited activation path supplies an incrementing application-local protocol
serial. The separate EVDEV text-emission path passes zero, but hardware routed
through the system IME need not follow that path.

Consequently, neither `source == 35`, `source != 0`, panel ownership nor a
multi-character commit safely distinguishes a prediction from a literal batch.
[Normal Luma search](../src/bin/home-preview.rs) currently uses literal insertion.
The [draft](../src/drawer.rs) preserves commit boundaries and keeps a separate,
explicit word-replacement operation; enabling it needs an independently
verified prediction contract. A hardware-keyboard or injected batch must not
silently replace the preceding word.

## Ranges, languages and rendering remain separate work

The text-model protocol has distinct preedit, cursor and delete-surrounding
operations. In the audited library, delete-surrounding can also produce an empty
`SDL_TEXTEDITING` event carrying a signed index and length; a special sentinel
case produces a key event. These ranges must not automatically be treated as a
normal preedit selection. The current consumer displays the preedit string
separately; it does not establish composition-caret positioning or apply the
full protocol's surrounding-text edits. Actual composing-language traces and
expected draft/readback checks are still needed.

The bounded draft, grapheme-aware navigation/deletion, NFC-normalized matching
and clipped native field solve different problems from font coverage, glyph
shaping, bidirectional layout and language-specific IME semantics. NFC plus
lowercasing is not full Unicode case folding. Correctly received CJK, combining
or emoji text does not establish that its glyphs render correctly. Before,
preedit and after are independently measured runs; shaping across their
boundaries remains unqualified. Long-query geometry tests do not establish TV
GPU limits, sustained performance or real-font overhang fidelity. An indivisible
grapheme exceeding the field's upload budget is left unpainted while the draft
is retained; it is never split merely to fit a texture.

## Attribution and ownership

[PlxNative](https://github.com/GLinnik21/plx-native), by Gleb Linnik and contributors,
provides the pinned native rendering/UI framework and valuable prior research
into LG keyboard activation, event layouts, prediction behaviour and host
injection pitfalls. Luma's audit checks the newer target rather than transferring
older firmware observations unchanged.

Generic `TextBuffer` and `TextEntry` improvements remain in the explicitly
attributed [framework integration patch](../patches/plxnative-framework-integration.patch),
with the original authorship and GPL-3.0-or-later licence preserved through the
[dependency lock](../dependency-lock.json). They are candidates for a separate
upstream proposal, not changes claimed to have been accepted upstream. The SDL
adapter, runtime window ownership, compatibility gates and dashboard search
controller are Luma-owned integration code. This uses the framework without
bringing Plex client, account, server or playback features into Luma Home.
