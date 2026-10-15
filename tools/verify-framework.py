#!/usr/bin/env python3
"""Read-only verification of the complete pinned external framework checkout.

No preparation, reset, fetch, filters, build, or repair occurs here. Git HEAD
defines every upstream blob and executable bit; the locked patch defines the
only permitted changed/new files. Ignored files are checked too, including
configuration and build inputs outside the five selected crates.
"""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import sys


class VerificationError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise VerificationError(message)


def safe_path(value):
    require(isinstance(value, str) and value and not any(ord(c) < 32 for c in value),
            'Invalid framework path')
    path = PurePosixPath(value)
    require(not path.is_absolute() and str(path) == value
            and all(part not in ('', '.', '..', '.git') for part in path.parts),
            'Unsafe framework path: ' + value)
    return value


def git(checkout, *args):
    # Do not inherit alternate work-tree/object paths or replacement objects.
    env = {key: value for key, value in os.environ.items() if not key.startswith('GIT_')}
    env.update(GIT_OPTIONAL_LOCKS='0', GIT_TERMINAL_PROMPT='0',
               GIT_NO_LAZY_FETCH='1', LC_ALL='C')
    result = subprocess.run(
        ['git', '--no-optional-locks', '--no-replace-objects', '-C', str(checkout), *args],
        capture_output=True, env=env, check=False)
    require(result.returncode == 0, 'Git verification read failed: ' + ' '.join(args)
            + ': ' + result.stderr.decode('utf-8', errors='replace')[-1000:])
    return result.stdout


def patch_paths(raw):
    """Conservative same-path text modifications/additions; no rename/delete/mode edits."""
    try:
        text = raw.decode('utf-8')
    except UnicodeDecodeError as error:
        raise VerificationError('Integration patch must be UTF-8 text') from error
    result = {}
    current = None
    for line in text.splitlines():
        if line.startswith('diff --git '):
            match = re.fullmatch(r'diff --git a/(\S+) b/(\S+)', line)
            require(match is not None and match[1] == match[2],
                    'Unsupported integration patch rename/path')
            current = safe_path(match[1])
            require(current not in result, 'Duplicate integration patch path: ' + current)
            result[current] = None
        elif line.startswith('new file mode '):
            require(current is not None and result[current] is None,
                    'Invalid integration patch new-file declaration')
            mode = line.removeprefix('new file mode ')
            require(mode in ('100644', '100755'), 'Unsupported new framework file mode')
            result[current] = mode
        elif line.startswith(('deleted file mode ', 'old mode ', 'new mode ',
                              'rename from ', 'rename to ', 'copy from ', 'copy to ',
                              'GIT binary patch', 'Binary files ')):
            raise VerificationError('Unsupported integration patch operation: ' + line)
    require(result, 'Empty integration patch')
    return result


def expected_directories(files):
    directories = set()
    for name in files:
        directories.update(str(parent) for parent in PurePosixPath(name).parents
                           if str(parent) != '.')
    return directories


def verify(root):
    root = Path(root).resolve(strict=True)
    lock_path = root / 'dependency-lock.json'
    require(lock_path.is_file() and not lock_path.is_symlink(), 'Missing/linked dependency lock')
    lock_raw = lock_path.read_bytes()
    lock = json.loads(lock_raw)
    revision = lock['upstream']['revision']
    require(isinstance(revision, str) and re.fullmatch(r'[0-9a-f]{40}', revision),
            'Expected full upstream SHA-1 revision')
    patch_name = safe_path(lock['integration_patch'])
    patch = root / patch_name
    for path in (patch, *[root / parent for parent in PurePosixPath(patch_name).parents
                         if str(parent) != '.']):
        require(not path.is_symlink(), 'Linked integration patch path')
    patch_raw = patch.read_bytes()
    patch_hash = hashlib.sha256(patch_raw).hexdigest()
    require(patch_hash == lock.get('integration_patch_sha256'), 'Integration patch hash mismatch')
    patched = lock['patched_file_sha256']
    require(isinstance(patched, dict) and patched, 'Missing patched-file pins')
    for name, digest in patched.items():
        safe_path(name)
        require(isinstance(digest, str) and re.fullmatch(r'[0-9a-f]{64}', digest),
                'Invalid patched-file hash: ' + name)
    changes = patch_paths(patch_raw)
    require(set(changes) == set(patched), 'Patch paths and locked patched-file paths differ')
    dependencies = root / '.dependencies'
    checkout = dependencies / 'plx-native'
    require(dependencies.is_dir() and not dependencies.is_symlink()
            and checkout.is_dir() and not checkout.is_symlink(), 'Linked/missing framework root')
    require((checkout / '.git').exists() and not (checkout / '.git').is_symlink(),
            'Missing/linked Git metadata')
    require(git(checkout, 'rev-parse', '--verify', 'HEAD').decode().strip() == revision,
            'Framework HEAD revision mismatch')
    require(Path(git(checkout, 'rev-parse', '--show-toplevel').decode().strip()).resolve() == checkout,
            'Unexpected framework Git work tree')
    tree_raw = git(checkout, 'ls-tree', '-r', '-z', '--full-tree', 'HEAD')
    expected = {}
    for entry in tree_raw.split(b'\0'):
        if not entry:
            continue
        metadata, name_raw = entry.split(b'\t', 1)
        mode, kind, oid = metadata.decode('ascii').split()
        name = safe_path(name_raw.decode('utf-8'))
        require(mode in ('100644', '100755', '120000') and kind == 'blob'
                and re.fullmatch(r'[0-9a-f]{40}', oid), 'Unsupported upstream tree entry: ' + name)
        require(name not in expected, 'Duplicate upstream path')
        expected[name] = (mode, oid)
    require(expected, 'Empty upstream tree')
    for name, new_mode in changes.items():
        if new_mode is not None:
            require(name not in expected, 'Patch new file already exists upstream: ' + name)
            expected[name] = (new_mode, None)
        else:
            require(name in expected and expected[name][0] != '120000',
                    'Patch modification is not an upstream regular file: ' + name)
    directories = expected_directories(expected)
    observed = set()
    for base, dirs, files in os.walk(checkout, topdown=True, followlinks=False):
        if Path(base) == checkout:
            dirs[:] = [name for name in dirs if name != '.git']
            files = [name for name in files if name != '.git']
        for name in list(dirs):
            path = Path(base) / name
            relative = path.relative_to(checkout).as_posix()
            if path.is_symlink():
                dirs.remove(name)
                files.append(name)
            else:
                require(relative in directories, 'Unexpected framework directory: ' + relative)
        for name in files:
            path = Path(base) / name
            relative = path.relative_to(checkout).as_posix()
            require(relative in expected, 'Unexpected framework file (including ignored): ' + relative)
            metadata = path.lstat()
            mode, oid = expected[relative]
            if mode == '120000':
                require(stat.S_ISLNK(metadata.st_mode), 'Upstream symlink replaced: ' + relative)
                data = os.fsencode(os.readlink(path))
            else:
                require(stat.S_ISREG(metadata.st_mode), 'Framework source is linked/special: ' + relative)
                required_exec = 0o111 if mode == '100755' else 0
                require(metadata.st_mode & 0o111 == required_exec and metadata.st_mode & 0o7000 == 0,
                        'Framework executable/special mode changed: ' + relative)
                descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
                with os.fdopen(descriptor, 'rb') as stream:
                    opened = os.fstat(stream.fileno())
                    require((metadata.st_dev, metadata.st_ino, metadata.st_mode) ==
                            (opened.st_dev, opened.st_ino, opened.st_mode),
                            'Framework file replaced before read: ' + relative)
                    data = stream.read()
                after = path.lstat()
                require((metadata.st_dev, metadata.st_ino, metadata.st_size, metadata.st_mtime_ns,
                         metadata.st_ctime_ns) == (after.st_dev, after.st_ino, after.st_size,
                                                   after.st_mtime_ns, after.st_ctime_ns),
                        'Framework file changed during verification: ' + relative)
            if relative in patched:
                require(hashlib.sha256(data).hexdigest() == patched[relative],
                        'Patched framework hash mismatch: ' + relative)
            else:
                blob = b'blob ' + str(len(data)).encode('ascii') + b'\0' + data
                require(hashlib.sha1(blob).hexdigest() == oid,
                        'Upstream framework blob mismatch: ' + relative)
            observed.add(relative)
    require(observed == set(expected), 'Missing framework files: ' + ', '.join(sorted(set(expected) - observed)))
    git(checkout, 'apply', '--reverse', '--check', str(patch))
    require(lock_path.read_bytes() == lock_raw and patch.read_bytes() == patch_raw
            and git(checkout, 'ls-tree', '-r', '-z', '--full-tree', 'HEAD') == tree_raw
            and git(checkout, 'rev-parse', '--verify', 'HEAD').decode().strip() == revision,
            'Framework verification inputs changed during check')
    return {'schema': 1, 'status': 'verified', 'revision': revision,
            'integration_patch_sha256': patch_hash, 'tracked_upstream_blobs': len(expected) - sum(
                mode is not None for mode in changes.values()),
            'patched_files': len(patched), 'new_patch_files': sum(
                mode is not None for mode in changes.values()),
            'known_upstream_symlinks': sum(mode == '120000' for mode, _ in expected.values()),
            'scope': 'Fresh filesystem/Git read; not atomic against concurrent mutation or dependency build-script admission.'}


if __name__ == '__main__':
    try:
        require(len(sys.argv) == 2, 'Usage: verify-framework.py REPO_ROOT')
        print(json.dumps(verify(sys.argv[1]), sort_keys=True))
    except (VerificationError, OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        print('Framework verification refused: ' + str(error), file=sys.stderr)
        raise SystemExit(1)
