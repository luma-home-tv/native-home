"""Pure, observational fixed-Preview SDK mount exception. No I/O or mount authority."""
APP = 'com.ash.nativehome.preview'
JAIL = '/var/palm/jail/' + APP
# Explicitly observed on current G2/webOS24; not a universal namespace law.
MOUNT_ROOTS = (JAIL, '/mnt/lg/cmn_data' + JAIL, '/mnt/lg/flash/data' + JAIL, '/mnt/lg/user' + JAIL)
SDK = JAIL + '/media/cryptofs/apps/usr/palm/services/com.palmdts.devmode.service/binaries-armv71/opt/'
PAIR_PATHS = (SDK + 'openssh', SDK + 'devmode')

def need(ok, message):
    if not ok:
        raise ValueError(message)

def rows(raw):
    need(type(raw) is bytes and 0 < len(raw) <= 1048576 and raw.endswith(b'\n'), 'Bounded complete LF mount observation required')
    need(b'\r' not in raw and b'\x00' not in raw and b'\\' not in raw, 'No unsupported or escaped mount framing')
    result = raw[:-1].split(b'\n')
    for row in result:
        fields = row.split(b' ')
        need(len(fields) == 6 and all(fields) and all(32 < c < 127 for field in fields for c in field), 'Exact six ASCII proc-mount fields')
        need(fields[1].startswith(b'/') and fields[4] in (b'0', b'1', b'2') and fields[5] in (b'0', b'1', b'2'), 'Actual absolute supported mount rows')
    return result

def preview_rows(raw):
    # Preserve all four fixed, measured same-App target roots in original raw order.
    # Source fields and sibling app names do not select rows. Unknown same-App aliases refuse.
    selected = []
    roots = tuple(root.encode('ascii') for root in MOUNT_ROOTS)
    canonical = JAIL.encode('ascii')
    for row in rows(raw):
        target = row.split(b' ')[1]
        known = any(target == root or target.startswith(root + b'/') for root in roots)
        if known:
            selected.append(row)
        else:
            unexpected = canonical + b'/' in target or target.endswith(canonical)
            need(not unexpected, 'Unexpected same-App mount target alias; cannot drop preservation scope')
    return selected

def preview_scope_bytes(raw):
    selected = preview_rows(raw)
    need(selected, 'Fresh exact Preview mount-target subset required')
    return b'\n'.join(selected) + b'\n'

def validate_sdk_mount_delta(original, final_before, final_after, *, controlled_own_launches,
                             cache_shapes_before, cache_shapes_after):
    need(type(controlled_own_launches) is int and 0 <= controlled_own_launches <= 2, 'Actual bounded own-launch count required')
    need(type(cache_shapes_before) is dict and type(cache_shapes_after) is dict and
         set(cache_shapes_before) == set(cache_shapes_after) == {'/media/developer', JAIL + '/media/developer'}, 'Both independently observed fixed cache mount shapes required')
    for parent in cache_shapes_before:
        a, z = cache_shapes_before[parent], cache_shapes_after[parent]
        need(type(a) is list and type(z) is list and a and all(type(r) is str for r in a + z) and a == z, 'Relevant cache ancestry/leaf/subtree mount shape unchanged')
    baseline = rows(original)
    need(baseline == preview_rows(original), 'Original observation must be exact same-parser Preview mount-target scope')
    need(final_before == final_after, 'Two final mount observations must be byte-identical')
    current = preview_rows(final_before)
    need(len(current) >= len(baseline) and current[:len(baseline)] == baseline, 'Original raw sequence must be exact prefix; no removal/change/reorder')
    suffix = current[len(baseline):]
    need(len(suffix) % 2 == 0 and len(suffix) // 2 <= controlled_own_launches, 'Only bounded complete SDK-pair increments')
    if suffix:
        pair = suffix[:2]
        fields = [row.split(b' ') for row in pair]
        need(tuple(f[1].decode('ascii') for f in fields) == PAIR_PATHS, 'Only fixed same-Preview SDK pair in fixed order')
        need(fields[0][:1] + fields[0][2:] == fields[1][:1] + fields[1][2:], 'Pair has identical actual source/filesystem/options')
        need(all(b'ro' in f[3].split(b',') and b'rw' not in f[3].split(b',') for f in fields), 'Pair must be explicitly read-only')
        need(any(baseline[i:i + 2] == pair for i in range(len(baseline) - 1)), 'Exact adjacent pair must already exist in this fresh baseline')
        need(suffix == pair * (len(suffix) // 2), 'No foreign/additional/reordered suffix rows')
    return {'schema': 1, 'mounts_restored': not suffix,
            'mount_delta': {'original_rows': len(baseline), 'final_rows': len(current),
                            'appended_rows': len(suffix), 'duplicate_pair_increments': len(suffix) // 2,
                            'bounded_by_observed_own_launches': controlled_own_launches},
            'fixed_measured_mount_roots': list(MOUNT_ROOTS),
            'original_sequence_exact_prefix': True, 'final_observations_stable': True,
            'relevant_cache_mount_shapes_unchanged': True,
            'mount_operations_performed': False, 'SDK_change_cause_qualified': False,
            'universal_or_per_launch_SDK_behavior_qualified': False}
