#!/usr/bin/env python3
"""CSP of the page: the inline scripts of index.html are allowed by their SHA-256, not by 'unsafe-inline'.

`python3 tools/csp_hash.py` rewrites script-src in the meta of mzlab/web/index.html and the same policy in the desktop window
(crates/mzlab-desktop/tauri.conf.json); `--check` only verifies (tests/test_csp_hash.py). Edit an inline script -> run it again."""
import base64
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PAGE = ROOT / "mzlab/web/index.html"
CONF = ROOT / "crates/mzlab-desktop/tauri.conf.json"
EXTRA = " mzlab: http://mzlab.localhost ipc: http://ipc.localhost"      # sources of the desktop window (tests/test_csp_desktop.py)
META = re.compile(r'(Content-Security-Policy" content=")([^"]+)(")')
INLINE = re.compile(r"<script(?![^>]*\bsrc=)[^>]*>(.*?)</script>", re.S)
SCRIPT_SRC = re.compile(r"script-src [^;]+;")


def hashes(html: str) -> list[str]:
    """The 'sha256-...' sources of the inline scripts, in page order, without repeats."""
    out: list[str] = []
    for m in INLINE.finditer(html):
        h = "'sha256-" + base64.b64encode(hashlib.sha256(m.group(1).encode("utf-8")).digest()).decode() + "'"
        if h not in out:
            out.append(h)
    return out


def script_src(html: str) -> str:
    return "script-src 'self' " + " ".join(hashes(html)) + " 'wasm-unsafe-eval' blob:;"


def policy(html: str) -> str:
    """The CSP the page must carry, from the one it has now."""
    cur = META.search(html).group(2)
    new = SCRIPT_SRC.sub(script_src(html), cur, count=1)
    return re.sub(r"base-uri [^;]+;", "base-uri 'none';", new, count=1)


def main() -> int:
    html = PAGE.read_text(encoding="utf-8")
    want = policy(html)
    conf = json.loads(CONF.read_text(encoding="utf-8"))
    want_conf = want.replace("connect-src 'self'", "connect-src 'self'" + EXTRA, 1)
    ok = META.search(html).group(2) == want and conf["app"]["security"]["csp"] == want_conf
    if "--check" in sys.argv:
        print("csp: OK" if ok else "csp: out of date, run python3 tools/csp_hash.py")
        return 0 if ok else 1
    PAGE.write_text(META.sub(lambda m: m.group(1) + want + m.group(3), html, count=1), encoding="utf-8")
    text = CONF.read_text(encoding="utf-8")
    CONF.write_text(text.replace(json.dumps(conf["app"]["security"]["csp"])[1:-1], json.dumps(want_conf)[1:-1], 1), encoding="utf-8")
    print("csp: written")
    return 0


if __name__ == "__main__":
    sys.exit(main())
