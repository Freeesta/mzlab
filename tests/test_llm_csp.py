"""The assistant adds no host to the CSP of the page or of the window: only the document of the transport (mzlab/web/llm/trasporto.html) may reach the
network, and only the sources listed here ('self' and the local servers; the hosts of the providers wait for an explicit decision)."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOWED = {"'self'", "http://localhost:*", "http://127.0.0.1:*"}


def csp(text):
    return re.search(r'Content-Security-Policy" content="([^"]+)"', text).group(1)


def test_pagina_e_finestra_senza_host_dei_fornitori():
    page = csp((ROOT / "mzlab/web/index.html").read_text(encoding="utf-8"))
    conf = json.loads((ROOT / "crates/mzlab-desktop/tauri.conf.json").read_text(encoding="utf-8"))["app"]["security"]["csp"]
    connect = re.search(r"connect-src ([^;]+)", page).group(1)
    assert connect == "'self' blob:"
    for p in json.loads((ROOT / "crates/mzlab-llm/data/fornitori-1.json").read_text(encoding="utf-8"))["providers"]:
        host = re.sub(r"^(https?://[^/]+).*", r"\1", p["base_url"])
        assert host not in page and host not in conf


def test_trasporto_solo_host_elencati():
    t = (ROOT / "mzlab/web/llm/trasporto.html").read_text(encoding="utf-8")
    pol = csp(t)
    connect = set(re.search(r"connect-src ([^;]+)", pol).group(1).split())
    assert connect <= ALLOWED, connect - ALLOWED
    assert "default-src 'none'" in pol and "'unsafe-eval'" not in pol
    assert re.search(r"script-src 'self'(;|$)", pol)
    assert "<script src=" in t and not re.search(r"<script(?![^>]*\bsrc=)[^>]*>\s*\S", t)
