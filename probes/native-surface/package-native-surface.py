"""Root-only offline proposal; no SDK/device execution. Independent SDK ar audit remains required."""
import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import tarfile

HERE = Path(__file__).resolve().parent
APP = 'com.ash.nativehome.proof'
MAIN = 'surface-scene-proof'

def require(value, message):
    if not value:
        raise ValueError(message)

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def read(path, cap):
    path = Path(path)
    require(not any(p.is_symlink() for p in (path, *path.parents)), 'Symlink input')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        first = os.fstat(stream.fileno())
        require(stat.S_ISREG(first.st_mode) and first.st_uid == os.geteuid() and first.st_nlink == 1
                and not first.st_mode & 0o022 and 0 < first.st_size <= cap, 'Input metadata')
        raw = stream.read(first.st_size + 1)
        last = os.fstat(stream.fileno())
        named = path.stat(follow_symlinks=False)
    stamp = lambda s: tuple(getattr(s, k) for k in ('st_dev','st_ino','st_uid','st_gid','st_mode','st_nlink','st_size','st_mtime_ns','st_ctime_ns'))
    require(stamp(first) == stamp(last) == stamp(named) and len(raw) == first.st_size, 'Input changed')
    return raw

def tar_gz(rows):
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode='w', format=tarfile.USTAR_FORMAT) as archive:
        for name, content, mode in rows:
            item = tarfile.TarInfo('./' + name)
            item.uid = item.gid = item.mtime = 0
            item.uname = item.gname = ''
            item.mode = mode
            if content is None:
                item.type = tarfile.DIRTYPE
                archive.addfile(item)
            else:
                item.size = len(content)
                archive.addfile(item, io.BytesIO(content))
    return gzip.compress(output.getvalue(), mtime=0)

def ar(rows):
    output = b'!<arch>\n'
    for name, raw in rows:
        header = ('%-16s%-12d%-6d%-6d%-8o%-10d`\n' % (name+'/',0,0,0,0o100644,len(raw))).encode('ascii')
        require(len(header)==60, 'ar header')
        output += header + raw + (b'\n' if len(raw)%2 else b'')
    return output

def write_new(path, raw):
    with os.fdopen(os.open(path, os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW, 0o600), 'wb') as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--manifest-sha256',required=True)
    parser.add_argument('--build-result',required=True,type=Path)
    parser.add_argument('--build-result-sha256',required=True)
    parser.add_argument('--binary',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    args=parser.parse_args()
    manifest_raw=read(HERE/'manifest.json',1048576)
    require(sha(manifest_raw)==args.manifest_sha256,'Reviewed manifest mismatch')
    manifest=json.loads(manifest_raw)
    source_inputs={}
    for name,row in sorted(manifest['files'].items()):
        require(not name.startswith('/') and all(p not in ('','.','..') for p in name.split('/')),'Relative source path')
        raw=read(HERE/name,33554432)
        require({'sha256':sha(raw),'bytes':len(raw)}==row,'Source mismatch')
        source_inputs['private/'+name]=row
    project=HERE.parent.parent
    for name,row in sorted(manifest['external_inputs'].items()):
        require(not name.startswith('/') and all(p not in ('','.','..') for p in name.split('/')),'External relative input')
        raw=read(project/name,33554432)
        require({'sha256':sha(raw),'bytes':len(raw)}==row,'External source mismatch')
        source_inputs['external/'+name]=row
    build_raw=read(args.build_result,4194304)
    require(sha(build_raw)==args.build_result_sha256,'Reviewed build receipt mismatch')
    build=json.loads(build_raw)
    require(build['schema']==1 and build['source_manifest_sha256']==sha(manifest_raw)
            and build['source_inputs']==source_inputs and build['build_exit_code']==0
            and build['device_contact'] is False,'Build provenance mismatch')
    binary=read(args.binary,8388608)
    require(build['binary']['sha256']==sha(binary) and build['binary']['bytes']==len(binary),'Actual ELF mismatch')
    require(binary[:6]==b'\x7fELF\x01\x01' and binary[18:20]==b'(\0','Not ARM32 little-endian ELF')
    info=read(HERE/'appinfo.json',4096)
    require(json.loads(info)==manifest['appinfo'],'Closed appinfo mismatch')
    icon=read(HERE/'icon.png',1048576)
    control=('Package: '+APP+'\nVersion: 0.1.0\nSection: misc\nPriority: optional\nArchitecture: arm\nMaintainer: Native Home development\nDescription: Isolated ordinary held native surface probe\n').encode()
    prefix='usr/palm/applications/'+APP
    dirs=['usr','usr/palm','usr/palm/applications',prefix]
    files={'appinfo.json':(info,0o644),MAIN:(binary,0o755),'icon.png':(icon,0o644)}
    for name in ('appfont.ttf','appfont-bold.ttf','OFL.txt'):
        relative='resources/fonts/'+name
        font=read(project/relative,1048576)
        require({'bytes':len(font),'sha256':sha(font)}==manifest['external_inputs'][relative],'Exact owned font/licence binding')
        files[name]=(font,0o644)
    payload=tar_gz([(name,None,0o755) for name in dirs]+[(prefix+'/'+name,raw,mode) for name,(raw,mode) in sorted(files.items())])
    package=ar([('debian-binary',b'2.0\n'),('control.tar.gz',tar_gz([('control',control,0o644)])),('data.tar.gz',payload)])
    args.output.mkdir(mode=0o700,parents=False,exist_ok=False)
    target=args.output/(APP+'_0.1.0_arm.ipk')
    write_new(target,package)
    receipt={'schema':1,'app_id':APP,'main':MAIN,'version':'0.1.0','sha256':sha(package),'bytes':len(package),
        'source_manifest_sha256':sha(manifest_raw),'build_result_sha256':sha(build_raw),'binary_sha256':sha(binary),
        'resource_members':{name:{'sha256':sha(raw),'bytes':len(raw),'uid':0,'gid':0,'mode':mode} for name,(raw,mode) in sorted(files.items())},
        'directories':dirs,'services':[],'hooks':[],'permissions':[],'device_contact':False,'native_qualified':False,
        'independent_SDK_archive_audit_required':True,
        'font_source':'exact source-bound project resources/fonts files',
        'historical_three_file_package_qualification_transferred':False,
        'native_scene_or_artwork_authority_qualified':False}
    write_new(args.output/'package-receipt.json',(json.dumps(receipt,indent=2,sort_keys=True)+'\n').encode())
    print(json.dumps({'package':str(target),'sha256':sha(package),'device_contact':False}))

if __name__=='__main__':
    main()
