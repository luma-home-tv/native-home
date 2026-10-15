# C2 retained UI upgrade plan

This is a proposed backed-up upgrade of the C2's retained 0.20 dashboard to the current normal UI, with packaged artwork and no Root artwork provider. No upgrade or permission migration is established by this document. LG Home, Core files, firmware, boot, jail and LS2 policy remain outside the change.

## Recorded baseline and compatibility gap

The read-only overnight baseline and [isolated 0.44 static check](c2-icon-presentation-044.md) preserved the retained 0.20 installation. Its app-owned state directory and preference file currently have mode `0777`. The installed bundle directory and all recorded bundle files also have mode `0777`. These are historical facts to preserve in the backup, rather than modes to infer from a package.

The modern [preference Store](../src/preferences.rs) requires an ordinary app-owned directory and regular preference file with no group or other permission bits; the file must have one link. Creating a new directory uses `0700`, but opening an existing insecure directory does not repair it. The upgrade therefore needs an explicit compatibility transaction before modern preference loading. Falling back to default preferences after a storage error would not demonstrate preservation of the user's configuration.

The recorded original 0.20 archive has directories and its executable at `0755`, and other regular resources at `0644`. Those archive modes differ from the actual installed `0777` metadata. Reinstalling the original archive can restore its bytes without necessarily restoring the historical installed modes. Byte restoration and metadata restoration must be checked separately.

## Packaged artwork is the smallest useful delivery

The current [Shell](../src/shell.rs) prefers a live texture when one exists, then a packaged texture keyed to the application, then initials on the missing-artwork surface. Its packaged loader reads bounded relative asset paths, decodes bounded images and uploads them. The [live catalogue merge](../src/live.rs) preserves existing packaged icon mappings for matching applications.

The retained C2 assets cover many current applications, but three currently eligible applications have no packaged mapping. This source path can reuse existing icons; it cannot create those missing images. A candidate must preserve the reviewed C2 asset set rather than substitute G2 assets or synthetic fixtures. It needs no Root broker, privileged artwork reads or helper policy changes.

The 0.44 C2 result proves presentation of controlled synthetic icons in an isolated app. It does not prove that the retained normal app can decode, upload and display every real packaged asset after a live catalogue refresh. That requires a normal-identity trial and inspection of its actual frames. Initials in earlier target-only artwork experiments were not evidence of a general icon-loading failure.

## Qualify installer metadata before the retained upgrade

A small installer-only component can resolve uncertainty without launching the retained app or a test app:

1. Under a fresh device lease, require quiet LG Home and absence of the fixed isolated Proof identity. Save fresh retained bundle/state byte and metadata manifests, the selected Core hashes and state-domain readbacks.
2. Install a reviewed inert package under that isolated identity through the supported installer. Never launch it. Include no services, Root helper, startup hooks, policy additions or capability trial.
3. Read back the installed package's exact file inventory, bytes, owner/group and modes. If needed, use a second reviewed version under the same isolated identity to observe supported update and downgrade metadata behavior.
4. Require authoritative installer completion, then remove that isolated package through the supported service and verify registration and its own filesystem namespaces are absent. A never-launched package supplies no ordinary-process or assigned-runtime-UID proof.
5. Recheck the retained baseline, Core/state readbacks and LG Home, then close out uploads, lease and private host agent.

An unknown installer outcome blocks conflicting installation or removal and remains an explicit recovery condition. This component qualifies metadata behavior for an inert isolated identity; it does not substitute for the actual retained same-ID restore check.

## Two-path state permission transaction

The migration is limited to the fixed retained state directory and its preference file. Before mutation, require the recorded ordinary app owner/group, expected old modes, directory/regular-file types, a single file link, unchanged preference bytes and bounded size. Bind those checks to held no-follow descriptors and stable device/inode identities. A path check followed by an unrelated path-based chmod is insufficient under the historical writable hierarchy.

Change only the state directory to `0700` and the preference file to `0600`, in that order. Recheck the same identities, bytes, owner/group and resulting modes; request the applicable durability operations. Do not deserialize and rewrite the preferences merely to change their permissions. Do not chown, recursively chmod, repair parent directories or weaken the Store's checks.

An implementation may run as an explicit ordinary-app migration before Store initialization, or use a separately reviewed fixed-descriptor operator transaction. The latter must first qualify its descriptor behavior on the actual platform. Neither approach grants authority from a caller-supplied path. The transaction and reverse operation need a receipt for each completed step, including a partial failure.

## Backed-up normal-identity trial

Before installation, preserve the exact original package, complete retained bundle and state bytes, owner/group/modes and initialized preference contents privately. Require no retained process is running, quiet LG Home, adequate storage and the dated device-use window. Admit an exact normal package with the intended C2 resources and no diagnostic marker, Root provider activation or helper payload.

After the supported same-ID update, verify modern Store acceptance of the actual retained preferences. The only intended state metadata changes are the two modes above; preference bytes and owner/group remain equal to the backup. Exercise the actual normal identity in separate sessions, including packaged icons after catalogue refresh, read-only navigation, keyboard Done/Back and idle/wake behavior. Avoid launching app or input cards and avoid setting changes. Capture and inspect real-asset frames; source support and the synthetic 0.44 result are prerequisites, not substitutes.

Recheck full bundle/state expectations, Core hashes, selected state domains, original current/last-input observations and cleanup. Keep the existing working default until this explicit trial is admitted. A trial may deliberately restore 0.20 after qualified closure before a later decision to retain the new version.

## Restore contract and failure barriers

Restore uses the supported same-ID installer without uninstalling the retained app or resetting its state. Installer uncertainty, lease loss, unexpected processes or changed state bytes block conflicting operations; they do not justify overwriting preferences or deleting unfamiliar artifacts.

After known process and installer closure, the reverse state transaction may restore the preference file to its recorded `0777` first and the directory last, with the same fixed-descriptor identity and byte checks. This restores a historical baseline rather than endorsing its permissions for the modern Store.

Bundle metadata needs its own explicit decision. Compare the supported rollback's actual bundle against both the original archive manifest and the historical installed manifest. If the installer normalizes modes, byte equality alone is not full historical restoration. An optional, separately scoped restoration contract may restore only the fixed own-bundle entries from the backup after exact inventory, byte, type, owner and link checks bound to their descriptors. It must not become a general recursive chmod, parent-directory repair or Core modification. Without that qualified contract, report the remaining metadata difference honestly and retain the recovery evidence.

No Root artwork helper or weakened helper trust boundary is needed for this plan. Permission migration, installer metadata behavior, real packaged-icon rendering and normal-session acceptance are separate admission checks; none alone establishes the whole upgrade.
