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
    paths = {'luma-surface-scene-proof', 'plx_base', 'plx_gfx', 'plx_machine'}
    if sum(package['name']=='luma-surface-scene-proof' and not package.get('source') for package in actual)!=1:
        raise RuntimeError('Standalone lock must name the ordinary surface root exactly once')
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
    objects = []
    object_sources = [(framework / 'src/svg.c', 'svg.o'),
                      (framework / 'src/compat/getauxval.c', 'getauxval.o'),
                      (HERE / 'sdl-events.c', 'sdl-events.o')]
    for source, name in object_sources:
        target = output / name
        subprocess.run([str(cc), '--sysroot=' + str(sysroot), '-c', str(source),
                        '-I' + str(sysroot / 'usr/include/SDL2'),
                        '-I' + str(framework / 'src'),
                        '-I' + str(framework / 'vendor/nanosvg'),
                        '-DNATIVE_HOME_WEBOS=1', '-fPIC', '-O2', '-o', str(target)], check=True)
        objects.append(target)
    objects_before = {path.name: observe(path) for path in objects}
    env = os.environ.copy()
    env.update(CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
               CARGO_TARGET_DIR=str(output / 'cargo'),
               CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABI_LINKER=str(cc),
               SURFACE_PROBE_GLUE_SHA256=manifest['files']['src/main.rs']['sha256'])
    flags = ['-C target-cpu=cortex-a9', '-C target-feature=-neon',
             '-C link-arg=--sysroot=' + str(sysroot)]
    flags += ['-C link-arg=' + str(path) for path in objects]
    flags += ['-C link-arg=-l' + name for name in ('SDL2', 'SDL2_ttf', 'GLESv2', 'EGL', 'm')]
    flags += ['-C link-arg=-Wl,-rpath-link,' + str(sysroot / name) for name in ('usr/lib', 'lib')]
    env['RUSTFLAGS'] = ' '.join(flags)
    cargo = env.get('NATIVE_HOME_CARGO', 'cargo')
    toolchain = env.get('NATIVE_HOME_RUST_TOOLCHAIN', 'nightly-2026-07-02')
    command = [cargo, '+' + toolchain, 'build', '--offline', '--locked', '--manifest-path',
               str(HERE / 'Cargo.toml'), '--release', '--target', 'arm-unknown-linux-gnueabi',
               '-Z', 'build-std=std,panic_abort', '--bin', 'surface-scene-proof']
    subprocess.run(command, cwd=HERE, env=env, check=True)
    subprocess.run(['python3', str(PROJECT / 'tools/verify-framework.py'), str(PROJECT)], check=True)
    after = inputs(manifest)
    compiler_after = dict(observe(cc.resolve(strict=True)), path=str(cc), realpath=str(cc.resolve(strict=True)))
    if compiler_after != compiler_before:
        raise RuntimeError('SDK compiler changed during build')
    objects_after = {path.name: observe(path) for path in objects}
    if before != after or observe(HERE / 'Cargo.lock') != lock or objects_after != objects_before:
        raise RuntimeError('Build input changed during compile')
    binary = output / 'cargo/arm-unknown-linux-gnueabi/release/surface-scene-proof'
    save(output / 'build-result.json', {
        'schema': 1, 'source_manifest_sha256': args.manifest_sha256,
        'source_inputs': before, 'private_lock': lock, 'toolchain': toolchain,
        'sdk_compiler': compiler_before, 'command': command, 'rustflags': flags,
        'objects': objects_before, 'objects_unchanged_after_compile': True,
        'binary': dict(observe(binary), path=str(binary)),
        'inherited_override_policy': 'presence_rejected_even_empty',
        'forbidden_override_keys': list(FORBIDDEN_OVERRIDES),
        'inherited_override_keys_present': [],
        'build_exit_code': 0, 'packaged': False, 'device_contact': False,
        'native_scene_qualified': False,
        'scope': 'Local standalone build only; SDK/ELF/package/device admission remains separate',
    })
    print('Local standalone build completed; no package or device operation')

if __name__ == '__main__':
    main()
