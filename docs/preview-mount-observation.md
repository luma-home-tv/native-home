# Preview SDK mount observation

The second 0.58 trial exposed a strict mount-sequence equality check after the
verified rollback. Four copies of existing read-only SDK rows had been appended;
the original rows were unchanged. The successful cleanup recorded and preserved
that difference without changing mounts or claiming complete mount restoration.

[The pure observer](../tools/sdk_mount_observation.py) now describes that bounded
exception. It performs no I/O or mount operation. Callers must independently
establish current ownership, process/Root closure, source provenance, cache
newness, relevant mount shapes, deadlines and permission for any later cleanup.
Supplied metadata and a successful comparison grant none of that authority.

It preserves the complete original ordered subset across four measured Preview
mount-target roots. Source-field substring matches and sibling app names never
select rows; unexpected same-app aliases refuse. The first host version retained
only the canonical root and incorrectly dropped 102 mirrored rows. Its failed
fixtures remain preserved; the corrected observer retains all four views.

Two final full observations must be byte-identical. The selected original sequence
must remain an exact prefix. Only repetitions of the canonical read-only
openssh/devmode pair, already adjacent in the fresh baseline, may append, bounded
by the independently observed own launches. Mirrored read/write rows must remain
unchanged and cannot qualify as a new suffix. Changed, removed, reordered,
foreign, escaped, incomplete or unstable rows refuse. Both fixed cache-parent
ancestry/leaf/subtree shapes must also remain unchanged.

The private carrier's 35 existing host cases, 15 mount cases and offline check
pass. The public module has nine synthetic tests, runnable without a TV:

```sh
python3 -m unittest discover -s tools -p test_sdk_mount_observation.py -v
```

The observer is deliberately limited to measured G2/webOS24 target roots. It is
not a universal or per-launch SDK behaviour rule. Its `mounts_restored` field means
the selected sequence equals the baseline; it does not prove a mount-restoration
operation, every namespace or native cleanup. The private carrier remains
undated and unadmitted. Future runs need a new baseline, lease and reviewed scope.
