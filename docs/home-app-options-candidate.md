# Home app-options candidate

[Draft PR10](https://github.com/luma-home-tv/native-home/pull/10) builds on the
separate [Controls label candidate](https://github.com/luma-home-tv/native-home/pull/8).
Its functional source is `736ba42223d94a80c3dc21718eb7d9734f686b58`.
Neither candidate is installed on a TV.

Holding a favourite on Home now opens the existing shared app-options menu while
remaining on Home. The opening hold and its release do not unpin or launch the
app. Back or Close restores the same app identity; explicit Unpin closes the
menu and selects the adjacent favourite, or View all apps for an empty shelf.
Reordering uses the existing preferences action. Catalogue expiry, removal and
page changes retain their cancellation and action gates.

Only `src/shell.rs` and `src/shell_tests.rs` change. There is no new menu renderer,
preference field, service, worker or dependency. The pinned attributed framework,
integration patch and standalone dependency graph are unchanged.

## Verification

The author ran 75 Shell tests and strict all-target Clippy. An independent
review ran 81 Shell tests, including six additional cases covering adjacent
focus, stable identity after rename/reorder, stale activation, page changes,
one explicit held-button commit and persistence through a source restart.
The independent review found no blocker within that source/host scope.

An actual ARM UI build completed from the frozen 143 selected source inputs:

| Artifact | SHA-256 |
| --- | --- |
| Dashboard ELF | `4b31c12538fbca1df3937f52f91a04bef730d1f88b9545d923043361dc057ebf` |
| Build receipt | `37a85fd280867b2c3708917de6df0286a71abd6f546024b5e03de05a05d17f2f` |

Source/framework/compiler fingerprints matched before and after the build.
The build used fresh outputs and produced no package or target execution.
The initial compilation failure from a discarded extra model field and the
review's corrected held-button expectation remain in the private evidence.

The shared menu's appearance over Home, physical hold/release, and persistence
on the current TV build still need native qualification. Build success and
host focus tests do not establish those results or performance. This candidate
does not change the retained dashboard or the existing disposable Proof artifact.
