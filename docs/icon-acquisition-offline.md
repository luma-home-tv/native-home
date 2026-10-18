# Offline icon acquisition inputs

This source candidate replaces the automatic shell icon copier in
`collect-snapshot.py`. Metadata collection is icon-free unless the operator
supplies `--icon-acquisition /private/input/manifest.json`. The collector does
not run the public reader, stage an executable or provide a device lease. Its
existing metadata SSH calls still require separately authorised coordination;
this change does not admit that standalone transport for device use.

The planner and data-ingestion API are in `icon_acquisition.py`. They do not
contact a TV, invoke a process, decode an image, publish a durable cache or grant
Root authority. Normal artwork remains ordinary packaged fallback/cache reads.
This input route is not the existing privileged FD-safe broker.

## Planner

`plan(raw_catalogue, own_id, declared_ids)` consumes the two complete raw
`replies.apps` and `replies.launch_points` replies, using the SourceMap admission
rules in `src/app_artwork.rs`. The declared IDs are a sorted unique set of1–64
admitted apps; complete reply arrays have at most512 entries. First valid
registered/visible/allowed-type/title/ID ownership wins. Sources require a
version and canonical folder. A valid `largeIcon` precedes `icon`.

Each row has `app_id`, full `source`, optional three-element `request` argv data
and an optional `unavailable` reason. The full source has exactly
`app_id,version,class,folder,relative,kind`; relative is a component array.
SourceMap supports five root classes but this frozen reader supports only
`lg_system`, `encrypted_apps`, `developer_apps`. Its ASCII/extension grammar is
also narrower. Unsupported sources are unavailable; no root/path policy is
extended and no shell fallback exists. A failed preferred read is not silently
relabelled as a different source.

## Completed external data

All input files live in one operator-owned private0700 directory, are plain
single-link0600 files and are referenced by a single leaf and complete SHA-256:
`{"file":"before.json","sha256":"<64 lowercase hex>"}`. All ancestors are
opened without following symlinks. Complete held/named file identity is checked
around bounded reads. JSON/framing limits and a16MiB aggregate acquired-body
limit apply; these are payload limits, not a process-memory/time guarantee.

The schema1 manifest has exactly:

- `schema`, `own_id`, sorted `declared_ids`;
- `before`, `after`: references to complete raw catalogue JSON;
- `closed_scope`: reference to the external completed-scope record;
- `outcomes`: one ordered result for every declared ID.

The scope record has `schema:1`, a32-hex `acquisition_id`, exact
`helper_source_sha256`96c28e30… and `helper_elf_sha256`a8ec073d…,
`qualification_sha256` identifying the separate external qualification record,
and `state:"externally_recorded_closed"`. These are *data assertions*, not
qualification or closure verified by this module. A forged manifest can lie;
only the independently qualified coordinator can establish the native facts.
The parser refuses unknown/missing completion states and pins, but is not a
native admission or authentication system.

An acquired outcome has exactly `app_id,source,status:"acquired",frame,stderr,
completion`. `frame` is the raw LPI1 stdout reference, capped at the complete
protocol/header/body limit. `stderr` is the exact reader PID-line reference.
`completion` references a schema1 JSON record with exactly
`schema,acquisition_id,app_id,source,reader_pid,returncode,closure,frame_sha256,
stderr_sha256`. It must associate the same source/scope/body/PID, integer
returncode0, and `closure:"externally_recorded_exact_pid_absent"`. Unknown
closure, partial output, wrong key, mismatched PID or nonzero/bool status cannot
produce an asset. The external coordinator must independently establish that
these assertions are true, including actual PID identity/absence and all
staging/host-child/lease cleanup.

A not-acquired outcome has exactly `app_id,source,status:"unavailable",reason`.
Its reason is the planner's `no_source`, `unsupported_root` or
`reader_key_unavailable`, or `not_acquired` for a supported key. It carries no
successful read or native lifecycle claim. An external controller remains
responsible for closing every attempt, including refused/partial attempts;
this schema is not a generic parser for failed native transports.

`prepare(manifest_path,current_raw,snapshot)` requires the full selected keys to
match before, after and the current raw catalogue. The snapshot must have
unique admitted app identities and no pre-existing icon labels. It returns a
new snapshot, original encoded bodies and a local receipt. `publish` writes to
a fresh existing0700 output without overwriting: `snapshot.json`,
`icon-acquisition.json`, and0600 content-addressed `icons/<sha256>.<extension>`.
The caller can use `make_private_output` for anchored exclusive directory
creation. Any partial local IO failure is preserved, not deleted/retried.
Original alpha/encoded bytes are retained; the existing native worker alone
performs bounded decoding. Acquisition signature success is not decode/pixel
proof.

## Package seam

For these assets the operator must supply both:

```
NATIVE_HOME_ICON_ACQUISITION=/private/assets/icon-acquisition.json
NATIVE_HOME_ICON_ACQUISITION_SHA256=<exact receipt SHA-256>
```

`NATIVE_HOME_SNAPSHOT_ASSETS` is that same private assets directory. A present
acquisition receipt requires the explicit pair; omission does not fall through
to the old local-assets path. The pure `package_assets` validator binds the
pinned receipt, exact snapshot ID/leaf map, complete body hashes/lengths and
private held files. The packager stages the already-validated bytes rather than
reopening input icon paths, and its external package receipt records
`icon_acquisition_receipt_sha256`. It adds no app runtime marker/profile/helper,
service or publisher role. Existing separately sourced local package assets
remain a compatibility path; they are never automatically assigned new reader
provenance. Removing or forging provenance is not made trustworthy by this API.

Both ingestion and package validation report `native_authority:false` and
`native_closure_verified:false`. The receipt is a body-association record, not a
SourceKey freshness generation or cache producer. Packaged fallback stays an
ID/path/package-epoch snapshot. App updates/new installs need recollection and
repackaging; separate device asset sets need independent evidence.

The new imported planner/protocol files are added to build provenance. Existing
compiled receipts therefore do not authorise packaging with this changed
source. No new compiler/build/package or native result was produced here.
Future matching build, package audit, external primitive/controller qualification,
retained update preservation and ordinary rendering/cleanup on each TV remain
required. G2 evidence never qualifies C2.

Run the synthetic and real-file integration checks with
`python3 -B test_icon_acquisition.py`. They deny child processes and sockets;
metadata CLI calls are synthetic fixed-GET stubs, not TV calls. They cover
SourceMap-rule fixtures and the actual pure public-reader protocol, not an
independent execution of the Rust SourceMap parser or an image codec.
