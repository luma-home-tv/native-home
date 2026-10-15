# Framework inputs and build receipts

The PlxNative framework remains an explicitly attributed external dependency at
the revision in `dependency-lock.json`. The integration patch and each affected
file have separate SHA-256 pins. Run `prepare-framework.py` to prepare that exact
checkout, then use `build-client.py` for builds.

The wrapper verifies all upstream tracked blob contents and modes, all patched
file bytes, the patch mapping and hash, and the checkout revision before invoking
either the C compiler or Cargo. Unexpected files, including ignored local Cargo
configuration, are refused. The verifier only reads; it never fetches, resets or
repairs a checkout. It checks again after compilation and refuses changed inputs.
The root Cargo build script also verifies the framework, with dependency-directory
change tracking for cached builds. That script cannot run before all dependency
build scripts; the wrapper's initial check provides that earlier boundary.
Cargo decides when to rerun build scripts from its cache and modification times;
mtime-preserving edits or a restored cache can evade that trigger. Only the wrapper
provides a fresh check on every invocation.

Successful wrapper builds of a binary write an adjacent private `.build.json`
receipt containing its actual hash, source/assets/font input hashes, framework
verification and declared toolchain. Packaging requires an ARM receipt whose
inputs and binary still match. It also checks the copied executable and repeats
validation before creating the package. Receipts are private development records,
not app resources. Direct Cargo builds do not issue package-admission receipts.
These checks are not an atomic filesystem snapshot or a reproducible-build claim.
The compiler, SDK and full process environment are not hermetically attested;
toolchain fields describe the selected wrapper configuration.
They do not prove TV behaviour, physical input, firmware compatibility or speed.

## Historical formatting drift

While preparing 0.31.0, inspection found that an earlier `cargo fmt --all` had
formatted the external framework workspace too. The 0.29/0.30 builds had not run
the preparation verifier, so their declared framework pins did not describe the
actual dependency bytes compiled. Their frozen executables, package hashes and
bounded runtime observations remain valid; their source/dependency declarations
do **not** establish byte-exact reproduction of those executables.

The original checkout and diff were preserved privately before recreating the
pinned dependency. It had 620 tracked paths modified relative to upstream.
Comparing those paths with the correctly patched checkout, both non-Rust files
were byte identical. Canonical formatting made 617 of
618 Rust pairs byte identical. The remaining pair differed only in indentation
on five plain comment lines; a separate inspection records that residual. The
primary comparison retains its `inspection_required` result. This supports a
formatting explanation, not executable equivalence or a retroactive correction
of the old build inputs. Historical packages and receipts were not rewritten.

The new read-only form API is included in an explicit nine-file integration
patch with updated pins. Format only owned source files: avoid workspace-wide
formatting that traverses this external path dependency. Any future intended
framework change must update the attributed patch and lock before building.
