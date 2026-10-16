# Scoped closed-negative grading 0.52

This source handles two authenticated artwork failures without releasing an
image: context withdrawal after a fully received/read-back image, or while
checking Finish after a valid Complete. Host tests pass. No 0.52 executable has
run on a TV, and ordinary Root artwork remains disabled. G2 retains 0.47 and
C2 retains 0.48 with their existing resources and favourites.

The ReadAfter path consumes the Receiver, checks every declared chunk, exact
length and digest, frees its encoded buffer and returns sealed metadata only.
Partial, malformed, poisoned and terminal receivers cannot produce that witness.
Finish distinguishes actual same-receive EOF from a strict Finished exchange;
legacy public receive behavior stays unchanged. A Complete followed by withdrawal
still discards its bytes before any decode, cache or texture commit.

EOF alone never establishes closure. The failed attempt must retain authenticated
Identify, actual final socket credentials and a freshly remeasured unchanged
ordinary identity while the connection is still held. Deadline checks surround
those measurements. Job cancellation does not erase this cleanup evidence; it
still prevents image delivery. Only a bounded candidate keeps its single already
started HBC observer alive for joining within the original finite observation
budget. It starts no replacement helper and does not retry an uncertain outcome.

The separate pure grader reparses the bounded raw HBC envelope, checks its exact
negative helper report and source-derived stderr, and binds role, fixed command,
nonce, ordinary/Root identities, protected installation, Core executable and
every reader result. Core measurements require their complete typed metadata,
not merely equal hashes. The admitted prefix and narrow rejected context must
match the helper's policy. Thirteen ordered, unique, completed child records are
required at ReadAfter and sixteen at Finish, with an exact matching raw journal.
Missing, repeated, contradictory, timed-out or unknown evidence stays Unknown.

A successful grade returns a nonserializable, non-Clone proof with a private
per-fetch seal. Only its matching captured Ticket can consume it once. The
current name, descriptor, bytes and directory must still pass `finish_known`,
including sync, before the provider reports known settlement. An old proof cannot
clear a later Ticket even when their durable identity fields are identical.
Storage failure leaves the provider quarantined/Unknown; a post-unlink directory
sync failure may already have removed the marker and proves no power-loss safety.

Known negative settlement returns an error, never an encoded file. Cancellation
or expiry can still be the job's result after known cleanup. ReadBefore,
pre-Identify, incomplete-body, malformed-packet and unclassified failures retain
the existing Unknown path. Older pending records receive no recovery. Positive
Finished, typed resource-refusal and known-no-start checks remain strict through
shared outcome-independent validation.

Root ran all 563 Rust tests (556 library and seven dashboard), all 58 Python
checks and Clippy with warnings denied. The 35 new test methods cover the
[17 required groups](artwork-context-closure-next.md), real controlled workers,
actual host Ticket files and simulated storage failures. Raw HBC is regenerated
after helper mutations. The first full run exposed a test fixture that rewrote
request byte 123 while retargeting PIDs; the fixture is corrected, and failed
logs/private candidates remain preserved. No runtime check was weakened.

These are host/source results. Isolated native ReadAfter/Finish cases, actual
LG-keyboard/external-app handoffs, Preview-role delivery, C2 Root permission,
performance, reliability and recovery from old unknown operations remain
separate gates. No new TV package or protected helper installation is admitted
by this record. Configured deadlines do not prove a hard lifetime for blocked
native calls. The [0.50 socket probe](artwork-transport-eof-050.md) and
[0.49 native component](artwork-transition-049.md) retain their frozen scopes.

Rendering still uses the explicitly attributed PlxNative framework; this change
adds no Plex application feature and changes no LG service or Home behavior.
