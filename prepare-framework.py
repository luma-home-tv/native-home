#!/usr/bin/env python3
"""Fetch a pinned, explicitly attributed external PlxNative framework checkout.

The checkout is ignored by this project's Git tree. It is never branded as Luma
Home source. Only the five declared framework crates are application dependencies.
"""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
LOCK = json.loads((ROOT / 'dependency-lock.json').read_text())
DEST = ROOT / '.dependencies/plx-native'
PATCH = ROOT / LOCK['integration_patch']
REV = LOCK['upstream']['revision']
if hashlib.sha256(PATCH.read_bytes()).hexdigest() != LOCK['integration_patch_sha256']:
    raise SystemExit('Integration patch does not match the dependency lock')


def git(*args, check=True):
    return subprocess.run(['git', '-C', str(DEST), *args], text=True,
                          capture_output=True, check=check)


if not DEST.exists():
    DEST.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(['git', 'clone', '--filter=blob:none', '--no-checkout',
                    LOCK['upstream']['repository'], str(DEST)], check=True)
if not (DEST / '.git').exists():
    raise SystemExit('Dependency path is not a Git checkout')
# A fresh --no-checkout clone has no working files or index to preserve.
if {p.name for p in DEST.iterdir()} == {'.git'}:
    git('checkout', '--detach', REV)
status = git('status', '--porcelain').stdout.strip()
if status:
    # A prepared checkout is accepted only when it has this exact patch applied.
    prepared = git('rev-parse', 'HEAD').stdout.strip() == REV
    expected = subprocess.check_output(['git', 'apply', '--numstat', str(PATCH)], text=True, cwd=ROOT)
    observed_paths = {line.split('\t')[-1] for line in git('diff', 'HEAD', '--numstat').stdout.splitlines()}
    expected_paths = {line.split('\t')[-1] for line in expected.splitlines()}
    reverse = git('apply', '--reverse', '--check', str(PATCH), check=False)
    exact_files = all(hashlib.sha256((DEST / path).read_bytes()).hexdigest() == expected for path, expected in LOCK['patched_file_sha256'].items())
    unexpected_files = git('ls-files', '--others', '--exclude-standard').stdout.strip()
    if not prepared or observed_paths != expected_paths or reverse.returncode or not exact_files or unexpected_files:
        raise SystemExit('Dependency has unrelated changes; preserve and inspect it')
else:
    if git('cat-file', '-e', REV + '^{commit}', check=False).returncode:
        git('fetch', 'origin', REV)
    git('checkout', '--detach', REV)
    git('apply', '--check', str(PATCH))
    git('apply', str(PATCH))
for path, expected in LOCK['patched_file_sha256'].items():
    if hashlib.sha256((DEST / path).read_bytes()).hexdigest() != expected:
        raise SystemExit('Patched framework file differs from the locked result: ' + path)
git('remote', 'set-url', 'origin', LOCK['upstream']['repository'])
print(json.dumps({'upstream': LOCK['upstream']['repository'], 'revision': REV,
                  'integration_patch_sha256': hashlib.sha256(PATCH.read_bytes()).hexdigest(),
                  'crates': LOCK['crates'], 'external_checkout': str(DEST)}))
