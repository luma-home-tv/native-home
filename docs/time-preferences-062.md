# Protected time-preference readings

The isolated 0.62 source adds three view-only General rows from the existing
`localeInfo` settings reply: Clock format preference, Time-zone preference and
Date/time format locale. It adds no service request, setting write, Root route
or worker.

Clock format accepts the stored strings `12`, `24` and `locale`. Time zone is a
bounded printable ASCII preference string; it is not interpreted as an IANA zone
or UTC offset. Format locale uses the existing language-tag validation. Values
are validated independently, and failed or expired readings remain unavailable.
Opaque time-zone and format-locale strings retain their original case instead of
passing through generic On/Off presentation.

The header clock and its time conversion are unchanged. These rows describe
stored preferences; they do not establish regional clock correctness or add time
synchronisation/control.

## Validation

Functional commit `60e8148d8b6a3ed3dd7c5aff216f01644a598ce6` has 136 selected
source inputs. Recorded host checks pass: 763 library tests, 10 UI tests, strict
client Clippy and 84 Python packaging tests. The earlier opaque-display regression
failure is preserved alongside its correction.

The matching ARM dashboard build completed with exit 0. The ELF is 4,517,764 bytes,
SHA-256 `bc26a86a9643e581b6c348df12553e98a94ca87bd733a1cd895b84f519798169`;
its build receipt is
`346d6328121267a7b46c5f7e30ea1e970ebfa4d667580d1dd7530406d7b76153`.
Fifteen actual SDK readelf commands inspected this artifact and the qualified
0.57/0.61 references. ARM32 little-endian EABI5 soft-float PIE, the direct loader,
11 direct libraries and required GLIBC versions match; no RPATH/RUNPATH appeared.
The coordinator independently rehashed 58 bound files and all 136 current source
inputs, checked the ELF header/interpreter and reconciled the direct ABI outputs.

The known unstable-NEON compiler warning remains recorded. An initial local
pre-build schema-comparison failure occurred before Cargo/compiler execution; its
correction changed the comparison and output directory, not source code.

No helper build, package, target execution, TV contact or native qualification
follows from this result. It does not qualify transitive libraries, ordinary
caller permission, regional behavior, rendering or performance. The new UI alone
cannot inherit the 0.61 cache's producer association. G2 remains on its separately
verified retained version until a backed-up native campaign succeeds.

Machine-readable evidence: [milestone](time-preferences-062-milestone.json).
