"""Ordinary consumer association artifact checks; no native/publisher authority."""
import hashlib,json,os,subprocess
from pathlib import Path,PurePosixPath
from build_provenance import _read,RECEIPT_LIMIT
from cache_consumer_contract import native_descriptor,semantic_fields,native_selection
LEGACY_NATIVE_RECIPE_SHA='aa1a96e3847df4eb406a03ce1b018672c46a61f73119029acb78f5fa9ffe3492'
LEGACY_NATIVE_SOURCE='f5b4d7758e700e22b6f88183535cc0537d60a5e0'
LEGACY_NATIVE_UI_SHA='46743fad9f205c320072a59f4173368a715aa829e2d323dfa3661f46743061ed'
LEGACY_NATIVE_HELPER_SHA='981918ae4b73b1551253df86ee22bcd22e21aaee7416ab6c6adf78b7030795c7'

APP='com.ash.nativehome.preview'
FIELDS={'schema','app_id','producer_kind','source_revision','ui_sha256','broker_sha256'}
CONTRACT_FILES={'source_grammar_sha256':'src/app_artwork.rs','app_id_grammar_sha256':'src/model.rs','record_parser_sha256':'src/durable_icon_cache.rs','maintenance_planner_sha256':'src/durable_icon_cache_maintenance.rs','file_reader_sha256':'src/packaged_icon_reader.rs','decoder_policy_sha256':'src/icon_decode.rs','cargo_lock_sha256':'Cargo.lock','dependency_lock_sha256':'dependency-lock.json','framework_patch_sha256':'patches/plxnative-framework-integration.patch'}
def need(ok,message):
 if not ok:raise ValueError(message)
def sha(raw):return hashlib.sha256(raw).hexdigest()
def hexadecimal(value,size):return type(value)is str and len(value)==size and all(c in '0123456789abcdef'for c in value)
def strict(raw):
 def pairs(rows):
  out={}
  for key,value in rows:
   need(key not in out,'Duplicate historical producer/receipt field');out[key]=value
  return out
 return json.loads(raw,object_pairs_hook=pairs,parse_constant=lambda _:(_ for _ in()).throw(ValueError('Nonfinite provenance')))
def contract(snapshot,build_contract):
 need(type(snapshot)is dict and type(snapshot.get('schema'))is int and snapshot['schema']==1 and type(snapshot.get('source_inputs'))is dict,'Actual complete source snapshot')
 need(type(build_contract)is dict and build_contract.get('manifest_input_sha256')==snapshot['source_inputs'].get('Cargo.toml',{}).get('sha256'),'Actual native manifest input binding')
 out={'schema':1,**semantic_fields(build_contract)}
 for key,name in CONTRACT_FILES.items():
  row=snapshot['source_inputs'].get(name);need(type(row)is dict and set(row)=={'bytes','sha256'}and type(row['bytes'])is int and 0<row['bytes']<=16777216 and hexadecimal(row['sha256'],64),'Exact critical reader/parser/source/policy input');out[key]=row['sha256']
 return out
def selector(current,old_ui,old_helper,producer):
 for row,name in((current,'home-preview'),(old_ui,'home-preview'),(old_helper,'artwork-broker')):
  need(type(row)is dict and type(row.get('schema'))is int and row['schema']==1 and row.get('target')=='arm'and type(row.get('binary'))is dict and row['binary'].get('name')==name and hexadecimal(row['binary'].get('sha256'),64)and type(row['binary'].get('bytes'))is int and 52<=row['binary']['bytes']<=16777216 and hexadecimal(row.get('git_revision'),40),'Exact ARM artifact receipt roles')
 need(old_ui.get('consumer_build_contract')==old_helper.get('consumer_build_contract')and old_ui['git_revision']==old_helper['git_revision']and old_ui['snapshot']==old_helper['snapshot']and old_ui['toolchain']==old_helper['toolchain'],'Historical producer UI/helper actual same source inputs')
 need(contract(current['snapshot'],current.get('consumer_build_contract'))==contract(old_ui['snapshot'],old_ui.get('consumer_build_contract')),'Exact unchanged reader/parser/source grammar/cache ABI bytes required')
 need(current['snapshot']['framework']==old_ui['snapshot']['framework']and current['toolchain']==old_ui['toolchain'],'Exact framework/patch and declared target/compiler policy')
 need(type(producer)is dict and set(producer)==FIELDS and type(producer['schema'])is int and producer['schema']==1 and producer['app_id']==APP and producer['producer_kind']=='joined-single-positive-v1'and producer['source_revision']==old_ui['git_revision']and producer['ui_sha256']==old_ui['binary']['sha256']and producer['broker_sha256']==old_helper['binary']['sha256'],'Original producer profile preserved, never relabelled to current consumer')
 return {'schema':1,'app_id':APP,'consumer_ui_sha256':current['binary']['sha256'],'reader_contract':contract(current['snapshot'],current.get('consumer_build_contract')),'producer':dict(producer)}
def read_inputs(repo,current,old_ui_path,old_helper_path,profile_path):
 inputs={}
 def read(path,private=False,limit=16777216):
  path=Path(path).absolute();raw=_read(path,private=private,limit=limit);inputs[str(path)]={'sha256':sha(raw),'bytes':len(raw),'private':private,'limit':limit};return raw
 receipts=[]
 for path,name in((Path(old_ui_path),'home-preview'),(Path(old_helper_path),'artwork-broker')):
  need(path.name==name,'Fixed historical artifact filenames')
  raw=read(path);need(len(raw)>=52 and raw[:7]==b'\x7fELF\x01\x01\x01'and int.from_bytes(raw[18:20],'little')==40,'Actual historical ARM32 little-endian ELF, never executed')
  row=strict(read(path.with_name(path.name+'.build.json'),private=True,limit=RECEIPT_LIMIT));need(row.get('binary')=={'name':name,'sha256':sha(raw),'bytes':len(raw)},'Historical binary matches actual adjacent immutable receipt');receipts.append(row)
 profile_raw=read(profile_path,limit=4096);profile=strict(profile_raw)
 old=receipts[0];rows=old['snapshot']['source_inputs'];blobs={};need(0<len(rows)<=512,'Bounded entire original source graph')
 for name,row in rows.items():
  need(type(name)is str and PurePosixPath(name).as_posix()==name and not PurePosixPath(name).is_absolute()and all(p not in('.','..','.git')for p in PurePosixPath(name).parts)and ':'not in name and not any(ord(c)<32 for c in name),'Safe exact historical Git input path')
  need(type(row)is dict and set(row)=={'bytes','sha256'}and type(row['bytes'])is int and 0<=row['bytes']<=16777216 and hexadecimal(row['sha256'],64),'Bounded historical Git blob binding')
  env={k:v for k,v in os.environ.items()if not k.startswith('GIT_')};env.update(GIT_NO_LAZY_FETCH='1',GIT_TERMINAL_PROMPT='0',GIT_OPTIONAL_LOCKS='0')
  blob=subprocess.run(['git','--no-optional-locks','--no-replace-objects','-C',str(repo),'show',old['git_revision']+':'+name],capture_output=True,timeout=10,env=env)
  need(blob.returncode==0 and len(blob.stdout)==row['bytes']and sha(blob.stdout)==row['sha256'],'Every original producer source input matches actual immutable Git blob');blobs[name]=blob.stdout
 need('Cargo.toml'in blobs and 'build-client.py'in blobs,'Historical actual manifest/build recipe required')
 need(receipts[0].get('consumer_build_contract')==receipts[1].get('consumer_build_contract'),'Historical UI/helper native selection must match')
 recorded=old.get('consumer_build_contract')
 if recorded is None:
  need(old['git_revision']==LEGACY_NATIVE_SOURCE and old['binary']['sha256']==LEGACY_NATIVE_UI_SHA and receipts[1]['binary']['sha256']==LEGACY_NATIVE_HELPER_SHA and sha(blobs['build-client.py'])==LEGACY_NATIVE_RECIPE_SHA,'Unknown legacy native artifact or feature selection; no guessing')
  recorded=native_descriptor(blobs['Cargo.toml'],native_selection(blobs['Cargo.toml']))
 else:need(recorded==native_descriptor(blobs['Cargo.toml'],recorded.get('selection')),'Historical native descriptor differs from actual Git manifest')
 receipts=[dict(row,consumer_build_contract=recorded)for row in receipts]
 return selector(current,*receipts,profile),inputs
def recheck(inputs):
 for path,row in inputs.items():
  raw=_read(Path(path),private=row['private'],limit=row['limit']);need(len(raw)==row['bytes']and sha(raw)==row['sha256'],'Historical selector source changed before package completion')
