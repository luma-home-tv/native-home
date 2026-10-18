"""Read-only metadata snapshot; icons require completed private acquisition data.
Nothing is installed, launched, set or written on the TV. Captures are private build artifacts.
"""
import argparse
import datetime
import json
from pathlib import Path
import re
import shlex
import subprocess
from icon_acquisition import catalogue, prepare, publish, PrivateDirectory, encoded, make_private_output


def app_id(value):
    return isinstance(value, str) and bool(re.fullmatch(r'[A-Za-z0-9._-]{1,180}', value))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', required=True)
    parser.add_argument('--identity', type=Path, required=True)
    parser.add_argument('--known-hosts', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--own-id', default='com.ash.nativehome.preview')
    parser.add_argument('--icon-acquisition', type=Path, help='Completed external acquisition manifest; never invokes a reader')
    args = parser.parse_args()
    if not re.fullmatch(r'[A-Za-z0-9.:-]+', args.host):
        parser.error('Invalid host')
    output = make_private_output(args.out)
    base = ['ssh', '-i', str(args.identity.resolve()), '-o', 'IdentitiesOnly=yes', '-o', 'BatchMode=yes',
            '-o', 'StrictHostKeyChecking=yes', '-o', 'UserKnownHostsFile=' + str(args.known_hosts.resolve()),
            '-o', 'ConnectTimeout=5', 'root@' + args.host]
    receipts = {}
    def luna(name, uri, payload):
        command = 'luna-send -i -n 1 -w 5000 -f luna://' + uri + ' ' + shlex.quote(json.dumps(payload))
        result = subprocess.run(base[:-1] + ['-tt', base[-1], command], capture_output=True, timeout=12, check=True)
        data = json.loads(result.stdout)
        receipts[name] = data
        return data
    luna('apps', 'com.webos.applicationManager/listApps', {})
    points = luna('launch_points', 'com.webos.applicationManager/listLaunchPoints', {}).get('launchPoints', [])
    inputs = luna('inputs', 'com.webos.service.eim/getAllInputStatus', {}).get('devices', [])
    picture = luna('picture', 'com.webos.service.settings/getSystemSettings', {'category': 'picture', 'keys': ['pictureMode', 'backlight', 'contrast', 'brightness']})
    sound = luna('audio', 'com.webos.service.audio/master/getVolume', {})
    raw_catalogue = {'replies': {'apps': receipts['apps'], 'launch_points': receipts['launch_points']}}
    admitted = catalogue(raw_catalogue, args.own_id)
    ids = {p['id'] for p in points if type(p) is dict and app_id(p.get('id')) and p['id'] != args.own_id}
    applications = [row['app'] for _, row in sorted(admitted.items())]
    dimension = picture.get('dimension', {})
    context = {'input': dimension.get('input', 'unknown'), 'dynamic_range': dimension.get('dynamicRange', 'unknown'),
               'picture_mode': dimension.get('pictureMode', 'unknown')}
    def row(identity, key, label, value, available=True, protected=False):
        return {'id': identity, 'focus_key': key, 'label': label, 'value': str(value), 'context': context,
                'protected': protected, 'available': available}
    values = picture.get('settings', {})
    settings = {'picture': [row(key, i + 1, label, values.get(key, 'Unavailable'), key in values)
                           for i, (key, label) in enumerate([('pictureMode','Picture mode'),('backlight','OLED pixel brightness'),('contrast','Contrast'),('brightness','Black level')])]}
    audio = sound.get('volumeStatus', {})
    settings['sound'] = [row(key, i + 20, label, audio.get(key, 'Unavailable'), key in audio)
                         for i, (key, label) in enumerate([('soundOutput','Sound output'),('volume','Volume'),('muteStatus','Muted')])]
    for i, (section, label) in enumerate([('network','Connection settings'),('accessibility','Accessibility settings'),('general','System settings'),('panel_care','OLED care and protections')]):
        settings[section] = [row(section, 40+i, label, 'Managed by LG', False, True)]
    captured = datetime.datetime.now(datetime.timezone.utc).isoformat(timespec='seconds')
    snapshot = {'captured_at': captured, 'applications': applications, 'launch_ids': sorted(ids),
                'inputs': [{'id': d['id'], 'label': d.get('label', d['id']), 'plugged': bool(d.get('hdmiPlugIn')),
                            'signal': bool(d.get('hdmiSignalExist')), 'active': bool(d.get('activate'))}
                           for d in inputs if re.fullmatch(r'HDMI_[1-4]', d.get('id', ''))],
                'context': context, 'settings': settings}
    if args.icon_acquisition is not None:
        snapshot, bodies, record = prepare(args.icon_acquisition, raw_catalogue, snapshot)
        publish(output, snapshot, bodies, record)
    else:
        # Same private publication path, without any icon provenance/authority claim.
        with PrivateDirectory(output) as directory:
            directory.write('snapshot.json', encoded(snapshot))
    with PrivateDirectory(output) as directory:
        directory.write('read-only-receipts.json', encoded(receipts))
    print(json.dumps({'applications': len(applications), 'icons': sum('icon' in a for a in snapshot['applications']),
                      'inputs': len(snapshot['inputs']), 'captured_at': captured, 'tv_writes': False,
                      'icon_reader_invoked': False, 'native_icon_authority': False}))

if __name__ == '__main__': main()
