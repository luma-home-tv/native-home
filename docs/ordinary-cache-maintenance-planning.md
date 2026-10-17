# Pure ordinary-cache maintenance planning

The Rust maintenance planner proposes unsupported ordinary records without
opening, writing or deleting files. It is separate from cache mutation,
publication, source adoption, Root sessions and Tickets. No production worker,
renderer or CLI invokes it. The module is enabled for ARM builds and host tests only;
host checks do not establish native behavior.

`durable_icon_cache::maintenance` reuses the existing private `Record`,
`ProducerVersion`, `SourceRecord`, validation, canonical header/name serializer
and `FileIdentity`. The anchored FD reader and pure inspector call the same
pure framing/header decoder. There is no second cache format grammar. Source declarations
are returned as untrusted SourceKey data, never as an adoptable Source or Demand.
Existing fixed-profile parsers have narrow pure inspection wrappers returning
only ProducerVersion data; ExpectedProducer and ReadExpectation do not escape
those wrappers. The wrappers retain the existing 4096-byte profile/selector
limit, a stricter bound than the private Python prototype's selector cap.

The caller supplies stable complete root/file observations, both inventories,
ordinary owner IDs and bounded raw record bodies. Those inputs do not prove
their provenance or currentness. The planner validates every leaf and aggregate
budget before returning a result. A `.publish-next` slot, unknown leaf, duplicate
or changed inventory, malformed header, wrong name/digest, ownership/mode/link
anomaly or capacity violation refuses the whole plan. No partial candidate list
escapes and no old slot is stolen by age or PID guesses.

The independently supplied retention union includes the current producer and
every explicit compatibility-accepted producer. The raw selector set must match
an independent exact registry of selector SHA-256 values: omissions,
substitutions and duplicates refuse. The complete producer identity is used;
matching a source revision alone is insufficient. All records of retained
producers remain, including old versions and same-key duplicates. Only fully
parsed records outside the union are proposed. Completeness/provenance of the
independent registry remains a caller obligation.

The result always reports that mutation, source/Root authority and input
provenance/currentness are false. There is no apply method. Same-key refresh,
automatic obsolete-version eviction within a retained producer, disk-full
publication and stale-slot restart recovery remain open.

## Contract boundary

ReaderContract now requires `maintenance_planner_sha256`, computed from the
compile-bound child module bytes. Packaging's critical input map includes the
same file, and the full build snapshot already walks the entire source tree.
A child-only change therefore changes the exact reader contract. Native build
selection, descriptor checks and current/historical contract-equality guards
remain unchanged.

This expanded contract rejects old d221/f5 selectors/receipts that lack the new
input. Historical d221's real-f5 positive fixture still runs against its exact
hash-pinned historical packaging policy from local Git. A separate new-source
fixture expects refusal of that old critical graph. Neither result grants this
source wave automatic f5 adoption, cache preparation, native permission or Root
authority. A future accepted selector needs separately validated actual builds
and a deliberately reviewed compatibility boundary.

## Validation and remaining work

Host checks cover 19 new cases using actual Rust Record/profile serializers and
parsers: current-plus-accepted retention, multiple accepted origins, complete
selector registry, full producer identity, mandatory child digest, malformed and
duplicate schema, unknown leaves, held/ancient slots, changed root/file identity,
body/name/header corruption, limits and non-authorizing outputs. The full 817
library tests, 15 UI tests, strict Luma Clippy, 87 packaging tests and focused
contract tests pass. Explicit private fixtures separately preserve d221's
historical positive and verify new-source refusal. No TV or ARM/package result
is claimed by these checks.

Independent source review and seven additional adverse cases also pass. The
coordinator verified the reviewed source and selected evidence bindings. The
frozen implementation and its original checks remain separate from this
documentation clarification.

An eventual mutator must run off the render thread after actual relevant
generation withdrawal and worker join, with the same originating anchored cache,
exclusive writer slot, complete freshly rebound retention/inventory/body/FD
evidence and exact opaque record identity. Ordered Adopt/decode draining alone
must not be relabelled as a complete worker join. Root pending, preferences,
session and Ticket state remain untouched. Matching ARM builds, packaging/ABI
review and isolated native acceptance are separate later work.
