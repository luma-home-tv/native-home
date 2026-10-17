"""Real build/package entrypoints in temporary fixtures; no compiler or TV access.

The framework checker and tool executables are explicit fixture substitutes.
Only refusal paths run; staged-copy substitution is injected in the child
process, without modifying the actual package entrypoint under test.
"""
from cache_consumer_contract import native_selection
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('entrypoint_provenance', REPO / 'build_provenance.py')
PROVENANCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROVENANCE)


class EntrypointTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name).resolve()
        self.root = self.base / 'repo'
        self.root.mkdir()
        for name in PROVENANCE.ROOT_FILES:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            source = REPO / name
            if source.is_file():
                shutil.copyfile(source, path)
            else:
                path.write_text('fixture input\n')
        for name in PROVENANCE.TREES:
            directory = self.root / name
            directory.mkdir(parents=True, exist_ok=True)
            (directory / 'fixture.txt').write_text('fixture\n')
        for name in ('appfont.ttf', 'appfont-bold.ttf', 'OFL.txt'):
            (self.root / 'resources/fonts' / name).write_bytes(b'fixture font')
        (self.root / 'assets/luma-home/icon-160.png').write_bytes(b'fixture icon')
        (self.root / 'patches').mkdir()
        (self.root / 'patches/fixture.patch').write_text('fixture patch\n')
        (self.root / 'dependency-lock.json').write_text(json.dumps({
            'integration_patch': 'patches/fixture.patch'
        }))
        (self.root / 'tools/verify-framework.py').write_text(
            'import json,pathlib,sys\n'
            'if (pathlib.Path(sys.argv[1])/"framework-drift").exists():\n'
            ' sys.stderr.write("fixture framework drift refused\\n");sys.exit(1)\n'
            'print(json.dumps({"schema":1,"status":"verified","fixture":True}))\n')
        self.output = self.base / 'package'
        self.output.mkdir()
        self.assets = self.base / 'snapshot-assets'
        self.assets.mkdir()
        (self.assets / 'snapshot.json').write_text('{"applications":[]}')
        self.sdk = self.base / 'sdk'
        (self.sdk / 'bin').mkdir(parents=True)
        self.compiler = self.fake_tool(self.sdk / 'bin/arm-webos-linux-gnueabi-gcc', 'compiler')
        self.ar = self.fake_tool(self.sdk / 'bin/arm-webos-linux-gnueabi-ar', 'ar')
        self.cargo = self.fake_tool(self.base / 'cargo', 'cargo')
        self.binary = self.base / 'home-preview'
        elf = bytearray(52)
        elf[:7] = b'\x7fELF\x01\x01\x01'
        elf[18:20] = (40).to_bytes(2, 'little')
        self.binary.write_bytes(elf)
        self.receipt = self.binary.with_name('home-preview.build.json')
        self.env = {key: value for key, value in os.environ.items()
                    if not key.startswith(('NATIVE_HOME_', 'PYTHON', 'GIT_'))}
        self.env.update(
            PYTHONDONTWRITEBYTECODE='1',
            NATIVE_HOME_CARGO=str(self.cargo),
            NATIVE_HOME_WEBOS_SDK=str(self.sdk),
            NATIVE_HOME_BUILD_DIR=str(self.output),
            NATIVE_HOME_ARM_BINARY=str(self.binary),
            NATIVE_HOME_SNAPSHOT_ASSETS=str(self.assets))

    def fake_tool(self, path, role):
        marker = self.base / (role + '-executed')
        path.write_text('#!' + sys.executable + '\nfrom pathlib import Path\n'
                        + 'Path(' + repr(str(marker)) + ').write_text("unexpected tool execution")\n')
        path.chmod(0o755)
        return path

    def record(self):
        before = PROVENANCE.snapshot(self.root)
        PROVENANCE.record_build_receipt(self.root, self.binary, 'arm', before,
                                        PROVENANCE.snapshot(self.root), build_selection=native_selection((self.root / 'Cargo.toml').read_bytes()))

    def run_entrypoint(self, script, *args, env=None):
        return subprocess.run([sys.executable, str(self.root / script), *args],
                              cwd=self.root, env=self.env if env is None else env,
                              capture_output=True, text=True, timeout=20)

    def assert_no_tool_or_package(self):
        self.assertFalse(any(self.base.glob('*-executed')))
        self.assertFalse(any(self.output.glob('*.ipk')))
        self.assertFalse((self.output / 'control.tar.gz').exists())

    def test_framework_refusal_precedes_fake_cargo_and_c_compiler_for_all_targets(self):
        (self.root / 'framework-drift').write_text('refuse')
        for target in ('host', 'arm', 'test'):
            with self.subTest(target=target):
                result = self.run_entrypoint('build-client.py', target, '--bin', 'home-preview')
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('CalledProcessError', result.stderr)
                self.assert_no_tool_or_package()
                self.assertFalse(self.receipt.exists())

    def test_fixed_preview_helper_build_choice_does_not_bypass_framework_admission(self):
        (self.root / 'framework-drift').write_text('refuse')
        result = self.run_entrypoint('build-client.py', 'arm', '--bin', 'artwork-broker')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('CalledProcessError', result.stderr)
        self.assertNotIn('invalid choice', result.stderr)
        self.assert_no_tool_or_package()

    def test_installed_helpers_can_never_be_packaged_as_the_ui_executable(self):
        raw = self.binary.read_bytes()
        for name in ('artwork-broker-startup-proof', 'artwork-broker'):
            with self.subTest(name=name):
                self.binary = self.base / name
                self.binary.write_bytes(raw)
                self.receipt = self.binary.with_name(name + '.build.json')
                self.env['NATIVE_HOME_ARM_BINARY'] = str(self.binary)
                self.record()
                result = self.run_entrypoint('package-preview.py')
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('Unexpected preview binary', result.stderr)
                self.assertFalse((self.output / 'preview-package').exists())
                self.assert_no_tool_or_package()

    def test_missing_or_tampered_receipt_refuses_before_staging(self):
        result = self.run_entrypoint('package-preview.py')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.output / 'preview-package').exists())
        self.record()
        receipt = json.loads(self.receipt.read_text())
        receipt['binary']['sha256'] = '0' * 64
        self.receipt.write_text(json.dumps(receipt))
        result = self.run_entrypoint('package-preview.py')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Binary bytes/name differ', result.stderr)
        self.assertFalse((self.output / 'preview-package').exists())
        self.assert_no_tool_or_package()

    def test_changed_binary_or_source_refuses_before_staging(self):
        self.record()
        original = self.binary.read_bytes()
        self.binary.write_bytes(original + b'changed')
        result = self.run_entrypoint('package-preview.py')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Binary bytes/name differ', result.stderr)
        self.binary.write_bytes(original)
        (self.root / 'src/fixture.txt').write_text('changed source')
        result = self.run_entrypoint('package-preview.py')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Current source/dependency inputs differ', result.stderr)
        self.assertFalse((self.output / 'preview-package').exists())
        self.assert_no_tool_or_package()

    def test_staged_binary_substitution_refuses_before_tar_or_ar(self):
        self.record()
        injection = self.base / 'child-fixture-injection'
        injection.mkdir()
        marker = self.base / 'staged-substitution-observed'
        (injection / 'sitecustomize.py').write_text(
            'import shutil\nfrom pathlib import Path\noriginal=shutil.copy2\n'
            'def substitute(source,destination,*args,**kwargs):\n'
            ' result=original(source,destination,*args,**kwargs)\n'
            ' if Path(source).name=="home-preview":\n'
            '  Path(destination).write_bytes(Path(destination).read_bytes()+b"substitution")\n'
            '  Path(' + repr(str(marker)) + ').write_text("injected after preflight")\n'
            ' return result\nshutil.copy2=substitute\n')
        result = self.run_entrypoint('package-preview.py', env={**self.env, 'PYTHONPATH': str(injection)})
        self.assertTrue(marker.exists(), 'The actual package copy path must exercise substitution')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Staged binary differs from verified build receipt', result.stderr)
        self.assert_no_tool_or_package()


if __name__ == '__main__':
    unittest.main()
