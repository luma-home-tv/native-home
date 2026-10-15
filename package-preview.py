"""Package only the isolated native preview; no services, hooks or system overlays."""
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile

here = Path(__file__).resolve().parent
repo = here
output = Path(os.environ['NATIVE_HOME_BUILD_DIR']).resolve()
sdk = Path(os.environ['NATIVE_HOME_WEBOS_SDK']).resolve()
binary = Path(os.environ['NATIVE_HOME_ARM_BINARY']).resolve()
main = binary.name
app_id = 'com.ash.nativehome.targetproof' if main == 'launch-target' else 'com.ash.nativehome.preview'
test_identity = os.environ.get('NATIVE_HOME_TEST_IDENTITY') == '1'
if test_identity:
    if main != 'home-preview': raise ValueError('Test identity is for the dashboard only')
    app_id = 'com.ash.nativehome.proof'
stage = output / 'preview-package/usr/palm/applications' / app_id
stage.mkdir(parents=True, exist_ok=False)
if main not in ('control-preview', 'home-preview', 'launch-target'): raise ValueError('Unexpected preview binary')
version = '0.21.0' if main == 'home-preview' else '0.1.0'
interactive = os.environ.get('NATIVE_HOME_INTERACTIVE_SESSION') == '1'
if interactive and (main != 'home-preview' or any(os.environ.get(name) for name in (
    'NATIVE_HOME_DRAWER_PROBE', 'NATIVE_HOME_MANAGEMENT_PROBE', 'NATIVE_HOME_CONTROLS_PROBE', 'NATIVE_HOME_LAUNCH_PROOF', 'NATIVE_HOME_ROUTE_PROOF',
    'NATIVE_HOME_PREFERENCES_PROBE', 'NATIVE_HOME_NATIVE_READ_PROBE', 'NATIVE_HOME_POINTER_PROOF', 'NATIVE_HOME_POINTER_UI_PROOF', 'NATIVE_HOME_NAVIGATION_PROOF', 'NATIVE_HOME_DESCRIPTION_LEVEL_PROOF', 'NATIVE_HOME_RANGE_LAYOUT_PROOF', 'NATIVE_HOME_DESCRIPTION_UI_PROOF'))):
    raise ValueError('Interactive package must not include scripted proof actions')
shutil.copy2(binary, stage / main)
(stage / main).chmod(0o755)
for name in ('appfont.ttf', 'appfont-bold.ttf', 'OFL.txt'):
    shutil.copy2(repo / 'resources/fonts' / name, stage / name)
# Independent Luma Home branding; no PlxNative mark is used as this client's identity.
icon = Path(os.environ.get('NATIVE_HOME_PREVIEW_ICON', repo / 'assets/luma-home/icon-160.png')).resolve()
shutil.copy2(icon, stage / 'icon.png')
info = {'id': app_id, 'version': version, 'vendor': 'Native Home development',
        'type': 'native', 'main': main, 'title': 'Native Launch Target' if main == 'launch-target' else 'Luma Home Test' if test_identity else 'Luma Home Preview',
        'appDescription': 'Native dashboard preview; verified pointer-speed control and LG-managed settings',
        'icon': 'icon.png', 'largeIcon': 'icon.png', 'transparent': False,
        'nativeLifeCycleInterfaceVersion': 2, 'handlesRelaunch': False,
        'requiredMemory': 96, 'requiredPermissions': []}
(stage / 'appinfo.json').write_text(json.dumps(info, indent=2) + '\n')
if main == 'home-preview':
    assets = Path(os.environ['NATIVE_HOME_SNAPSHOT_ASSETS']).resolve()
    if os.environ.get('NATIVE_HOME_PERFORMANCE_PROOF') == '1':
        if not test_identity or not interactive: raise ValueError('Performance proof requires isolated interactive identity')
        (stage / 'performance-proof.json').write_text('{}\n')
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
           'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'bytes': ipk.stat().st_size,
           'performance_proof': os.environ.get('NATIVE_HOME_PERFORMANCE_PROOF') == '1',
           'controls_probe': os.environ.get('NATIVE_HOME_CONTROLS_PROBE') == '1',
           'management_probe': os.environ.get('NATIVE_HOME_MANAGEMENT_PROBE') == '1',
           'preference_hold_probe': os.environ.get('NATIVE_HOME_PREFERENCES_PROBE') == '1',
           'native_read_probe': os.environ.get('NATIVE_HOME_NATIVE_READ_PROBE') == '1',
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
