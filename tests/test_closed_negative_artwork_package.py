"""Exercise the actual packager's closed one-job recipe and archive boundary."""
import importlib.util
import io
import json
from pathlib import Path
import re
import tarfile
import unittest

REPO = Path(__file__).resolve().parents[1]
_SPEC = importlib.util.spec_from_file_location(
    'closed_negative_fixtures', REPO / 'tests/test_broker_negative_package.py')
_BASE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_BASE)


class ClosedNegativeArtworkPackageTests(unittest.TestCase):
    def setUp(self):
        self.fixture = _BASE.BrokerNegativePackageTests(
            'test_both_fixed_cases_package_exact_nested_marker_without_root_payload')
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.f = self.fixture.f

    def environment(self, case='read_after'):
        return {'NATIVE_HOME_TEST_IDENTITY': '1',
                'NATIVE_HOME_CLOSED_NEGATIVE_ARTWORK_PROOF': '1',
                'NATIVE_HOME_CLOSED_NEGATIVE_ARTWORK_CASE': case,
                'NATIVE_HOME_BROKER_STARTUP_PINS': str(self.fixture.pins_path),
                'NATIVE_HOME_STARTUP_BROKER_BINARY': str(self.fixture.helper)}

    def test_each_case_has_only_ordinary_payload_and_exact_bound_marker(self):
        for case in ('read_after', 'finish'):
            with self.subTest(case=case):
                result, output = self.fixture.package(self.environment(case))
                self.assertEqual(result.returncode, 0, result.stderr)
                receipt = json.loads((output / 'preview-package-receipt.json').read_text())
                ipk = Path(receipt['ipk'])
                self.assertEqual(ipk.name, _BASE.APP + '_0.56.0_arm.ipk')
                self.assertEqual(_BASE.sha(ipk.read_bytes()), receipt['sha256'])
                archives = _BASE.ar_members(ipk.read_bytes())
                self.assertEqual(set(archives), {'debian-binary', 'control.tar.gz', 'data.tar.gz'})
                files = {}
                prefix = 'usr/palm/applications/' + _BASE.APP + '/'
                with tarfile.open(fileobj=io.BytesIO(archives['data.tar.gz'])) as data:
                    for row in data:
                        self.assertTrue(row.isdir() or row.isfile())
                        self.assertEqual((row.uid, row.gid), (0, 0))
                        self.assertFalse(row.mode & 0o7000)
                        if row.isfile():
                            name = row.name.removeprefix('./')
                            self.assertTrue(name.startswith(prefix))
                            relative = name[len(prefix):]
                            self.assertNotIn(relative, files)
                            files[relative] = data.extractfile(row).read()
                            self.assertEqual(row.mode, 0o755 if relative == 'home-preview' else 0o644)
                self.assertEqual(set(files), {'appinfo.json', 'home-preview', 'snapshot.json',
                    'appfont.ttf', 'appfont-bold.ttf', 'OFL.txt', 'icon.png',
                    'artwork-closed-negative-proof.json'})
                pins = self.fixture.pins
                self.assertEqual(json.loads(files['artwork-closed-negative-proof.json']),
                    {'schema': 1, 'nonce': pins['rendezvous_nonce'], 'ui_sha256': pins['ui_sha256'],
                     'broker_sha256': pins['broker_sha256'], 'case': case})
                self.assertEqual(files['home-preview'], self.f.binary.read_bytes())
                info = json.loads(files['appinfo.json'])
                self.assertEqual((info['id'], info['version'], info['main']),
                                 (_BASE.APP, '0.56.0', 'home-preview'))
                self.assertEqual(info['requiredPermissions'], [])
                self.assertIs(receipt['closed_negative_artwork_proof'], True)
                self.assertEqual(receipt['closed_negative_artwork_case'], case)
                for mode in ('installed_artwork_proof', 'broker_startup_proof',
                             'broker_negative_proof', 'preview_artwork_admission'):
                    self.assertIs(receipt[mode], False)
                for field in ('services', 'permissions', 'boot_hooks', 'system_files'):
                    self.assertEqual(receipt[field], [])
                self.assertEqual(receipt['broker_startup_helper']['binary_sha256'], pins['broker_sha256'])
                self.assertEqual(receipt['broker_startup_root_pins_sha256'],
                                 _BASE.sha(self.fixture.pins_path.read_bytes()))
                self.assertFalse((self.f.base / 'compiler-executed').exists())
                self.assertFalse((self.f.base / 'cargo-executed').exists())

    def test_closed_environment_rejects_all_other_current_and_future_parameters(self):
        allowed = set(self.environment()) | {'NATIVE_HOME_BUILD_DIR', 'NATIVE_HOME_WEBOS_SDK',
            'NATIVE_HOME_ARM_BINARY', 'NATIVE_HOME_SNAPSHOT_ASSETS', 'NATIVE_HOME_PREVIEW_ICON',
            'NATIVE_HOME_CARGO', 'NATIVE_HOME_RUST_TOOLCHAIN'}
        names = set(re.findall(r'NATIVE_HOME_[A-Z0-9_]+',
                              (self.f.root / 'package-preview.py').read_text())) - allowed
        names |= {'NATIVE_HOME_FUTURE_ACTION', 'NATIVE_HOME_TARGET_ID',
                  'NATIVE_HOME_ROOT_PLAN', 'NATIVE_HOME_DELAY_MS'}
        for name in sorted(names):
            with self.subTest(name=name):
                self.fixture.refused(self.environment() | {name: '1'})
        self.fixture.refused(self.environment() | {'NATIVE_HOME_FUTURE_ACTION': '0'})

    def test_case_mode_identity_and_interactivity_are_closed(self):
        for changes in ({'NATIVE_HOME_CLOSED_NEGATIVE_ARTWORK_CASE': ''},
                        {'NATIVE_HOME_CLOSED_NEGATIVE_ARTWORK_CASE': 'read_before'},
                        {'NATIVE_HOME_CLOSED_NEGATIVE_ARTWORK_CASE': 'Finish'},
                        {'NATIVE_HOME_CLOSED_NEGATIVE_ARTWORK_PROOF': '0'},
                        {'NATIVE_HOME_CLOSED_NEGATIVE_ARTWORK_PROOF': 'true'},
                        {'NATIVE_HOME_TEST_IDENTITY': '0'},
                        {'NATIVE_HOME_INTERACTIVE_SESSION': '1'}):
            with self.subTest(changes=changes):
                self.fixture.refused(self.environment() | changes)
        self.fixture.refused({'NATIVE_HOME_CLOSED_NEGATIVE_ARTWORK_CASE': 'finish'})

    def test_static_catalogue_or_icons_cannot_expand_the_closed_payload(self):
        snapshot = self.f.assets / 'snapshot.json'
        for applications in ([{'id': 'com.ash.nativehome.targetproof'}],
                             [{'id': 'other.app', 'icon': 'icons/app.png'}], None, {}):
            with self.subTest(applications=applications):
                snapshot.write_text(json.dumps({'applications': applications}))
                self.fixture.refused(self.environment(), 'empty packaged catalogue')
        snapshot.write_text('{}')
        self.fixture.refused(self.environment(), 'empty packaged catalogue')

    def test_host_build_and_other_executables_cannot_request_this_recipe(self):
        receipt = json.loads(self.f.receipt.read_text())
        receipt['target'] = 'host'
        self.f.receipt.write_text(json.dumps(receipt) + '\n')
        self.fixture.refused(self.environment())
        self.f.record()
        control = self.f.base / 'control-preview'
        control.write_bytes(self.f.binary.read_bytes())
        snapshot = _BASE.PROVENANCE.snapshot(self.f.root)
        _BASE.PROVENANCE.record_build_receipt(self.f.root, control, 'arm', snapshot,
                                             _BASE.PROVENANCE.snapshot(self.f.root))
        self.fixture.refused(self.environment() | {'NATIVE_HOME_ARM_BINARY': str(control)})

    def test_pins_and_helper_must_bind_actual_same_source_arm_pair(self):
        self.fixture.write_pins(self.fixture.pins | {'profile': 'preview'})
        self.fixture.refused(self.environment())
        self.fixture.write_pins(self.fixture.pins | {'ui_sha256': 'd' * 64})
        self.fixture.refused(self.environment())
        self.fixture.write_pins(self.fixture.pins)
        self.fixture.refused(self.environment() | {'NATIVE_HOME_STARTUP_BROKER_BINARY': str(self.f.binary)})
        self.fixture.helper.write_bytes(self.fixture.helper.read_bytes() + b'drift')
        self.fixture.refused(self.environment())


if __name__ == '__main__':
    unittest.main()
