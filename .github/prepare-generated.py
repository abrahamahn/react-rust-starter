"""Temporary bootstrap: store reviewed generated files as objects, never commit/push.
Removed once canonical lockfiles and rustfmt output are committed. No dependency
build scripts run in the job that receives this short-lived repository token.
"""
import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess
import urllib.request

repo = 'abrahamahn/react-rust-starter'
assert os.environ['GITHUB_REPOSITORY'] == repo
assert os.environ['GITHUB_REF'] == 'refs/heads/main'
root = Path(os.environ['GITHUB_WORKSPACE']).resolve()
paths = set(subprocess.check_output(['git', 'diff', '--name-only'], cwd=root, text=True).splitlines())
paths.update(p for p in ('Cargo.lock', 'pnpm-lock.yaml') if (root / p).exists())
assert len(paths) <= 100
records = []
for name in sorted(paths):
    assert name in ('Cargo.lock', 'pnpm-lock.yaml') or (name.startswith('apps/server/') and name.endswith('.rs'))
    path = root / name
    assert path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root)
    data = path.read_bytes()
    assert len(data) < 2_000_000
    expected = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
    request = urllib.request.Request('https://api.github.com/repos/' + repo + '/git/blobs',
        data=json.dumps({'encoding': 'base64', 'content': base64.b64encode(data).decode()}).encode(),
        headers={'Authorization': 'Bearer ' + os.environ['REVIEW_TOKEN'], 'Accept': 'application/vnd.github+json'}, method='POST')
    with urllib.request.urlopen(request, timeout=30) as response:
        result = json.load(response)
    assert result['sha'] == expected
    records.append({'path': name, 'sha': expected, 'bytes': len(data)})
print('GENERATED_REVIEW_BEGIN')
print(json.dumps(records, indent=2))
print('GENERATED_REVIEW_END')
