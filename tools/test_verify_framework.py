"""Small temporary Git fixtures; no application build, device access, or network."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('framework_verifier', Path(__file__).with_name('verify-framework.py'))
VERIFIER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERIFIER)


class VerificationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.checkout = self.root / '.dependencies/plx-native'
        self.checkout.mkdir(parents=True)
        self.git('init', '-q')
        self.git('config', 'user.name', 'Framework fixture')
        self.git('config', 'user.email', 'fixture@example.invalid')
        for index in range(8):
            self.write(f'src/file{index}.rs', f'original {index}\n')
        self.write('src/unchanged.rs', 'unchanged\n')
        self.write('.gitignore', '.cargo/\n*.cache\n')
        self.write('script.sh', '#!/bin/sh\nexit 0\n')
        (self.checkout / 'script.sh').chmod(0o755)
        self.write('.agents/skills/fixture.txt', 'fixture\n')
        (self.checkout / '.claude').mkdir()
        (self.checkout / '.claude/skills').symlink_to('../.agents/skills')
        self.git('add', '.')
        self.git('commit', '-qm', 'upstream fixture')
        revision = self.git('rev-parse', 'HEAD').decode().strip()
        for index in range(8):
            self.write(f'src/file{index}.rs', f'patched {index}\n')
        self.write('src/new.rs', 'new patch file\n')
        self.git('add', '-N', 'src/new.rs')
        patch = self.git('diff', '--no-ext-diff', '--binary', 'HEAD')
        self.patch = self.root / 'patches/integration.patch'
        self.patch.parent.mkdir()
        self.patch.write_bytes(patch)
        self.lock = {
            'upstream': {'revision': revision, 'repository': 'unused@example.invalid'},
            'integration_patch': 'patches/integration.patch',
            'integration_patch_sha256': hashlib.sha256(patch).hexdigest(),
            'patched_file_sha256': {name: hashlib.sha256((self.checkout / name).read_bytes()).hexdigest()
                                    for name in [*[f'src/file{i}.rs' for i in range(8)], 'src/new.rs']},
            'unrelated_documentation_field': 'allowed lock metadata',
        }
        self.save_lock()

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.checkout), *args], stderr=subprocess.PIPE)

    def write(self, name, text):
        path = self.checkout / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def save_lock(self):
        (self.root / 'dependency-lock.json').write_text(json.dumps(self.lock))

    def refuses(self, fragment):
        with self.assertRaisesRegex(VERIFIER.VerificationError, fragment):
            VERIFIER.verify(self.root)

    def test_valid_nine_patch_files_new_file_and_known_symlink_are_read_only(self):
        before = self.git('status', '--porcelain=v1', '--untracked-files=all')
        index = (self.checkout / '.git/index').read_bytes()
        report = VERIFIER.verify(self.root)
        self.assertEqual(report['status'], 'verified')
        self.assertEqual(report['patched_files'], 9)
        self.assertEqual(report['new_patch_files'], 1)
        self.assertEqual(report['known_upstream_symlinks'], 1)
        self.assertEqual(before, self.git('status', '--porcelain=v1', '--untracked-files=all'))
        self.assertEqual(index, (self.checkout / '.git/index').read_bytes())

    def test_unpatched_tracked_bytes_drift_refused(self):
        self.write('src/unchanged.rs', 'formatted or arbitrary changed bytes\n')
        self.refuses('Upstream framework blob mismatch')

    def test_patched_bytes_drift_refused(self):
        self.write('src/file0.rs', 'unlocked patch bytes\n')
        self.refuses('Patched framework hash mismatch')

    def test_missing_tracked_and_missing_new_patch_file_refused(self):
        for name in ('src/unchanged.rs', 'src/new.rs'):
            path = self.checkout / name
            original = path.read_bytes()
            path.unlink()
            self.refuses('Missing framework files')
            path.write_bytes(original)

    def test_revision_mismatch_refused(self):
        self.lock['upstream']['revision'] = '0' * 40
        self.save_lock()
        self.refuses('HEAD revision mismatch')

    def test_patch_hash_mismatch_refused(self):
        self.patch.write_bytes(self.patch.read_bytes() + b'\n')
        self.refuses('Integration patch hash mismatch')

    def test_patch_mapping_extra_or_missing_pin_refused(self):
        original = dict(self.lock['patched_file_sha256'])
        self.lock['patched_file_sha256']['src/unchanged.rs'] = '0' * 64
        self.save_lock()
        self.refuses('Patch paths and locked patched-file paths differ')
        self.lock['patched_file_sha256'] = original
        del self.lock['patched_file_sha256']['src/file0.rs']
        self.save_lock()
        self.refuses('Patch paths and locked patched-file paths differ')

    def test_untracked_and_ignored_files_and_configuration_refused(self):
        for name in ('src/injected.rs', 'build.cache', '.cargo/config.toml'):
            path = self.checkout / name
            self.write(name, 'unknown source or config\n')
            self.refuses('Unexpected framework (file|directory)')
            path.unlink()
            if path.parent.name == '.cargo':
                path.parent.rmdir()

    def test_empty_unknown_directory_refused(self):
        (self.checkout / '.cargo').mkdir()
        self.refuses('Unexpected framework directory')

    def test_source_file_symlink_and_directory_symlink_refused(self):
        source = self.checkout / 'src/unchanged.rs'
        original = source.read_bytes()
        source.unlink()
        source.symlink_to('file0.rs')
        self.refuses('Framework source is linked/special')
        source.unlink()
        source.write_bytes(original)
        (self.checkout / 'src').rename(self.checkout / 'saved-src')
        (self.checkout / 'src').symlink_to('saved-src', target_is_directory=True)
        self.refuses('Unexpected framework (file|directory)')

    def test_known_symlink_target_or_type_drift_refused(self):
        link = self.checkout / '.claude/skills'
        link.unlink()
        link.symlink_to('../src')
        self.refuses('Upstream framework blob mismatch')
        link.unlink()
        link.write_text('../.agents/skills')
        self.refuses('Upstream symlink replaced')

    def test_executable_mode_drift_refused(self):
        (self.checkout / 'script.sh').chmod(0o644)
        self.refuses('Framework executable/special mode changed')

    def test_staged_extra_file_is_not_admitted_by_index(self):
        self.write('src/staged.rs', 'not in HEAD or patch\n')
        self.git('add', 'src/staged.rs')
        self.refuses('Unexpected framework file')


if __name__ == '__main__':
    unittest.main()
