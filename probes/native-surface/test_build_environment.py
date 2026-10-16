"""Pure host fixtures; no compile, package or device operation."""
import importlib.util
import os
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('surface_build_recipe', Path(__file__).with_name('build-arm.py'))
BUILD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILD)

class BuildEnvironmentTests(unittest.TestCase):
    def test_present_empty_or_nonempty_override_refuses_before_subprocess(self):
        for name in BUILD.FORBIDDEN_OVERRIDES:
            for value in ('', 'synthetic-override'):
                with self.subTest(name=name, value=value), patch.dict(os.environ, {name: value}, clear=True), \
                     patch.object(sys, 'argv', ['build-arm.py', '--manifest-sha256', 'a'*64,
                                              '--lock-sha256', 'b'*64, '--output', '/unused-output']), \
                     patch.object(BUILD.subprocess, 'run') as process:
                    with self.assertRaisesRegex(RuntimeError, 'Inherited compiler/encoded/wrapper'):
                        BUILD.main()
                    process.assert_not_called()
        BUILD.reject_inherited_overrides({'NATIVE_HOME_CARGO': 'fixed-cargo', 'NATIVE_HOME_WEBOS_SDK': 'fixed-sdk'})

if __name__ == '__main__':
    unittest.main()
