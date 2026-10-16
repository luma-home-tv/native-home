# Ordinary durable icon-cache foundation

The new [cache module](../src/durable_icon_cache.rs) provides bounded ordinary
storage, immutable snapshot lookup and worker-side image decoding. Its 22 real-file
tests and all 665 Rust host tests (657 library and eight UI cases) pass, as does strict Clippy.
This is a library foundation: no dashboard, image-worker or provider callsite uses
it yet. Neither TV was contacted or updated. Normal Root artwork remains off.

The matching ARM dashboard build also passes. All 119 selected input files match
their Git blobs at the functional source revision; SDK readelf confirms ARM32
little-endian soft-float. The existing unstable-neon warning remains recorded.
This build was not packaged or executed on a TV and supplies no native cache proof.

This implements the storage part of the [responsive artwork plan](artwork-responsive-cache-next.md).
Normal navigation will eventually read already prepared artwork through the
existing ordinary image worker. Separately qualified preparation will populate
it after authenticated positive transfers have joined and their current operation
records have closed. Those integrations and their native evidence are still required.

## Stored data and bounds

The production entry point permits the ordinary Preview or Proof identity and
walks fixed ancestors without following symlinks. Each owns a separate private
cache directory beside its preferences. Existing foreign, symlinked or incorrectly
permissioned data is refused without repair. The module cannot read original app
icon paths, call a service, launch a helper or touch a Root pending record.

Each immutable content-addressed file contains a bounded metadata header and
encoded image. Records retain the complete SourceKey (app ID, version, root
class, folder, relative path and icon kind), encoded digest/length, observed source
file identity and typed producer revision/dashboard/helper association. These are
ordinary, untrusted association data. A matching record grants no Root authority
and cannot manufacture the independently admitted current catalogue Source.

Default limits are 256 records and 16 MiB of encoded payload, configurable up to
512 records and 64 MiB. Each body is at most 1 MiB and each header at most 8 KiB.
Metadata, filesystem overhead and one bounded publication temporary are additional.
These limits govern sampled inventories and cooperating writers using the same
configuration; they are not a filesystem quota against same-UID bypasses.
Unknown, malformed or overlimit inventories are refused without deletion.

Publication holds one exclusive create-new writer slot through capacity admission,
complete-body fsync, exclusive link publication and final inventory checking.
The slot is then removed through its measured descriptor/name identity and the
directory synced. Existing entries are never overwritten. A changed or interrupted
slot is not age-stolen. A crash or sync failure may leave a new leaf or stale slot
despite an error: an error does not prove durable absence.

## Snapshot and worker contract

Every adopted snapshot gets a fresh in-process generation seal, including repeated
adoption of identical contents. Lookup requires the current Source and an expected
producer association supplied independently of cache metadata. Missing matches
stay missing; multiple same-key matches are ambiguous. There is no newest-by-clock
selection or privileged fallback.

The worker reads through anchored descriptors with bounded no-follow file access,
stable descriptor/named metadata and digest checks. It reuses PlxNative's existing
bounded decoder, the packaged-reader constants and DecodedIcon: maximum 512 pixels
per axis and 1 MiB RGBA. Alpha and aspect are preserved. Original source-file
identity stays distinct from the new cache file's owner/identity. Successful
publication alone is not successful image decoding.

Final image admission checks the current generation, complete Source and expected
producer again. The future render controller must serialize this check with current
catalogue/visibility and GL admission. It remains responsible for one outstanding
image job, stale-result discard, render-thread texture ownership and joined shutdown.
The foundation starts no thread and performs no GL operation. File and codec calls
have no hard deadline or total-heap guarantee.

Withdrawal prevents old demands and decoded results from being admitted. Retirement
requires a withdrawn generation, the originating Cache and the exact entry identity;
the caller must first join old consumers. It never clears a Root latch. Automatic
eviction, old-version retirement after restart and stale-slot recovery are not yet
implemented. Cache-full handling must be finished before product delivery.

## Checks and retained failures

The real-file tests cover PNG alpha/aspect and reopen, full source/producer matching,
same-key ambiguity, withdrawal and repeated-adoption ABA, independent entry/byte
budgets, cooperating publishers, held/changed writer slots, unknown inventories,
symlink/FIFO/sparse files, truncation/growth/replacement, root replacement/mode
changes, hardlink metadata changes and invalid headers/digests/codecs. Adjacent
preference and pending-record bytes remain unchanged. The owner-mismatch fixture
injects a wrong expected stamp; it does not claim a real foreign-owner chown test.

The first strict Clippy run failed on an inventory comparison spelling and a large
enum variant. The final source uses an equivalent comparison and boxes the Ready
demand. Independent review also found that releasing the writer slot before the
final scan could make a successful publication report failure under another writer.
The final source keeps that scan inside the slot. Both the original proposal and
failed lint result are preserved privately; final host checks pass.

The [machine-readable milestone](durable-icon-cache-foundation-milestone.json)
binds the final source and check receipts. The attributed PlxNative dependency and
existing packaged reader/worker, Root provider and session guard are unchanged.

## Integration still required

1. Extend the existing ordinary worker/controller with complete SourceKey and cache
   generation/result binding. Scan/read/decode on its worker; retain packaged art
   and warming/missing behavior. Avoid a second image worker or a new GL owner.
2. Maintain catalogue freshness independently of the optional Root artwork runtime.
   Failure, expiry, removal or changed association must withdraw dynamic bindings.
   Unchanged catalogue polls must not churn generations or retry failed art endlessly.
3. Add an optional diagnostic-only publisher after a successful encoded result,
   authenticated joined completion and current-Ticket closure. Negative, unknown,
   cancelled or incomplete outcomes publish nothing. Cache I/O errors must not
   alter the existing image result or privileged settlement.
4. Supply an independently admitted expected producer association for the same
   dashboard/helper artifacts. The normal consumer cannot learn its expected
   producer from the cache itself. A new executable needs its own matching native
   qualification; historical positive transfers do not transfer automatically.
5. Qualify native pixels, invalidation, retirement, keyboard/app departure, joined
   shutdown and zero Root dispatch on hits. Measure cold preparation separately
   from normal navigation. Same-key source mutation currently requires an explicit
   refresh; the foundation promises no TTL or continuous freshness.
