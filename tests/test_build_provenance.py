"""Temporary source/artifact fixtures, mocked read-only framework checker; no builds/TV."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import stat
import tempfile
import unittest
from unittest import mock

MODULE = Path(__file__).resolve().parents[1] / 'build_provenance.py'
spec = importlib.util.spec_from_file_location('build_provenance', MODULE)
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


class ProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name).resolve() / 'repo'
        self.root.mkdir()
        for name in p.ROOT_FILES:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('fixture ' + name)
        (self.root / 'dependency-lock.json').write_text(json.dumps({'integration_patch': 'patches/framework.patch'}))
        (self.root / 'patches').mkdir()
        (self.root / 'patches/framework.patch').write_text('fixed patch')
        for directory in p.TREES:
            path = self.root / directory
            path.mkdir(parents=True)
            (path / 'input.txt').write_text('input')
        self.framework = {'schema': 1, 'status': 'verified', 'revision': 'a' * 40,
                          'integration_patch_sha256': 'b' * 64, 'patched_files': 9}
        self.verifier = mock.patch.object(p, '_framework', side_effect=lambda root: copy.deepcopy(self.framework))
        self.verifier.start(); self.addCleanup(self.verifier.stop)
        self.binary = Path(self.tmp.name).resolve() / 'home-preview'
        elf = bytearray(52); elf[:7] = b'\x7fELF\x01\x01\x01'; elf[18:20] = (40).to_bytes(2, 'little')
        self.binary.write_bytes(elf)
        self.receipt = self.binary.with_name('home-preview.build.json')

    def record(self, target='arm'):
        before = p.snapshot(self.root)
        return p.record_build_receipt(self.root, self.binary, target, before, p.snapshot(self.root))

    def test_real_hash_receipt_private_atomic_and_validates(self):
        result = self.record()
        self.assertEqual(result, self.receipt)
        self.assertEqual(stat.S_IMODE(result.stat().st_mode), 0o600)
        value = p.validate_build_receipt(self.root, self.binary, result)
        self.assertEqual(value['binary'], p._hash(self.binary.read_bytes()) | {'name': 'home-preview'})
        self.assertEqual(list(self.binary.parent.glob('.build-receipt-*')), [])
        self.assertEqual(value['toolchain']['arm_target'], 'arm-unknown-linux-gnueabi')
        self.receipt.chmod(0o400)
        p.validate_build_receipt(self.root, self.binary, self.receipt)

    def test_missing_receipt_or_binary_refuses(self):
        with self.assertRaises(FileNotFoundError):
            p.validate_build_receipt(self.root, self.binary, self.receipt)
        self.record(); self.binary.unlink()
        with self.assertRaises(FileNotFoundError):
            p.validate_build_receipt(self.root, self.binary, self.receipt)

    def test_binary_source_patch_and_dependency_drift_refuse(self):
        self.record()
        original = self.binary.read_bytes(); self.binary.write_bytes(original + b'changed')
        with self.assertRaisesRegex(ValueError, 'Binary bytes'):
            p.validate_build_receipt(self.root, self.binary, self.receipt)
        self.binary.write_bytes(original)
        for relative in ('src/input.txt', 'Cargo.lock', 'patches/framework.patch', 'build.rs'):
            path = self.root / relative; raw = path.read_bytes(); path.write_bytes(raw + b'drift')
            with self.assertRaisesRegex(ValueError, 'Current source/dependency'):
                p.validate_build_receipt(self.root, self.binary, self.receipt)
            path.write_bytes(raw)
        self.framework['revision'] = 'c' * 40
        with self.assertRaisesRegex(ValueError, 'Current source/dependency'):
            p.validate_build_receipt(self.root, self.binary, self.receipt)

    def test_before_after_or_current_drift_prevents_receipt_write(self):
        before = p.snapshot(self.root)
        (self.root / 'src/input.txt').write_text('changed')
        for after in (p.snapshot(self.root), before):
            with self.assertRaisesRegex(ValueError, 'changed during build'):
                p.record_build_receipt(self.root, self.binary, 'arm', before, after)
            self.assertFalse(self.receipt.exists())

    def test_untracked_sources_assets_fonts_included_docs_and_caches_ignored(self):
        self.record()
        (self.root / 'docs').mkdir(); (self.root / 'docs/plan.md').write_text('new plan')
        cache = self.root / 'src/__pycache__'; cache.mkdir(); (cache / 'cache.pyc').write_bytes(b'cache')
        (self.root / 'src/other.pyc').write_bytes(b'cache')
        p.validate_build_receipt(self.root, self.binary, self.receipt)
        for directory in p.TREES:
            path = self.root / directory / 'untracked.rs'; path.write_text('new input')
            with self.assertRaisesRegex(ValueError, 'Current source/dependency'):
                p.validate_build_receipt(self.root, self.binary, self.receipt)
            path.unlink()

    def test_host_receipt_and_non_arm_artifact_refuse(self):
        self.record('host')
        with self.assertRaisesRegex(ValueError, 'ARM build receipt'):
            p.validate_build_receipt(self.root, self.binary, self.receipt)
        self.binary.write_bytes(b'host executable')
        with self.assertRaisesRegex(ValueError, 'Actual ARM32'):
            self.record('arm')
        with self.assertRaisesRegex(ValueError, 'Unsupported binary'):
            p._binary(self.binary.parent / '..' / 'arbitrary', 'arm')

    def test_binary_receipt_and_source_links_refuse(self):
        self.record(); real = self.binary.with_name('real-binary'); self.binary.rename(real); self.binary.symlink_to(real)
        with self.assertRaisesRegex(ValueError, 'Linked input'):
            p.validate_build_receipt(self.root, self.binary, self.receipt)
        self.binary.unlink(); real.rename(self.binary)
        real_receipt = self.receipt.with_name('real-receipt'); self.receipt.rename(real_receipt); self.receipt.symlink_to(real_receipt)
        with self.assertRaisesRegex(ValueError, 'Linked input'):
            p.validate_build_receipt(self.root, self.binary, self.receipt)
        self.receipt.unlink(); real_receipt.rename(self.receipt)
        source = self.root / 'src/input.txt'; source.unlink(); source.symlink_to(real)
        with self.assertRaisesRegex(ValueError, 'Linked compile input'):
            p.snapshot(self.root)
        source.unlink(); target = self.root / 'outside'; target.mkdir(); (self.root / 'src/linked').symlink_to(target)
        with self.assertRaisesRegex(ValueError, 'Linked source directory'):
            p.snapshot(self.root)

    def test_linked_artifact_parent_refuses_even_with_regular_leaf_files(self):
        self.record()
        linked = self.binary.parent / 'linked-output'
        linked.symlink_to(self.binary.parent, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'Linked input or parent'):
            p.validate_build_receipt(self.root, linked / self.binary.name, linked / self.receipt.name)

    def test_receipt_must_be_adjacent_bounded_private_json(self):
        self.record(); wrong = self.receipt.with_name('different.json'); wrong.write_bytes(self.receipt.read_bytes())
        with self.assertRaisesRegex(ValueError, 'adjacent'):
            p.validate_build_receipt(self.root, self.binary, wrong)
        self.receipt.chmod(0o644)
        with self.assertRaisesRegex(ValueError, 'Private'):
            p.validate_build_receipt(self.root, self.binary, self.receipt)
        self.receipt.chmod(0o600); self.receipt.write_bytes(b' ' * (p.RECEIPT_LIMIT + 1))
        with self.assertRaisesRegex(ValueError, 'exceeds'):
            p.validate_build_receipt(self.root, self.binary, self.receipt)

    def test_framework_checker_command_and_refusal_are_read_only(self):
        self.verifier.stop()
        reply = mock.Mock(stdout=json.dumps(self.framework).encode())
        with mock.patch.object(p.subprocess, 'run', return_value=reply) as run:
            value = p._framework(self.root)
        self.assertEqual(value, self.framework)
        self.assertEqual(run.call_args.args[0], ['python3', str(self.root / 'tools/verify-framework.py'), str(self.root)])
        reply.stdout = b'{"schema":1,"status":"failed"}'
        with mock.patch.object(p.subprocess, 'run', return_value=reply), self.assertRaisesRegex(ValueError, 'verification'):
            p._framework(self.root)


if __name__ == '__main__':
    unittest.main()
