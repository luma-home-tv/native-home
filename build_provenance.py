"""Future build/package binding; snapshots and receipts never prove TV behavior."""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import subprocess
import tempfile

ROOT_FILES = ('Cargo.toml', 'Cargo.lock', 'build.rs', 'sdl-events.c', 'build-client.py',
              'package-preview.py', 'prepare-framework.py', 'dependency-lock.json',
              'build_provenance.py', 'tools/verify-framework.py')
TREES = ('src', 'assets/luma-home', 'resources/fonts')
BINARY_NAMES = {'home-preview', 'control-preview', 'launch-target', 'artwork-broker-proof', 'artwork-broker-startup-proof'}
RECEIPT_LIMIT = 4 * 1024 * 1024


def _require(ok, message):
    if not ok:
        raise ValueError(message)


def _read(path, private=False, limit=None):
    path = Path(path)
    _require(not any(part.is_symlink() for part in (path.absolute(), *path.absolute().parents)),
             'Linked input or parent: ' + str(path))
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno())
        _require(stat.S_ISREG(before.st_mode), 'Regular file required: ' + str(path))
        if private:
            _require(before.st_uid == os.geteuid() and stat.S_IMODE(before.st_mode) in (0o400, 0o600)
                     and before.st_nlink == 1, 'Private0400/0600 receipt required')
        _require(limit is None or before.st_size <= limit, 'File exceeds provenance bound')
        raw = stream.read() if limit is None else stream.read(limit + 1)
        after = os.fstat(stream.fileno())
    _require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns)
             == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)
             and len(raw) == before.st_size, 'File changed during provenance read')
    return raw


def _hash(raw):
    return {'sha256': hashlib.sha256(raw).hexdigest(), 'bytes': len(raw)}


def _framework(root):
    result = subprocess.run(['python3', str(root / 'tools/verify-framework.py'), str(root)],
                            capture_output=True, check=True, timeout=120)
    _require(len(result.stdout) <= RECEIPT_LIMIT, 'Framework verification output exceeds bound')
    value = json.loads(result.stdout)
    _require(type(value) is dict and value.get('schema') == 1 and value.get('status') == 'verified',
             'Fresh framework verification required')
    return value


def snapshot(root):
    root = Path(root)
    _require(not root.is_symlink(), 'Linked repository root')
    root = root.resolve(strict=True)
    lock = json.loads(_read(root / 'dependency-lock.json', limit=RECEIPT_LIMIT))
    patch = lock['integration_patch']
    relative = PurePosixPath(patch)
    _require(type(patch) is str and not relative.is_absolute() and str(relative) == patch
             and all(part not in ('.', '..', '.git') for part in relative.parts), 'Unsafe patch path')
    paths = set(ROOT_FILES) | {patch}
    for name in TREES:
        directory = root / name
        _require(directory.is_dir() and not directory.is_symlink(), 'Missing/linked source tree: ' + name)
        for base, dirs, files in os.walk(directory, followlinks=False):
            dirs[:] = sorted(d for d in dirs if d != '__pycache__')
            for child in dirs:
                _require(not (Path(base) / child).is_symlink(), 'Linked source directory')
            paths.update(str((Path(base) / f).relative_to(root)) for f in files if not f.endswith('.pyc'))
    inputs = {}
    for name in sorted(paths):
        current = root
        for part in PurePosixPath(name).parts:
            current = current / part
            _require(not current.is_symlink(), 'Linked compile input: ' + name)
        inputs[name] = _hash(_read(current))
    return {'schema': 1, 'source_inputs': inputs, 'framework': _framework(root),
            'toolchain': {'rust_toolchain': os.environ.get('NATIVE_HOME_RUST_TOOLCHAIN', 'nightly-2026-07-02'),
                          'arm_target': 'arm-unknown-linux-gnueabi', 'target_cpu': 'cortex-a9',
                          'target_features': '-neon'}}


def _binary(binary, target):
    binary = Path(binary)
    _require(binary.name in BINARY_NAMES and target in ('arm', 'host'), 'Unsupported binary name/target')
    raw = _read(binary)
    if target == 'arm':
        _require(len(raw) >= 52 and raw[:7] == b'\x7fELF\x01\x01\x01'
                 and int.from_bytes(raw[18:20], 'little') == 40, 'Actual ARM32 little-endian ELF required')
    return {'name': binary.name, **_hash(raw)}


def record_build_receipt(root, binary, target, before, after):
    _require(before == after and after == snapshot(root), 'Build inputs/framework changed during build')
    identity = _binary(binary, target)
    git = subprocess.run(['git', '--no-optional-locks', '-C', str(Path(root).resolve()),
                          'rev-parse', '--verify', 'HEAD'], capture_output=True, timeout=10)
    value = {'schema': 1, 'target': target, 'binary': identity, 'snapshot': after,
             'git_revision': git.stdout.decode().strip() if git.returncode == 0 else None,
             'toolchain': after['toolchain'],
             'scope': 'Actual binary hash and matching compile inputs/framework; not TV validation.'}
    destination = Path(binary).with_name(Path(binary).name + '.build.json')
    _require(not destination.is_symlink(), 'Linked receipt destination')
    fd, temporary = tempfile.mkstemp(prefix='.build-receipt-', dir=destination.parent)
    try:
        os.fchmod(fd, 0o600)
        with os.fdopen(fd, 'w') as stream:
            json.dump(value, stream, sort_keys=True, indent=2, allow_nan=False)
            stream.write('\n'); stream.flush(); os.fsync(stream.fileno())
        os.replace(temporary, destination)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    return destination


def validate_build_receipt(root, binary, receipt):
    binary, receipt = Path(binary), Path(receipt)
    _require(receipt.absolute() == binary.with_name(binary.name + '.build.json').absolute(),
             'Receipt must be adjacent to its binary')
    value = json.loads(_read(receipt, private=True, limit=RECEIPT_LIMIT))
    _require(type(value) is dict and value.get('schema') == 1 and value.get('target') == 'arm',
             'ARM build receipt required')
    _require(value.get('binary') == _binary(binary, 'arm'), 'Binary bytes/name differ from build receipt')
    current = snapshot(root)
    _require(value.get('snapshot') == current and value.get('toolchain') == current['toolchain'],
             'Current source/dependency inputs differ from build receipt')
    return value
