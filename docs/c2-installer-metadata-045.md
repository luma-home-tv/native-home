# C2 installer metadata measurement, 0.45

One isolated, never-launched Proof package was installed, read back and removed
through the supported installer. Root observed operator execution handle 7450
exit 0. This is an installer metadata result, not an application runtime result.
The [sanitized milestone](c2-installer-metadata-045-milestone.json) binds the exact
candidate, carrier, contract, archive and independent audit.

The archive contained only application metadata, the frozen dashboard executable
and a branding image. It contained no snapshot, trial marker, helper, service,
webOS permission grant or Root configuration. It requested mode `0777` on its **new own
directory and three files**, with standard archive ancestor entries at `0755`.
The installed directory and all three files were Root-owned, retained the exact
archive bytes, and read back as `0777`.

No application launch, key input or setting write was requested. The carrier
issued no direct `chmod`/`chown` or operation to change retained/system
permissions. This does **not** mean that the archive contained no permission
metadata: the new own entries explicitly requested `0777`. Existing retained
permissions were measured and preserved rather than repaired or migrated.

The executable came from the historical 0.45 source commit
`9c6af7247fe62f22f34c2cdbfac6d53808e73e5f`, with 99 selected build inputs and
SHA-256 `eeb70d4cab122debff46c98131bbff856268cb867beb6466622daccd5fac5528`.
It was not launched by this component. The historical receipt is not an admission
of equivalence to the current source or evidence of executing that UI on C2.

Fresh full backups and readbacks verified all 40 retained 0.20 application files
and the complete saved state. Their bytes, ownership and actual `0777` metadata
were unchanged. The historical archive beginning `3876ef89` matched those file
bytes, but its archive modes differ from the installed metadata. It was a
read-only reference and was never supplied as an install or rollback target.

The independent saved-evidence audit also replays the original Proof installer
terminal, supported removal, seven sampled process guards, empty own runtime and
state surfaces, registration/jail/mount absence, quiet LG Home, five Core file
hashes and 20 preservation domains. The no-wake lease adapter, guarded own upload
cleanup and private agent/lease closeout completed without recorded errors.
Process guards are samples, not continuous execution tracing; no Proof execution
was observed at those samples and no ordinary producer was expected.

The corrected host suite passed 16 cases. Its first run preserved three fixture
setup errors caused by a missing run label. Independent auditor v1 then exited 1
because it incorrectly treated input-source replies as settings envelopes.
Auditor v2 checked the actual input-source envelope shapes and replayed the same
saved evidence successfully: 2,961 assertions, with no TV retry for that repair.
The earlier logs remain preserved; assertion count is not a coverage measure.

This qualifies requested metadata behavior for one new inert Proof package only.
It does not qualify same-identity rollback, a retained upgrade, metadata migration,
runtime rendering, Root artwork, physical input or performance. Core hash and
reply preservation are scoped measurements, not exhaustive system metadata
coverage. The retained 0.20 application and favourites were not updated or reset.
