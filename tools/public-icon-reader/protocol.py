"""Pure request/framing boundary for the unqualified public-icon CLI.

This module does not stage, execute or trust a helper, contact a TV, or grant Root
or catalogue freshness authority. A future reviewed transport must do that.
"""
import hashlib
import json
import re
import stat

ROOTS = {
    'lg_system': '/usr/palm/applications',
    'encrypted_apps': '/media/cryptofs/apps/usr/palm/applications',
    'developer_apps': '/media/developer/apps/usr/palm/applications',
}
MAX_BODY = 1048576
MAX_HEADER = 4096
FIELDS = {'schema', 'root_kind', 'app_id', 'relative', 'bytes', 'device', 'inode',
          'uid', 'gid', 'mode', 'nlink', 'mtime_seconds', 'mtime_nanos',
          'ctime_seconds', 'ctime_nanos'}


def _need(value, message):
    if not value:
        raise ValueError(message)


def validate_key(root_kind, app_id, relative):
    _need(type(root_kind) is str and root_kind in ROOTS, 'Closed public root kind')
    _need(type(app_id) is str and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,179}', app_id),
          'Validated single app directory')
    _need(type(relative) is str and 0 < len(relative) <= 512 and
          all(32 <= ord(c) <= 126 and c not in '\\:' for c in relative),
          'Bounded printable relative path only')
    parts = relative.split('/')
    _need(1 <= len(parts) <= 8 and all(0 < len(p) <= 128 and p not in ('.', '..') for p in parts),
          'No absolute, empty, traversal or unbounded path components')
    extension = parts[-1].rsplit('.', 1)
    _need(len(extension) == 2 and extension[-1].lower() in ('png', 'jpg', 'jpeg'),
          'PNG/JPEG filename only')
    return root_kind, app_id, relative


def icon_request(folder, image, app_id):
    """Normalize eligible metadata into argv data; it never opens a path."""
    _need(type(folder) is str and type(image) is str and type(app_id) is str, 'Typed folder/icon/app metadata')
    roots = [kind for kind, root in ROOTS.items() if folder == root + '/' + app_id]
    _need(len(roots) == 1, 'Exact public root plus app directory')
    relative = image[len(folder) + 1:] if image.startswith(folder + '/') else image
    return validate_key(roots[0], app_id, relative)


def _object(pairs):
    out = {}
    for key, value in pairs:
        _need(key not in out, 'Duplicate metadata key')
        out[key] = value
    return out


def decode_response(raw, root_kind, app_id, relative):
    """Strict complete-frame parser, not helper identity or native qualification."""
    validate_key(root_kind, app_id, relative)
    _need(type(raw) is bytes and 8 < len(raw) <= 8 + MAX_HEADER + MAX_BODY and raw[:4] == b'LPI1',
          'Complete bounded public icon frame')
    header_size = int.from_bytes(raw[4:8], 'big')
    _need(0 < header_size < MAX_HEADER and 8 + header_size < len(raw), 'Bounded complete metadata')
    header = json.loads(raw[8:8 + header_size].decode('utf-8'), object_pairs_hook=_object,
                        parse_constant=lambda _: (_ for _ in ()).throw(ValueError('Nonfinite metadata')))
    _need(type(header) is dict and set(header) == FIELDS, 'Exact metadata schema')
    _need(type(header['schema']) is int and header['schema'] == 1 and
          (header['root_kind'], header['app_id'], header['relative']) == (root_kind, app_id, relative),
          'Same explicit requested key')
    for key in FIELDS - {'root_kind', 'app_id', 'relative'}:
        _need(type(header[key]) is int, 'Exact integer metadata')
    _need(all(0 <= header[k] <= 2**64 - 1 for k in
              ('bytes', 'device', 'inode', 'uid', 'gid', 'mode', 'nlink')) and
          all(-2**63 <= header[k] < 2**63 for k in ('mtime_seconds', 'ctime_seconds')) and
          all(0 <= header[k] < 1000000000 for k in ('mtime_nanos', 'ctime_nanos')),
          'Bounded native metadata numbers')
    _need(header['uid'] == header['gid'] == 0 and header['nlink'] == 1 and header['mode'] <= 0xffff and
          stat.S_ISREG(header['mode']) and header['mode'] & stat.S_IROTH and
          not header['mode'] & ~(stat.S_IFREG | 0o777), 'Root-owned public readable plain regular single-link file')
    body = raw[8 + header_size:]
    _need(0 < len(body) == header['bytes'] <= MAX_BODY, 'Exact complete body and EOF, no truncation/tail')
    png = relative.rsplit('.', 1)[-1].lower() == 'png'
    _need(body.startswith(b'\x89PNG\r\n\x1a\n') if png else body.startswith(b'\xff\xd8\xff'),
          'Matching image signature only; decode validity remains separate')
    return {'metadata': header, 'body': body, 'sha256': hashlib.sha256(body).hexdigest()}
