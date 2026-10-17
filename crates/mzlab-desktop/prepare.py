#!/usr/bin/env python3
"""Prepares the web folder of the desktop app: `python3 tools/build_site.py` first, then `python3 crates/mzlab-desktop/prepare.py`.
Copies site/ (the page as published) to crates/mzlab-desktop/dist/ and changes only a few things there: the CSP of index.html allows the
mzlab:// protocol, static/rust-worker.js is the native one and static/desktop-shim.js is loaded before browser.js.
Nothing of mzlab/web or tools/ is touched. Stdlib only."""
import re
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SITE = ROOT / "site"
DIST = HERE / "dist"

CSP_EXTRA = " mzlab: http://mzlab.localhost ipc: http://ipc.localhost"


def prepare(site: Path = SITE, dist: Path = DIST) -> None:
    if not (site / "index.html").is_file():
        sys.exit(f"{site}/index.html not found: run python3 tools/build_site.py first")
    if dist.exists():
        shutil.rmtree(dist)
    shutil.copytree(site, dist)
    index = dist / "index.html"
    html = index.read_text(encoding="utf-8")
    html, n = re.subn(r"(connect-src 'self')", r"\1" + CSP_EXTRA, html, count=1)
    if n != 1:
        sys.exit("connect-src not found in the CSP of index.html")
    tag = '<script src="static/browser.js"></script>'
    if tag not in html:
        sys.exit("browser.js script tag not found in index.html")
    html = html.replace(tag, '<script src="static/desktop-shim.js"></script>\n' + tag, 1)
    index.write_text(html, encoding="utf-8")
    static = dist / "static"
    shutil.copy(HERE / "shim" / "desktop-worker.js", static / "rust-worker.js")
    shutil.copy(HERE / "shim" / "desktop-shim.js", static / "desktop-shim.js")
    (static / "sw.js").unlink(missing_ok=True)   # the app is on disk: no service worker (it cannot register on a custom scheme)
    print(f"desktop dist ready: {dist}")


if __name__ == "__main__":
    prepare()
