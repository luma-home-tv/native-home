"""Actual entrypoint with synthetic ARM/source receipts and inert ar; no build/TV."""
import importlib.util
import json
from pathlib import Path
import unittest
REPO=Path(__file__).resolve().parents[1]
SPEC=importlib.util.spec_from_file_location('passive_producer_fixture',REPO/'tests/test_cache_producer_package.py')
BASE=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(BASE)
ENV='NATIVE_HOME_PREVIEW_PASSIVE_PERFORMANCE'
MARKER='preview-performance-observer.json'
class PassivePackage(unittest.TestCase):
    def setUp(self):
        self.f=BASE.CacheProducerPackageTests();self.f.setUp();self.addCleanup(self.f.doCleanups)
    def test_optin_changes_only_marker_and_explicit_receipt_not_producer_payload(self):
        normal,nr=self.f.files(*self.f.package())
        observed,receipt=self.f.files(*self.f.package(extra={ENV:'1'}))
        self.assertEqual(set(observed),set(normal)|{MARKER})
        for n in normal:
            self.assertEqual(observed[n][1],normal[n][1],n)
            self.assertEqual(observed[n][0].mode,normal[n][0].mode,n)
        marker={'schema':1,'lifetime_seconds':240,'snapshot_interval_ms':1000}
        self.assertEqual(json.loads(observed[MARKER][1]),marker)
        self.assertEqual(observed[MARKER][0].mode,0o644)
        self.assertEqual(receipt['preview_passive_performance'],marker)
        self.assertNotIn('preview_passive_performance',nr)
        self.assertNotIn(MARKER,normal)
        for n in ['performance-proof.json','session-proof.json','preview-artwork-admission.json','artwork-cache-preparation.json','artwork-broker']:
            self.assertNotIn(n,observed)
        for n in ['permissions','services','boot_hooks','system_files']:self.assertEqual(receipt[n],[])
    def test_mixed_modes_and_invalid_flag_refuse_before_archive(self):
        for extra in [{ENV:''},{ENV:'0'},{ENV:'1','NATIVE_HOME_SESSION_CAPTURE_PROOF':'1'},{ENV:'1','NATIVE_HOME_PERFORMANCE_PROOF':'1'},{ENV:'1','NATIVE_HOME_TEST_IDENTITY':'1'},{ENV:'1','NATIVE_HOME_KEYBOARD_OBSERVER_PROOF':'1'}]:
            with self.subTest(extra=extra):self.f.refused(extra=extra)
    def test_missing_association_or_diagnostic_noninteractive_refuses(self):
        self.f.refused(association=False,extra={ENV:'1'})
        self.f.refused(diagnostic=True,extra={ENV:'1'})
if __name__=='__main__':unittest.main()
