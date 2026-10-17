"""Canonical application Cargo policy + actual native build selection, no cache IO.
Framework manifests remain bound by the existing exact verified framework tree.
"""
import hashlib,json,os,re,sys,tomllib
from pathlib import Path
SCHEMA=1
METADATA={'version','description','authors','license','license-file','publish','repository','homepage','documentation','readme','categories','keywords','metadata'}
NATIVE='arm-unknown-linux-gnueabi'
def need(ok,message):
 if not ok:raise ValueError(message)
def raw(value):return json.dumps(value,sort_keys=True,separators=(',',':'),allow_nan=False).encode()
def sha(value):return hashlib.sha256(value).hexdigest()
def manifest(data):
 value=tomllib.loads(data.decode('utf8'));need(type(value)is dict and type(value.get('package'))is dict,'Application Cargo manifest required')
 def deps(rows):
  need(type(rows)is dict,'Typed dependency table');out={}
  for name,item in rows.items():
   item={'version':item}if type(item)is str else dict(item);need(type(name)is str and type(item)is dict,'Typed dependency identity');item.setdefault('default-features',True);item.setdefault('optional',False);item['features']=sorted(set(item.get('features',[])));out[name]=item
  return out
 def walk(item):
  if type(item)is dict:
   return {key:(deps(value)if key in('dependencies','build-dependencies','dev-dependencies')else walk(value))for key,value in item.items()}
  if type(item)is list:return [walk(x)for x in item]
  need(type(item)in(str,int,float,bool),'Unsupported Cargo semantic value');return item
 value=walk(value);value['package']={k:v for k,v in value['package'].items()if k not in METADATA}
 if 'workspace'in value and 'package'in value['workspace']:value['workspace']['package']={k:v for k,v in value['workspace']['package'].items()if k not in METADATA}
 if 'features'in value:value['features']={k:sorted(set(v))for k,v in value['features'].items()}
 return value

def feature_env(features):
 out=[]
 for name in features:
  need(type(name)is str and re.fullmatch('[A-Za-z0-9_+-]+',name),'Bounded ASCII feature name');out.append(name.upper().replace('-','_'))
 need(len(set(out))==len(out),'Ambiguous Cargo feature environment names');return sorted(out)
def enabled_default(policy):
 graph=dict(policy.get('features',{}));suppressed={v[4:]for values in graph.values()for v in values if v.startswith('dep:')}
 def optional(rows):
  for name,row in rows.items():
   if row.get('optional')is True and name not in suppressed:graph.setdefault(name,['dep:'+name])
 optional(policy.get('dependencies',{}))
 for target in policy.get('target',{}).values():optional(target.get('dependencies',{}))
 feature_env(graph);pending=['default']if'default'in graph else[];seen=set()
 while pending:
  key=pending.pop();need(key in graph,'Unknown local feature in default graph');
  if key in seen:continue
  seen.add(key)
  for value in graph[key]:
   need(type(value)is str,'Typed Cargo feature edge')
   if not value.startswith('dep:')and'/'not in value:pending.append(value)
 return feature_env(seen)
def native_selection(data):
 return {'schema':1,'target':NATIVE,'profile':'release','default_features':True,'requested_features':[],'enabled_feature_env':enabled_default(manifest(data)),'target_cpu':'cortex-a9','target_feature':'-neon'}
def descriptor(data,selection):
 need(type(selection)is dict and type(selection.get('schema'))is int and selection['schema']==1,'Explicit build selection');return {'schema':1,'manifest_input_sha256':sha(data),'manifest_semantics':manifest(data),'selection':selection}
def native_descriptor(data,selection):
 need(selection==native_selection(data)and type(selection['default_features'])is bool,'Only actual supported native build recipe');return descriptor(data,selection)
def observed(env):
 values=[]
 for name,value in env.items():
  if name.startswith('CARGO_FEATURE_'):
   need(value=='1','Cargo feature environment marker');values.append(name[len('CARGO_FEATURE_'):])
 return sorted(values)
def compile_descriptor(root,env):
 data=(Path(root)/'Cargo.toml').read_bytes();target=env['TARGET'];profile=env['PROFILE'];features=observed(env)
 if env.get('CARGO_CFG_TARGET_ARCH')=='arm':
  selection=json.loads(env['NATIVE_HOME_CONSUMER_BUILD_SELECTION']);result=native_descriptor(data,selection);need(target==NATIVE and profile=='release'and features==selection['enabled_feature_env'],'Actual Cargo target/profile/features differ from declared native invocation')
  flags=env.get('CARGO_ENCODED_RUSTFLAGS','').split('\x1f');code=[];i=0
  while i<len(flags):
   need(flags[i]=='-C'and i+1<len(flags),'Only reviewed native codegen/link arguments');value=flags[i+1];i+=2
   if not value.startswith('link-arg='):code.append(value)
  need(code==['target-cpu=cortex-a9','target-feature=-neon'],'Actual native Rust codegen selection differs')
  return result
 return descriptor(data,{'schema':1,'target':target,'profile':profile,'default_features':None,'requested_features':None,'enabled_feature_env':features,'target_cpu':None,'target_feature':None})
def semantic_fields(value):
 need(type(value)is dict and set(value)=={'schema','manifest_input_sha256','manifest_semantics','selection'}and type(value['schema'])is int and value['schema']==1,'Exact build contract descriptor')
 return {'cargo_semantics_sha256':sha(raw(value['manifest_semantics'])),'native_build_selection_sha256':sha(raw(value['selection']))}
if __name__=='__main__':
 need(len(sys.argv)==3,'Fixed compile root/output');result=compile_descriptor(Path(sys.argv[1]),os.environ);Path(sys.argv[2]).write_bytes(raw(result))
 for key,value in semantic_fields(result).items():print('cargo:rustc-env=LUMA_CACHE_'+key.upper()+'='+value)
