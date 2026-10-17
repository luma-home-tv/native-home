import unittest
from preview_performance_packaging import select, ENV
class Selection(unittest.TestCase):
    def args(self): return dict(main='home-preview',target='arm',app_id='com.ash.nativehome.preview',test_identity=False,interactive=True)
    def env(self): return {ENV:'1','NATIVE_HOME_INTERACTIVE_SESSION':'1','NATIVE_HOME_CACHE_PRODUCER_BROKER':'bound-current-helper'}
    def test_default_off(self): self.assertIsNone(select({},**self.args()))
    def test_only_strict_marker_normal_profile(self):
        self.assertEqual(select(self.env(),**self.args()),{'schema':1,'lifetime_seconds':240,'snapshot_interval_ms':1000})
        for k,v in [('main','control-preview'),('target','host'),('app_id','com.ash.nativehome.proof'),('test_identity',True),('interactive',False)]:
            args=self.args();args[k]=v
            with self.subTest(k=k),self.assertRaises(ValueError):select(self.env(),**args)
    def test_no_capture_Proof_actions_or_unknown_selectors_even_empty(self):
        for k in ['NATIVE_HOME_PERFORMANCE_PROOF','NATIVE_HOME_SESSION_CAPTURE_PROOF','NATIVE_HOME_KEYBOARD_OBSERVER_PROOF','NATIVE_HOME_PREVIEW_ARTWORK_ADMISSION','NATIVE_HOME_POINTER_PROOF','NATIVE_HOME_NEW_MODE']:
            for v in ['1','']:
                e=self.env();e[k]=v
                with self.subTest(k=k,v=v),self.assertRaises(ValueError):select(e,**self.args())
    def test_marker_bad_value_or_missing_cache_refuses(self):
        for v in ['','0','true']:
            e=self.env();e[ENV]=v
            with self.assertRaises(ValueError):select(e,**self.args())
        e=self.env();del e['NATIVE_HOME_CACHE_PRODUCER_BROKER']
        with self.assertRaises(ValueError):select(e,**self.args())
    def test_existing_consumer_mode_allowed_without_rewriting_it(self):
        e=self.env();e.update(NATIVE_HOME_CACHE_CONSUMER_COMPATIBILITY='1',NATIVE_HOME_CACHE_CONSUMER_OLD_UI='old-ui',NATIVE_HOME_CACHE_CONSUMER_OLD_BROKER='old-helper',NATIVE_HOME_CACHE_CONSUMER_OLD_PROFILE='old-profile');before=dict(e);self.assertIsNotNone(select(e,**self.args()));self.assertEqual(e,before)
if __name__=='__main__':unittest.main()
