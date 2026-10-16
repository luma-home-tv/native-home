"""Actual packager with temporary Git/ARM-receipt/inert-ar fixtures; no TV/build."""
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tarfile
import unittest


REPO = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    'cache_package_fixtures', REPO / 'tests/test_preview_artwork_package.py')
BASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)
SIDECAR = 'artwork-cache-producer.json'
OPTION = 'NATIVE_HOME_CACHE_PRODUCER_BROKER'


class CacheProducerPackageTests(unittest.TestCase):
    def setUp(self):
        self.fixture = BASE.PreviewArtworkPackageTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.f = self.fixture.f
        self.serial = 0
        self.git('-c', 'init.templateDir=', 'init', '-q')
        self.git('add', '.')
        self.git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                 '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null',
                 'commit', '-qm', 'Synthetic package fixture sources')
        self.revision = self.git('rev-parse', 'HEAD').stdout.strip()
        self.record_pair()

    def git(self, *args):
        return subprocess.run(['git', '--no-optional-locks', '-C', str(self.f.root),
                               *args], capture_output=True, text=True, check=True,
                              timeout=10, env=self.f.env)

    def record_pair(self):
        before = BASE.PROVENANCE.snapshot(self.f.root)
        BASE.PROVENANCE.record_build_receipt(
            self.f.root, self.fixture.helper, 'arm', before,
            BASE.PROVENANCE.snapshot(self.f.root))
        self.f.record()

    def package(self, *, diagnostic=False, extra=None, association=True):
        self.serial += 1
        output = self.f.output / ('cache-%03d' % self.serial)
        output.mkdir()
        env = {**self.f.env, 'NATIVE_HOME_BUILD_DIR': str(output)}
        if diagnostic:
            env.update(self.fixture.environment())
        else:
            env['NATIVE_HOME_INTERACTIVE_SESSION'] = '1'
        if association:
            env[OPTION] = str(self.fixture.helper)
        env.update(extra or {})
        self.fixture.ar_marker.unlink(missing_ok=True)
        return self.f.run_entrypoint('package-preview.py', env=env), output

    def files(self, result, output):
        self.assertEqual(result.returncode, 0, result.stderr)
        paths = list(output.glob('*.ipk'))
        self.assertEqual(len(paths), 1)
        members = BASE.ar_members(paths[0].read_bytes())
        with tarfile.open(fileobj=io.BytesIO(members['data.tar.gz']), mode='r:gz') as t:
            files = {Path(row.name).name: (row, t.extractfile(row).read())
                     for row in t.getmembers() if row.isfile()}
        return files, json.loads((output / 'preview-package-receipt.json').read_text())

    def refused(self, **kwargs):
        result, output = self.package(**kwargs)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(list(output.glob('*.ipk')))
        self.assertFalse((output / 'control.tar.gz').exists())
        self.assertFalse((output / 'preview-package-receipt.json').exists())
        self.assertFalse(self.fixture.ar_marker.exists())
        return result

    def test_same_elf_normal_and_diagnostic_share_exact_post_build_association(self):
        normal, nr = self.files(*self.package())
        diagnostic, dr = self.files(*self.package(diagnostic=True))
        self.assertEqual(normal['home-preview'][1], diagnostic['home-preview'][1])
        self.assertEqual(normal[SIDECAR][1], diagnostic[SIDECAR][1])
        expected = {'schema': 1, 'app_id': BASE.APP,
                    'producer_kind': 'joined-single-positive-v1',
                    'source_revision': self.revision,
                    'ui_sha256': BASE.sha(self.f.binary.read_bytes()),
                    'broker_sha256': BASE.sha(self.fixture.helper.read_bytes())}
        self.assertEqual(json.loads(normal[SIDECAR][1]), expected)
        self.assertEqual(normal[SIDECAR][0].mode, 0o644)
        self.assertEqual((normal[SIDECAR][0].uid, normal[SIDECAR][0].gid), (0, 0))
        self.assertEqual(nr['cache_producer'], expected)
        self.assertEqual(nr['cache_producer_sidecar_sha256'], BASE.sha(normal[SIDECAR][1]))
        self.assertEqual(nr['cache_producer'], dr['cache_producer'])
        self.assertGreater(nr['cache_producer_matching_Git_blobs'], 0)
        self.assertNotIn('preview-artwork-admission.json', normal)
        self.assertIn('interactive-session.json', normal)
        self.assertNotIn('interactive-session.json', diagnostic)
        self.assertIn('preview-artwork-admission.json', diagnostic)
        self.assertNotIn('artwork-cache-preparation.json', normal)
        self.assertNotIn('artwork-cache-preparation.json', diagnostic)
        self.assertFalse(nr['cache_preparation_selected'])
        self.assertFalse(dr['cache_preparation_selected'])
        self.assertNotIn('artwork-broker', normal)
        self.assertNotIn('artwork-broker', diagnostic)
        self.assertEqual(nr['permissions'], [])
        self.assertEqual(nr['boot_hooks'], [])

    def test_default_packages_add_no_cache_association(self):
        for diagnostic in (False, True):
            with self.subTest(diagnostic=diagnostic):
                files, receipt = self.files(*self.package(
                    diagnostic=diagnostic, association=False))
                self.assertNotIn(SIDECAR, files)
                self.assertIsNone(receipt['cache_producer'])
                self.assertEqual(receipt['cache_producer_matching_Git_blobs'], 0)

    def test_wrong_role_empty_option_or_mixed_trials_are_rejected(self):
        for extra in [
            {OPTION: ''},
            {'NATIVE_HOME_TEST_IDENTITY': '1'},
            {'NATIVE_HOME_NAVIGATION_PROOF': '1'},
            {'NATIVE_HOME_FUTURE_ACTION': '1'},
            {'NATIVE_HOME_SESSION_CAPTURE_PROOF': '0'},
        ]:
            with self.subTest(extra=extra):
                self.refused(extra=extra)

    def test_other_helper_basename_and_missing_receipt_are_rejected(self):
        other = self.f.base / 'artwork-broker-startup-proof'
        other.write_bytes(self.fixture.helper.read_bytes())
        self.refused(extra={OPTION: str(other)})
        self.fixture.helper_receipt.unlink()
        self.refused()

    def test_changed_helper_or_build_context_is_rejected(self):
        original = self.fixture.helper.read_bytes()
        self.fixture.helper.write_bytes(original + b'drift')
        self.refused()
        self.fixture.helper.write_bytes(original)
        receipt = json.loads(self.fixture.helper_receipt.read_text())
        receipt['git_revision'] = 'f' * 40
        self.fixture.helper_receipt.write_text(json.dumps(receipt))
        self.refused()

    def test_revision_label_without_matching_committed_input_bytes_is_rejected(self):
        (self.f.root / 'src/fixture.txt').write_text('uncommitted changed source\n')
        self.record_pair()
        result = self.refused()
        self.assertIn('actual input blobs', result.stderr)

    def test_documentation_head_can_follow_the_actual_source_revision(self):
        (self.f.root / 'README.md').write_text('Later documentation only\n')
        self.git('add', 'README.md')
        self.git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                 '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null',
                 'commit', '-qm', 'Later fixture documentation')
        self.assertNotEqual(self.git('rev-parse', 'HEAD').stdout.strip(), self.revision)
        files, _ = self.files(*self.package())
        self.assertEqual(json.loads(files[SIDECAR][1])['source_revision'], self.revision)

    def test_diagnostic_protected_helper_pins_remain_independent(self):
        pins = dict(self.fixture.pins, broker_sha256='f' * 64)
        self.fixture.write_pins(pins)
        self.refused(diagnostic=True)


if __name__ == '__main__':
    unittest.main()
