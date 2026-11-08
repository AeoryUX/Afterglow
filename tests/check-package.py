#!/usr/bin/env python3
"""Check the built artifact, including launcher visibility under a restrictive umask."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile

archive = str(Path(sys.argv[1]).resolve())
with tempfile.TemporaryDirectory() as folder:
    subprocess.run(['bsdtar', '-xpf', archive, '-C', folder], check=True)
    root = Path(folder)
    desktop = root / 'usr/share/applications/dev.afterglow.desktop.desktop'
    assert desktop.stat().st_mode & 0o777 == 0o644, 'Desktop entry must be world readable'
    assert (root / 'usr/bin/afterglow').stat().st_mode & 0o111 == 0o111
    assert (root / 'usr/share/icons/hicolor/128x128/apps/dev.afterglow.desktop.png').stat().st_mode & 0o004
    for parent in desktop.parents:
        if parent == root:
            break
        assert parent.stat().st_mode & 0o005 == 0o005, f'Launcher directory unreadable: {parent}'
    subprocess.run(['desktop-file-validate', str(desktop)], check=True)
    security=root / 'usr/share/doc/afterglow/SECURITY.md'
    assert security.stat().st_mode & 0o777 == 0o644
    assert 'Remaining boundaries' in security.read_text()
    metadata = (root / '.PKGINFO').read_text()
    assert 'depend = desktop-file-utils\n' in metadata
    assert 'Exec=/usr/bin/afterglow\n' in desktop.read_text()
    assert 'TryExec=/usr/bin/afterglow\n' in desktop.read_text()
    assert 'NoDisplay=true' not in desktop.read_text()
print('PASS: packaged launcher, icon, directories, executable, and desktop cache dependency.')
