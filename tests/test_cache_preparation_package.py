"""PRIVATE UNRUN actual packager fixtures; synthetic Git/ARM receipts/inert ar.

No compiler, helper, Root service or TV is used by this proposed host suite.
Root owns execution. Public fixture strings are synthetic, never household data.
"""
import importlib.util
import json
from pathlib import Path
import unittest

REPO = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    'preparation_cache_fixture', REPO / 'tests/test_cache_producer_package.py')
BASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)
MODE = 'NATIVE_HOME_CACHE_PREPARATION'
PINS = 'NATIVE_HOME_CACHE_PREPARATION_PINS'
MARKER = 'artwork-cache-preparation.json'


class CachePreparationPackageTests(unittest.TestCase):
    def setUp(self):
        self.fixture = BASE.CacheProducerPackageTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.f = self.fixture.f
        self.path = self.f.base / 'catalogue-preparation-pins.json'
        self.pins = {'schema': 1,
                     'ui_sha256': BASE.BASE.sha(self.f.binary.read_bytes()),
                     'cursor': None}
        self.write(self.pins)

    def write(self, value):
        self.path.unlink(missing_ok=True)
        raw = value if isinstance(value, bytes) else (json.dumps(value) + '\n').encode()
        self.path.write_bytes(raw)
        self.path.chmod(0o600)

    def extra(self, changes=None):
        return {MODE: '1', PINS: str(self.path), **(changes or {})}

    def package(self, **kwargs):
        return self.fixture.package(diagnostic=True, extra=self.extra(), **kwargs)

    def refused(self, *, changes=None, diagnostic=True, association=True):
        result = self.fixture.refused(diagnostic=diagnostic, association=association,
                                      extra=self.extra(changes))
        self.assertFalse((self.f.base / 'compiler-executed').exists())
        self.assertFalse((self.f.base / 'cargo-executed').exists())
        return result

    def test_null_cursor_selects_only_exact_current_arm_diagnostic_marker(self):
        files, receipt = self.fixture.files(*self.package())
        expected = (json.dumps(self.pins, separators=(',', ':')) + '\n').encode()
        self.assertEqual(files[MARKER][1], expected)
        self.assertEqual((files[MARKER][0].mode, files[MARKER][0].uid,
                          files[MARKER][0].gid), (0o644, 0, 0))
        self.assertEqual(set(files), {'appinfo.json', 'home-preview', 'snapshot.json',
            'appfont.ttf', 'appfont-bold.ttf', 'OFL.txt', 'icon.png',
            'preview-artwork-admission.json', 'artwork-cache-producer.json', MARKER})
        self.assertEqual(json.loads(files['appinfo.json'][1])['version'], '0.59.0')
        self.assertNotIn('interactive-session.json', files)
        self.assertNotIn('artwork-broker', files)
        self.assertEqual(receipt['cache_preparation_marker_sha256'], BASE.BASE.sha(expected))
        self.assertEqual(receipt['cache_preparation_pins_sha256'],
                         BASE.BASE.sha(self.path.read_bytes()))
        self.assertTrue(receipt['cache_preparation_selected'])
        self.assertIsNone(receipt['cache_preparation_cursor'])
        self.assertEqual(receipt['cache_preparation_scope'], 'catalogue_preferred_enumeration_only')
        for key in ('cache_preparation_execution_verified',
                    'cache_preparation_prepared_coverage_verified',
                    'cache_preparation_Root_authority_granted'):
            self.assertIs(receipt[key], False)
        for key in ('permissions', 'services', 'boot_hooks', 'system_files'):
            self.assertEqual(receipt[key], [])

    def test_cursor_is_canonical_bounded_enumeration_metadata_not_completion(self):
        for ordinal in (0, 16, 17, 512):
            with self.subTest(ordinal=ordinal):
                cursor = {'next_ordinal': ordinal, 'keyset_sha256': 'a' * 64, 'schema': 1}
                self.write(dict(self.pins, cursor=cursor))
                files, receipt = self.fixture.files(*self.package())
                self.assertEqual(json.loads(files[MARKER][1])['cursor'], cursor)
                self.assertEqual(receipt['cache_preparation_cursor'], cursor)
                self.assertFalse(receipt['cache_preparation_execution_verified'])
                # Plan checks actual keyset/range/alignment. Packager grammar does not.
                self.assertNotIn('execution_verified', json.loads(files[MARKER][1]))

    def test_absent_mode_is_inert_for_normal_and_existing_diagnostic_sidecar(self):
        for diagnostic in (False, True):
            files, receipt = self.fixture.files(*self.fixture.package(diagnostic=diagnostic))
            self.assertNotIn(MARKER, files)
            self.assertFalse(receipt['cache_preparation_selected'])
            self.assertIsNone(receipt['cache_preparation_marker_sha256'])
            self.assertIsNone(receipt['cache_preparation_scope'])

    def test_mode_requires_literal_one_and_required_private_pins(self):
        for value in ('', '0', 'true', '01', '2'):
            with self.subTest(value=value):
                self.refused(changes={MODE: value})
        self.refused(changes={PINS: ''})
        self.path.unlink()
        self.refused()

    def test_orphan_pins_are_rejected_even_empty_without_mode(self):
        for value in ('', str(self.path)):
            self.fixture.refused(diagnostic=True, extra={PINS: value})

    def test_existing_preview_admission_cache_producer_and_role_are_required(self):
        self.refused(diagnostic=False)
        self.refused(association=False)
        self.refused(changes={'NATIVE_HOME_PREVIEW_ARTWORK_ADMISSION': '0'})
        self.refused(changes={'NATIVE_HOME_TEST_IDENTITY': '1'})
        self.refused(changes={'NATIVE_HOME_INTERACTIVE_SESSION': '1'})

    def test_unknown_and_other_modes_are_rejected_including_empty_values(self):
        for key, value in [('NATIVE_HOME_FUTURE_ACTION', ''),
                           ('NATIVE_HOME_FUTURE_ACTION', '1'),
                           ('NATIVE_HOME_ICON_PRESENTATION_PROOF', '1'),
                           ('NATIVE_HOME_SESSION_CAPTURE_PROOF', ''),
                           ('NATIVE_HOME_ROUTE_PROOF', 'hdmi1'),
                           ('NATIVE_HOME_PERFORMANCE_PROOF', '1')]:
            with self.subTest(key=key, value=value):
                self.refused(changes={key: value})

    def test_marker_schema_identity_and_authority_extensions_are_closed(self):
        for change in ({'schema': True}, {'schema': 2}, {'ui_sha256': 'b' * 64},
                       {'ui_sha256': 'A' * 64}, {'ui_sha256': False},
                       {'path': '/untrusted'}, {'Root_authority_granted': True}):
            with self.subTest(change=change):
                self.write(self.pins | change)
                self.refused()
        self.write({'schema': 1, 'ui_sha256': self.pins['ui_sha256']})
        self.refused()

    def test_cursor_schema_digest_integer_and_unknown_fields_are_closed(self):
        original = {'schema': 1, 'keyset_sha256': 'a' * 64, 'next_ordinal': 16}
        for change in ({'schema': True}, {'schema': 2}, {'keyset_sha256': 'A' * 64},
                       {'keyset_sha256': 'a' * 63}, {'next_ordinal': True},
                       {'next_ordinal': 16.0}, {'next_ordinal': -1},
                       {'next_ordinal': 513}, {'completed': True}, {'path': '/untrusted'}):
            with self.subTest(change=change):
                self.write(self.pins | {'cursor': original | change})
                self.refused()
        for value in ([], 'cursor', 16, False):
            self.write(self.pins | {'cursor': value})
            self.refused()

    def test_duplicates_nonfinite_and_oversize_raw_inputs_refuse(self):
        ui = self.pins['ui_sha256']
        cursor = '"cursor":{"schema":1,"keyset_sha256":"' + 'a'*64 + '","next_ordinal":16,"next_ordinal":16}'
        for raw in [('{' + '"schema":1,"schema":1,"ui_sha256":"'+ui+'","cursor":null}').encode(),
                    ('{"schema":1,"ui_sha256":"'+ui+'",'+cursor+'}').encode(),
                    ('{"schema":1,"ui_sha256":"'+ui+'","cursor":NaN}').encode(),
                    b' ' * 4097]:
            self.write(raw)
            self.refused()

    def test_private_nofollow_singlelink_inputs_required(self):
        import os
        original = self.path.read_bytes()
        self.path.chmod(0o644)
        self.refused()
        self.write(original)
        linked = self.f.base / 'preparation-hardlink'
        os.link(self.path, linked)
        self.refused()
        linked.unlink()
        self.path.unlink()
        target = self.f.base / 'preparation-target'
        target.write_bytes(original)
        target.chmod(0o600)
        self.path.symlink_to(target)
        self.refused()

    def test_independent_startup_helper_pins_must_still_match(self):
        old = self.fixture.fixture.pins.copy()
        self.fixture.fixture.write_pins(old | {'broker_sha256': 'f' * 64})
        self.refused()


if __name__ == '__main__':
    unittest.main()
