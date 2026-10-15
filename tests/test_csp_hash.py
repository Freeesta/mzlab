"""The CSP of the page allows its inline scripts by SHA-256 (tools/csp_hash.py), never by 'unsafe-inline' in script-src; base-uri is 'none'."""
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HTML = (ROOT / "mzlab/web/index.html").read_text(encoding="utf-8")
CSP = re.search(r'Content-Security-Policy" content="([^"]+)"', HTML).group(1)


def directive(name):
    return re.search(name + r" ([^;]+)", CSP).group(1)


def test_hashes_and_desktop_policy_are_current():
    r = subprocess.run([sys.executable, str(ROOT / "tools/csp_hash.py"), "--check"], capture_output=True, text=True)
    assert r.returncode == 0, r.stdout + r.stderr


def test_script_src_has_no_unsafe_inline_or_eval():
    src = directive("script-src")
    assert "'unsafe-inline'" not in src and "'unsafe-eval'" not in src
    assert len(re.findall(r"'sha256-[A-Za-z0-9+/]{43}='", src)) == len(re.findall(r"<script(?![^>]*\bsrc=)[^>]*>", HTML))


def test_base_and_plugins_are_closed():
    assert directive("base-uri") == "'none'" and directive("object-src") == "'none'"
    assert "<base" not in HTML.lower()


def test_no_inline_event_handlers_or_javascript_urls():
    assert not re.search(r"<[^>]+\son[a-z]+\s*=", HTML) and "javascript:" not in HTML.lower()
