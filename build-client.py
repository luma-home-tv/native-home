"""Build the separate Home client, never the Plex package or its deployment targets."""
import argparse
import os
from pathlib import Path
import subprocess
from build_provenance import snapshot, record_build_receipt

parser = argparse.ArgumentParser()
parser.add_argument('target', choices=('host', 'arm', 'test'))
parser.add_argument('--bin', choices=('control-preview', 'home-preview', 'launch-target', 'artwork-broker-proof', 'artwork-broker-startup-proof'), default='control-preview')
args = parser.parse_args()
here = Path(__file__).resolve().parent
repo = here
framework = here / ".dependencies/plx-native"
env = os.environ.copy()
env['CARGO_INCREMENTAL'] = '0'
cargo = env.get('NATIVE_HOME_CARGO', 'cargo')
toolchain = env.get('NATIVE_HOME_RUST_TOOLCHAIN', 'nightly-2026-07-02')
common = [cargo, '+' + toolchain]
manifest = ['--locked', '--manifest-path', str(here / 'Cargo.toml')]
# Check before even invoking the C compiler or Cargo. The root build script also
# checks direct Cargo invocations, but cannot precede dependency build scripts.
before = snapshot(here)
if args.target == 'test':
    command = common + ['test'] + manifest + ['--lib']
elif args.target == 'host':
    command = common + ['build'] + manifest + ['--features', 'hostsim', '--bin', args.bin]
else:
    sdk = Path(env['NATIVE_HOME_WEBOS_SDK']).resolve()
    output = Path(env['NATIVE_HOME_BUILD_DIR']).resolve()
    output.mkdir(parents=True, exist_ok=True)
    sysroot = sdk / 'arm-webos-linux-gnueabi/sysroot'
    cc = sdk / 'bin/arm-webos-linux-gnueabi-gcc'
    objects = []
    for source, name in [(framework / 'src/svg.c', 'svg-arm.o'), (framework / 'src/compat/getauxval.c', 'getauxval-arm.o'), (here / 'sdl-events.c', 'sdl-events-arm.o')]:
        obj = output / name
        subprocess.run([str(cc), '--sysroot=' + str(sysroot), '-c', str(source),
                        '-I' + str(sysroot / 'usr/include/SDL2'), '-I' + str(framework / 'src'), '-I' + str(framework / 'vendor/nanosvg'),
                        '-DNATIVE_HOME_WEBOS=1', '-fPIC', '-O2', '-o', str(obj)], check=True)
        objects.append(obj)
    env['CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABI_LINKER'] = str(cc)
    flags = ['-C target-cpu=cortex-a9', '-C target-feature=-neon',
             '-C link-arg=--sysroot=' + str(sysroot)]
    flags += ['-C link-arg=' + str(obj) for obj in objects]
    flags += ['-C link-arg=-l' + lib for lib in ('SDL2', 'SDL2_ttf', 'GLESv2', 'EGL', 'm')]
    flags += ['-C link-arg=-Wl,-rpath-link,' + str(sysroot / part) for part in ('usr/lib', 'lib')]
    env['RUSTFLAGS'] = ' '.join(flags)
    command = common + ['build'] + manifest + ['--release', '--target', 'arm-unknown-linux-gnueabi',
                 '-Z', 'build-std=std,panic_abort', '--bin', args.bin]
result = subprocess.run(command, cwd=repo, env=env)
if result.returncode:
    raise SystemExit(result.returncode)
after = snapshot(here)
if before != after:
    raise RuntimeError('Build inputs/framework changed during build')
if args.target != 'test':
    target_dir = Path(env.get('CARGO_TARGET_DIR', here / 'target'))
    if not target_dir.is_absolute():
        target_dir = here / target_dir
    binary = (target_dir / 'debug' / args.bin if args.target == 'host' else
              target_dir / 'arm-unknown-linux-gnueabi/release' / args.bin)
    print('Build receipt:', record_build_receipt(here, binary, args.target, before, after))
