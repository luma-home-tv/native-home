"""Synthetic/real-file offline integration checks, with process/network denied."""
import ast
import copy
import errno
import importlib.util
import json
import os
from pathlib import Path
import socket
import shutil
import subprocess
import tempfile
import types
import unittest
from unittest.mock import patch
import icon_acquisition as m

OWN = 'com.example.own'
APP = 'com.example.icon'
PNG = b'\x89PNG\r\n\x1a\nsynthetic encoded bytes, decode intentionally unclaimed'


def app(identity=APP, **values):
    return dict(dict(id=identity, title='Synthetic', type='web', visible=True,
                     version='1', folderPath='/usr/palm/applications/' + identity,
                     icon='small.png', largeIcon='images/large.png'), **values)


def catalogue(apps=None):
    apps = [app()] if apps is None else apps
    return {'replies': {'apps': {'returnValue': True, 'apps': apps},
                        'launch_points': {'returnValue': True, 'launchPoints': [{'id': a['id']} for a in apps]}}}


def snapshot(raw):
    return dict(applications=[v['app'] for _, v in sorted(m.catalogue(raw, OWN).items())],
                launch_ids=sorted(m.catalogue(raw, OWN)), inputs=[], settings={}, captured_at='synthetic')


def write(root, name, raw):
    path = root / name
    path.write_bytes(raw)
    path.chmod(0o600)
    return {'file': name, 'sha256': m.digest(raw)}


def fixture(root):
    raw = catalogue()
    key = m.plan(raw, OWN, [APP])[0]['source']
    metadata = dict(schema=1, root_kind='lg_system', app_id=APP, relative='images/large.png',
                    bytes=len(PNG), device=1, inode=2, uid=0, gid=0, mode=0o100644, nlink=1,
                    mtime_seconds=0, mtime_nanos=0, ctime_seconds=0, ctime_nanos=0)
    header = m.encoded(metadata)
    frame = b'LPI1' + len(header).to_bytes(4, 'big') + header + PNG
    stderr = b'LUMA_PUBLIC_ICON_READER_V1 pid=123\n'
    completion = dict(schema=1, acquisition_id='a'*32, app_id=APP, source=key, reader_pid=123,
                      returncode=0, closure='externally_recorded_exact_pid_absent',
                      frame_sha256=m.digest(frame), stderr_sha256=m.digest(stderr))
    scope = dict(schema=1, acquisition_id='a'*32, helper_source_sha256=m.SOURCE_SHA,
                 helper_elf_sha256=m.ELF_SHA, qualification_sha256='b'*64,
                 state='externally_recorded_closed')
    manifest = dict(schema=1, own_id=OWN, declared_ids=[APP],
                    before=write(root, 'before.json', m.encoded(raw)),
                    after=write(root, 'after.json', m.encoded(raw)),
                    closed_scope=write(root, 'scope.json', m.encoded(scope)),
                    outcomes=[dict(app_id=APP, source=key, status='acquired',
                                   frame=write(root, 'frame.bin', frame),
                                   stderr=write(root, 'stderr.txt', stderr),
                                   completion=write(root, 'completion.json', m.encoded(completion)))])
    write(root, 'manifest.json', m.encoded(manifest))
    return raw, manifest, completion


def actual_package_code():
    tree=ast.parse(Path('package-preview.py').read_text())
    outer=next(n for n in tree.body if isinstance(n,ast.If)and ast.unparse(n.test)=="main == 'home-preview' and (not preferences_migration_proof)")
    names={'assets','acquired_assets','acquisition_receipt_sha256','acquisition_path','acquisition_pin','snapshot_bytes','snapshot'}
    nodes=[]
    for n in outer.body:
        if isinstance(n,ast.Assign)and any(isinstance(t,ast.Name)and t.id in names for t in n.targets):nodes.append(n)
        elif isinstance(n,ast.ImportFrom)and n.module=='icon_acquisition':nodes.append(n)
        elif isinstance(n,ast.If)and (ast.unparse(n.test).startswith('acquisition_path is not None')or ast.unparse(n.test)=='acquired_assets is not None'):nodes.append(n)
        elif isinstance(n,ast.Expr)and ast.unparse(n).startswith("(stage / 'snapshot.json')."):nodes.append(n)
    code=compile(ast.Module(body=nodes,type_ignores=[]),'actual package input+stage AST','exec')
    return code


class OfflineIcons(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.denials = [patch.object(subprocess, 'Popen', side_effect=AssertionError('No child')),
                       patch.object(socket, 'socket', side_effect=AssertionError('No network')),
                       patch.object(socket, 'create_connection', side_effect=AssertionError('No network'))]
        for denial in cls.denials:
            denial.start()

    @classmethod
    def tearDownClass(cls):
        for denial in reversed(cls.denials):
            denial.stop()

    def setUp(self):
        # Resolve macOS's /var alias only for synthetic fixture placement.
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.root.chmod(0o700)
        self.input = self.root / 'input'
        self.input.mkdir(mode=0o700)
        self.output = self.root / 'output'
        self.output.mkdir(mode=0o700)
        self.raw, self.manifest, self.completion = fixture(self.input)
        self.snap = snapshot(self.raw)

    def tearDown(self):
        self.temp.cleanup()

    def prepare(self):
        return m.prepare(self.input/'manifest.json', self.raw, self.snap)

    def save_manifest(self):
        write(self.input, 'manifest.json', m.encoded(self.manifest))

    def bad(self, fn, *args):
        with self.assertRaises((ValueError, OSError)):
            fn(*args)

    def publish(self):
        result = self.prepare()
        m.publish(self.output, *result)
        receipt = self.output/'icon-acquisition.json'
        return result, receipt, m.digest(receipt.read_bytes())

    def test_preferred_source_and_source_map_admission_fixture_parity(self):
        # Same first-valid/registered/visible/title/own rules and full keys as Rust SourceMap.
        hidden = app(visible=False)
        first = app(version='first')
        second = app(version='second')
        no_version = app('com.example.noversion');del no_version['version']
        raw = catalogue([hidden, first, second, app(OWN), no_version])
        apps = m.catalogue(raw, OWN)
        self.assertEqual(set(apps), {APP, 'com.example.noversion'})
        self.assertEqual(apps[APP]['source'], dict(app_id=APP, version='first', **{'class':'lg_system'},
                                                folder='/usr/palm/applications/'+APP,
                                                relative=['images','large.png'], kind='large_icon'))
        self.assertIsNone(apps['com.example.noversion']['source'])
        first['largeIcon'] = '../bad.png'
        self.assertEqual(m.plan(catalogue([first]), OWN, [APP])[0]['source']['kind'], 'icon')
        raw['replies']['apps']['returnValue'] = False
        self.bad(m.catalogue, raw, OWN)

    def test_five_source_roots_three_reader_roots_and_narrower_grammar(self):
        for kind, root in m.ROOTS.items():
            row = m.plan(catalogue([app(folderPath=root+'/'+APP)]), OWN, [APP])[0]
            self.assertEqual(row['source']['class'], kind)
            self.assertEqual(row['unavailable'], None if kind in m.PROTOCOL.ROOTS else 'unsupported_root')
            if row['request']:
                self.assertEqual(tuple(row['request']), m.PROTOCOL.icon_request(root+'/'+APP, 'images/large.png', APP))
        row = m.plan(catalogue([app(largeIcon='é.png')]), OWN, [APP])[0]
        self.assertEqual(row['unavailable'], 'reader_key_unavailable')
        for ids in ([], [APP, APP], ['missing']):
            self.bad(m.plan, self.raw, OWN, ids)

    def test_complete_frame_ingestion_retains_original_bytes_no_native_authority(self):
        snap, bodies, record = self.prepare()
        self.assertEqual(list(bodies.values()), [PNG])
        self.assertEqual(snap['applications'][0]['icon'], 'icons/'+m.digest(PNG)+'.png')
        self.assertFalse(record['native_authority']);self.assertFalse(record['native_closure_verified'])
        self.assertTrue(record['external_closure_assertions_checked'])
        self.assertNotIn('icon', self.snap['applications'][0])

    def test_version_path_kind_and_current_catalogue_drift_refuse(self):
        for field, value in [('version','2'),('largeIcon','other.png'),('folderPath',m.ROOTS['encrypted_apps']+'/'+APP)]:
            raw = copy.deepcopy(self.raw);raw['replies']['apps']['apps'][0][field] = value
            self.manifest['after'] = write(self.input, 'after.json', m.encoded(raw));self.save_manifest()
            self.bad(self.prepare)
        self.manifest['after'] = write(self.input, 'after.json', m.encoded(self.raw));self.save_manifest()
        self.raw['replies']['apps']['apps'][0]['version'] = '2'
        self.bad(self.prepare)

    def test_partial_wrong_key_tail_and_nonzero_or_unknown_closure_refuse(self):
        original = (self.input/'frame.bin').read_bytes()
        for frame in (original[:-1], original+b'x', original.replace(APP.encode(), b'x'+APP.encode()[1:])):
            self.manifest['outcomes'][0]['frame'] = write(self.input,'frame.bin',frame);self.save_manifest()
            self.bad(self.prepare)
        self.manifest['outcomes'][0]['frame'] = write(self.input,'frame.bin',original)
        for field, value in [('returncode', False),('returncode',1),('reader_pid',True),('closure','unknown'),('source',{})]:
            c = dict(self.completion, **{field:value})
            self.manifest['outcomes'][0]['completion'] = write(self.input,'completion.json',m.encoded(c));self.save_manifest()
            self.bad(self.prepare)

    def test_missing_outcome_and_pid_stream_scope_substitution_refuse(self):
        original = copy.deepcopy(self.manifest)
        for outcomes in ([], self.manifest['outcomes']*2):
            self.manifest['outcomes'] = outcomes;self.save_manifest();self.bad(self.prepare)
        self.manifest = original
        self.manifest['outcomes'][0]['stderr'] = write(self.input,'stderr.txt',b'LUMA_PUBLIC_ICON_READER_V1 pid=124\n')
        self.save_manifest();self.bad(self.prepare)

    def test_explicit_unavailable_complete_set_and_no_legacy_bytes(self):
        self.manifest['outcomes'] = [dict(app_id=APP, source=self.manifest['outcomes'][0]['source'],
                                          status='unavailable', reason='not_acquired')]
        self.save_manifest();snap,bodies,r = self.prepare()
        self.assertEqual(bodies,{});self.assertNotIn('icon',snap['applications'][0])
        self.snap['applications'][0]['icon'] = 'icons/old.png';self.bad(self.prepare)

    def test_private_file_parent_symlink_hardlink_mode_and_bound_refuse(self):
        (self.input/'frame.bin').chmod(0o644);self.bad(self.prepare)
        (self.input/'frame.bin').chmod(0o600)
        os.link(self.input/'frame.bin',self.root/'alias');self.bad(self.prepare)
        (self.root/'alias').unlink()
        link=self.root/'linked';link.symlink_to(self.input, target_is_directory=True)
        self.bad(m.prepare, link/'manifest.json', self.raw,self.snap)
        original=(self.input/'frame.bin').read_bytes();(self.input/'frame.bin').unlink()
        (self.input/'frame.bin').symlink_to(self.root/'body');write(self.root,'body',original)
        self.bad(self.prepare)

    def test_output_link_refused_before_creation_and_body_limit(self):
        alias=self.root/'alias-output';alias.symlink_to(self.output,target_is_directory=True)
        self.bad(m.make_private_output,alias/'must-not-appear')
        self.assertFalse((self.output/'must-not-appear').exists())
        with m.PrivateDirectory(self.input) as directory:
            write(self.input,'limit.bin',b'x'*m.PROTOCOL.MAX_BODY)
            self.assertEqual(len(directory.read('limit.bin',m.PROTOCOL.MAX_BODY)),m.PROTOCOL.MAX_BODY)
            write(self.input,'limit.bin',b'x'*(m.PROTOCOL.MAX_BODY+1))
            self.bad(directory.read,'limit.bin',m.PROTOCOL.MAX_BODY)

    def test_manifest_duplicate_schema_pin_and_finite_set_refuse(self):
        original=(self.input/'manifest.json').read_bytes()
        write(self.input,'manifest.json',b'{"schema":1,"schema":1}')
        self.bad(self.prepare)
        write(self.input,'manifest.json',original)
        scope=m.strict((self.input/'scope.json').read_bytes());scope['helper_elf_sha256']='0'*64
        self.manifest['closed_scope']=write(self.input,'scope.json',m.encoded(scope));self.save_manifest();self.bad(self.prepare)
        ids=[f'com.example.app{i:03}'for i in range(65)]
        self.bad(m.plan,catalogue([app(i)for i in ids]),OWN,ids)
        self.bad(m.catalogue,catalogue([app()]*513),OWN)

    def test_same_bytes_new_inode_during_held_read_refuses(self):
        original = os.fstat
        calls = 0
        def replace_after_read(fd):
            nonlocal calls
            result=original(fd)
            if stat_is_file(result) and result.st_size == len((self.input/'manifest.json').read_bytes()):
                calls += 1
                if calls == 2:
                    body=(self.input/'manifest.json').read_bytes()
                    write(self.input,'new.json',body);os.replace(self.input/'new.json',self.input/'manifest.json')
            return result
        with patch.object(m.os,'fstat',side_effect=replace_after_read):self.bad(self.prepare)

    def test_publish_no_overwrite_and_package_actual_body_and_snapshot_association(self):
        result,receipt,pin=self.publish()
        saved=(self.output/'snapshot.json').read_bytes()
        self.bad(m.publish,self.output,*result)
        self.assertEqual((self.output/'snapshot.json').read_bytes(),saved)
        raw,bodies,observed=m.package_assets(self.output,receipt,pin)
        self.assertEqual(observed,pin);self.assertEqual(list(bodies.values()),[PNG])
        stage=self.root/'stage';stage.mkdir()
        m.stage_package_icons(stage,bodies)
        self.assertEqual(sorted(str(f.relative_to(stage))for f in stage.rglob('*')if f.is_file()),list(bodies))
        self.assertEqual(next((stage/'icons').iterdir()).read_bytes(),PNG)
        self.assertEqual(next((stage/'icons').iterdir()).stat().st_mode&0o777,0o644)
        self.bad(m.stage_package_icons,stage,bodies)
        self.bad(m.package_assets,self.output,receipt,'0'*64)
        changed=json.loads(raw);changed['applications'][0]['id']='com.example.other'
        write(self.output,'snapshot.json',m.encoded(changed));self.bad(m.package_assets,self.output,receipt,pin)

    def test_package_swapped_body_and_ledger_relabel_refuse(self):
        _,receipt,pin=self.publish()
        body=next((self.output/'icons').iterdir());body.write_bytes(PNG+b'x')
        self.bad(m.package_assets,self.output,receipt,pin)
        body.write_bytes(PNG)
        r=m.strict(receipt.read_bytes());r['outcomes'][0]['app_id']='com.example.other'
        write(self.output,'icon-acquisition.json',m.encoded(r))
        self.bad(m.package_assets,self.output,receipt,pin)
        self.bad(m.package_assets,self.output,receipt,m.digest(receipt.read_bytes()))

    def test_metadata_cli_never_issues_icon_command_and_explicit_merge(self):
        spec=importlib.util.spec_from_file_location('collector',Path(__file__).with_name('collect-snapshot.py'))
        c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
        responses=[self.raw['replies']['apps'],self.raw['replies']['launch_points'],{'devices':[]},{'settings':{}},{'volumeStatus':{}}]
        for explicit in (False,True):
            out=self.root/('with-icons'if explicit else 'metadata')
            argv=['collector','--host','synthetic','--identity',str(self.root/'unused-key'),'--known-hosts',str(self.root/'unused-hosts'),'--out',str(out),'--own-id',OWN]
            if explicit:argv+=['--icon-acquisition',str(self.input/'manifest.json')]
            seen=[]
            def run(args,**kwargs):
                seen.append(args[-1]);return types.SimpleNamespace(stdout=json.dumps(responses[len(seen)-1]).encode())
            with patch('sys.argv',argv),patch.object(c.subprocess,'run',side_effect=run):c.main()
            self.assertEqual(len(seen),5)
            self.assertTrue(all(cmd.startswith('luna-send ')for cmd in seen))
            self.assertEqual('icon'in json.loads((out/'snapshot.json').read_bytes())['applications'][0],explicit)
            self.assertFalse((out/'icons').exists())if not explicit else None

    def test_exact_total_payload_and_declaration_boundaries(self):
        with patch.object(m,'MAX_TOTAL',len(PNG)):
            self.assertEqual(len(self.prepare()[1]),1)
        with patch.object(m,'MAX_TOTAL',len(PNG)-1):
            self.bad(self.prepare)
        ids=[f'com.example.app{i:03}'for i in range(64)]
        self.assertEqual(len(m.plan(catalogue([app(i)for i in ids]),OWN,ids)),64)

    def test_actual_packager_asset_branch_uses_pinned_bytes_and_refuses_missing_pin(self):
        _,receipt,pin=self.publish()
        code=actual_package_code()
        for present in (True,False):
            stage=self.root/('actual-stage'if present else 'refused-stage');stage.mkdir()
            environment={'NATIVE_HOME_SNAPSHOT_ASSETS':str(self.output),'NATIVE_HOME_ICON_ACQUISITION':str(receipt)}
            if present:environment['NATIVE_HOME_ICON_ACQUISITION_SHA256']=pin
            ns={'Path':Path,'os':os,'json':json,'stage':stage}
            with patch.dict(os.environ,environment,clear=True):
                if present:exec(code,ns)
                else:
                    with self.assertRaises(ValueError):exec(code,ns)
            if present:
                self.assertEqual(ns['acquisition_receipt_sha256'],pin)
                self.assertEqual((stage/'snapshot.json').read_bytes(),(self.output/'snapshot.json').read_bytes())
                files=sorted(str(f.relative_to(stage))for f in stage.rglob('*')if f.is_file())
                self.assertEqual(files,sorted(['snapshot.json','icons/'+m.digest(PNG)+'.png']))
            else:self.assertEqual(list(stage.iterdir()),[])

    def test_actual_packager_presence_gate_all_present_and_unknown_states_refuse(self):
        _,receipt,pin=self.publish()
        original=receipt.read_bytes()
        code=actual_package_code()
        variants=('dangling','live-link','directory','regular','EACCES','EIO')
        for variant in variants:
            if receipt.is_symlink()or receipt.is_file():receipt.unlink()
            elif receipt.is_dir():receipt.rmdir()
            if variant=='dangling':receipt.symlink_to(self.output/'missing.json')
            elif variant=='live-link':
                write(self.root,'real-receipt.json',original);receipt.symlink_to(self.root/'real-receipt.json')
            elif variant=='directory':receipt.mkdir(mode=0o700)
            else:write(self.output,'icon-acquisition.json',original)
            stage=self.root/('presence-'+variant);stage.mkdir()
            ns={'Path':Path,'os':os,'json':json,'shutil':shutil,'stage':stage}
            env={'NATIVE_HOME_SNAPSHOT_ASSETS':str(self.output)}
            if variant in ('EACCES','EIO'):
                error=PermissionError(errno.EACCES,'synthetic denied')if variant=='EACCES'else OSError(errno.EIO,'synthetic IO')
                with patch.dict(os.environ,env,clear=True),patch.object(m.os,'lstat',side_effect=error):
                    self.bad(exec,code,ns)
            else:
                with patch.dict(os.environ,env,clear=True):self.bad(exec,code,ns)
            self.assertEqual(list(stage.iterdir()),[],variant)
        # Explicit pins retain nofollow refusal for a dangling receipt.
        receipt.unlink();receipt.symlink_to(self.output/'missing.json')
        stage=self.root/'explicit-linked';stage.mkdir()
        env={'NATIVE_HOME_SNAPSHOT_ASSETS':str(self.output),'NATIVE_HOME_ICON_ACQUISITION':str(receipt),
             'NATIVE_HOME_ICON_ACQUISITION_SHA256':pin}
        with patch.dict(os.environ,env,clear=True):
            self.bad(exec,code,{'Path':Path,'os':os,'json':json,'shutil':shutil,'stage':stage})
        self.assertEqual(list(stage.iterdir()),[])

    def test_actual_packager_only_literal_ENOENT_allows_legacy_without_provenance(self):
        _,receipt,_=self.publish();receipt.unlink()
        stage=self.root/'literal-missing';stage.mkdir()
        ns={'Path':Path,'os':os,'json':json,'shutil':shutil,'stage':stage}
        with patch.dict(os.environ,{'NATIVE_HOME_SNAPSHOT_ASSETS':str(self.output)},clear=True):
            exec(actual_package_code(),ns)
        self.assertIsNone(ns['acquisition_receipt_sha256'])
        self.assertIsNone(ns['acquired_assets'])
        self.assertEqual(next((stage/'icons').iterdir()).read_bytes(),PNG)
        with patch.object(m.os,'lstat',side_effect=FileNotFoundError(errno.ENOENT,'missing')):
            self.assertFalse(m.acquisition_receipt_present(self.output))
        with patch.object(m.os,'lstat',side_effect=FileNotFoundError(errno.EIO,'unknown')):
            self.bad(m.acquisition_receipt_present,self.output)

    def test_actual_cli_link_directory_or_unknown_manifest_never_publishes_icons(self):
        spec=importlib.util.spec_from_file_location('collector-refusal',Path(__file__).with_name('collect-snapshot.py'))
        c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
        responses=[self.raw['replies']['apps'],self.raw['replies']['launch_points'],{'devices':[]},{'settings':{}},{'volumeStatus':{}}]
        for variant in ('dangling','directory','unknown'):
            manifest=self.input/('invalid-'+variant+'.json')
            if variant=='dangling':manifest.symlink_to(self.input/'missing.json')
            elif variant=='directory':manifest.mkdir(mode=0o700)
            else:write(self.input,manifest.name,b'{}')
            out=self.root/('cli-'+variant)
            argv=['collector','--host','synthetic','--identity',str(self.root/'unused-key'),'--known-hosts',str(self.root/'unused-hosts'),
                  '--out',str(out),'--own-id',OWN,'--icon-acquisition',str(manifest)]
            seen=[]
            def run(args,**kwargs):
                seen.append(args[-1]);return types.SimpleNamespace(stdout=json.dumps(responses[len(seen)-1]).encode())
            original=m.PrivateDirectory.read
            def unknown(directory,name,cap=m.MAX_JSON):
                if name==manifest.name:raise OSError(errno.EIO,'unknown manifest read')
                return original(directory,name,cap)
            with patch('sys.argv',argv),patch.object(c.subprocess,'run',side_effect=run):
                if variant=='unknown':
                    with patch.object(m.PrivateDirectory,'read',new=unknown):self.bad(c.main)
                else:self.bad(c.main)
            self.assertTrue(all(command.startswith('luna-send ')for command in seen))
            self.assertFalse((out/'snapshot.json').exists());self.assertFalse((out/'icons').exists())
            self.assertFalse((out/'icon-acquisition.json').exists())

    def test_packager_calls_validator_and_never_reopens_acquired_body_by_path(self):
        tree=ast.parse(Path('package-preview.py').read_text())
        self.assertTrue(any(isinstance(n,ast.Call)and isinstance(n.func,ast.Name)and n.func.id=='package_assets'and len(n.args)==3 for n in ast.walk(tree)))
        branch=next(n for n in ast.walk(tree)if isinstance(n,ast.If)and isinstance(n.test,ast.Compare)and isinstance(n.test.left,ast.Name)and n.test.left.id=='acquired_assets'and any(isinstance(x,ast.Call)and isinstance(x.func,ast.Name)and x.func.id=='stage_package_icons'for row in n.body for x in ast.walk(row)))
        calls=[n.func.id for row in branch.body for n in ast.walk(row)if isinstance(n,ast.Call)and isinstance(n.func,ast.Name)]
        self.assertEqual(calls,['stage_package_icons'])


def stat_is_file(s):
    import stat
    return stat.S_ISREG(s.st_mode)


if __name__=='__main__':
    unittest.main(verbosity=2)
