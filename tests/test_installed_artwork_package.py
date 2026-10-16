"""Actual packager in composed temporary fixtures; no compiler, SDK or TV calls."""
import importlib.util
import io
import json
from pathlib import Path
import re
import tarfile
import unittest

REPO = Path(__file__).resolve().parents[1]
_SPEC = importlib.util.spec_from_file_location(
    'installed_artwork_package_fixtures', REPO / 'tests/test_broker_negative_package.py')
_BASE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_BASE)


class InstalledArtworkPackageTests(unittest.TestCase):
    def setUp(self):
        # Compose only fixture setup/helpers. Do not inherit or rerun the
        # negative-mode class's test methods as installed-artwork tests.
        self.fixture = _BASE.BrokerNegativePackageTests(
            'test_both_fixed_cases_package_exact_nested_marker_without_root_payload')
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.f = self.fixture.f

    def environment(self):
        return {
            'NATIVE_HOME_TEST_IDENTITY': '1',
            'NATIVE_HOME_INSTALLED_ARTWORK_PROOF': '1',
            'NATIVE_HOME_BROKER_STARTUP_PINS': str(self.fixture.pins_path),
            'NATIVE_HOME_STARTUP_BROKER_BINARY': str(self.fixture.helper),
        }

    def refused(self, changes, message=None):
        self.fixture.refused(self.environment() | changes, message)

    def guard_environment(self):
        result = self.environment()
        result.pop('NATIVE_HOME_INSTALLED_ARTWORK_PROOF')
        result['NATIVE_HOME_ARTWORK_GUARD_RELAUNCH_PROOF'] = '1'
        return result

    def test_guard_relaunch_is_a_sole_ordinary_marker_without_root_payload(self):
        result, output = self.fixture.package(self.guard_environment())
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads((output / 'preview-package-receipt.json').read_text())
        members = _BASE.ar_members(Path(receipt['ipk']).read_bytes())
        files = {}
        prefix = 'usr/palm/applications/' + _BASE.APP + '/'
        with tarfile.open(fileobj=io.BytesIO(members['data.tar.gz'])) as data:
            for row in data:
                self.assertTrue(row.isdir() or row.isfile())
                if row.isfile():
                    name = row.name.removeprefix('./')
                    self.assertTrue(name.startswith(prefix))
                    files[name[len(prefix):]] = data.extractfile(row).read()
        self.assertEqual(set(files), {'appinfo.json', 'home-preview', 'snapshot.json',
                                     'appfont.ttf', 'appfont-bold.ttf', 'OFL.txt',
                                     'icon.png', 'artwork-guard-relaunch-proof.json'})
        pins = self.fixture.pins
        self.assertEqual(json.loads(files['artwork-guard-relaunch-proof.json']),
                         {'schema': 1, 'nonce': pins['rendezvous_nonce'],
                          'ui_sha256': pins['ui_sha256'], 'broker_sha256': pins['broker_sha256']})
        self.assertIs(receipt['artwork_guard_relaunch_proof'], True)
        self.assertIs(receipt['installed_artwork_proof'], False)
        self.assertIs(receipt['broker_startup_proof'], False)
        self.assertIs(receipt['broker_negative_proof'], False)
        self.assertEqual(json.loads(files['appinfo.json'])['requiredPermissions'], [])
        for field in ('services', 'permissions', 'boot_hooks', 'system_files'):
            self.assertEqual(receipt[field], [])

    def test_guard_relaunch_refuses_normal_interactive_or_mixed_modes(self):
        for changes in ({'NATIVE_HOME_TEST_IDENTITY': '0'},
                        {'NATIVE_HOME_INTERACTIVE_SESSION': '1'},
                        {'NATIVE_HOME_INSTALLED_ARTWORK_PROOF': '1'},
                        {'NATIVE_HOME_BROKER_STARTUP_PROOF': '1'},
                        {'NATIVE_HOME_NATIVE_READ_PROBE': '1'},
                        {'NATIVE_HOME_ICON_PRESENTATION_PROOF': '1'}):
            with self.subTest(changes=changes):
                self.fixture.refused(self.guard_environment() | changes)

    def test_exact_ordinary_payload_marker_and_helper_context_without_root_files(self):
        result, output = self.fixture.package(self.environment())
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads((output / 'preview-package-receipt.json').read_text())
        ipk = Path(receipt['ipk'])
        self.assertEqual(ipk.parent, output)
        self.assertEqual(ipk.name, _BASE.APP + '_0.61.0_arm.ipk')
        self.assertEqual(_BASE.sha(ipk.read_bytes()), receipt['sha256'])
        archives = _BASE.ar_members(ipk.read_bytes())
        self.assertEqual(set(archives), {'debian-binary', 'control.tar.gz', 'data.tar.gz'})
        self.assertEqual(archives['debian-binary'], b'2.0\n')
        with tarfile.open(fileobj=io.BytesIO(archives['control.tar.gz'])) as control:
            members = control.getmembers()
            self.assertEqual([m.name for m in members], ['./control'])
            self.assertTrue(members[0].isfile())
            fields = dict(line.split(': ', 1) for line in
                          control.extractfile(members[0]).read().decode().splitlines())
            self.assertEqual(fields['Package'], _BASE.APP)
            self.assertEqual(fields['Version'], '0.61.0')
        files, modes = {}, {}
        prefix = 'usr/palm/applications/' + _BASE.APP + '/'
        with tarfile.open(fileobj=io.BytesIO(archives['data.tar.gz'])) as data:
            for member in data:
                self.assertTrue(member.isfile() or member.isdir())
                self.assertEqual((member.uid, member.gid), (0, 0))
                self.assertFalse(member.mode & 0o7000)
                if member.isfile():
                    name = member.name.removeprefix('./')
                    self.assertTrue(name.startswith(prefix))
                    relative = name[len(prefix):]
                    self.assertNotIn(relative, files)
                    files[relative] = data.extractfile(member).read()
                    modes[relative] = member.mode
        self.assertEqual(set(files), {'appinfo.json', 'home-preview', 'snapshot.json',
                                     'appfont.ttf', 'appfont-bold.ttf', 'OFL.txt',
                                     'icon.png', 'installed-artwork-proof.json'})
        pins = self.fixture.pins
        marker = {'schema': 1, 'nonce': pins['rendezvous_nonce'],
                  'ui_sha256': pins['ui_sha256'], 'broker_sha256': pins['broker_sha256']}
        self.assertEqual(files['installed-artwork-proof.json'],
                         (json.dumps(marker, separators=(',', ':')) + '\n').encode())
        self.assertEqual(modes['installed-artwork-proof.json'], 0o644)
        self.assertEqual(files['home-preview'], self.f.binary.read_bytes())
        self.assertEqual(modes['home-preview'], 0o755)
        self.assertEqual(files['snapshot.json'], (self.f.assets / 'snapshot.json').read_bytes())
        for name in ('appfont.ttf', 'appfont-bold.ttf', 'OFL.txt'):
            self.assertEqual(files[name], (self.f.root / 'resources/fonts' / name).read_bytes())
        self.assertEqual(files['icon.png'], (self.f.root / 'assets/luma-home/icon-160.png').read_bytes())
        info = json.loads(files['appinfo.json'])
        self.assertEqual((info['id'], info['version'], info['main'], info['type']),
                         (_BASE.APP, '0.61.0', 'home-preview', 'native'))
        self.assertEqual(info['requiredPermissions'], [])
        self.assertIs(receipt['installed_artwork_proof'], True)
        self.assertIs(receipt['broker_startup_proof'], False)
        self.assertIs(receipt['broker_negative_proof'], False)
        self.assertIsNone(receipt['broker_negative_case'])
        for field in ('services', 'permissions', 'boot_hooks', 'system_files'):
            self.assertEqual(receipt[field], [])
        helper_build = json.loads(self.fixture.helper_receipt.read_text())
        self.assertEqual(receipt['broker_startup_helper'], {
            'binary_sha256': pins['broker_sha256'],
            'build_receipt_sha256': _BASE.sha(self.fixture.helper_receipt.read_bytes()),
            'build_source_revision': helper_build['git_revision']})
        self.assertEqual(receipt['broker_startup_root_pins_sha256'],
                         _BASE.sha(self.fixture.pins_path.read_bytes()))
        self.assertFalse((self.f.base / 'compiler-executed').exists())
        self.assertFalse((self.f.base / 'cargo-executed').exists())

    def test_normal_interactive_and_wrong_main_identity_are_refused(self):
        for changes in ({'NATIVE_HOME_TEST_IDENTITY': '0'},
                        {'NATIVE_HOME_INTERACTIVE_SESSION': '1'}):
            with self.subTest(changes=changes):
                self.refused(changes, 'isolated non-interactive proof')
        control = self.f.base / 'control-preview'
        control.write_bytes(self.f.binary.read_bytes())
        snapshot = _BASE.PROVENANCE.snapshot(self.f.root)
        _BASE.PROVENANCE.record_build_receipt(
            self.f.root, control, 'arm', snapshot, _BASE.PROVENANCE.snapshot(self.f.root))
        self.refused({'NATIVE_HOME_ARM_BINARY': str(control)},
                     'Test identity is for the dashboard only')

    def test_every_other_proof_probe_and_pin_mode_is_exclusive(self):
        names = set(re.findall(r'NATIVE_HOME_[A-Z0-9_]+',
                               (self.f.root / 'package-preview.py').read_text()))
        names = {name for name in names if name.endswith(('_PROOF', '_PROBE', '_PINS'))}
        names -= {'NATIVE_HOME_INSTALLED_ARTWORK_PROOF', 'NATIVE_HOME_BROKER_STARTUP_PINS'}
        names |= {'NATIVE_HOME_UNKNOWN_PROOF', 'NATIVE_HOME_UNKNOWN_PROBE', 'NATIVE_HOME_UNKNOWN_PINS'}
        for name in sorted(names):
            with self.subTest(name=name):
                self.refused({name: '1'})
        self.refused({'NATIVE_HOME_NATIVE_READ_PROBE': '0'})
        self.refused({'NATIVE_HOME_BROKER_NEGATIVE_CASE': 'wrong_identify_nonce'})

    def test_orphan_or_untrusted_pins_cannot_request_installed_mode(self):
        for extra in ({'NATIVE_HOME_BROKER_STARTUP_PINS': str(self.fixture.pins_path)},
                      {'NATIVE_HOME_STARTUP_BROKER_BINARY': str(self.fixture.helper)}):
            with self.subTest(extra=extra):
                self.fixture.refused(extra, 'pins/helper require explicit isolated mode')
        self.refused({'NATIVE_HOME_INSTALLED_ARTWORK_PROOF': '0'},
                     'pins/helper require explicit isolated mode')
        for change in ({'schema': True}, {'profile': 'preview'}, {'command': 'not-allowed'},
                       {'ui_sha256': 'd' * 64}, {'rendezvous_nonce': 'A' * 32}):
            with self.subTest(change=change):
                self.fixture.write_pins(self.fixture.pins | change)
                self.refused({}, 'Closed Root installation pins required')
        self.fixture.write_pins(self.fixture.pins)
        self.fixture.pins_path.write_text(json.dumps(self.fixture.pins).replace('{', '{"schema":1,', 1))
        self.refused({}, 'Duplicate broker installation pin')

    def test_helper_name_hash_and_source_context_must_match(self):
        self.refused({'NATIVE_HOME_STARTUP_BROKER_BINARY': str(self.f.binary)},
                     'Fixed startup helper binary required')
        self.fixture.write_pins(self.fixture.pins | {'broker_sha256': 'd' * 64})
        self.refused({}, 'Root helper must share actual UI source/framework build context')
        self.fixture.write_pins(self.fixture.pins)
        helper = json.loads(self.fixture.helper_receipt.read_text())
        helper['git_revision'] = 'd' * 40
        self.fixture.helper_receipt.write_text(json.dumps(helper) + '\n')
        self.refused({}, 'Root helper must share actual UI source/framework build context')

    def test_actual_helper_bytes_and_snapshot_drift_refuse_before_tools(self):
        original = self.fixture.helper.read_bytes()
        self.fixture.helper.write_bytes(original + b'drift')
        self.refused({}, 'Binary bytes/name differ from build receipt')
        self.fixture.helper.write_bytes(original)
        (self.f.root / 'src/fixture.txt').write_text('selected input changed\n')
        self.f.record()
        self.refused({}, 'Current source/dependency inputs differ from build receipt')


if __name__ == '__main__':
    unittest.main()
