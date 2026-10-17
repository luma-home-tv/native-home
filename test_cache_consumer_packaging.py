"""Pure/local selector-provenance tests; synthetic artifacts are NOT build evidence."""
import copy,hashlib,json,socket,subprocess,tempfile,types,unittest
from pathlib import Path
from unittest.mock import patch
import cache_consumer_packaging as m
import cache_consumer_contract as c

def raw(v):return json.dumps(v,separators=(',',':')).encode()
def identity(data,name):return {'name':name,'bytes':len(data),'sha256':m.sha(data)}
def elf(tag):
 data=bytearray(52);data[:7]=b'\x7fELF\x01\x01\x01';data[18:20]=(40).to_bytes(2,'little');data[40]=tag;return bytes(data)
def fixture():
 files={name:('synthetic '+name).encode()for name in m.CONTRACT_FILES.values()};snapshot={'schema':1,'source_inputs':{n:{'bytes':len(v),'sha256':m.sha(v)}for n,v in files.items()},'framework':{'schema':1,'status':'verified','revision':'d'*40,'integration_patch_sha256':'f'*64},'toolchain':{'rust_toolchain':'synthetic','arm_target':'arm-unknown-linux-gnueabi'}}
 files['Cargo.toml']=b'[package]\nname="native_home"\nversion="0.1.0"\n[features]\nhostsim=[]\n';files['build-client.py']=b'synthetic explicit descriptor build recipe';snapshot['source_inputs'].update({n:{'bytes':len(v),'sha256':m.sha(v)}for n,v in files.items()});build_contract=c.native_descriptor(files['Cargo.toml'],c.native_selection(files['Cargo.toml']))
 old={'consumer_build_contract':build_contract,'schema':1,'target':'arm','git_revision':'a'*40,'binary':identity(elf(1),'home-preview'),'snapshot':snapshot,'toolchain':snapshot['toolchain']};helper=copy.deepcopy(old);helper['binary']=identity(elf(2),'artwork-broker');current=copy.deepcopy(old);current['git_revision']='b'*40;current['binary']=identity(elf(3),'home-preview');producer={'schema':1,'app_id':m.APP,'producer_kind':'joined-single-positive-v1','source_revision':old['git_revision'],'ui_sha256':old['binary']['sha256'],'broker_sha256':helper['binary']['sha256']};return current,old,helper,producer,files

class SelectorPackagingTests(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.denials=[patch.object(subprocess,'Popen',side_effect=AssertionError('Child denied')),patch.object(socket,'socket',side_effect=AssertionError('Network denied')),patch.object(socket,'create_connection',side_effect=AssertionError('Network denied'))]
  for p in cls.denials:p.start()
 @classmethod
 def tearDownClass(cls):
  for p in reversed(cls.denials):p.stop()
 def test_distinct_current_consumer_and_original_producer_never_relabel(self):
  current,old,helper,producer,_=fixture();before=copy.deepcopy(producer);selected=m.selector(current,old,helper,producer);self.assertEqual(selected['consumer_ui_sha256'],current['binary']['sha256']);self.assertEqual(selected['producer'],before);self.assertNotEqual(selected['consumer_ui_sha256'],selected['producer']['ui_sha256']);self.assertEqual(producer,before);self.assertEqual(selected['reader_contract'],m.contract(current['snapshot'],current['consumer_build_contract']))
 def test_every_exact_contract_framework_toolchain_and_source_binding_refuses(self):
  for name in m.CONTRACT_FILES.values():
   current,old,helper,producer,_=fixture();current['snapshot']['source_inputs'][name]['sha256']='0'*64
   with self.subTest(name=name),self.assertRaises(ValueError):m.selector(current,old,helper,producer)
  for name in('framework','toolchain','old-helper-source','relabel','extraURI','boolschema'):
   current,old,helper,producer,_=fixture()
   if name=='framework':current['snapshot']['framework']['revision']='0'*40
   elif name=='toolchain':current['toolchain']['rust_toolchain']='different'
   elif name=='old-helper-source':helper['git_revision']='0'*40
   elif name=='relabel':producer['ui_sha256']=current['binary']['sha256']
   elif name=='extraURI':producer['uri']='luna://untrusted'
   else:producer['schema']=True
   with self.subTest(name=name),self.assertRaises(ValueError):m.selector(current,old,helper,producer)
 def test_actual_historical_file_reader_git_blob_checks_and_final_recheck(self):
  current,old,helper,producer,files=fixture()
  with tempfile.TemporaryDirectory()as d:
   root=Path(d).resolve();ui=root/'home-preview';broker=root/'artwork-broker';profile=root/'producer.json'
   for path,data in((ui,elf(1)),(broker,elf(2)),(profile,raw(producer)),(ui.with_name(ui.name+'.build.json'),raw(old)),(broker.with_name(broker.name+'.build.json'),raw(helper))):path.write_bytes(data);path.chmod(0o600)
   calls=[]
   def git(argv,**kw):
    self.assertEqual(argv[:5],['git','--no-optional-locks','--no-replace-objects','-C',str(root)]);self.assertEqual(argv[5],'show');self.assertEqual(kw['env']['GIT_NO_LAZY_FETCH'],'1');revision,name=argv[6].split(':',1);self.assertEqual(revision,old['git_revision']);calls.append(name);return types.SimpleNamespace(returncode=0,stdout=files[name],stderr=b'')
   with patch.object(m.subprocess,'run',side_effect=git):selected,inputs=m.read_inputs(root,current,ui,broker,profile)
   self.assertEqual(sorted(calls),sorted(files));self.assertEqual(selected['producer'],producer);m.recheck(inputs);profile.write_bytes(raw(dict(producer,source_revision='c'*40)))
   with self.assertRaises(ValueError):m.recheck(inputs)
 def test_bad_legacy_binary_or_blob_never_becomes_accepted_profile(self):
  current,old,helper,producer,files=fixture()
  with tempfile.TemporaryDirectory()as d:
   root=Path(d).resolve();ui=root/'home-preview';broker=root/'artwork-broker';profile=root/'producer.json'
   for path,data in((ui,elf(1)),(broker,elf(2)),(profile,raw(producer)),(ui.with_name(ui.name+'.build.json'),raw(old)),(broker.with_name(broker.name+'.build.json'),raw(helper))):path.write_bytes(data);path.chmod(0o600)
   with patch.object(m.subprocess,'run',return_value=types.SimpleNamespace(returncode=0,stdout=b'wrong-blob',stderr=b'')),self.assertRaises(ValueError):m.read_inputs(root,current,ui,broker,profile)
   ui.write_bytes(b'not an ARM executable')
   with self.assertRaises(ValueError):m.read_inputs(root,current,ui,broker,profile)

if __name__=='__main__':unittest.main()
