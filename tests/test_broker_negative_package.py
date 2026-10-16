"""Current packager admission with temporary ARM/provenance/tool fixtures.

No compiler, SDK executable, service, Root process, network or TV is used.
The ar substitute only wraps the packager's actual tar bytes in a real archive.
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
_SPEC = importlib.util.spec_from_file_location(
    'negative_package_entrypoint_fixtures', REPO / 'tests/test_build_entrypoints.py')
_FIXTURES = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_FIXTURES)
PROVENANCE = _FIXTURES.PROVENANCE
APP = 'com.ash.nativehome.proof'
CASES = ('wrong_identify_nonce', 'wrong_finish_nonce')


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def ar_members(raw):
    if raw[:8] != b'!<arch>\n':
        raise ValueError('Missing real fixture ar header')
    offset, members = 8, {}
    while offset < len(raw):
        header = raw[offset:offset + 60]
        if len(header) != 60 or header[58:] != b'`\n':
            raise ValueError('Invalid fixture ar member')
        name = header[:16].decode().strip().rstrip('/')
        size = int(header[48:58])
        if name in members or size < 0 or offset + 60 + size > len(raw):
            raise ValueError('Duplicate/truncated fixture ar member')
        members[name] = raw[offset + 60:offset + 60 + size]
        offset += 60 + size + size % 2
    if offset != len(raw):
        raise ValueError('Truncated fixture ar padding')
    return members


class BrokerNegativePackageTests(unittest.TestCase):
    def setUp(self):
        self.f = _FIXTURES.EntrypointTests(
            'test_missing_or_tampered_receipt_refuses_before_staging')
        self.f.setUp()
        self.addCleanup(self.f.doCleanups)
        self.serial = 0
        self.helper = self.f.base / 'artwork-broker-startup-proof'
        self.helper.write_bytes(self.f.binary.read_bytes() + b'fixed helper fixture')
        self.helper_receipt = self.helper.with_name(self.helper.name + '.build.json')
        snapshot = PROVENANCE.snapshot(self.f.root)
        PROVENANCE.record_build_receipt(self.f.root, self.helper, 'arm', snapshot,
                                       PROVENANCE.snapshot(self.f.root))
        self.f.record()
        self.pins = {
            'schema': 1, 'profile': 'proof', 'rendezvous_nonce': 'a' * 32,
            'ui_sha256': sha(self.f.binary.read_bytes()),
            'broker_sha256': sha(self.helper.read_bytes()),
            'context_reader_sha256': 'c' * 64,
        }
        self.pins_path = self.f.base / 'root-installation-pins.json'
        self.write_pins(self.pins)
        marker = self.f.base / 'ar-executed'
        # No external ar/SDK tool is invoked. The entrypoint still assembles and
        # reads back an actual ar container, using its real generated tar files.
        self.f.ar.write_text(
            '#!' + sys.executable + '\n'
            'from pathlib import Path\nimport sys\n'
            'Path(' + repr(str(marker)) + ').write_text("fixture ar used")\n'
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

    def write_pins(self, value):
        self.pins_path.write_text(json.dumps(value) + '\n')
        self.pins_path.chmod(0o600)

    def environment(self, scenario=CASES[0]):
        return {
            'NATIVE_HOME_TEST_IDENTITY': '1',
            'NATIVE_HOME_BROKER_NEGATIVE_PROOF': '1',
            'NATIVE_HOME_BROKER_NEGATIVE_CASE': scenario,
            'NATIVE_HOME_BROKER_STARTUP_PINS': str(self.pins_path),
            'NATIVE_HOME_STARTUP_BROKER_BINARY': str(self.helper),
        }

    def package(self, extra):
        self.serial += 1
        output = self.f.output / ('case-%03d' % self.serial)
        output.mkdir()
        env = {**self.f.env, 'NATIVE_HOME_BUILD_DIR': str(output), **extra}
        result = self.f.run_entrypoint('package-preview.py', env=env)
        return result, output

    def refused(self, extra, message=None):
        result, output = self.package(extra)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        if message:
            self.assertIn(message, result.stderr)
        self.assertFalse(list(output.rglob('*.ipk')))
        self.assertFalse((output / 'preview-package-receipt.json').exists())
        self.assertFalse((output / 'control.tar.gz').exists())
        self.assertFalse((self.f.base / 'ar-executed').exists())
        self.assertFalse((self.f.base / 'compiler-executed').exists())
        self.assertFalse((self.f.base / 'cargo-executed').exists())

    def test_both_fixed_cases_package_exact_nested_marker_without_root_payload(self):
        for scenario in CASES:
            with self.subTest(scenario=scenario):
                result, output = self.package(self.environment(scenario))
                self.assertEqual(result.returncode, 0, result.stderr)
                receipt = json.loads((output / 'preview-package-receipt.json').read_text())
                ipk = Path(receipt['ipk'])
                self.assertEqual(ipk.parent, output)
                self.assertEqual(ipk.name, APP + '_0.51.0_arm.ipk')
                self.assertEqual(sha(ipk.read_bytes()), receipt['sha256'])
                archives = ar_members(ipk.read_bytes())
                self.assertEqual(set(archives), {'debian-binary', 'control.tar.gz', 'data.tar.gz'})
                self.assertEqual(archives['debian-binary'], b'2.0\n')
                with tarfile.open(fileobj=io.BytesIO(archives['control.tar.gz'])) as control:
                    self.assertEqual([m.name for m in control], ['./control'])
                files, modes = {}, {}
                prefix = 'usr/palm/applications/' + APP + '/'
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
                                             'icon.png', 'broker-negative-proof.json'})
                expected = {'schema': 1, 'scenario': scenario,
                            'marker': {'schema': 1, 'nonce': self.pins['rendezvous_nonce'],
                                       'ui_sha256': self.pins['ui_sha256'],
                                       'broker_sha256': self.pins['broker_sha256']}}
                self.assertEqual(files['broker-negative-proof.json'],
                                 (json.dumps(expected, separators=(',', ':')) + '\n').encode())
                self.assertEqual(modes['broker-negative-proof.json'], 0o644)
                info = json.loads(files['appinfo.json'])
                self.assertEqual((info['id'], info['version'], info['main'], info['type']),
                                 (APP, '0.51.0', 'home-preview', 'native'))
                self.assertEqual(info['requiredPermissions'], [])
                self.assertEqual(modes['home-preview'], 0o755)
                self.assertEqual(files['home-preview'], self.f.binary.read_bytes())
                self.assertEqual(receipt['broker_startup_root_pins_sha256'], sha(self.pins_path.read_bytes()))
                helper_build = json.loads(self.helper_receipt.read_text())
                self.assertEqual(receipt['broker_startup_helper'], {
                    'binary_sha256': self.pins['broker_sha256'],
                    'build_receipt_sha256': sha(self.helper_receipt.read_bytes()),
                    'build_source_revision': helper_build['git_revision']})
                self.assertIs(receipt['broker_negative_proof'], True)
                self.assertIs(receipt['broker_startup_proof'], False)
                self.assertEqual(receipt['broker_negative_case'], scenario)
                self.assertEqual(receipt['app_id'], APP)
                for field in ('permissions', 'services', 'boot_hooks', 'system_files'):
                    self.assertEqual(receipt[field], [])

    def test_normal_interactive_and_wrong_binary_identity_refuse(self):
        for changes in ({'NATIVE_HOME_TEST_IDENTITY': '0'},
                        {'NATIVE_HOME_INTERACTIVE_SESSION': '1'}):
            with self.subTest(changes=changes):
                self.refused(self.environment() | changes, 'isolated non-interactive proof')
        control = self.f.base / 'control-preview'
        control.write_bytes(self.f.binary.read_bytes())
        snapshot = PROVENANCE.snapshot(self.f.root)
        PROVENANCE.record_build_receipt(self.f.root, control, 'arm', snapshot,
                                       PROVENANCE.snapshot(self.f.root))
        self.refused(self.environment() | {'NATIVE_HOME_ARM_BINARY': str(control)},
                     'Test identity is for the dashboard only')

    def test_unknown_missing_and_orphan_cases_refuse(self):
        for scenario in ('', 'wrong_nonce', '../wrong_identify_nonce', 'WRONG_FINISH_NONCE'):
            with self.subTest(scenario=scenario):
                self.refused(self.environment(scenario), 'one explicit fixed case')
        self.refused({'NATIVE_HOME_BROKER_NEGATIVE_CASE': CASES[0]},
                     'Refusal case requires explicit isolated refusal mode')
        self.refused(self.environment() | {'NATIVE_HOME_BROKER_NEGATIVE_PROOF': '0'},
                     'Refusal case requires explicit isolated refusal mode')

    def test_positive_and_all_other_proof_probe_pin_modes_cannot_mix(self):
        modes = set(re.findall(r'NATIVE_HOME_[A-Z0-9_]+',
                               (self.f.root / 'package-preview.py').read_text()))
        modes = {k for k in modes if k.endswith(('_PROOF', '_PROBE', '_PINS'))}
        modes -= {'NATIVE_HOME_BROKER_NEGATIVE_PROOF', 'NATIVE_HOME_BROKER_STARTUP_PINS'}
        modes |= {'NATIVE_HOME_UNKNOWN_PROOF', 'NATIVE_HOME_UNKNOWN_PROBE', 'NATIVE_HOME_UNKNOWN_PINS'}
        for mode in sorted(modes):
            with self.subTest(mode=mode):
                self.refused(self.environment() | {mode: '1'})
        self.refused(self.environment() | {'NATIVE_HOME_NATIVE_READ_PROBE': '0'})
        self.refused(self.environment() | {'NATIVE_HOME_ROUTE_PROOF': 'hdmi1'})

    def test_orphan_helper_pins_and_invalid_private_pins_refuse(self):
        self.refused({'NATIVE_HOME_BROKER_STARTUP_PINS': str(self.pins_path)},
                     'pins/helper require explicit isolated mode')
        self.refused({'NATIVE_HOME_STARTUP_BROKER_BINARY': str(self.helper)},
                     'pins/helper require explicit isolated mode')
        for changes in ({'schema': True}, {'profile': 'preview'},
                        {'command': 'fixture-untrusted'}, {'ui_sha256': 'd' * 64},
                        {'rendezvous_nonce': 'A' * 32}):
            with self.subTest(changes=changes):
                self.write_pins(self.pins | changes)
                self.refused(self.environment(), 'Closed Root installation pins required')
        self.write_pins(self.pins)
        self.pins_path.write_text(json.dumps(self.pins).replace('{', '{"schema":1,', 1))
        self.refused(self.environment(), 'Duplicate broker installation pin')

    def test_helper_hash_and_source_identity_mismatch_refuse(self):
        self.write_pins(self.pins | {'broker_sha256': 'd' * 64})
        self.refused(self.environment(), 'Root helper must share actual UI source/framework build context')
        self.write_pins(self.pins)
        helper = json.loads(self.helper_receipt.read_text())
        helper['git_revision'] = 'd' * 40
        self.helper_receipt.write_text(json.dumps(helper) + '\n')
        self.refused(self.environment(), 'Root helper must share actual UI source/framework build context')

    def test_unreviewed_helper_binary_name_refuses_before_tools(self):
        self.refused(self.environment() | {'NATIVE_HOME_STARTUP_BROKER_BINARY': str(self.f.binary)},
                     'Fixed startup helper binary required')

    def test_stale_helper_source_snapshot_cannot_pair_with_fresh_ui(self):
        (self.f.root / 'src/fixture.txt').write_text('new selected source input\n')
        self.f.record()
        self.refused(self.environment(), 'Current source/dependency inputs differ from build receipt')


if __name__ == '__main__':
    unittest.main()
