#!/usr/bin/env python3
"""Query OSV for the public crate names/versions pinned by Cargo.lock."""
import json
import hashlib
from datetime import date
from pathlib import Path
import sys
import tomllib
import urllib.request

root = Path(__file__).resolve().parents[1]
packages = [p for p in tomllib.loads((root / 'src-tauri/Cargo.lock').read_text())['package'] if p['name'] != 'afterglow']
policy = json.loads((root / 'dependency-review.json').read_text())
if date.today() > date.fromisoformat(policy['expires']):
    raise RuntimeError('Dependency exceptions require a fresh review')
for item in policy['patched']:
    if hashlib.sha256((root / item['file']).read_bytes()).hexdigest() != item['sha256']:
        raise RuntimeError('Reviewed backport checksum mismatch')
findings = []
unreviewed = []
for offset in range(0, len(packages), 100):
    batch = packages[offset:offset + 100]
    payload = {'queries': [{'package': {'name': p['name'], 'ecosystem': 'crates.io'}, 'version': p['version']} for p in batch]}
    request = urllib.request.Request('https://api.osv.dev/v1/querybatch', data=json.dumps(payload).encode(), headers={'Content-Type': 'application/json'})
    with urllib.request.urlopen(request, timeout=30) as response:
        results = json.load(response)['results']
    if len(results) != len(batch):
        raise RuntimeError('Incomplete advisory response')
    for package, result in zip(batch, results):
        for advisory in result.get('vulns', []):
            findings.append({'package': package['name'], 'version': package['version'], 'advisory': advisory['id']})
            patched = any(p['package'] == package['name'] and p['version'] == package['version'] and advisory['id'] in p['advisories'] for p in policy['patched'])
            informational = any(p['package'] == package['name'] and p['version'] == package['version'] and advisory['id'] == p['advisory'] for p in policy['informational'])
            status = 'BACKPORTED FIX' if patched else 'MAINTENANCE WARNING' if informational else 'NEEDS REVIEW'
            print(f"{status}: {package['name']} {package['version']}: https://osv.dev/vulnerability/{advisory['id']}")
            if not patched and not informational:
                unreviewed.append(advisory['id'])
print(f'Checked {len(packages)} locked crates; {len(findings)} advisory matches; {len(unreviewed)} unreviewed.')
if unreviewed:
    sys.exit(1)
