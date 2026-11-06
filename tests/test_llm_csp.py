"""The assistant adds no host to the CSP of the page or of the window: only the document of the transport (mzlab/web/llm/trasporto.html) may reach the
network, and only the sources listed here ('self', the local servers and the proxy of the public demo, whose host is the one constant of llm/proxy-host.js
that tools/build_site.py writes into the built CSP; no provider host, so no personal key can be used from the site)."""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import build_site  # noqa: E402

PROXY = build_site.proxy_host(ROOT / "mzlab/web")
ALLOWED = {"'self'", "http://localhost:*", "http://127.0.0.1:*"} | ({PROXY} if PROXY else set())


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


def test_il_sito_costruito_aggiunge_solo_il_proxy():
    t = (ROOT / "mzlab/web/llm/trasporto.html").read_text(encoding="utf-8")
    host = "https://assist.example.org"
    built = set(re.search(r"connect-src ([^;]+)", csp(build_site.transport_csp(t, host))).group(1).split())
    assert built == {"'self'", "http://localhost:*", "http://127.0.0.1:*", host}
    assert build_site.transport_csp(t, "") == t
    assert "api.anthropic.com" not in t and (not PROXY or PROXY.startswith("https://"))
    # the same constant feeds the CSP and every client (worker, page, app route)
    for f in ("llm-worker.js", "llm-clienti.js", "llm-agente.js"):
        assert 'from "./proxy-host.js"' in (ROOT / "mzlab/web/llm" / f).read_text(encoding="utf-8"), f
