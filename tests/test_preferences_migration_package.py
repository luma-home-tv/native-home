"""Actual packager with temporary provenance/ARM/fake-ar fixtures; no TV or build."""
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import unittest


REPO = Path(__file__).resolve().parents[1]
_SPEC = importlib.util.spec_from_file_location(
    'migration_package_fixtures', REPO / 'tests/test_broker_negative_package.py')
_BASE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_BASE)
APP = 'com.ash.nativehome.preview'
MARKER = 'preferences-migration-proof.json'


class PreferencesMigrationPackageTests(unittest.TestCase):
    def setUp(self):
        # Compose only setup/tool helpers, not the other mode's test methods.
        self.fixture = _BASE.BrokerNegativePackageTests(
            'test_both_fixed_cases_package_exact_nested_marker_without_root_payload')
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.f = self.fixture.f
        self.pins = {
            'schema': 1, 'direction': 'forward',
            'expected_preferences_sha256': 'a' * 64,
            'ui_sha256': _BASE.sha(self.f.binary.read_bytes()),
            'expected_uid': 6249, 'expected_gid': 5000,
        }
        self.pins_path = self.f.base / 'migration-pins.json'
        self.write_pins(self.pins)

    def write_pins(self, value):
        self.write_raw((json.dumps(value) + '\n').encode())

    def write_raw(self, raw):
        self.pins_path.write_bytes(raw)
        self.pins_path.chmod(0o600)

    def environment(self):
        return {
            'NATIVE_HOME_PREFERENCES_MIGRATION_PROOF': '1',
            'NATIVE_HOME_PREFERENCES_MIGRATION_PINS': str(self.pins_path),
        }

    def package(self, extra=None, migration=True):
        self.fixture.serial += 1
        output = self.f.output / ('migration-%03d' % self.fixture.serial)
        output.mkdir()
        env = dict(self.f.env)
        if migration:
            # No dashboard snapshot input is allowed or needed in this mode.
            env.pop('NATIVE_HOME_SNAPSHOT_ASSETS', None)
            env.update(self.environment())
        env.update(NATIVE_HOME_BUILD_DIR=str(output))
        env.update(extra or {})
        (self.f.base / 'ar-executed').unlink(missing_ok=True)
        return self.f.run_entrypoint('package-preview.py', env=env), output

    def refused(self, extra=None, message=None):
        result, output = self.package(extra)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        if message:
            self.assertIn(message, result.stderr)
        self.assertFalse(list(output.rglob('*.ipk')))
        self.assertFalse((output / 'preview-package-receipt.json').exists())
        self.assertFalse((output / 'control.tar.gz').exists())
        for tool in ('ar', 'compiler', 'cargo'):
            self.assertFalse((self.f.base / (tool + '-executed')).exists())

    def unpack(self, output):
        receipt = json.loads((output / 'preview-package-receipt.json').read_text())
        ipk = Path(receipt['ipk'])
        self.assertEqual(ipk.name, APP + '_0.48.0_arm.ipk')
        self.assertEqual(_BASE.sha(ipk.read_bytes()), receipt['sha256'])
        members = _BASE.ar_members(ipk.read_bytes())
        self.assertEqual(set(members), {'debian-binary', 'control.tar.gz', 'data.tar.gz'})
        self.assertEqual(members['debian-binary'], b'2.0\n')
        with tarfile.open(fileobj=io.BytesIO(members['control.tar.gz'])) as control:
            rows = control.getmembers()
            self.assertEqual([row.name for row in rows], ['./control'])
            text = control.extractfile(rows[0]).read().decode()
            self.assertEqual(set(line.split(': ', 1)[0] for line in text.splitlines()),
                             {'Package', 'Version', 'Section', 'Priority',
                              'Architecture', 'Maintainer', 'Description'})
            self.assertIn('Version: 0.48.0\n', text)
        files = {}
        prefix = 'usr/palm/applications/' + APP + '/'
        with tarfile.open(fileobj=io.BytesIO(members['data.tar.gz'])) as data:
            for row in data:
                self.assertTrue(row.isfile() or row.isdir())
                self.assertEqual((row.uid, row.gid), (0, 0))
                self.assertFalse(row.mode & 0o7000)
                if row.isfile():
                    name = row.name.removeprefix('./')
                    self.assertTrue(name.startswith(prefix))
                    relative = name[len(prefix):]
                    self.assertNotIn(relative, files)
                    files[relative] = data.extractfile(row).read()
                    self.assertEqual(row.mode, 0o755 if relative == 'home-preview' else 0o644)
        return receipt, files

    def test_forward_and_reverse_exact_four_file_packages_have_distinct_bytes(self):
        hashes = []
        for direction in ('forward', 'reverse'):
            with self.subTest(direction=direction):
                self.pins['direction'] = direction
                self.write_pins(self.pins)
                result, output = self.package()
                self.assertEqual(result.returncode, 0, result.stderr)
                receipt, files = self.unpack(output)
                self.assertEqual(set(files), {'appinfo.json', 'home-preview', 'icon.png', MARKER})
                self.assertEqual(files[MARKER],
                                 (json.dumps(self.pins, separators=(',', ':')) + '\n').encode())
                self.assertEqual(files['home-preview'], self.f.binary.read_bytes())
                self.assertEqual(files['icon.png'], (self.f.root / 'assets/luma-home/icon-160.png').read_bytes())
                info = json.loads(files['appinfo.json'])
                self.assertEqual((info['id'], info['version'], info['main'], info['type']),
                                 (APP, '0.48.0', 'home-preview', 'native'))
                self.assertEqual(info['requiredPermissions'], [])
                self.assertIs(receipt['preferences_migration_proof'], True)
                self.assertEqual(receipt['preferences_migration_direction'], direction)
                self.assertEqual(receipt['preferences_migration_pins_sha256'],
                                 _BASE.sha(self.pins_path.read_bytes()))
                self.assertIs(receipt['test_identity'], False)
                self.assertIs(receipt['interactive_session'], False)
                self.assertIsNone(receipt['broker_startup_helper'])
                for field in ('permissions', 'services', 'boot_hooks', 'system_files'):
                    self.assertEqual(receipt[field], [])
                for field in ('preview_artwork_admission', 'installed_artwork_proof',
                              'broker_startup_proof', 'broker_negative_proof',
                              'native_read_probe', 'icon_presentation_proof'):
                    self.assertIs(receipt[field], False)
                hashes.append(receipt['sha256'])
        # Same version is intentional; actual SDK same-version replacement is
        # a separate device observation, not qualified by this packaging test.
        self.assertNotEqual(*hashes)

    def test_no_dashboard_fonts_or_snapshot_are_read(self):
        (self.f.assets / 'snapshot.json').unlink()
        for name in ('appfont.ttf', 'appfont-bold.ttf', 'OFL.txt'):
            (self.f.root / 'resources/fonts' / name).unlink()
        self.f.record()
        self.pins.update(expected_uid=1000, expected_gid=0)
        self.write_pins(self.pins)
        result, output = self.package()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(set(self.unpack(output)[1]), {'appinfo.json', 'home-preview', 'icon.png', MARKER})

    def test_wrong_ui_digest_or_host_build_receipt_refused(self):
        self.write_pins(self.pins | {'ui_sha256': 'b' * 64})
        self.refused(message='actual ARM UI')
        self.write_pins(self.pins)
        receipt = json.loads(self.f.receipt.read_text())
        receipt['target'] = 'host'
        self.f.receipt.write_text(json.dumps(receipt))
        self.refused(message='ARM build receipt required')

    def test_test_identity_interactive_or_non_dashboard_refused(self):
        for extra in ({'NATIVE_HOME_TEST_IDENTITY': '1'},
                      {'NATIVE_HOME_TEST_IDENTITY': '0'},
                      {'NATIVE_HOME_INTERACTIVE_SESSION': '1'}):
            with self.subTest(extra=extra):
                self.refused(extra)
        raw = self.f.binary.read_bytes()
        self.f.binary = self.f.base / 'control-preview'
        self.f.binary.write_bytes(raw)
        self.f.receipt = self.f.binary.with_name('control-preview.build.json')
        self.f.env['NATIVE_HOME_ARM_BINARY'] = str(self.f.binary)
        self.f.record()
        self.refused()

    def test_all_other_mode_assets_and_future_parameters_refused(self):
        for key in ('NATIVE_HOME_SNAPSHOT_ASSETS', 'NATIVE_HOME_NATIVE_READ_PROBE',
                    'NATIVE_HOME_TEXT_INPUT_PROOF', 'NATIVE_HOME_PREVIEW_ARTWORK_ADMISSION',
                    'NATIVE_HOME_BROKER_STARTUP_PROOF', 'NATIVE_HOME_ARTWORK_GUARD_RELAUNCH_PROOF',
                    'NATIVE_HOME_STARTUP_CAPABILITY_PROOF', 'NATIVE_HOME_FUTURE_FLAG',
                    'NATIVE_HOME_FUTURE_PARAMETER', 'NATIVE_HOME_ROUTE_PROOF'):
            with self.subTest(key=key):
                self.refused({key: '1'})

    def test_root_helper_service_and_policy_parameters_refused(self):
        for extra in ({'NATIVE_HOME_STARTUP_BROKER_BINARY': str(self.fixture.helper)},
                      {'NATIVE_HOME_BROKER_STARTUP_PINS': str(self.fixture.pins_path)},
                      {'NATIVE_HOME_SERVICES': 'service'},
                      {'NATIVE_HOME_SYSTEM_FILES': '/arbitrary'},
                      {'NATIVE_HOME_ARTWORK_TARGET_FIXTURE': 'a'}):
            with self.subTest(extra=extra):
                self.refused(extra)

    def test_pins_must_be_private_regular_unlinked_files(self):
        self.pins_path.chmod(0o644)
        self.refused(message='Private0400/0600')
        self.pins_path.chmod(0o600)
        link = self.f.base / 'pins-link.json'
        link.symlink_to(self.pins_path)
        self.refused({'NATIVE_HOME_PREFERENCES_MIGRATION_PINS': str(link)}, 'Linked input')
        import os
        hard = self.f.base / 'hard-pins.json'
        os.link(self.pins_path, hard)
        self.refused(message='Private0400/0600')
        hard.unlink()
        linked_parent = self.f.base / 'linked-parent'
        linked_parent.symlink_to(self.f.base, target_is_directory=True)
        self.refused({'NATIVE_HOME_PREFERENCES_MIGRATION_PINS': str(linked_parent / self.pins_path.name)},
                     'Linked input')

    def test_empty_oversized_trailing_or_non_object_json_refused(self):
        for raw in (b'', b' ' * 1025, b'[]', b'null',
                    json.dumps(self.pins).encode() + b' {}'):
            with self.subTest(raw_length=len(raw)):
                self.write_raw(raw)
                self.refused()

    def test_unknown_missing_duplicate_fields_refused(self):
        missing = dict(self.pins)
        missing.pop('expected_gid')
        duplicate = json.dumps(self.pins)[:-1] + ',"direction":"reverse"}'
        for raw in (json.dumps(self.pins | {'path': '/arbitrary'}).encode(),
                    json.dumps(missing).encode(), duplicate.encode()):
            with self.subTest(raw=raw):
                self.write_raw(raw)
                self.refused()

    def test_schema_direction_and_digest_types_are_not_coerced(self):
        for change in ({'schema': True}, {'schema': 1.0}, {'schema': '1'},
                       {'direction': True}, {'direction': 'Forward'}, {'direction': 'repair'},
                       {'expected_preferences_sha256': 'A' * 64},
                       {'expected_preferences_sha256': 'a' * 63},
                       {'expected_preferences_sha256': 0}, {'ui_sha256': None}):
            with self.subTest(change=change):
                self.write_pins(self.pins | change)
                self.refused()

    def test_uid_gid_integer_bounds_and_boolean_rejection(self):
        for field, values in (('expected_uid', (True, '6249', 6249.0, 0, 999, 2**32)),
                              ('expected_gid', (False, '5000', 5000.0, -1, 2**32))):
            for value in values:
                with self.subTest(field=field, value=value):
                    self.write_pins(self.pins | {field: value})
                    self.refused()

    def test_orphan_pins_missing_pins_and_invalid_mode_refused(self):
        self.refused({'NATIVE_HOME_PREFERENCES_MIGRATION_PROOF': ''}, 'require explicit mode')
        self.refused({'NATIVE_HOME_PREFERENCES_MIGRATION_PINS': ''})
        self.refused({'NATIVE_HOME_PREFERENCES_MIGRATION_PROOF': 'true'}, 'explicitly 1')

    def test_normal_package_has_no_migration_marker_or_pins_claim(self):
        result, output = self.package(migration=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt, files = self.unpack(output)
        self.assertNotIn(MARKER, files)
        self.assertIn('snapshot.json', files)
        self.assertIn('appfont.ttf', files)
        self.assertIs(receipt['preferences_migration_proof'], False)
        self.assertIsNone(receipt['preferences_migration_direction'])
        self.assertIsNone(receipt['preferences_migration_pins_sha256'])


if __name__ == '__main__':
    unittest.main()
