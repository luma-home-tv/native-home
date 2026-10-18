# Combined Home beta candidate

Functional source `adfa2ac53df1c6e76b38d9d36fa0973e2e2a9f9f` combines the
reviewed Home options, focus bookmark, Controls labels/task guides and offline
icon-acquisition tooling. Neither retained TV installation has received it.

## Intended behaviour

- Holding a Home favourite opens the same explicit options menu as Apps. A hold
  or its release does not immediately unpin or launch it. Back closes the menu.
- A deliberate Home selection can survive a normal exit and explicit fresh
  launch. The first fresh catalogue resolves the stable ID once; removed apps,
  an empty shelf and intervening user input have bounded fallbacks.
- Controls names LG-managed tasks more clearly, including timers, CEC,
  Bluetooth, programmes, app management and Memory Optimiser. These are existing
  category links or read-only guides, with no new setters or direct task APIs.
- The collector defaults to metadata only. An explicit offline acquisition
  flow can bind supplied public-icon bodies to full source keys, before/after
  catalogues and a pinned receipt, then package those exact bytes. It does not
  execute the reader, establish native closure or grant Root authority.

The black canvas and unboxed real app icons are unchanged. Favourites retain
their existing schema; focus uses a separate bounded `home-focus.json` sidecar
on the existing persistence worker. Home/boot handling remains under LG control.
Ordinary departure still ends Luma; reentry is an explicit launch through LG.

## Local evidence

The combined tree passes 836 library tests, 16 UI tests and strict all-target
Clippy. Python icon/contract checks ran 36 cases: 35 passed and one existing
private ARM-fixture case was skipped. The separate provenance suite passes all
11 cases. These are actual runs against the final 146-input tree.

The initial host attempt stopped at SDL linking because its isolated environment
omitted the real HOME needed by Homebrew. A read-only comparison identified the
cause; fresh successful runs inherited that unchanged value. No source,
dependency or system installation was changed to resolve it.

A fresh ARM UI build completed successfully in about 115 seconds. All 146
working source files, immutable Git blobs, saved source copies and receipt
entries agree. Source/framework snapshots and five compiler/tool fingerprints
are unchanged before and after compilation.

| Artifact | SHA-256 |
| --- | --- |
| Dashboard ELF | `6b752f23e71ba73d503f903002e99ca7078cda69cb0c577ef1dd131d726996f7` |
| Build receipt | `ebed469aa229217ce44b7839f46e9eb4eaf663daf2c1d2acbf14c762eb9c95e9` |

Independent artifact review and a byte-identical coordinator replay pass. The
replay denies network access throughout its process tree and checks all 398
review bindings before and after. Twenty-one adverse parser/source-count/path/
role/pin checks pass. The actual compiler output directory, compiled contract,
receipt and embedded semantic values agree. Its 11 direct libraries and 223
complete imported symbol/version/type/binding tuples add nothing over the exact
retained UI role. This is a direct-ABI check, not transitive target loading.

The framework revision and integration patch are unchanged. The 146 source
selection is the retained 143 paths plus exactly `src/control_tasks.rs`,
`icon_acquisition.py` and `tools/public-icon-reader/protocol.py`. The reader is a
separate primitive; compiling this dashboard does not bundle or authorize it.

Independent source review preserved two focus-startup prototype failures and a
receipt-presence bug that could treat a dangling receipt link as legacy assets.
Their corrected cases pass. The combined merge preserved both sets of Shell
tests; the first failed local merge-resolution assertion remains recorded.

## Next native sequence

1. Use a fresh C2 availability/context/lease and complete actual retained-state
   baseline. Establish the isolated app's actual native state access without
   permission repair. The separate one-icon reader can be qualified independently;
   it does not block testing a fixed-catalogue UI. Historical G2 results do not
   qualify either C2 operation.
2. Inspect a matching isolated ordinary interactive Proof package. Capture,
   observer and performance profiles exclude bookmark writes and cannot qualify
   this feature. The new test must account for both favourite and focus files.
3. In separate app-owned state, check explicit Pin, hold/menu/Back, deliberate
   selection, normal exit and a fresh process restoring the saved state. Remove
   the whole isolated identity and verify retained bundle/state preservation.
4. When the user is present, check physical pointer/wheel/hold and one LG
   keyboard. Then qualify ordinary app/Settings handoffs in their own scope.

No new native execution, rendering, icon decoding, physical input, performance
or retained promotion follows from the local results above. Packaged icons are
static ID/path fallbacks; live acquisition, refresh/version handling and broad
artwork coverage remain unfinished. Normal Root artwork stays off. C2 input
switching, storage-capacity semantics and comprehensive native control parity
also remain open.
