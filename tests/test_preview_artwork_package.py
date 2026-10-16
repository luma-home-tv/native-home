"""Actual packager policy with synthetic ARM receipts and an inert ar wrapper.

No compiler, SDK tool, service, privileged process or device is used. Positive
cases inspect the real control/data archives assembled by the entrypoint.
"""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import re
import sys
import tarfile
import unittest


REPO = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    'preview_artwork_entrypoint_fixtures', REPO / 'tests/test_build_entrypoints.py')
FIXTURES = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FIXTURES)
PROVENANCE = FIXTURES.PROVENANCE
APP = 'com.ash.nativehome.preview'
MODE = 'NATIVE_HOME_PREVIEW_ARTWORK_ADMISSION'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def ar_members(raw):
    if raw[:8] != b'!<arch>\n':
        raise ValueError('Real ar fixture required')
    offset, members = 8, {}
    while offset < len(raw):
        header = raw[offset:offset + 60]
        if len(header) != 60 or header[58:] != b'`\n':
            raise ValueError('Complete ar member required')
        name = header[:16].decode().strip().rstrip('/')
        size = int(header[48:58])
        if name in members or size < 0 or offset + 60 + size > len(raw):
            raise ValueError('Unique bounded member required')
        members[name] = raw[offset + 60:offset + 60 + size]
        offset += 60 + size + size % 2
    if offset != len(raw):
        raise ValueError('Complete ar padding required')
    return members


class PreviewArtworkPackageTests(unittest.TestCase):
    def setUp(self):
        self.f = FIXTURES.EntrypointTests(
            'test_missing_or_tampered_receipt_refuses_before_staging')
        self.f.setUp()
        self.addCleanup(self.f.doCleanups)
        self.serial = 0
        self.helper = self.f.base / 'artwork-broker'
        self.helper.write_bytes(self.f.binary.read_bytes() + b'Preview helper fixture')
        self.helper_receipt = self.helper.with_name(self.helper.name + '.build.json')
        snapshot = PROVENANCE.snapshot(self.f.root)
        PROVENANCE.record_build_receipt(self.f.root, self.helper, 'arm', snapshot,
                                       PROVENANCE.snapshot(self.f.root))
        self.f.record()
        self.pins = {
            'schema': 1, 'profile': 'preview', 'rendezvous_nonce': 'a' * 32,
            'ui_sha256': sha(self.f.binary.read_bytes()),
            'broker_sha256': sha(self.helper.read_bytes()),
            'context_reader_sha256': 'c' * 64,
        }
        self.pins_path = self.f.base / 'protected-preview-pins.json'
        self.write_pins(self.pins)
        self.ar_marker = self.f.base / 'ar-executed'
        self.f.ar.write_text(
            '#!' + sys.executable + '\n'
            'from pathlib import Path\nimport sys\n'
            'Path(' + repr(str(self.ar_marker)) + ').write_text("fixture ar used")\n'
            'if sys.argv[1] == "rc":\n'
            ' raw=b"!<arch>\\n"\n'
            ' for name in sys.argv[3:]:\n'
            '  data=Path(name).read_bytes()\n'
            '  header=(f"{name+chr(47):<16}{0:<12}{0:<6}{0:<6}{chr(49)+chr(48)+chr(48)+chr(54)+chr(52)+chr(52):<8}{len(data):<10}`\\n").encode()\n'
            '  raw+=header+data+(b"\\n" if len(data)%2 else b"")\n'
            ' Path(sys.argv[2]).write_bytes(raw)\n'
            'elif sys.argv[1] == "t":\n'
            ' print("debian-binary\\ncontrol.tar.gz\\ndata.tar.gz")\n'
            'else: raise SystemExit("unexpected fixture ar operation")\n')
        self.f.ar.chmod(0o755)

    def write_pins(self, pins):
        self.pins_path.write_text(json.dumps(pins) + '\n')
        self.pins_path.chmod(0o600)

    def environment(self):
        return {MODE: '1',
                'NATIVE_HOME_BROKER_STARTUP_PINS': str(self.pins_path),
                'NATIVE_HOME_STARTUP_BROKER_BINARY': str(self.helper)}

    def package(self, extra):
        self.serial += 1
        output = self.f.output / ('case-%03d' % self.serial)
        output.mkdir()
        env = {**self.f.env, 'NATIVE_HOME_BUILD_DIR': str(output), **extra}
        return self.f.run_entrypoint('package-preview.py', env=env), output

    def refused(self, extra, message=None):
        result, output = self.package(extra)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        if message:
            self.assertIn(message, result.stderr)
        self.assertFalse(list(output.rglob('*.ipk')))
        self.assertFalse((output / 'preview-package-receipt.json').exists())
        self.assertFalse((output / 'control.tar.gz').exists())
        self.assertFalse(self.ar_marker.exists())
        self.assertFalse((self.f.base / 'compiler-executed').exists())
        self.assertFalse((self.f.base / 'cargo-executed').exists())

    def archive(self, output):
        receipt = json.loads((output / 'preview-package-receipt.json').read_text())
        raw = Path(receipt['ipk']).read_bytes()
        self.assertEqual(sha(raw), receipt['sha256'])
        members = ar_members(raw)
        self.assertEqual(set(members), {'debian-binary', 'control.tar.gz', 'data.tar.gz'})
        self.assertEqual(members['debian-binary'], b'2.0\n')
        with tarfile.open(fileobj=io.BytesIO(members['control.tar.gz'])) as control:
            self.assertEqual([m.name for m in control], ['./control'])
        files, modes = {}, {}
        prefix = 'usr/palm/applications/' + receipt['app_id'] + '/'
        with tarfile.open(fileobj=io.BytesIO(members['data.tar.gz'])) as data:
            for item in data:
                self.assertTrue(item.isfile() or item.isdir())
                self.assertEqual((item.uid, item.gid), (0, 0))
                self.assertFalse(item.mode & 0o7000)
                if item.isfile():
                    name = item.name.removeprefix('./')
                    self.assertTrue(name.startswith(prefix))
                    relative = name[len(prefix):]
                    self.assertNotIn(relative, files)
                    files[relative] = data.extractfile(item).read()
                    modes[relative] = item.mode
        return receipt, files, modes

    def test_actual_preview_four_field_marker_and_no_root_payload(self):
        result, output = self.package(self.environment())
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt, files, modes = self.archive(output)
        self.assertEqual(Path(receipt['ipk']).name, APP + '_0.47.0_arm.ipk')
        self.assertEqual(set(files), {'appinfo.json', 'home-preview', 'snapshot.json',
                                     'appfont.ttf', 'appfont-bold.ttf', 'OFL.txt',
                                     'icon.png', 'preview-artwork-admission.json'})
        marker = {'schema': 1, 'nonce': self.pins['rendezvous_nonce'],
                  'ui_sha256': self.pins['ui_sha256'],
                  'broker_sha256': self.pins['broker_sha256']}
        self.assertEqual(files['preview-artwork-admission.json'],
                         (json.dumps(marker, separators=(',', ':')) + '\n').encode())
        self.assertEqual(modes['preview-artwork-admission.json'], 0o644)
        self.assertEqual(modes['home-preview'], 0o755)
        self.assertEqual(files['home-preview'], self.f.binary.read_bytes())
        self.assertNotIn(self.helper.read_bytes(), files.values())
        info = json.loads(files['appinfo.json'])
        self.assertEqual((info['id'], info['version'], info['main'], info['type']),
                         (APP, '0.47.0', 'home-preview', 'native'))
        self.assertEqual(info['requiredPermissions'], [])
        self.assertIs(receipt['preview_artwork_admission'], True)
        self.assertIs(receipt['test_identity'], False)
        self.assertIs(receipt['interactive_session'], False)
        for key, value in receipt.items():
            if key.endswith(('_proof', '_probe')):
                self.assertFalse(value, key)
        self.assertIsNone(receipt['broker_negative_case'])
        self.assertEqual(receipt['broker_startup_root_pins_sha256'],
                         sha(self.pins_path.read_bytes()))
        helper_build = json.loads(self.helper_receipt.read_text())
        self.assertEqual(receipt['broker_startup_helper'], {
            'binary_sha256': self.pins['broker_sha256'],
            'build_receipt_sha256': sha(self.helper_receipt.read_bytes()),
            'build_source_revision': helper_build['git_revision']})
        for key in ('permissions', 'services', 'boot_hooks', 'system_files'):
            self.assertEqual(receipt[key], [])

    def test_default_normal_package_does_not_activate_admission(self):
        result, output = self.package({})
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt, files, _ = self.archive(output)
        self.assertIs(receipt['preview_artwork_admission'], False)
        self.assertEqual(receipt['app_id'], APP)
        self.assertNotIn('preview-artwork-admission.json', files)
        self.assertIsNone(receipt['broker_startup_helper'])

    def test_proof_and_interactive_roles_cannot_select_preview_admission(self):
        for changes in ({'NATIVE_HOME_TEST_IDENTITY': '1'},
                        {'NATIVE_HOME_INTERACTIVE_SESSION': '1'}):
            with self.subTest(changes=changes):
                self.refused(self.environment() | changes)

    def test_other_ui_binaries_cannot_select_preview_admission(self):
        raw = self.f.binary.read_bytes()
        for name in ('control-preview', 'launch-target'):
            with self.subTest(name=name):
                self.f.binary = self.f.base / name
                self.f.binary.write_bytes(raw)
                self.f.receipt = self.f.binary.with_name(name + '.build.json')
                self.f.record()
                self.refused(self.environment() | {'NATIVE_HOME_ARM_BINARY': str(self.f.binary)})

    def test_all_trial_action_and_unknown_parameters_refuse_before_tools(self):
        allowed = {MODE, 'NATIVE_HOME_BROKER_STARTUP_PINS',
                   'NATIVE_HOME_STARTUP_BROKER_BINARY', 'NATIVE_HOME_BUILD_DIR',
                   'NATIVE_HOME_WEBOS_SDK', 'NATIVE_HOME_ARM_BINARY',
                   'NATIVE_HOME_SNAPSHOT_ASSETS', 'NATIVE_HOME_PREVIEW_ICON',
                   'NATIVE_HOME_CARGO'}
        params = set(re.findall(r'NATIVE_HOME_[A-Z0-9_]+',
                                (self.f.root / 'package-preview.py').read_text()))
        params -= allowed
        params |= {'NATIVE_HOME_FUTURE_ADMISSION', 'NATIVE_HOME_UNKNOWN_PROOF',
                   'NATIVE_HOME_UNKNOWN_PROBE', 'NATIVE_HOME_UNKNOWN_ACTION'}
        for param in sorted(params):
            with self.subTest(param=param):
                self.refused(self.environment() | {param: '1'})
        self.refused(self.environment() | {'NATIVE_HOME_ROUTE_PROOF': 'hdmi1'})
        self.refused(self.environment() | {'NATIVE_HOME_NATIVE_READ_PROBE': '0'})

    def test_six_pin_schema_and_preview_profile_are_closed(self):
        for changes in ({'schema': True}, {'profile': 'proof'},
                        {'command': 'untrusted'}, {'caller_pid': 123},
                        {'ui_sha256': 'd' * 64}, {'rendezvous_nonce': 'A' * 32},
                        {'context_reader_sha256': False}):
            with self.subTest(changes=changes):
                self.write_pins(self.pins | changes)
                self.refused(self.environment(), 'Closed Root installation pins required')
        self.write_pins(self.pins)
        self.pins_path.write_text(json.dumps(self.pins).replace('{', '{"schema":1,', 1))
        self.refused(self.environment(), 'Duplicate broker installation pin')

    def test_missing_or_orphan_private_inputs_never_make_admission_package(self):
        for key in ('NATIVE_HOME_BROKER_STARTUP_PINS', 'NATIVE_HOME_STARTUP_BROKER_BINARY'):
            env = self.environment()
            del env[key]
            with self.subTest(key=key):
                self.refused(env)
        self.refused({'NATIVE_HOME_BROKER_STARTUP_PINS': str(self.pins_path)})
        self.refused({'NATIVE_HOME_STARTUP_BROKER_BINARY': str(self.helper)})

    def test_unreviewed_or_proof_helper_basename_is_rejected(self):
        for name in ('artwork-broker-startup-proof', 'renamed-artwork-broker'):
            with self.subTest(name=name):
                other = self.f.base / name
                other.write_bytes(self.helper.read_bytes())
                self.refused(self.environment() | {'NATIVE_HOME_STARTUP_BROKER_BINARY': str(other)},
                             'Fixed startup helper binary required')

    def test_helper_hash_source_target_and_snapshot_must_match_actual_ui(self):
        self.write_pins(self.pins | {'broker_sha256': 'd' * 64})
        self.refused(self.environment(), 'Root helper must share actual UI source/framework build context')
        self.write_pins(self.pins)
        original = self.helper_receipt.read_text()
        for key, value in (('git_revision', 'd' * 40), ('target', 'host')):
            changed = json.loads(original)
            changed[key] = value
            self.helper_receipt.write_text(json.dumps(changed) + '\n')
            with self.subTest(key=key):
                self.refused(self.environment())
        self.helper_receipt.write_text(original)
        (self.f.root / 'src/fixture.txt').write_text('changed selected input\n')
        self.f.record()
        self.refused(self.environment(), 'Current source/dependency inputs differ from build receipt')

    def test_private_pin_metadata_cannot_be_downgraded(self):
        self.pins_path.chmod(0o644)
        self.refused(self.environment())

    def test_existing_proof_recipe_stays_proof_only_and_admission_false(self):
        helper = self.f.base / 'artwork-broker-startup-proof'
        helper.write_bytes(self.helper.read_bytes())
        snapshot = PROVENANCE.snapshot(self.f.root)
        PROVENANCE.record_build_receipt(self.f.root, helper, 'arm', snapshot,
                                       PROVENANCE.snapshot(self.f.root))
        self.write_pins(self.pins | {'profile': 'proof'})
        result, output = self.package({
            'NATIVE_HOME_TEST_IDENTITY': '1',
            'NATIVE_HOME_BROKER_NEGATIVE_PROOF': '1',
            'NATIVE_HOME_BROKER_NEGATIVE_CASE': 'wrong_identify_nonce',
            'NATIVE_HOME_BROKER_STARTUP_PINS': str(self.pins_path),
            'NATIVE_HOME_STARTUP_BROKER_BINARY': str(helper)})
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt, files, _ = self.archive(output)
        self.assertIs(receipt['preview_artwork_admission'], False)
        self.assertEqual(receipt['app_id'], 'com.ash.nativehome.proof')
        self.assertIn('broker-negative-proof.json', files)
        self.assertNotIn('preview-artwork-admission.json', files)


if __name__ == '__main__':
    unittest.main()
