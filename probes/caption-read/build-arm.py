"""Root-reviewed local build recipe only; no packaging or device operations."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tomllib

HERE = Path(__file__).resolve().parent
PROJECT = HERE.parent.parent

# Presence rejects even an empty encoded flag or wrapper, since Cargo gives
# some of these precedence over the explicitly recorded ordinary RUSTFLAGS.
FORBIDDEN_OVERRIDES = (
    'CARGO_ENCODED_RUSTFLAGS', 'RUSTFLAGS', 'RUSTC', 'RUSTC_WRAPPER',
    'RUSTC_WORKSPACE_WRAPPER', 'RUSTC_BOOTSTRAP', 'RUSTDOC', 'RUSTDOCFLAGS',
    'CARGO_BUILD_RUSTC', 'CARGO_BUILD_RUSTC_WRAPPER',
    'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_RUSTFLAGS',
    'CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABI_RUSTFLAGS',
    'CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABI_LINKER',
)

def reject_inherited_overrides(environ):
    present = sorted(key for key in FORBIDDEN_OVERRIDES if key in environ)
    if present:
        raise RuntimeError('Inherited compiler/encoded/wrapper override refused: ' + ','.join(present))

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def observe(path):
    if path.is_symlink() or not path.is_file():
        raise RuntimeError('Expected pinned regular build input: ' + str(path))
    raw = path.read_bytes()
    return {'sha256': sha(raw), 'bytes': len(raw)}

def inputs(manifest):
    result = {}
    for name, expected in sorted(manifest['files'].items()):
        value = observe(HERE / name)
        if value != expected:
            raise RuntimeError('Private source changed: ' + name)
        result['private/' + name] = value
    for name, expected in sorted(manifest['external_inputs'].items()):
        value = observe(PROJECT / name)
        if value != expected:
            raise RuntimeError('Pinned framework/build input changed: ' + name)
        result['external/' + name] = value
    return result

def validate_lock():
    original = tomllib.loads((PROJECT / 'Cargo.lock').read_text())['package']
    actual = tomllib.loads((HERE / 'Cargo.lock').read_text())['package']
    registry = {(p['name'], p['version'], p.get('source'), p.get('checksum'))
                for p in original if p.get('source')}
    paths = {'luma-caption-read-proof', 'plx_base', 'plx_platform', 'plx_machine'}
    if sum(p['name']=='luma-caption-read-proof' and not p.get('source') for p in actual)!=1:
        raise RuntimeError('Private lock must name the standalone root exactly once')
    for package in actual:
        if package.get('source'):
            if (package['name'], package['version'], package.get('source'), package.get('checksum')) not in registry:
                raise RuntimeError('Registry version/checksum differs from frozen production lock')
        elif package['name'] not in paths:
            raise RuntimeError('Unexpected application/path dependency')

def save(path, value):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, 'w') as file:
        json.dump(value, file, indent=2, sort_keys=True)
        file.write('\n')
        file.flush()
        os.fsync(file.fileno())

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--manifest-sha256', required=True)
    parser.add_argument('--lock-sha256', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    reject_inherited_overrides(os.environ)  # Before any subprocess or compiler.
    raw = (HERE / 'manifest.json').read_bytes()
    if sha(raw) != args.manifest_sha256:
        raise RuntimeError('Root-reviewed source manifest mismatch')
    manifest = json.loads(raw)
    before = inputs(manifest)
    lock = observe(HERE / 'Cargo.lock')
    validate_lock()
    if lock['sha256'] != args.lock_sha256:
        raise RuntimeError('Generate/admit the private offline lock before building')
    subprocess.run(['python3', str(PROJECT / 'tools/verify-framework.py'), str(PROJECT)], check=True)
    sdk = Path(os.environ['NATIVE_HOME_WEBOS_SDK']).resolve()
    output = args.output.resolve()
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    sysroot = sdk / 'arm-webos-linux-gnueabi/sysroot'
    cc = sdk / 'bin/arm-webos-linux-gnueabi-gcc'
    framework = PROJECT / '.dependencies/plx-native'
    compiler_before = dict(observe(cc.resolve(strict=True)), path=str(cc), realpath=str(cc.resolve(strict=True)))
    env = os.environ.copy()
    env.update(CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
               CARGO_TARGET_DIR=str(output / 'cargo'),
               CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABI_LINKER=str(cc),
               CAPTION_PROBE_SOURCE_SHA256=manifest['files']['src/main.rs']['sha256'])
    flags = ['-C target-cpu=cortex-a9', '-C target-feature=-neon',
             '-C link-arg=--sysroot=' + str(sysroot)]
    flags += ['-C link-arg=-l' + name for name in ('luna-service2', 'glib-2.0')]
    flags += ['-C link-arg=-Wl,-rpath-link,' + str(sysroot / name) for name in ('usr/lib', 'lib')]
    env['RUSTFLAGS'] = ' '.join(flags)
    cargo = env.get('NATIVE_HOME_CARGO', 'cargo')
    toolchain = env.get('NATIVE_HOME_RUST_TOOLCHAIN', 'nightly-2026-07-02')
    command = [cargo, '+' + toolchain, 'build', '--offline', '--locked', '--manifest-path',
               str(HERE / 'Cargo.toml'), '--release', '--target', 'arm-unknown-linux-gnueabi',
               '-Z', 'build-std=std,panic_abort', '--bin', 'caption-read-proof']
    subprocess.run(command, cwd=HERE, env=env, check=True)
    subprocess.run(['python3', str(PROJECT / 'tools/verify-framework.py'), str(PROJECT)], check=True)
    after = inputs(manifest)
    compiler_after = dict(observe(cc.resolve(strict=True)), path=str(cc), realpath=str(cc.resolve(strict=True)))
    if compiler_after != compiler_before:
        raise RuntimeError('SDK compiler changed during build')
    if before != after or observe(HERE / 'Cargo.lock') != lock:
        raise RuntimeError('Build input changed during compile')
    binary = output / 'cargo/arm-unknown-linux-gnueabi/release/caption-read-proof'
    save(output / 'build-result.json', {
        'schema': 1, 'source_manifest_sha256': args.manifest_sha256,
        'source_inputs': before, 'private_lock': lock, 'toolchain': toolchain,
        'sdk_compiler': compiler_before, 'command': command, 'rustflags': flags,
        'binary': dict(observe(binary), path=str(binary)),
        'inherited_override_policy': 'presence_rejected_even_empty',
        'forbidden_override_keys': list(FORBIDDEN_OVERRIDES),
        'inherited_override_keys_present': [],
        'build_exit_code': 0, 'packaged': False, 'device_contact': False,
        'native_caption_compatibility_qualified': False,
        'scope': 'Local standalone build only; Link dependencies/ELF/package/device admission remains separate',
    })
    print('Local standalone build completed; no package or device operation')

if __name__ == '__main__':
    main()
