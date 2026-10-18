"""Offline public-icon planning and completed-data ingestion; never target authority.

No transport, process, native reader invocation, cache producer or normal Root path.
The external coordinator owns qualification, actual PID closure and device cleanup.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import stat
import unicodedata

_SPEC = importlib.util.spec_from_file_location(
    'public_icon_protocol', Path(__file__).parent / 'tools/public-icon-reader/protocol.py')
PROTOCOL = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(PROTOCOL)
SOURCE_SHA = '96c28e30bce3b17e9b7f3b2641b56090a5338d0414df1ae652c5ab41eb68371d'
ELF_SHA = 'a8ec073d2c401d18924fbbc02a19e788c4e090f444a36b304f7ade8a5bf2f44f'
ROOTS = {**PROTOCOL.ROOTS,
         'system_apps': '/media/system/apps/usr/palm/applications',
         'optional_content': '/mnt/otncabi/usr/palm/applications'}
MAX_APPS = 512
MAX_DECLARED = 64
MAX_JSON = 1048576
MAX_TOTAL = 16 * 1048576


def need(value, message):
    if not value:
        raise ValueError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def strict(raw):
    def pairs(rows):
        out = {}
        for key, value in rows:
            need(key not in out, 'Duplicate JSON key')
            out[key] = value
        return out
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: need(False, 'Nonfinite JSON'))


def encoded(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False) + '\n').encode()


def app_id(value):
    return type(value) is str and re.fullmatch(r'[A-Za-z0-9._-]{1,180}', value) is not None


def text(value, cap):
    return (type(value) is str and 0 < len(value.encode()) <= cap
            and not any(unicodedata.category(c) == 'Cc' for c in value))


def source_key(app, kind):
    """Mirror Source construction in app_artwork.rs; reader grammar is narrower."""
    identity = app['id']
    version, folder = app.get('version'), app.get('folderPath')
    if not text(version, 128) or not text(folder, 512):
        return None
    classes = [k for k, root in ROOTS.items()
               if identity not in ('.', '..') and folder == root + '/' + identity]
    relative = app.get('largeIcon' if kind == 'large_icon' else 'icon')
    if not classes or not text(relative, 512) or relative.startswith('/'):
        return None
    parts = relative.split('/')
    if ('\\' in relative or ':' in relative or len(parts) > 8
            or any(p in ('', '.', '..') or len(p.encode()) > 128 for p in parts)):
        return None
    return dict(app_id=identity, version=version, **{'class': classes[0]},
                folder=folder, relative=parts, kind=kind)


def catalogue(raw, own_id):
    """First valid duplicate owns the app, even when its artwork is unavailable."""
    need(app_id(own_id), 'Explicit own ID')
    replies = raw.get('replies', {}) if type(raw) is dict else {}
    apps, points = replies.get('apps'), replies.get('launch_points')
    need(type(apps) is dict and apps.get('returnValue') is True
         and type(points) is dict and points.get('returnValue') is True,
         'Complete successful catalogue replies')
    apps, points = apps.get('apps'), points.get('launchPoints')
    need(type(apps) is list and type(points) is list
         and len(apps) <= MAX_APPS and len(points) <= MAX_APPS, 'Bounded complete arrays')
    registered = {p.get('id') for p in points if type(p) is dict
                  and app_id(p.get('id')) and p['id'] != own_id}
    out = {}
    for app in apps:
        if type(app) is not dict:
            continue
        identity = app.get('id')
        if (not app_id(identity) or identity == own_id or identity not in registered
                or app.get('visible') is not True or app.get('type') not in ('native', 'native_builtin', 'web')
                or not text(app.get('title'), 512) or identity in out):
            continue
        out[identity] = dict(app={k: app[k] for k in ('id', 'title', 'type', 'visible')},
                             source=source_key(app, 'large_icon') or source_key(app, 'icon'))
    return out


def plan(raw, own_id, declared_ids):
    need(type(declared_ids) is list and 0 < len(declared_ids) <= MAX_DECLARED
         and all(app_id(i) for i in declared_ids)
         and declared_ids == sorted(set(declared_ids)), 'Explicit unique sorted finite app set')
    apps = catalogue(raw, own_id)
    need(all(i in apps for i in declared_ids), 'Declared apps admitted by complete catalogue')
    rows = []
    for identity in declared_ids:
        source = apps[identity]['source']
        request, reason = None, 'no_source'
        if source:
            if source['class'] not in PROTOCOL.ROOTS:
                reason = 'unsupported_root'
            else:
                try:
                    request = list(PROTOCOL.validate_key(source['class'], identity, '/'.join(source['relative'])))
                    reason = None
                except ValueError:
                    reason = 'reader_key_unavailable'
        rows.append(dict(app_id=identity, source=source, request=request, unavailable=reason))
    return rows


_STAMP = ('st_dev', 'st_ino', 'st_mode', 'st_uid', 'st_gid', 'st_nlink',
          'st_size', 'st_mtime_ns', 'st_ctime_ns')
_DIR_STAMP = ('st_dev', 'st_ino', 'st_mode', 'st_uid', 'st_gid')


class PrivateDirectory:
    """Anchored nofollow local input/output; all ancestors opened, never resolved."""
    def __init__(self, path, private=True):
        self.path = Path(path).absolute()
        need('..' not in self.path.parts, 'No parent traversal')
        self.chain = []
        try:
            fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
            self.chain.append((fd, None, None, os.fstat(fd)))
            for part in self.path.parts[1:]:
                parent = fd
                fd = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent)
                self.chain.append((fd, parent, part, os.fstat(fd)))
            self.fd = fd
            a = os.fstat(fd)
            need(not private or (a.st_uid == os.getuid() and stat.S_IMODE(a.st_mode) == 0o700),
                 'Private current-user directory700')
            self.check()
        except BaseException:
            self.close()
            raise

    def check(self):
        for fd, parent, name, original in self.chain:
            held = os.fstat(fd)
            named = os.stat(name, dir_fd=parent, follow_symlinks=False) if parent is not None else os.stat('/')
            need(all(getattr(original, k) == getattr(held, k) == getattr(named, k)
                     for k in _DIR_STAMP), 'Stable held/named ancestor identity')

    def close(self):
        for fd, _, _, _ in reversed(self.chain):
            os.close(fd)
        self.chain = []

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    def read(self, name, cap=MAX_JSON):
        need(type(name) is str and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,179}', name)
             and name not in ('.', '..'), 'Single local leaf only')
        self.check()
        fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=self.fd)
        with os.fdopen(fd, 'rb') as f:
            a = os.fstat(f.fileno())
            need(stat.S_ISREG(a.st_mode) and a.st_nlink == 1 and a.st_uid == os.getuid()
                 and stat.S_IMODE(a.st_mode) == 0o600 and 0 < a.st_size <= cap,
                 'Private bounded single-link regular file600')
            raw = f.read(cap + 1)
            z = os.fstat(f.fileno())
        named = os.stat(name, dir_fd=self.fd, follow_symlinks=False)
        need(len(raw) == a.st_size and all(getattr(a, k) == getattr(z, k) == getattr(named, k)
                                         for k in _STAMP), 'Stable complete held+named body')
        self.check()
        return raw

    def write(self, name, raw, mode=0o600):
        need(type(name) is str and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,179}', name), 'Single output leaf')
        self.check()
        fd = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode, dir_fd=self.fd)
        with os.fdopen(fd, 'wb') as f:
            os.fchmod(f.fileno(), mode)
            f.write(raw)
            f.flush()
            os.fsync(f.fileno())
        self.check()


def make_private_output(path):
    path = Path(path).absolute()
    need(path.name not in ('', '.', '..'), 'Named fresh output')
    with PrivateDirectory(path.parent, private=False) as parent:
        parent.check()
        os.mkdir(path.name, 0o700, dir_fd=parent.fd)
        parent.check()
    with PrivateDirectory(path):
        pass
    return path


def reference(directory, ref, cap=MAX_JSON):
    need(type(ref) is dict and set(ref) == {'file', 'sha256'}
         and type(ref['sha256']) is str and re.fullmatch('[0-9a-f]{64}', ref['sha256']), 'Exact local body reference')
    raw = directory.read(ref['file'], cap)
    need(digest(raw) == ref['sha256'], 'Referenced complete body SHA')
    return raw


def prepare(manifest_path, current_raw, snapshot):
    """Validate completed external DATA and return encoded assets, no output yet.

    Reported closure is externally asserted evidence, never target verification by
    this function. The caller still requires separate native scope qualification.
    """
    snapshot = strict(encoded(snapshot))
    manifest_path = Path(manifest_path).absolute()
    with PrivateDirectory(manifest_path.parent) as directory:
        raw = directory.read(manifest_path.name)
        m = strict(raw)
        need(type(m) is dict and set(m) == {'schema', 'own_id', 'declared_ids', 'before', 'after', 'closed_scope', 'outcomes'}
             and type(m['schema']) is int and m['schema'] == 1, 'Exact acquisition schema1')
        before = strict(reference(directory, m['before']))
        after = strict(reference(directory, m['after']))
        rows = plan(before, m['own_id'], m['declared_ids'])
        need(rows == plan(after, m['own_id'], m['declared_ids'])
             == plan(current_raw, m['own_id'], m['declared_ids']), 'Full selected keys unchanged before/after/current')
        scope = strict(reference(directory, m['closed_scope']))
        need(type(scope) is dict and set(scope) == {'schema', 'acquisition_id', 'helper_source_sha256',
             'helper_elf_sha256', 'qualification_sha256', 'state'} and type(scope['schema']) is int and scope['schema'] == 1
             and type(scope['acquisition_id']) is str and re.fullmatch('[0-9a-f]{32}', scope['acquisition_id'])
             and scope['helper_source_sha256'] == SOURCE_SHA and scope['helper_elf_sha256'] == ELF_SHA
             and type(scope['qualification_sha256']) is str and re.fullmatch('[0-9a-f]{64}', scope['qualification_sha256'])
             and scope['state'] == 'externally_recorded_closed', 'Pinned externally closed acquisition data; no native authority')
        outcomes = m['outcomes']
        need(type(outcomes) is list and len(outcomes) == len(rows), 'Complete declared finite outcomes')
        apps = catalogue(current_raw, m['own_id'])
        need(type(snapshot) is dict and type(snapshot.get('applications')) is list
             and type(snapshot.get('launch_ids')) is list, 'Snapshot arrays')
        actual = snapshot['applications']
        need(all(type(a) is dict and type(a.get('id')) is str for a in actual)
             and len({a['id'] for a in actual}) == len(actual), 'No duplicate snapshot IDs')
        by_id = {a['id']: a for a in actual}
        need(all('icon' not in a for a in actual), 'No unproven prelabelled icon assets')
        assets, ledger, total = {}, [], 0
        for row, outcome in zip(rows, outcomes):
            need(type(outcome) is dict and outcome.get('app_id') == row['app_id']
                 and outcome.get('source') == row['source'], 'Exact ordered outcome/source association')
            identity = row['app_id']
            need(by_id.get(identity) == apps[identity]['app'] and identity in snapshot['launch_ids'], 'Snapshot app identity association')
            if outcome.get('status') == 'unavailable':
                need(set(outcome) == {'app_id', 'source', 'status', 'reason'}
                     and outcome['reason'] == (row['unavailable'] or 'not_acquired'), 'Explicit unavailable, never invented successful read')
                ledger.append(outcome.copy())
                continue
            need(set(outcome) == {'app_id', 'source', 'status', 'frame', 'stderr', 'completion'}
                 and outcome['status'] == 'acquired' and row['request'] is not None, 'Supported acquired outcome only')
            frame = reference(directory, outcome['frame'], 8 + PROTOCOL.MAX_HEADER + PROTOCOL.MAX_BODY)
            decoded = PROTOCOL.decode_response(frame, *row['request'])
            stderr = reference(directory, outcome['stderr'], 128)
            completion = strict(reference(directory, outcome['completion'], 4096))
            need(type(completion) is dict and set(completion) == {'schema', 'acquisition_id', 'app_id', 'source', 'reader_pid',
                 'returncode', 'closure', 'frame_sha256', 'stderr_sha256'}
                 and type(completion['schema']) is int and completion['schema'] == 1
                 and completion['acquisition_id'] == scope['acquisition_id'] and completion['app_id'] == identity
                 and completion['source'] == row['source'] and type(completion['reader_pid']) is int and completion['reader_pid'] > 1
                 and type(completion['returncode']) is int and completion['returncode'] == 0
                 and completion['closure'] == 'externally_recorded_exact_pid_absent'
                 and completion['frame_sha256'] == digest(frame) and completion['stderr_sha256'] == digest(stderr)
                 and stderr == f"LUMA_PUBLIC_ICON_READER_V1 pid={completion['reader_pid']}\n".encode(),
                 'Exact externally asserted successful reader/PID/closure/body associations')
            body = decoded['body']
            total += len(body)
            need(total <= MAX_TOTAL, 'Finite total acquired payload16MiB')
            extension = row['request'][2].rsplit('.', 1)[-1].lower()
            leaf = decoded['sha256'] + '.' + extension
            need(leaf not in assets or assets[leaf] == body, 'No content leaf alias')
            assets[leaf] = body
            by_id[identity]['icon'] = 'icons/' + leaf
            ledger.append(dict(app_id=identity, source=row['source'], status='acquired',
                               icon='icons/' + leaf, sha256=decoded['sha256'], bytes=len(body),
                               frame_sha256=digest(frame), completion_sha256=outcome['completion']['sha256']))
        record = dict(schema=1, acquisition_manifest_sha256=digest(raw), closed_scope_sha256=m['closed_scope']['sha256'],
                      helper_source_sha256=SOURCE_SHA, helper_elf_sha256=ELF_SHA,
                      snapshot_sha256=digest(encoded(snapshot)), declared_ids=m['declared_ids'], outcomes=ledger,
                      external_closure_assertions_checked=True, native_closure_verified=False, native_authority=False)
        return snapshot, assets, record


def publish(output, snapshot, assets, record):
    """Fresh private output only. Any partial IO failure stays visible; no rollback deletion."""
    with PrivateDirectory(output) as root:
        # Precheck before writing anything; O_EXCL is still the authority at write.
        for name in ('snapshot.json', 'icon-acquisition.json', 'icons'):
            try:
                os.stat(name, dir_fd=root.fd, follow_symlinks=False)
            except FileNotFoundError:
                continue
            raise ValueError('Fresh snapshot output required')
        os.mkdir('icons', 0o700, dir_fd=root.fd)
        with PrivateDirectory(Path(output) / 'icons') as icons:
            for leaf, body in assets.items():
                icons.write(leaf, body)
        root.write('snapshot.json', encoded(snapshot))
        root.write('icon-acquisition.json', encoded(record))
        os.fsync(root.fd)


def package_assets(assets, receipt_path, expected_sha256):
    """Offline exact body/snapshot association. No fresh-source or native claim."""
    assets = Path(assets).absolute()
    need(Path(receipt_path).absolute() == assets / 'icon-acquisition.json', 'Fixed acquisition receipt in assets')
    with PrivateDirectory(assets) as root:
        raw = root.read('icon-acquisition.json')
        need(type(expected_sha256) is str and re.fullmatch('[0-9a-f]{64}', expected_sha256)
             and digest(raw) == expected_sha256, 'Explicit pinned ingestion receipt')
        r = strict(raw)
        snapshot_raw = root.read('snapshot.json')
        snapshot = strict(snapshot_raw)
        need(type(r) is dict and set(r) == {'schema', 'acquisition_manifest_sha256', 'closed_scope_sha256',
             'helper_source_sha256', 'helper_elf_sha256', 'snapshot_sha256', 'declared_ids', 'outcomes',
             'external_closure_assertions_checked', 'native_closure_verified', 'native_authority'}
             and type(r['schema']) is int and r['schema'] == 1 and r['helper_source_sha256'] == SOURCE_SHA
             and r['helper_elf_sha256'] == ELF_SHA and r['snapshot_sha256'] == digest(snapshot_raw)
             and r['native_authority'] is False and r['native_closure_verified'] is False
             and r['external_closure_assertions_checked'] is True, 'Exact ingestion receipt/snapshot binding, no native authority')
        need(type(r['declared_ids']) is list and 0 < len(r['declared_ids']) <= MAX_DECLARED
             and all(app_id(i) for i in r['declared_ids']) and r['declared_ids'] == sorted(set(r['declared_ids']))
             and type(r['outcomes']) is list and len(r['outcomes']) == len(r['declared_ids']), 'Complete bounded receipt outcomes')
        apps = snapshot.get('applications')
        need(type(apps) is list and all(type(a) is dict and app_id(a.get('id')) for a in apps)
             and len({a['id'] for a in apps}) == len(apps), 'Unique packaged app IDs')
        need(set(r['declared_ids']) <= {a['id'] for a in apps}, 'All declared outcomes still associated with snapshot apps')
        expected = {}
        for identity, row in zip(r['declared_ids'], r['outcomes']):
            need(type(row) is dict and row.get('app_id') == identity, 'Ordered package outcomes')
            if row.get('status') == 'acquired':
                need(set(row) == {'app_id', 'source', 'status', 'icon', 'sha256', 'bytes', 'frame_sha256', 'completion_sha256'}
                     and type(row['sha256']) is str and re.fullmatch('[0-9a-f]{64}', row['sha256'])
                     and type(row['bytes']) is int and 0 < row['bytes'] <= PROTOCOL.MAX_BODY
                     and type(row['icon']) is str and re.fullmatch('icons/' + row['sha256'] + r'\.(png|jpg|jpeg)', row['icon']), 'Content-addressed packaged binding')
                expected[identity] = row
            else:
                need(row.get('status') == 'unavailable' and set(row) == {'app_id', 'source', 'status', 'reason'}
                     and row['reason'] in ('no_source', 'unsupported_root', 'reader_key_unavailable', 'not_acquired'), 'Honest unavailable package row')
        actual = {a['id']: a['icon'] for a in apps if 'icon' in a}
        need(actual == {i: row['icon'] for i, row in expected.items()}, 'Exact snapshot app/leaf outcome association')
        bodies, total = {}, 0
        with PrivateDirectory(assets / 'icons') as icons:
            for row in expected.values():
                body = icons.read(row['icon'][6:], PROTOCOL.MAX_BODY)
                need(len(body) == row['bytes'] and digest(body) == row['sha256'], 'Exact acquired package body')
                total += len(body)
                need(total <= MAX_TOTAL, 'Finite packaged payload')
                bodies[row['icon']] = body
        return snapshot_raw, bodies, digest(raw)


def stage_package_icons(stage, bodies):
    """Only supplied already-validated bytes; the caller owns its fresh package stage."""
    if not bodies:
        return
    icons = Path(stage) / 'icons'
    icons.mkdir(mode=0o755, exist_ok=False)
    for relative, body in bodies.items():
        need(re.fullmatch(r'icons/[0-9a-f]{64}\.(png|jpg|jpeg)', relative), 'Closed staged icon leaf')
        fd = os.open(icons / relative[6:], os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o644)
        with os.fdopen(fd, 'wb') as f:
            os.fchmod(f.fileno(), 0o644)
            f.write(body)
