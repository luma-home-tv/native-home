"""Package only the isolated native preview; no services, hooks or system overlays."""
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
from build_provenance import validate_build_receipt

here = Path(__file__).resolve().parent
repo = here
output = Path(os.environ['NATIVE_HOME_BUILD_DIR']).resolve()
sdk = Path(os.environ['NATIVE_HOME_WEBOS_SDK']).resolve()
binary = Path(os.environ['NATIVE_HOME_ARM_BINARY']).absolute()
build_receipt_path = binary.with_name(binary.name + '.build.json')
build_receipt = validate_build_receipt(here, binary, build_receipt_path)
build_receipt_sha256 = hashlib.sha256(build_receipt_path.read_bytes()).hexdigest()
main = binary.name
if main not in ('control-preview', 'home-preview', 'launch-target'): raise ValueError('Unexpected preview binary; temporary helpers cannot be packaged')
app_id = 'com.ash.nativehome.targetproof' if main == 'launch-target' else 'com.ash.nativehome.preview'
test_identity = os.environ.get('NATIVE_HOME_TEST_IDENTITY') == '1'
if test_identity:
    if main != 'home-preview': raise ValueError('Test identity is for the dashboard only')
    app_id = 'com.ash.nativehome.proof'
stage = output / 'preview-package/usr/palm/applications' / app_id
stage.mkdir(parents=True, exist_ok=False)
version = '0.38.0' if main == 'home-preview' else '0.1.0'
artwork_target_fixture = os.environ.get('NATIVE_HOME_ARTWORK_TARGET_FIXTURE', '')
if artwork_target_fixture:
    if main != 'launch-target' or test_identity or artwork_target_fixture not in ('a','c'):
        raise ValueError('Artwork fixture is only the fixed owned native target')
    version = '1.0.0' if artwork_target_fixture == 'a' else '2.0.0'
if any(os.environ.get(k)=='1' for k in ('NATIVE_HOME_MANUAL_TEXT_PROOF','NATIVE_HOME_KEYBOARD_FLOW_PROOF')) and os.environ.get('NATIVE_HOME_TEXT_INPUT_PROOF') != '1':
    raise ValueError('Manual text trial requires isolated text proof')
interactive = os.environ.get('NATIVE_HOME_INTERACTIVE_SESSION') == '1'
sound_read_proof = os.environ.get('NATIVE_HOME_SOUND_READ_PROOF') == '1'
broker_identify_proof = os.environ.get('NATIVE_HOME_BROKER_IDENTIFY_PROOF') == '1'
broker_artwork_read_proof = os.environ.get('NATIVE_HOME_ARTWORK_READ_PROOF') == '1'
live_artwork_proof = os.environ.get('NATIVE_HOME_LIVE_ARTWORK_PROOF') == '1'
icon_presentation_proof = os.environ.get('NATIVE_HOME_ICON_PRESENTATION_PROOF') == '1'
startup_capability_proof = os.environ.get('NATIVE_HOME_STARTUP_CAPABILITY_PROOF') == '1'
if startup_capability_proof:
    others = [k for k,v in os.environ.items() if v and k.startswith('NATIVE_HOME_') and
              (k.endswith('_PROOF') or k.endswith('_PROBE') or k.endswith('_PINS')) and
              k != 'NATIVE_HOME_STARTUP_CAPABILITY_PROOF']
    if main != 'home-preview' or not test_identity or interactive or others or artwork_target_fixture:
        raise ValueError('Startup capability requires only the isolated non-interactive proof')
icon_presentation_marker = None
if icon_presentation_proof:
    others = [k for k,v in os.environ.items() if v and k.startswith('NATIVE_HOME_') and
              (k.endswith('_PROOF') or k.endswith('_PROBE')) and k != 'NATIVE_HOME_ICON_PRESENTATION_PROOF']
    if main != 'home-preview' or not test_identity or interactive or others or artwork_target_fixture:
        raise ValueError('Icon presentation requires only the isolated non-interactive proof')
    icon_presentation_marker = {'schema':1,
        'fixture_ids':['synthetic.fixture.icon.'+name for name in ('square','wide','tall','missing')],
        'dimensions':[[128,128],[128,64],[64,128],None]}
live_artwork_marker = None
if live_artwork_proof:
    others = [k for k,v in os.environ.items() if v and k.startswith('NATIVE_HOME_') and
              (k.endswith('_PROOF') or k.endswith('_PROBE')) and k != 'NATIVE_HOME_LIVE_ARTWORK_PROOF']
    if main != 'home-preview' or not test_identity or interactive or others or artwork_target_fixture:
        raise ValueError('Live artwork requires only the isolated non-interactive lifecycle proof')
    def strict_pairs(pairs):
        value={}
        for key,item in pairs:
            if key in value: raise ValueError('Duplicate artwork recipe key')
            value[key]=item
        return value
    raw=Path(os.environ['NATIVE_HOME_LIVE_ARTWORK_PINS']).read_bytes()
    if not 0<len(raw)<=4096: raise ValueError('Bounded fixed artwork recipe required')
    live_artwork_marker=json.loads(raw,object_pairs_hook=strict_pairs)
    marker=live_artwork_marker
    if (type(marker) is not dict or set(marker)!={'schema','target_id','encoded_sha256','versions','max_seconds'} or
        type(marker['schema']) is not int or marker['schema']!=1 or marker['target_id']!='com.ash.nativehome.targetproof' or
        marker['versions']!=['1.0.0','1.0.0','2.0.0'] or type(marker['max_seconds']) is not int or marker['max_seconds']!=120 or
        type(marker['encoded_sha256']) is not list or len(marker['encoded_sha256'])!=3 or
        any(type(h) is not str or len(h)!=64 or any(c not in '0123456789abcdef' for c in h) for h in marker['encoded_sha256']) or
        len(set(marker['encoded_sha256']))!=3): raise ValueError('Invalid fixed artwork recipe')
elif os.environ.get('NATIVE_HOME_LIVE_ARTWORK_PINS'):
    raise ValueError('Artwork recipe requires explicit isolated lifecycle mode')
if broker_artwork_read_proof and not broker_identify_proof:
    raise ValueError('Artwork transfer requires explicit isolated broker mode')
broker_marker = None
if broker_identify_proof:
    allowed = {'NATIVE_HOME_BROKER_IDENTIFY_PROOF', 'NATIVE_HOME_NATIVE_READ_PROBE'}
    if broker_artwork_read_proof:
        allowed.add('NATIVE_HOME_ARTWORK_READ_PROOF')
    others = [k for k, v in os.environ.items() if v and k.startswith('NATIVE_HOME_')
              and (k.endswith('_PROOF') or k.endswith('_PROBE')) and k not in allowed]
    if main != 'home-preview' or not test_identity or interactive or os.environ.get('NATIVE_HOME_NATIVE_READ_PROBE') != '1' or others:
        raise ValueError('Broker Identify requires isolated non-interactive native read diagnostics and no other trial')
    nonce = os.environ.get('NATIVE_HOME_BROKER_NONCE', '')
    broker_sha256 = os.environ.get('NATIVE_HOME_BROKER_SHA256', '')
    if len(nonce) != 32 or any(c not in '0123456789abcdef' for c in nonce) or len(broker_sha256) != 64 or any(c not in '0123456789abcdef' for c in broker_sha256):
        raise ValueError('Strict nonce and broker executable digest required')
    broker_marker = {'schema': 1, 'nonce': nonce, 'ui_sha256': build_receipt['binary']['sha256'], 'broker_sha256': broker_sha256}
elif any(os.environ.get(k) for k in ('NATIVE_HOME_BROKER_NONCE', 'NATIVE_HOME_BROKER_SHA256')):
    raise ValueError('Broker marker values require explicit isolated Identify mode')
if sound_read_proof and (main != 'home-preview' or not test_identity or interactive or os.environ.get('NATIVE_HOME_NATIVE_READ_PROBE') != '1' or any(os.environ.get(k) for k in (
    'NATIVE_HOME_DRAWER_PROBE', 'NATIVE_HOME_MANAGEMENT_PROBE', 'NATIVE_HOME_CONTROLS_PROBE', 'NATIVE_HOME_LAUNCH_PROOF', 'NATIVE_HOME_ROUTE_PROOF',
    'NATIVE_HOME_PREFERENCES_PROBE', 'NATIVE_HOME_POINTER_PROOF', 'NATIVE_HOME_POINTER_UI_PROOF', 'NATIVE_HOME_NAVIGATION_PROOF', 'NATIVE_HOME_DESCRIPTION_LEVEL_PROOF',
    'NATIVE_HOME_RANGE_LAYOUT_PROOF', 'NATIVE_HOME_DESCRIPTION_UI_PROOF', 'NATIVE_HOME_TEXT_INPUT_PROOF', 'NATIVE_HOME_MANUAL_TEXT_PROOF', 'NATIVE_HOME_KEYBOARD_FLOW_PROOF',
    'NATIVE_HOME_KEYBOARD_OBSERVER_PROOF', 'NATIVE_HOME_KEYBOARD_RECIPE_PROOF', 'NATIVE_HOME_SESSION_CAPTURE_PROOF', 'NATIVE_HOME_PERFORMANCE_PROOF'))):
    raise ValueError('Sound read proof requires isolated native read diagnostics and no other trial')
if os.environ.get('NATIVE_HOME_NATIVE_READ_PROBE') == '1' and (main != 'home-preview' or not test_identity or interactive):
    raise ValueError('Native read/layout diagnostics require the isolated non-interactive dashboard')
keyboard_observer = os.environ.get('NATIVE_HOME_KEYBOARD_OBSERVER_PROOF') == '1'
keyboard_recipe = os.environ.get('NATIVE_HOME_KEYBOARD_RECIPE_PROOF') == '1'
if keyboard_recipe and not keyboard_observer: raise ValueError('Keyboard recipe requires explicit observer')
if keyboard_observer and (main != 'home-preview' or not test_identity or not interactive or any(os.environ.get(k) for k in ('NATIVE_HOME_SESSION_CAPTURE_PROOF','NATIVE_HOME_PERFORMANCE_PROOF'))):
    raise ValueError('Keyboard observer requires isolated native interactive identity and no other instrumentation')
if interactive and (main != 'home-preview' or any(os.environ.get(name) for name in (
    'NATIVE_HOME_DRAWER_PROBE', 'NATIVE_HOME_MANAGEMENT_PROBE', 'NATIVE_HOME_CONTROLS_PROBE', 'NATIVE_HOME_LAUNCH_PROOF', 'NATIVE_HOME_ROUTE_PROOF',
    'NATIVE_HOME_PREFERENCES_PROBE', 'NATIVE_HOME_NATIVE_READ_PROBE', 'NATIVE_HOME_POINTER_PROOF', 'NATIVE_HOME_POINTER_UI_PROOF', 'NATIVE_HOME_NAVIGATION_PROOF', 'NATIVE_HOME_DESCRIPTION_LEVEL_PROOF', 'NATIVE_HOME_RANGE_LAYOUT_PROOF', 'NATIVE_HOME_DESCRIPTION_UI_PROOF', 'NATIVE_HOME_TEXT_INPUT_PROOF'))):
    raise ValueError('Interactive package must not include scripted proof actions')
shutil.copy2(binary, stage / main)
(stage / main).chmod(0o755)
staged_binary_sha256 = hashlib.sha256((stage / main).read_bytes()).hexdigest()
if (staged_binary_sha256 != build_receipt['binary']['sha256'] or
        (stage / main).stat().st_size != build_receipt['binary']['bytes']):
    raise RuntimeError('Staged binary differs from verified build receipt')
for name in ('appfont.ttf', 'appfont-bold.ttf', 'OFL.txt'):
    shutil.copy2(repo / 'resources/fonts' / name, stage / name)
    (stage / name).chmod(0o644)
# Independent Luma Home branding; no PlxNative mark is used as this client's identity.
icon = Path(os.environ.get('NATIVE_HOME_PREVIEW_ICON', repo / 'assets/luma-home/icon-160.png')).resolve()
shutil.copy2(icon, stage / 'icon.png')
(stage / 'icon.png').chmod(0o644)
info = {'id': app_id, 'version': version, 'vendor': 'Native Home development',
        'type': 'native', 'main': main, 'title': 'Native Launch Target' if main == 'launch-target' else 'Luma Home Test' if test_identity else 'Luma Home Preview',
        'appDescription': 'Native dashboard preview; verified pointer-speed control and LG-managed settings',
        'icon': 'icon.png', 'largeIcon': 'icon.png', 'transparent': False,
        'nativeLifeCycleInterfaceVersion': 2, 'handlesRelaunch': False,
        'requiredMemory': 96, 'requiredPermissions': []}
if artwork_target_fixture:
    info['title']='AAA Luma Artwork Fixture'
(stage / 'appinfo.json').write_text(json.dumps(info, indent=2) + '\n')
if main == 'home-preview':
    assets = Path(os.environ['NATIVE_HOME_SNAPSHOT_ASSETS']).resolve()
    if startup_capability_proof:
        marker_path = stage / 'startup-capability-proof.json'
        marker_path.write_text('{"schema":1}\n')
        marker_path.chmod(0o644)
    if icon_presentation_marker is not None:
        marker_path = stage / 'icon-presentation-proof.json'
        marker_path.write_text(json.dumps(icon_presentation_marker,separators=(',',':'))+'\n')
        marker_path.chmod(0o644)
    if os.environ.get('NATIVE_HOME_PERFORMANCE_PROOF') == '1':
        if not test_identity or not interactive: raise ValueError('Performance proof requires isolated interactive identity')
        (stage / 'performance-proof.json').write_text('{}\n')
    if keyboard_observer:
        source_revision=os.environ.get('NATIVE_HOME_OBSERVER_SOURCE_REVISION') or subprocess.check_output(['git','-C',str(repo),'rev-parse','HEAD'],text=True).strip()
        if len(source_revision)!=40 or any(c not in '0123456789abcdef' for c in source_revision): raise ValueError('Observer source declaration must be a full lowercase Git revision')
        marker={'schema':1,'source_revision':source_revision,'binary_sha256':staged_binary_sha256}
        (stage / 'keyboard-observer.json').write_text(json.dumps(marker)+'\n')
        if keyboard_recipe: (stage / 'keyboard-recipe.json').write_text('{}\n')
    if interactive:
        (stage / 'interactive-session.json').write_text('{}\n')
        if os.environ.get('NATIVE_HOME_SESSION_CAPTURE_PROOF') == '1':
            (stage / 'session-proof.json').write_text('{}\n')
    if os.environ.get('NATIVE_HOME_CONTROLS_PROBE') == '1':
        if not test_identity: raise ValueError('Controls proof requires separate test identity')
        (stage / 'controls-proof.json').write_text('{}\n')
    if os.environ.get('NATIVE_HOME_MANAGEMENT_PROBE') == '1':
        if not test_identity: raise ValueError('Management proof requires separate test identity')
        (stage / 'management-proof.json').write_text('{}\n')
    if os.environ.get('NATIVE_HOME_DRAWER_PROBE') == '1':
        (stage / 'drawer-proof.json').write_text('{}\n')
    snapshot = json.loads((assets / 'snapshot.json').read_text())
    shutil.copy2(assets / 'snapshot.json', stage / 'snapshot.json')
    (stage / 'snapshot.json').chmod(0o644)
    if os.environ.get('NATIVE_HOME_LAUNCH_PROOF') == '1':
        (stage / 'launch-proof.json').write_text(json.dumps({'kind':'Application','id':'com.ash.nativehome.targetproof'})+'\n')
    route = os.environ.get('NATIVE_HOME_ROUTE_PROOF')
    if route:
        sections = {'settings-sound':'sound','settings-network':'network','settings-accessibility':'accessibility','settings-general':'general','settings-panel-care':'panel_care'}
        if route == 'home': proof = {'kind':'LgHome'}
        elif route == 'settings': proof = {'kind':'LgSettings'}
        elif route in ('hdmi1','hdmi2','hdmi3','hdmi4'): proof = {'kind':'Input','id':'HDMI_'+route[-1]}
        elif route in sections: proof = {'kind':'LgSettingsSection','section':sections[route]}
        else: raise ValueError('Unknown proof route')
        (stage / 'launch-proof.json').write_text(json.dumps(proof)+'\n')
    if os.environ.get('NATIVE_HOME_PREFERENCES_PROBE') == '1':
        shutil.copy2(assets / 'preferences-hold-probe.json', stage / 'preferences-hold-probe.json')
    if os.environ.get('NATIVE_HOME_NAVIGATION_PROOF') == '1':
        if not test_identity or interactive: raise ValueError('Navigation proof requires isolated diagnostic identity')
        (stage / 'navigation-proof.json').write_text('{}\n')
    if os.environ.get('NATIVE_HOME_POINTER_UI_PROOF') == '1':
        if not test_identity or interactive: raise ValueError('Pointer UI proof requires isolated diagnostic identity')
        (stage / 'pointer-ui-proof.json').write_text('{}\n')
    if os.environ.get('NATIVE_HOME_TEXT_INPUT_PROOF') == '1':
        if not test_identity or interactive: raise ValueError('Text proof requires isolated diagnostic identity')
        if any(os.environ.get(k) for k in ('NATIVE_HOME_DESCRIPTION_UI_PROOF', 'NATIVE_HOME_DESCRIPTION_LEVEL_PROOF', 'NATIVE_HOME_RANGE_LAYOUT_PROOF', 'NATIVE_HOME_POINTER_PROOF', 'NATIVE_HOME_POINTER_UI_PROOF', 'NATIVE_HOME_ROUTE_PROOF', 'NATIVE_HOME_LAUNCH_PROOF', 'NATIVE_HOME_PREFERENCES_PROBE', 'NATIVE_HOME_MANAGEMENT_PROBE', 'NATIVE_HOME_CONTROLS_PROBE', 'NATIVE_HOME_DRAWER_PROBE', 'NATIVE_HOME_NAVIGATION_PROOF')): raise ValueError('One isolated trial per package')
        (stage / 'text-input-proof.json').write_text('{}\n')
        if os.environ.get('NATIVE_HOME_KEYBOARD_FLOW_PROOF') == '1':
            if os.environ.get('NATIVE_HOME_MANUAL_TEXT_PROOF'): raise ValueError('One text mode per package')
            (stage / 'keyboard-flow-proof.json').write_text('{}\n')
        if os.environ.get('NATIVE_HOME_MANUAL_TEXT_PROOF') == '1':
            (stage / 'manual-text-proof.json').write_text('{}\n')
    if os.environ.get('NATIVE_HOME_DESCRIPTION_UI_PROOF') == '1':
        if not test_identity or interactive: raise ValueError('Numeric UI proof requires isolated diagnostic identity')
        if any(os.environ.get(k) for k in ('NATIVE_HOME_DESCRIPTION_LEVEL_PROOF', 'NATIVE_HOME_RANGE_LAYOUT_PROOF', 'NATIVE_HOME_POINTER_PROOF', 'NATIVE_HOME_POINTER_UI_PROOF', 'NATIVE_HOME_ROUTE_PROOF', 'NATIVE_HOME_LAUNCH_PROOF', 'NATIVE_HOME_PREFERENCES_PROBE', 'NATIVE_HOME_MANAGEMENT_PROBE', 'NATIVE_HOME_CONTROLS_PROBE', 'NATIVE_HOME_DRAWER_PROBE', 'NATIVE_HOME_NAVIGATION_PROOF')): raise ValueError('One isolated trial per package')
        (stage / 'description-ui-proof.json').write_text('{}\n')
    if os.environ.get('NATIVE_HOME_RANGE_LAYOUT_PROOF') == '1':
        if not test_identity or interactive: raise ValueError('Range layout proof requires isolated diagnostic identity')
        if any(os.environ.get(k) for k in ('NATIVE_HOME_DESCRIPTION_LEVEL_PROOF', 'NATIVE_HOME_POINTER_PROOF', 'NATIVE_HOME_POINTER_UI_PROOF', 'NATIVE_HOME_ROUTE_PROOF', 'NATIVE_HOME_LAUNCH_PROOF', 'NATIVE_HOME_PREFERENCES_PROBE', 'NATIVE_HOME_MANAGEMENT_PROBE', 'NATIVE_HOME_CONTROLS_PROBE', 'NATIVE_HOME_DRAWER_PROBE', 'NATIVE_HOME_NAVIGATION_PROOF')): raise ValueError('One isolated trial per package')
        (stage / 'range-layout-proof.json').write_text('{}\n')
    if os.environ.get('NATIVE_HOME_DESCRIPTION_LEVEL_PROOF') == '1':
        if not test_identity or interactive: raise ValueError('Numeric proof requires isolated diagnostic identity')
        if any(os.environ.get(k) for k in ('NATIVE_HOME_POINTER_PROOF', 'NATIVE_HOME_POINTER_UI_PROOF', 'NATIVE_HOME_ROUTE_PROOF', 'NATIVE_HOME_LAUNCH_PROOF', 'NATIVE_HOME_PREFERENCES_PROBE', 'NATIVE_HOME_MANAGEMENT_PROBE', 'NATIVE_HOME_CONTROLS_PROBE', 'NATIVE_HOME_DRAWER_PROBE', 'NATIVE_HOME_NAVIGATION_PROOF')): raise ValueError('One isolated trial per package')
        (stage / 'description-level-proof.json').write_text('{}\n')
    if os.environ.get('NATIVE_HOME_POINTER_PROOF') == '1':
        if not test_identity or interactive: raise ValueError('Pointer proof requires isolated diagnostic identity')
        (stage / 'pointer-proof.json').write_text('{}\n')
    if os.environ.get('NATIVE_HOME_NATIVE_READ_PROBE') == '1':
        (stage / 'native-read-probe.json').write_text('{}\n')
    if broker_marker is not None:
        (stage / 'artwork-broker-proof.json').write_text(json.dumps(broker_marker, separators=(',', ':'))+'\n')
    if broker_artwork_read_proof:
        (stage / 'artwork-read-proof.json').write_text('{}\n')
    if live_artwork_marker is not None:
        marker_path=stage/'live-artwork-proof.json'
        marker_path.write_text(json.dumps(live_artwork_marker,separators=(',',':'))+'\n')
        marker_path.chmod(0o644)
    if sound_read_proof:
        (stage / 'sound-read-proof.json').write_text('{}\n')
    for app in snapshot['applications']:
        icon_path = app.get('icon')
        if not icon_path: continue
        relative = Path(icon_path)
        if relative.is_absolute() or len(relative.parts) != 2 or relative.parts[0] != 'icons' or '..' in relative.parts:
            raise ValueError('Unexpected icon path')
        source = assets / relative
        if source.is_symlink() or source.stat().st_size > 1024 * 1024:
            raise ValueError('Unsafe icon asset')
        (stage / 'icons').mkdir(exist_ok=True)
        shutil.copy2(source, stage / relative)
        (stage / relative).chmod(0o644)

if validate_build_receipt(here, binary, build_receipt_path) != build_receipt:
    raise RuntimeError('Build receipt changed during packaging')
control = ('Package: ' + app_id + '\nVersion: ' + version + '\nSection: misc\nPriority: optional\n'
           'Architecture: arm\nMaintainer: Native Home development\n'
           'Description: Isolated native interface preview\n').encode()
with tarfile.open(output / 'control.tar.gz', 'w:gz', format=tarfile.USTAR_FORMAT) as tar:
    entry = tarfile.TarInfo('./control'); entry.size = len(control); entry.mode = 0o644
    entry.uid = entry.gid = 0; entry.mtime = 1700000000
    tar.addfile(entry, io.BytesIO(control))
with tarfile.open(output / 'data.tar.gz', 'w:gz', format=tarfile.USTAR_FORMAT) as tar:
    for path in sorted((output / 'preview-package').rglob('*')):
        entry = tar.gettarinfo(str(path), arcname='./' + str(path.relative_to(output / 'preview-package')))
        entry.uid = entry.gid = 0; entry.uname = entry.gname = 'root'; entry.mtime = 1700000000
        if path.is_file():
            with path.open('rb') as stream: tar.addfile(entry, stream)
        else: tar.addfile(entry)
(output / 'debian-binary').write_text('2.0\n')
ipk = output / (app_id + '_' + version + '_arm.ipk')
ipk.unlink(missing_ok=True)
ar = sdk / 'bin/arm-webos-linux-gnueabi-ar'
subprocess.run([str(ar), 'rc', str(ipk), 'debian-binary', 'control.tar.gz', 'data.tar.gz'], cwd=output, check=True)
members = subprocess.check_output([str(ar), 't', str(ipk)], text=True).splitlines()
if members != ['debian-binary', 'control.tar.gz', 'data.tar.gz']: raise RuntimeError('Invalid IPK members')
receipt = {'app_id': app_id, 'ipk': str(ipk), 'sha256': hashlib.sha256(ipk.read_bytes()).hexdigest(),
           'build_receipt_sha256': build_receipt_sha256,
           'build_source_revision': build_receipt['git_revision'],
           'framework': build_receipt['snapshot']['framework'],
           'binary_sha256': staged_binary_sha256, 'bytes': ipk.stat().st_size,
           'performance_proof': os.environ.get('NATIVE_HOME_PERFORMANCE_PROOF') == '1',
           'controls_probe': os.environ.get('NATIVE_HOME_CONTROLS_PROBE') == '1',
           'management_probe': os.environ.get('NATIVE_HOME_MANAGEMENT_PROBE') == '1',
           'preference_hold_probe': os.environ.get('NATIVE_HOME_PREFERENCES_PROBE') == '1',
           'native_read_probe': os.environ.get('NATIVE_HOME_NATIVE_READ_PROBE') == '1',
           'broker_identify_proof': broker_identify_proof,
           'broker_artwork_read_proof': broker_artwork_read_proof,
           'live_artwork_proof': live_artwork_proof,
           'icon_presentation_proof': icon_presentation_proof,
           'startup_capability_proof': startup_capability_proof,
           'artwork_target_fixture': artwork_target_fixture or None,
           'native_sound_read_proof': sound_read_proof,
           'keyboard_observer_proof': keyboard_observer,
           'keyboard_recipe_proof': keyboard_recipe,
           'keyboard_flow_proof': os.environ.get('NATIVE_HOME_KEYBOARD_FLOW_PROOF') == '1',
           'text_input_proof': os.environ.get('NATIVE_HOME_TEXT_INPUT_PROOF') == '1',
           'description_ui_proof': os.environ.get('NATIVE_HOME_DESCRIPTION_UI_PROOF') == '1',
           'range_layout_proof': os.environ.get('NATIVE_HOME_RANGE_LAYOUT_PROOF') == '1',
           'description_level_proof': os.environ.get('NATIVE_HOME_DESCRIPTION_LEVEL_PROOF') == '1',
           'pointer_proof': os.environ.get('NATIVE_HOME_POINTER_PROOF') == '1',
           'pointer_ui_proof': os.environ.get('NATIVE_HOME_POINTER_UI_PROOF') == '1',
           'navigation_proof': os.environ.get('NATIVE_HOME_NAVIGATION_PROOF') == '1',
           'launch_proof': os.environ.get('NATIVE_HOME_LAUNCH_PROOF') == '1',
           'route_proof': os.environ.get('NATIVE_HOME_ROUTE_PROOF'),
           'interactive_session': interactive,
           'test_identity': test_identity,
           'session_capture_proof': interactive and os.environ.get('NATIVE_HOME_SESSION_CAPTURE_PROOF') == '1',
           'permissions': [], 'services': [], 'boot_hooks': [], 'system_files': []}
(output / 'preview-package-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps(receipt))
