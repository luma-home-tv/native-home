"""Host-only compiler/real-filesystem tests. Build outputs require an explicit private directory."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('public_icon_protocol', HERE / 'protocol.py')
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)
BUILD = None
EXTRA_FLAGS = []


class NativeFiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = BUILD / 'fixtures'
        cls.root.mkdir(mode=0o700)
        for source, binary in [('reader.c', 'reader'), ('test_reader.c', 'test-reader')]:
            result = subprocess.run([shutil.which('cc'), '-std=c11', '-Wall', '-Wextra', '-Werror', '-O2', *EXTRA_FLAGS,
                                     str(HERE / source), '-o', str(BUILD / binary)], capture_output=True, timeout=30)
            (BUILD / (binary + '-compile.stdout')).write_bytes(result.stdout)
            (BUILD / (binary + '-compile.stderr')).write_bytes(result.stderr)
            if result.returncode:
                raise AssertionError(result.stderr.decode())

    def case(self, name):
        with tempfile.TemporaryDirectory(dir=self.root) as fixture:
            # root-replaced deliberately retires this fixture; keep it within private build root.
            result = subprocess.run([str(BUILD / 'test-reader'), fixture, name], capture_output=True, timeout=4)
            if Path(fixture + '-old').exists():
                shutil.rmtree(fixture + '-old')
        return result

    def test_actual_descriptor_walk_public_png_jpeg_and_cap(self):
        for name, size in [('good', 32), ('plain777', 32), ('jpeg', 32), ('max', p.MAX_BODY)]:
            with self.subTest(name=name):
                result = self.case(name)
                self.assertEqual(result.returncode, 0, result.stderr)
                raw = result.stdout
                self.assertEqual(raw[:4], b'LPI1')
                header_size = int.from_bytes(raw[4:8], 'big')
                metadata = json.loads(raw[8:8 + header_size])
                self.assertEqual(metadata['bytes'], size)
                self.assertEqual(metadata['uid'], os.getuid())
                self.assertEqual(len(raw[8 + header_size:]), size)
                self.assertEqual(metadata['app_id'], 'com.example.app')
                # Fixture UID injection is not production Root authority.
                if os.getuid() != 0:
                    with self.assertRaises(ValueError):
                        p.decode_response(raw, 'developer_apps', 'com.example.app', metadata['relative'])

    def test_actual_filesystem_adverse_refusals_emit_zero_content(self):
        for name in ['parent-symlink', 'root-replaced', 'file-symlink', 'relative-link',
                     'named-replaced', 'late-replaced', 'before-open-replaced', 'grow', 'grow-before-read', 'rewrite-restored',
                     'hardlink', 'fifo', 'empty', 'oversize', 'special', 'wrong-owner',
                     'private-root', 'private-app', 'private-file', 'private-nested', 'wrong-signature',
                     'invalid-id', 'hidden-id', 'invalid-relative', 'absolute-relative',
                     'uri', 'trailing-slash', 'id-too-long', 'relative-too-long', 'many-components', 'control']:
            with self.subTest(name=name):
                result = self.case(name)
                self.assertEqual(result.returncode, 0, (name, result.stderr))
                self.assertEqual(result.stdout, b'')

    def test_actual_signal_deadline_before_emission(self):
        for case in ('timeout', 'timeout-blocked'):
            result = self.case(case)
            self.assertEqual(result.returncode, 124)
            self.assertEqual(result.stdout, b'')

    def test_actual_pid_lifecycle_write_and_path_admission(self):
        with tempfile.TemporaryDirectory(dir=self.root) as fixture:
            child = subprocess.Popen([str(BUILD / 'test-reader'), fixture, 'pid-correct'],
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            out, err = child.communicate(timeout=4)
            self.assertEqual(child.returncode, 0, err)
            self.assertEqual(out, f'LUMA_PUBLIC_ICON_READER_V1 pid={child.pid}\n'.encode())
        for case in ('pid-pipeline', 'pid-failed', 'pid-short', 'pid-closed', 'pid-interrupted'):
            result = self.case(case)
            self.assertEqual(result.returncode, 0, (case, result.stderr))
            self.assertEqual(result.stdout, b'')
        result = self.case('pid-blocked')
        self.assertEqual(result.returncode, 124)
        self.assertEqual(result.stdout, b'')  # No PATH_IO hook or image data.

    def test_production_has_no_test_root_UID_or_environment_entry(self):
        env = dict(os.environ, PIR_TEST_ROOT=str(self.root), PIR_TEST_UID=str(os.getuid()))
        for args in [[], ['--test-root', str(self.root)],
                     ['developer_apps', '.ssh', '../root.png'],
                     [str(self.root), 'com.example.app', 'icon.png']]:
            result = subprocess.run([str(BUILD / 'reader'), *args], env=env, capture_output=True, timeout=7)
            self.assertEqual(result.returncode, 1)
            self.assertEqual(result.stdout, b'')
        self.assertNotIn(b'PIR_TEST_ROOT', (BUILD / 'reader').read_bytes())


class Protocol(unittest.TestCase):
    def packet(self, body=b'\x89PNG\r\n\x1a\nsynthetic'):
        header = dict(schema=1, root_kind='developer_apps', app_id='com.example.app', relative='images/icon.png',
                      bytes=len(body), device=9, inode=10, uid=0, gid=0, mode=stat.S_IFREG | 0o644, nlink=1,
                      mtime_seconds=10, mtime_nanos=11, ctime_seconds=12, ctime_nanos=13)
        return header, body

    def frame(self, header, body):
        text = json.dumps(header, separators=(',', ':')).encode()
        return b'LPI1' + len(text).to_bytes(4, 'big') + text + body

    def decode(self, raw):
        return p.decode_response(raw, 'developer_apps', 'com.example.app', 'images/icon.png')

    def test_request_path_is_closed_and_compatible_with_metadata_absolute_icon(self):
        folder = p.ROOTS['developer_apps'] + '/com.example.app'
        self.assertEqual(p.icon_request(folder, folder + '/images/icon.png', 'com.example.app'),
                         ('developer_apps', 'com.example.app', 'images/icon.png'))
        self.assertEqual(p.icon_request(folder, 'icon.JPEG', 'com.example.app')[-1], 'icon.JPEG')
        for relative in ['/root/.ssh/icon.png', '../icon.png', 'x//icon.png', 'x/./icon.png',
                         'x/../icon.png', 'icon.png/', 'file:icon.png', 'x\\icon.png', '\nicon.png',
                         'x/' * 8 + 'icon.png', 'x' * 129 + '.png', 'icon.svg']:
            with self.assertRaises(ValueError):
                p.icon_request(folder, relative, 'com.example.app')
        for malformed in (None, False, [], {}):
            with self.assertRaises(ValueError):
                p.icon_request(folder, 'icon.png', malformed)
        for app in ['', '.', '..', '.ssh', 'a/b', 'x' * 181]:
            with self.assertRaises(ValueError):
                p.validate_key('developer_apps', app, 'icon.png')

    def test_complete_framing_and_public_metadata(self):
        header, body = self.packet()
        got = self.decode(self.frame(header, body))
        self.assertEqual(got['body'], body)
        self.assertEqual(len(got['sha256']), 64)
        for key, value in [('schema', True), ('uid', 501), ('nlink', 2), ('mode', stat.S_IFREG | 0o600),
                           ('mode', stat.S_IFREG | 0o4644), ('mode', stat.S_IFREG | 2**40 | 0o644),
                           ('mode', stat.S_IFIFO | 0o644),
                           ('bytes', True), ('inode', -1), ('mtime_nanos', 1000000000),
                           ('root_kind', 'arbitrary'), ('relative', 'wrong.png')]:
            bad = dict(header, **{key: value})
            with self.assertRaises(ValueError):
                self.decode(self.frame(bad, body))
        raw = self.frame(header, body)
        for bad in [raw[:-1], raw + b'x', b'WRONG' + raw, raw[:7], b'LPI1' + (4096).to_bytes(4, 'big') + raw[8:]]:
            with self.assertRaises(ValueError):
                self.decode(bad)

    def test_duplicate_nonfinite_trailing_or_extra_metadata_refused(self):
        header, body = self.packet()
        raw = json.dumps(header).encode()
        for text in [b'{"schema":1,"schema":1}', raw.replace(b'"uid": 0', b'"uid": NaN'),
                     raw + b'trailing', json.dumps(dict(header, unexpected=True)).encode()]:
            with self.assertRaises(ValueError):
                self.decode(b'LPI1' + len(text).to_bytes(4, 'big') + text + body)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--build-dir', required=True, type=Path)
    parser.add_argument('--sanitize', action='store_true')
    args, remaining = parser.parse_known_args()
    BUILD = args.build_dir.resolve()
    EXTRA_FLAGS = ['-fsanitize=address,undefined', '-fno-omit-frame-pointer'] if args.sanitize else []
    BUILD.mkdir(mode=0o700, exist_ok=False)
    unittest.main(argv=[__file__, *remaining], verbosity=2)
