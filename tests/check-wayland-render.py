#!/usr/bin/env python3
"""Verify actual Wayland compositor pixels, not only WebKit's DOM/snapshot.

Run in an isolated Weston QA session with output capture enabled. The running
WebKitWebDriver must inherit GDK_BACKEND=wayland, the Wayland socket, and private
XDG config/data/runtime directories. Requires Pillow and weston-screenshooter.
Example: python3 tests/check-wayland-render.py /path/to/afterglow
This is an optional native regression test, separate from the headless JS suite.
"""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.request
from PIL import Image

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('application')
parser.add_argument('--driver', default='http://127.0.0.1:4446')
parser.add_argument('--screenshooter', default='weston-screenshooter')
parser.add_argument('--output', type=Path)
args = parser.parse_args()
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

def request(method, path, data=None):
    body = json.dumps(data).encode() if data is not None else None
    req = urllib.request.Request(args.driver + path, data=body, method=method,
                                 headers={'Content-Type': 'application/json'})
    with opener.open(req, timeout=30) as response:
        result = json.load(response)['value']
    if isinstance(result, dict) and 'error' in result:
        raise RuntimeError(result)
    return result

session = request('POST', '/session', {'capabilities': {'alwaysMatch': {
    'webkitgtk:browserOptions': {'binary': str(Path(args.application).resolve())}}}})
sid = session['sessionId']
def execute(script):
    return request('POST', f'/session/{sid}/execute/sync', {'script': script, 'args': []})

try:
    deadline = time.monotonic() + 20
    while not execute("return !!document.querySelector('#toggle')"):
        if time.monotonic() >= deadline:
            raise AssertionError('Afterglow Home did not load')
        time.sleep(.1)
    execute("const e=document.createElement('div');e.id='wayland-paint-test';"
            "e.style.cssText='position:fixed;top:10px;left:10px;width:64px;height:64px;"
            "background:#fc03a5;z-index:2147483647';document.body.append(e)")
    # A DOM assertion and WebDriver screenshot can both pass with a blank onscreen
    # window. Only the compositor capture proves the marker reached the display.
    time.sleep(.3)
    with tempfile.TemporaryDirectory(prefix='afterglow-wayland-render-') as folder:
        subprocess.run([args.screenshooter], cwd=folder, check=True, timeout=15)
        screenshots = list(Path(folder).glob('*.png'))
        assert screenshots, 'Compositor returned no screenshot'
        screenshot = screenshots[0]
        with Image.open(screenshot) as image:
            pixels = image.convert('RGB').tobytes()
            marker = sum(1 for i in range(0, len(pixels), 3)
                         if pixels[i] > 230 and pixels[i + 1] < 25 and 145 < pixels[i + 2] < 185)
        assert marker >= 1000, 'UI exists in DOM but was not painted by the Wayland compositor'
        if args.output:
            args.output.write_bytes(screenshot.read_bytes())
    print('PASS: Afterglow loads and its paint marker appears in actual Wayland compositor pixels.')
finally:
    try:
        execute("document.querySelector('#wayland-paint-test')?.remove()")
        request('DELETE', f'/session/{sid}')
    except Exception:
        pass
