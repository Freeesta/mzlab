"""The CSP of the desktop window (tauri.conf.json) is the one of the page (index.html) plus the three extra sources of prepare.py."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXTRA = " mzlab: http://mzlab.localhost ipc: http://ipc.localhost"


def test_csp_desktop_uguale_a_quella_della_pagina():
    html = (ROOT / "mzlab/web/index.html").read_text(encoding="utf-8")
    page = re.search(r'Content-Security-Policy" content="([^"]+)"', html).group(1)
    conf = json.loads((ROOT / "crates/mzlab-desktop/tauri.conf.json").read_text(encoding="utf-8"))
    sec = conf["app"]["security"]
    assert sec["csp"] == page.replace("connect-src 'self'", "connect-src 'self'" + EXTRA, 1)
    assert "'unsafe-eval'" not in sec["csp"]
    # without this Tauri adds nonces and the inline scripts of the page stop running
    assert sec["dangerousDisableAssetCspModification"] is True
