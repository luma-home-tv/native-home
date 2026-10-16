# C2 retained installer check and explicit recovery, 0.47

The first installer-only check under the retained Preview identity **exited 1**
and entered HOLD. A separately reviewed recovery **exited 0** and restored the
complete original 0.20 installation and saved state. The recovery does not turn
the failed check into a passed roundtrip. The [sanitized milestone](c2-retained-installer-047-milestone.json)
records both executions and their saved-evidence audits separately.

The frozen executable came from source
`be6c06a341d72be1d06e92819a1be6c71ed52048`, with 101 selected build inputs. Its
SHA-256 is `72436fbe8717d7a65fa915ca1569ba1b0337cfec944ff348b1cf65e3742f1feb`
and its size is 3,879,936 bytes. Neither operation requested an application
launch, key input, native migration call, setting write, retained uninstall or
direct `chmod`/`chown`. No application runtime or rendering result is claimed.

## The installer succeeded; the coordinator's metadata expectation failed

The private inert package was derived from the frozen forward-migration package
by changing only its marker header size/checksum and one padded marker data
block. Its fixed, unsupported marker was deliberately unsuitable for native
migration. The executable, application metadata, branding image, all non-marker
uncompressed tar bytes and original control/debian members were preserved. The
data container was recompressed; the package contained no helper, service or hook.

The original SDK subscription reached a known installed terminal without local
observation failures. Independent full readback found all four regular files
with the exact intended bytes, ownership and executable/resource modes. The
only difference was the **existing application bundle directory**: the archive
requested `0755`, while the installed directory remained `0777`. The coordinator
had incorrectly expected archive and installed root-directory modes to match.

That mismatch triggered the sticky HOLD barrier. The failed coordinator did not
request an exception-triggered rollback, delete its staging uploads or release
the device lease. It saved read-only observations and cleaned up its private
host agent. Preference bytes, ownership and modes, five Core hashes and the 20
checked preservation domains matched the pre-install baseline. Runtime folder
membership matched; this does not establish continuous no-autolaunch tracing.

## A separate, positively admitted restoration

Recovery did not retry the failed workflow or infer safety merely from the
absence of a process. Its separate admission rechecked the original unexpired
lease with the **same token and unchanged expiry**, quiet LG Home, ordinary
process absence, all four current inert files, the original state, Core/domain
readbacks, runtime observations and the owned staging directory. It neither
acquired nor renewed a lease. The recovery source also checked the exact upload
file hashes and inode metadata before permitting their later removal.

Only then did it issue one supported same-identity install of the reviewed
metadata-restoring 0.20 artifact. Full readback matched the original **42 bundle
entries and 40 files**, including their historical `0777` modes, and the exact
saved-state bytes and metadata. This restores a recorded compatibility baseline;
it does not endorse those modes for the modern preference Store. The old UI was
not relaunched, and preference content was not rewritten.

The two verified staging uploads were removed, fresh readback confirmed staging
absence, and the original lease was released. The new private host agent was
cleaned up with no recorded closeout errors. The original local lease record
remained unchanged. Runtime folder membership and parent metadata were checked;
preexisting runtime file contents were not rehashed.

The saved-evidence recovery audit binds 185 files and checks the raw installer
terminal, full restored trees, Core/domain replies and final absence receipts.
Fresh pre-deletion upload hash/inode checks and host-agent process/socket closure
also depend on the pinned executed source and its closeout receipt; separate raw
transcripts of every such check were not retained.

## Preserved audit history and remaining gates

Root observed the failed coordinator exit 1 and the separate recovery exit 0.
Saved-only audits and Root replays exited 0. The first failure auditor listed 665
checks while its summary counter reported 664. Its version 2 corrects that count
without changing the evidence. The first recovery auditor similarly undercounted
one check; its version 2 replay has 934 consistent checks. Earlier scripts and
results remain preserved. No TV retry was needed for either bookkeeping
correction, and check counts are not a coverage metric.

The observed existing-directory behavior now has an explicit external installed
expectation: only the recorded bundle root may remain `0777`; archive metadata
and all leaf-file expectations stay unchanged. This is not a general installer
rule, permission repair or authorization to recover an unknown state.

Same-version payload replacement, native forward/reverse preference migration
and a normal packaged-icon upgrade remain separate gates. This record qualifies
none of them, nor physical input, performance, Root artwork or production
activation. LG Home, firmware, boot, jail policy and core functionality were
outside the mutation scope. Luma Home continues to credit the attributed
[PlxNative framework](../README.md#plxnative-framework-dependency) for its shared
native foundation, without Plex client features.
