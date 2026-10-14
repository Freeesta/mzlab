"""Test di sicurezza e funzionalità per tools/server_teoria.py."""
from __future__ import annotations

import json
import threading
import urllib.error
import urllib.request
from http.server import ThreadingHTTPServer
from pathlib import Path

import pytest

import tools.server_teoria as st


@pytest.fixture(scope="module")
def local_server():
    server = ThreadingHTTPServer(("127.0.0.1", 0), st.EditorHandler)
    port = server.server_address[1]
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    yield f"http://127.0.0.1:{port}"
    server.shutdown()


def test_allowed_origin_handling(local_server):
    # Origin non consentita
    req = urllib.request.Request(f"{local_server}/stato", headers={"Origin": "https://attacker.com"})
    try:
        urllib.request.urlopen(req)
        pytest.fail("Dovrebbe fallire con 403 per origin non consentita")
    except urllib.error.HTTPError as e:
        assert e.code == 403
        assert "Access-Control-Allow-Origin" not in e.headers

    # Origin consentita da GitHub Pages
    req_gh = urllib.request.Request(f"{local_server}/stato", headers={"Origin": "https://freeesta.github.io"})
    with urllib.request.urlopen(req_gh) as resp:
        assert resp.status == 200
        assert resp.headers.get("Access-Control-Allow-Origin") == "https://freeesta.github.io"
        data = json.loads(resp.read().decode())
        assert data["ok"] is True

    # Origin consentita locale
    req_loc = urllib.request.Request(f"{local_server}/stato", headers={"Origin": "http://127.0.0.1:8765"})
    with urllib.request.urlopen(req_loc) as resp:
        assert resp.status == 200
        assert resp.headers.get("Access-Control-Allow-Origin") == "http://127.0.0.1:8765"


def test_cors_options_preflight(local_server):
    # Preflight da attacker
    req_evil = urllib.request.Request(f"{local_server}/salva", headers={"Origin": "https://evil.org"}, method="OPTIONS")
    try:
        urllib.request.urlopen(req_evil)
        pytest.fail("Preflight da origin non consentita deve restituire 403")
    except urllib.error.HTTPError as e:
        assert e.code == 403

    # Preflight consentito
    req_ok = urllib.request.Request(f"{local_server}/salva", headers={"Origin": "https://freeesta.github.io"}, method="OPTIONS")
    with urllib.request.urlopen(req_ok) as resp:
        assert resp.status == 200
        assert resp.headers.get("Access-Control-Allow-Origin") == "https://freeesta.github.io"
        assert "POST" in resp.headers.get("Access-Control-Allow-Methods", "")
        assert "X-Teoria-Token" in resp.headers.get("Access-Control-Allow-Headers", "")


def test_post_salva_token_required(local_server):
    payload = json.dumps({"file": "04-lc.html", "html": "test"}).encode()

    # Senza token -> 403
    req_no_tok = urllib.request.Request(f"{local_server}/salva", data=payload, headers={"Content-Type": "application/json"}, method="POST")
    try:
        urllib.request.urlopen(req_no_tok)
        pytest.fail("POST senza token deve fallire con 403")
    except urllib.error.HTTPError as e:
        assert e.code == 403
        data = json.loads(e.read().decode())
        assert "Token" in data["error"]

    # Con token errato -> 403
    req_bad_tok = urllib.request.Request(
        f"{local_server}/salva",
        data=payload,
        headers={"Content-Type": "application/json", "X-Teoria-Token": "token-sbagliato-1234"},
        method="POST"
    )
    try:
        urllib.request.urlopen(req_bad_tok)
        pytest.fail("POST con token errato deve fallire con 403")
    except urllib.error.HTTPError as e:
        assert e.code == 403

    # Con token corretto in header
    assert st.verify_token(
        type("Handler", (), {"headers": {"X-Teoria-Token": st.TOKEN}, "path": "/salva"})(),
        {}
    ) is True


def test_path_traversal_blocked(local_server):
    # Tentativo di leggere file fuori da teoria tramite API /carica
    req = urllib.request.Request(f"{local_server}/carica?file=../../Cargo.toml")
    try:
        urllib.request.urlopen(req)
        pytest.fail("Deve bloccare file che non finiscono con .html o contengono slash")
    except urllib.error.HTTPError as e:
        assert e.code == 400

    # Tentativo di leggere file statico fuori da teoria
    try:
        urllib.request.urlopen(f"{local_server}/../Cargo.toml")
        pytest.fail("Non deve servire file fuori da mzlab/web/teoria/")
    except urllib.error.HTTPError as e:
        assert e.code in (400, 404)
