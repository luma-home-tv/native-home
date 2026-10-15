# Preview artwork admission

The [0.43 isolated result](artwork-installed-renderer-043.md) qualifies the
installed artwork path for Proof. The retained Preview stays on 0.37. Its
application identity, jail and LS2 permissions require their own admission;
a differently named test app cannot prove those properties.

Use the existing closed `broker_installation::Profile` mapping. Proof and Preview
have fixed app, jail, executable, protected policy and helper paths. Root selects
its role from the fixed helper entry and securely loaded Installation. The
ordinary client selects its expected role from its exact own app identity.
Requests and public markers do not select roles, paths, commands or permissions.
An ordinary UID or a successfully parsed response alone grants no authority.

The profile-aware protocol and artwork APIs preserve the existing Proof APIs as
strict wrappers. Explicit Preview validation must apply to Identify and every
nested identity in Header, Complete, Refusal and Finished, including the receiver
and final EOF checks. Keep the full kernel/own-process binding and the provider's
separate complete SourceKey checks. A successful Header does not bind its absent
folder/relative-path fields or prove a closed helper lifecycle.

The inert capability API uses only the fixed HBC `checkRoot {}` request. A future
normal trial must observe that request from the actual Preview identity before
the one literal installed-helper command. Keep platform, foreground, sole-card,
identity-change and reply-shape checks; do not relax permissions to make a
rejected caller work.

## Uncertain startup must survive a restart

An unknown helper outcome currently quarantines one provider instance. A new
instance must not silently erase that condition. The pending-operation guard is
negative state only: it can block dispatch and cannot authorize Root activity.
It uses one fixed file in the existing own private state directory, separate
from favourites. Create it exclusively and sync it before dispatch. A pending
record, malformed record or storage error blocks further dispatch; age, caller
death, a different nonce and app restart do not clear it.

Only the ticket that created the file can remove it after known no-start or
fully joined and reconciled completion. Dropping a ticket retains the record.
Checks bind the held directory, exact file identity/bytes, owner/group, mode and
single link. Directory identity excludes mutable timestamps and link counts so
normal preference writes can continue independently. File/directory sync requests
are not a guarantee against every filesystem or power-loss failure.

There is no automatic clear-old-record API. Recovery needs independent inspection
of the exact owned helper, namespace and all possible known child groups under
the device lease. Missing journals or HBC `false` alone do not establish zero
children. Keep the ordinary UI and packaged artwork usable while dispatch is
blocked. Do not clear the guard on a timer or signal guessed Root PIDs.

The current Proof provider now opens this guard and checks it during construction,
then remeasures itself and creates the pending ticket before starting its HBC
thread. Spawn failure and the existing known-completion grader may release it;
unknown completion leaves it pending. Failure to clear the matching record
discards even an otherwise accepted image. A later diagnostic-write failure can
quarantine an instance after a safely known closure has already cleared the
record; lost diagnostic evidence is separate from unknown remote work.

## Integration and delivery gates

1. Connect the closed profile through self measurement, capability, Root peer and
   context readers, installed enrollment/resource policy, fixed invocation and
   provider report grading. Retain legacy operator/Proof behavior. Do not accept
   generic caller-selected app IDs, paths, URIs or commands.
2. Integrate the pending guard around every installed-helper dispatch and its
   known-completion paths. Test an uncertain result followed by a new provider
   and process: both must remain blocked. A fully known typed resource refusal
   may clear the guard; an unreconciled early/context failure may not.
3. Run a new isolated Proof regression of the shared code, including real native
   artwork, all submitted background/drain jobs, helper/child absence, complete
   temporary removal and full retained 0.37 preservation.
4. Qualify the actual Preview identity through a backed-up same-ID trial. The
   same app ID cannot coexist as a second independent installation. Save fresh
   full 0.37 bundle/state bytes and metadata and retain its exact rollback IPK.
   Use matching frozen normal and diagnostic variants of the same executable;
   the trial loads real preferences but performs no favourite, setting, source or
   household-app actions. Installer UNKNOWN is a barrier to rollback/reinstall,
   removal, upload deletion and lease release until independently resolved.
5. Require actual Preview self/GID and helper bindings, capability, Complete,
   Finished/EOF, joined HBC and every helper/child result, visible native frames
   and unchanged favourites. A failed trial restores 0.37 only after known helper
   closure. A new probe's UID or Proof success cannot replace this evidence.
6. Promote the non-diagnostic package only after that admission. Check clean exit,
   a second new process, hide/IME/resume, relevant keyboard/idle behavior and
   actual artwork-arrival/frame work. Record the new owned persistent helper
   policy and its guarded removal procedure separately from fully removed tests.

The source passes 425 library tests, five dashboard tests, warnings-denied Clippy
and 41 Python build/provenance/framework/package tests. Controlled host cases
exercise Preview nested messages, strict Proof defaults, identity/context drift,
pending-record concurrency/recreation, known closure/no-start and discarded
results when marker clearing fails. These are host observations; the new guard
integration and Preview role have not been exercised on a TV.

Both ARM binaries compile from functional source
`dd409441d1c6f864edd920bf749f604ea69c6cb4`, with matching snapshots of 96 selected
inputs and the pinned framework. The [compiled foundation record](preview-artwork-foundation-candidate.json)
binds the actual executables and build receipts. The existing target-feature
stability warning remains. No new package was created or installed, and these
builds do not qualify the new dispatch guard or actual Preview capability.

The current source foundation does not activate the Preview provider, add a
Preview Root entry or update the retained TV app. Those are remaining
implementation and native-validation steps, along with the new Proof dispatch
guard's device regression. LG Home,
boot, firmware, jail/service policy, media and OLED protections stay untouched.
C2 remains outside this work.

Rendering and shared UI continue to build on the explicitly attributed
[PlxNative framework](../README.md#plxnative-framework-dependency), without Plex
client or playback features.
