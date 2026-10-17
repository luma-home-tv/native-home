"""Focused real contract/selector/receipt functions; fixtures are not native artifacts."""
import copy,hashlib,json,os,socket,subprocess,tempfile,types,unittest
from pathlib import Path
from unittest.mock import patch
import cache_consumer_contract as c
import cache_consumer_packaging as p
import build_provenance as provenance
from test_cache_consumer_packaging import fixture
ROOT=Path(__file__).resolve().parent
REAL_RUN=subprocess.run;REAL_POPEN=subprocess.Popen

def historical_d221_packaging():
 # Preserve the historical positive's actual policy, rather than silently
 # treating a new parser/planner contract as already compatible with f5.
 revision='d2215d5d8761fe385ae7ae3dd76ab173793c5bb8';name='cache_consumer_packaging.py'
 env={k:v for k,v in os.environ.items()if not k.startswith('GIT_')};env.update(GIT_NO_LAZY_FETCH='1',GIT_TERMINAL_PROMPT='0',GIT_OPTIONAL_LOCKS='0')
 with patch.object(subprocess,'Popen',REAL_POPEN):
  result=REAL_RUN(['git','--no-optional-locks','--no-replace-objects','-C',str(ROOT),'show',revision+':'+name],capture_output=True,timeout=10,env=env)
 if result.returncode!=0 or hashlib.sha256(result.stdout).hexdigest()!='d1dda60de411f30f4a2ad017157542e750f88265fb101fb51e71498f0add63a7':raise ValueError('Exact historical d221 packaging policy required')
 module=types.ModuleType('historical_d221_packaging');module.__file__=str(ROOT/name);exec(compile(result.stdout,revision+':'+name,'exec'),module.__dict__);return module

def put_manifest(row,data):
 row['snapshot']['source_inputs']['Cargo.toml']={'bytes':len(data),'sha256':c.sha(data)};row['consumer_build_contract']=c.native_descriptor(data,c.native_selection(data))
def rows():
 current,old,helper,producer,_=fixture();data=(ROOT/'Cargo.toml').read_bytes()
 for row in(current,old,helper):
  put_manifest(row,data);body=(ROOT/'src/model.rs').read_bytes();row['snapshot']['source_inputs']['src/model.rs']={'bytes':len(body),'sha256':c.sha(body)}
 return current,old,helper,producer,data
class ContractTests(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.denials=[patch.object(subprocess,'Popen',side_effect=AssertionError('No unexpected child')),patch.object(socket,'socket',side_effect=AssertionError('No network')),patch.object(socket,'create_connection',side_effect=AssertionError('No network'))]
  for q in cls.denials:q.start()
 @classmethod
 def tearDownClass(cls):
  for q in reversed(cls.denials):q.stop()
 def test_actual_defining_app_id_grammar_change_refuses_selector(self):
  current,old,helper,producer,_=rows();p.selector(current,old,helper,producer);name='src/model.rs';data=(ROOT/name).read_bytes();self.assertIn(b'id.len() <= 180',data);changed=data.replace(b'id.len() <= 180',b'id.len() <= 181');current['snapshot']['source_inputs'][name]={'bytes':len(changed),'sha256':c.sha(changed)}
  with self.assertRaisesRegex(ValueError,'reader/parser/source grammar'):p.selector(current,old,helper,producer)
 def test_child_only_maintenance_change_and_missing_input_refuse_selector(self):
  for change in ('change','missing'):
   current,old,helper,producer,_=rows();p.selector(current,old,helper,producer);name='src/durable_icon_cache_maintenance.rs'
   if change=='change':current['snapshot']['source_inputs'][name]['sha256']='0'*64
   else:del current['snapshot']['source_inputs'][name]
   with self.assertRaises(ValueError):p.selector(current,old,helper,producer)
 def test_full_build_snapshot_includes_new_child_bytes(self):
  with patch.object(provenance,'_framework',return_value={'synthetic':True}):snapshot=provenance.snapshot(ROOT)
  name='src/durable_icon_cache_maintenance.rs';raw=(ROOT/name).read_bytes();self.assertEqual(snapshot['source_inputs'][name],{'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
 def test_dependency_default_features_feature_graph_target_and_profile_changes_refuse(self):
  variants=[lambda b:b.replace(b'serde_json = "1"',b'serde_json = { version="1", default-features=false }'),lambda b:b.replace(b'features = ["derive"]',b'features = ["derive", "rc"]'),lambda b:b.replace(b'[features]',b'[features]\ndefault=["hostsim"]'),lambda b:b+b'\n[target.\'cfg(target_arch="arm")\'.dependencies]\nnew_dep="1"\n',lambda b:b+b'\n[patch.crates-io]\nserde={path="different"}\n',lambda b:b.replace(b'panic = "abort"',b'panic = "unwind"')]
  for change in variants:
   current,old,helper,producer,data=rows();changed=change(data);self.assertNotEqual(data,changed);put_manifest(current,changed)
   with self.subTest(changed=changed[-100:]),self.assertRaises(ValueError):p.selector(current,old,helper,producer)
 def test_version_description_and_format_only_allow_while_exact_lock_still_applies(self):
  current,old,helper,producer,data=rows();changed=data.replace(b'version = "0.1.0"',b'version = "99.0.0"').replace(b'Luma Home native dashboard;',b'Changed package description;');put_manifest(current,changed);selected=p.selector(current,old,helper,producer);self.assertEqual(selected['producer'],producer);self.assertEqual(c.manifest(data),c.manifest(changed));self.assertNotEqual(c.sha(data),c.sha(changed))
  current['snapshot']['source_inputs']['Cargo.lock']['sha256']='0'*64
  with self.assertRaises(ValueError):p.selector(current,old,helper,producer)
 def test_actual_cargo_environment_cannot_change_or_hide_enabled_features(self):
  with tempfile.TemporaryDirectory()as d:
   root=Path(d).resolve();data=(ROOT/'Cargo.toml').read_bytes();(root/'Cargo.toml').write_bytes(data);env={'TARGET':c.NATIVE,'PROFILE':'release','CARGO_CFG_TARGET_ARCH':'arm','CARGO_ENCODED_RUSTFLAGS':'-C\x1ftarget-cpu=cortex-a9\x1f-C\x1ftarget-feature=-neon\x1f-C\x1flink-arg=/private/synthetic.o','NATIVE_HOME_CONSUMER_BUILD_SELECTION':json.dumps(c.native_selection(data))};good=c.compile_descriptor(root,env);self.assertEqual(good,c.native_descriptor(data,c.native_selection(data)))
   for key,value in [('CARGO_FEATURE_HOSTSIM','1'),('TARGET','other-target'),('PROFILE','debug'),('CARGO_ENCODED_RUSTFLAGS','--cfg\x1funtrusted'),('NATIVE_HOME_CONSUMER_BUILD_SELECTION','{}')]:
    bad=dict(env);bad[key]=value
    with self.subTest(key=key),self.assertRaises((ValueError,KeyError)):c.compile_descriptor(root,bad)
 def test_default_feature_expansion_and_no_default_feature_ambiguity(self):
  data=b'[package]\nname="example"\nversion="1"\n[dependencies]\nimage={version="1",optional=true}\n[features]\ndefault=["quality"]\nquality=["image","image/codec"]\n';self.assertEqual(c.native_selection(data)['enabled_feature_env'],['DEFAULT','IMAGE','QUALITY'])
  with self.assertRaises(ValueError):c.native_selection(data+b'foo-bar=[]\nfoo_bar=[]\n')
 def test_real_build_receipt_records_and_validates_same_native_descriptor(self):
  with tempfile.TemporaryDirectory()as d:
   root=Path(d).resolve();data=(ROOT/'Cargo.toml').read_bytes();(root/'Cargo.toml').write_bytes(data);binary=root/'home-preview';raw=bytearray(52);raw[:7]=b'\x7fELF\x01\x01\x01';raw[18:20]=(40).to_bytes(2,'little');binary.write_bytes(raw);snap={'source_inputs':{},'toolchain':{}};selection=c.native_selection(data)
   with patch.object(provenance,'snapshot',return_value=snap),patch.object(provenance.subprocess,'run',return_value=type('R',(),{'returncode':0,'stdout':b'a'*40})()):receipt=provenance.record_build_receipt(root,binary,'arm',snap,snap,build_selection=selection)
   result=json.loads(receipt.read_bytes());self.assertEqual(result['consumer_build_contract'],c.native_descriptor(data,selection))
   with patch.object(provenance,'snapshot',return_value=snap):self.assertEqual(provenance.validate_build_receipt(root,binary,receipt),result)
   result['consumer_build_contract']['selection']['enabled_feature_env']=['HOSTSIM'];receipt.write_text(json.dumps(result))
   with patch.object(provenance,'snapshot',return_value=snap),self.assertRaises(ValueError):provenance.validate_build_receipt(root,binary,receipt)
 @unittest.skipUnless(os.environ.get('LUMA_TEST_PRIVATE_ARTIFACT_ROOT'), 'Explicit private ARM artifact fixture not configured')
 def test_actual_f5_legacy_git_receipts_derive_only_exact_known_native_recipe(self):
  B=Path(os.environ['LUMA_TEST_PRIVATE_ARTIFACT_ROOT']).resolve(strict=True)
  ui=B/'settings-handoff-lifecycle-arm-v3/cargo/arm-unknown-linux-gnueabi/release/home-preview';helper=B/'normal-v3-artifacts-v1/helper/cargo/arm-unknown-linux-gnueabi/release/artwork-broker';profile=B/'normal-v3-artifacts-v1/package-normal/preview-package/usr/palm/applications/com.ash.nativehome.preview/artwork-cache-producer.json';repo=B/'settings-handoff-lifecycle-build-source-f5';old=json.loads(ui.with_name(ui.name+'.build.json').read_bytes());self.assertNotIn('consumer_build_contract',old);current=copy.deepcopy(old);current['binary']['sha256']='11'*32;current['git_revision']='b'*40;data=(ROOT/'Cargo.toml').read_bytes();put_manifest(current,data);calls=[]
  def git(argv,**kw):
   self.assertEqual(argv[:6],['git','--no-optional-locks','--no-replace-objects','-C',str(repo),'show']);self.assertEqual(kw['env']['GIT_NO_LAZY_FETCH'],'1');self.assertTrue(argv[6].startswith(old['git_revision']+':'));calls.append(argv[6]);
   with patch.object(subprocess,'Popen',REAL_POPEN):return REAL_RUN(argv,**kw)
  historical=historical_d221_packaging()
  with patch.object(historical.subprocess,'run',side_effect=git):selected,inputs=historical.read_inputs(repo,current,ui,helper,profile)
  self.assertEqual(len(calls),len(old['snapshot']['source_inputs']));self.assertEqual(selected['producer']['ui_sha256'],old['binary']['sha256']);self.assertEqual(selected['consumer_ui_sha256'],'11'*32);historical.recheck(inputs)
  with patch.object(historical.subprocess,'run',side_effect=git),patch.object(historical,'LEGACY_NATIVE_RECIPE_SHA','0'*64),self.assertRaisesRegex(ValueError,'Unknown legacy'):historical.read_inputs(repo,current,ui,helper,profile)
  with patch.object(historical.subprocess,'run',side_effect=git),patch.object(historical,'LEGACY_NATIVE_UI_SHA','0'*64),self.assertRaisesRegex(ValueError,'Unknown legacy'):historical.read_inputs(repo,current,ui,helper,profile)
 @unittest.skipUnless(os.environ.get('LUMA_TEST_PRIVATE_ARTIFACT_ROOT'), 'Explicit private ARM artifact fixture not configured')
 def test_new_maintenance_contract_refuses_actual_historical_f5_missing_child(self):
  B=Path(os.environ['LUMA_TEST_PRIVATE_ARTIFACT_ROOT']).resolve(strict=True)
  ui=B/'settings-handoff-lifecycle-arm-v3/cargo/arm-unknown-linux-gnueabi/release/home-preview';helper=B/'normal-v3-artifacts-v1/helper/cargo/arm-unknown-linux-gnueabi/release/artwork-broker';profile=B/'normal-v3-artifacts-v1/package-normal/preview-package/usr/palm/applications/com.ash.nativehome.preview/artwork-cache-producer.json';repo=B/'settings-handoff-lifecycle-build-source-f5';old=json.loads(ui.with_name(ui.name+'.build.json').read_bytes());self.assertNotIn('src/durable_icon_cache_maintenance.rs',old['snapshot']['source_inputs']);current=copy.deepcopy(old);current['binary']['sha256']='11'*32;current['git_revision']='b'*40;put_manifest(current,(ROOT/'Cargo.toml').read_bytes())
  def git(argv,**kw):
   self.assertEqual(argv[:6],['git','--no-optional-locks','--no-replace-objects','-C',str(repo),'show']);self.assertTrue(argv[6].startswith(old['git_revision']+':'))
   with patch.object(subprocess,'Popen',REAL_POPEN):return REAL_RUN(argv,**kw)
  with patch.object(p.subprocess,'run',side_effect=git),self.assertRaisesRegex(ValueError,'Exact critical reader/parser/source/policy input'):p.read_inputs(repo,current,ui,helper,profile)
if __name__=='__main__':unittest.main(verbosity=2)
