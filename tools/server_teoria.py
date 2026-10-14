"""Mini-server locale per l'editor visivo della Teoria di mzLab.
Ascolta su http://127.0.0.1:8765. Serve esclusivamente i file di mzlab/web/teoria/.
Sicurezza:
1. CORS restrittivo: accetta solo Origin da 127.0.0.1 / localhost e https://freeesta.github.io (nessun '*').
2. Token casuale generato all'avvio: ogni richiesta POST /salva richiede il token per prevenire CSRF.
3. Path traversal bloccato: serve e modifica solo file all'interno di mzlab/web/teoria/.

Uso da terminale:
    python3 tools/server_teoria.py
"""
from __future__ import annotations

import hmac
import http.server
import json
import secrets
import subprocess
import sys
import urllib.parse
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TEORIA_DIR = ROOT / "mzlab" / "web" / "teoria"
APPNAME_FILE = ROOT / "mzlab" / "web" / "appname.js"
PORT = 8765

ALLOWED_ORIGINS = {
    "http://127.0.0.1:8765",
    "http://localhost:8765",
    "https://freeesta.github.io",
}

TOKEN = secrets.token_hex(16)


def get_allowed_origin(handler: http.server.BaseHTTPRequestHandler) -> str | None:
    origin = handler.headers.get("Origin")
    if not origin:
        return None
    clean = origin.strip().rstrip("/")
    for allowed in ALLOWED_ORIGINS:
        if clean == allowed.rstrip("/"):
            return origin.strip()
    return None


def send_json(handler: http.server.BaseHTTPRequestHandler, status: int, payload: dict) -> None:
    handler.send_response(status)
    handler.send_header("Content-Type", "application/json; charset=utf-8")
    allowed = get_allowed_origin(handler)
    if allowed:
        handler.send_header("Access-Control-Allow-Origin", allowed)
        handler.send_header("Vary", "Origin")
    handler.end_headers()
    handler.wfile.write(json.dumps(payload, ensure_ascii=False).encode("utf-8"))


def verify_token(handler: http.server.BaseHTTPRequestHandler, data: dict | None = None) -> bool:
    token = handler.headers.get("X-Teoria-Token")
    if not token:
        auth = handler.headers.get("Authorization", "")
        if auth.startswith("Bearer "):
            token = auth[7:].strip()
    if not token and data and isinstance(data, dict):
        raw_val = data.get("token")
        if isinstance(raw_val, str):
            token = raw_val
    if not token:
        parsed = urllib.parse.urlparse(handler.path)
        qs = urllib.parse.parse_qs(parsed.query)
        tokens = qs.get("token", [])
        if tokens:
            token = tokens[0]
    if not token:
        return False
    return hmac.compare_digest(token.strip(), TOKEN)


def handle_get_api(handler: http.server.BaseHTTPRequestHandler, parsed: urllib.parse.ParseResult) -> bool:
    origin = handler.headers.get("Origin")
    allowed = get_allowed_origin(handler)
    if origin and not allowed:
        send_json(handler, 403, {"ok": False, "error": "Origin non consentita"})
        return True

    if parsed.path == "/stato":
        send_json(handler, 200, {"ok": True, "servizio": "mzLab Editor Server"})
        return True

    if parsed.path == "/appname.js":
        if APPNAME_FILE.is_file():
            content = APPNAME_FILE.read_bytes()
            handler.send_response(200)
            handler.send_header("Content-Type", "application/javascript; charset=utf-8")
            handler.send_header("Content-Length", str(len(content)))
            if allowed:
                handler.send_header("Access-Control-Allow-Origin", allowed)
                handler.send_header("Vary", "Origin")
            handler.end_headers()
            handler.wfile.write(content)
            return True

    if parsed.path == "/carica":
        query = urllib.parse.parse_qs(parsed.query)
        filename = query.get("file", [""])[0]
        if not filename.endswith(".html") or "/" in filename or "\\" in filename or ".." in filename:
            send_json(handler, 400, {"ok": False, "error": "Nome file non valido"})
            return True
        target_file = (TEORIA_DIR / filename).resolve()
        if not target_file.is_relative_to(TEORIA_DIR.resolve()) or not target_file.is_file():
            send_json(handler, 404, {"ok": False, "error": f"File {filename} non trovato"})
            return True
        send_json(handler, 200, {"ok": True, "file": filename, "html": target_file.read_text(encoding="utf-8")})
        return True

    return False


def handle_salva_api(handler: http.server.BaseHTTPRequestHandler) -> None:
    origin = handler.headers.get("Origin")
    allowed = get_allowed_origin(handler)
    if origin and not allowed:
        send_json(handler, 403, {"ok": False, "error": "Origin non consentita"})
        return

    content_length = int(handler.headers.get("Content-Length", 0))
    if content_length <= 0 or content_length > 5 * 1024 * 1024:
        send_json(handler, 400, {"ok": False, "error": "Dimensione corpo richiesta non valida (max 5 MB)"})
        return

    body = handler.rfile.read(content_length).decode("utf-8")
    try:
        data = json.loads(body)
    except Exception:
        send_json(handler, 400, {"ok": False, "error": "JSON non valido"})
        return

    if not verify_token(handler, data):
        send_json(handler, 403, {
            "ok": False,
            "error": "Token di sicurezza mancante o errato. Controlla il token mostrato all'avvio di server_teoria.py."
        })
        return

    try:
        filename = data.get("file", "")
        html_content = data.get("html", "")
        if not filename.endswith(".html") or "/" in filename or "\\" in filename or ".." in filename:
            raise ValueError("Nome file non valido")
        target_file = (TEORIA_DIR / filename).resolve()
        if not target_file.is_relative_to(TEORIA_DIR.resolve()):
            raise ValueError("Percorso non consentito")
        if not target_file.is_file():
            raise FileNotFoundError(f"Il capitolo {filename} non esiste nella cartella teoria")

        target_file.write_text(html_content, encoding="utf-8")
        chk = subprocess.run(
            [sys.executable, str(ROOT / "tools" / "controlla_teoria.py"), "--path", str(target_file)],
            capture_output=True, text=True
        )
        if chk.returncode != 0:
            send_json(handler, 400, {
                "ok": False,
                "error": f"Errore nei controlli della Teoria:\n{chk.stdout}\n{chk.stderr}"
            })
            return

        st = subprocess.run(["git", "-C", str(ROOT), "status", "--porcelain", str(target_file)], capture_output=True, text=True)
        if not st.stdout.strip():
            send_json(handler, 200, {"ok": True, "msg": "Nessuna nuova modifica.", "commit": "Invariato"})
            return

        subprocess.run(["git", "-C", str(ROOT), "add", str(target_file)], check=True)
        msg = f"Teoria: aggiornato {filename} dall'editor visivo"
        c_res = subprocess.run(["git", "-C", str(ROOT), "commit", "-m", msg], capture_output=True, text=True)
        p_res = subprocess.run(["git", "-C", str(ROOT), "push", "origin", "HEAD:main"], capture_output=True, text=True)
        send_json(handler, 200, {
            "ok": True,
            "msg": "Capitolo salvato e pubblicato su GitHub!",
            "commit": c_res.stdout,
            "warning": p_res.stderr if p_res.returncode != 0 else None
        })
    except Exception as e:
        send_json(handler, 500, {"ok": False, "error": str(e)})


class EditorHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(TEORIA_DIR), **kwargs)

    def end_headers(self):
        allowed = get_allowed_origin(self)
        if allowed:
            self.send_header("Access-Control-Allow-Origin", allowed)
            self.send_header("Vary", "Origin")
        super().end_headers()

    def do_OPTIONS(self):
        origin = self.headers.get("Origin")
        allowed = get_allowed_origin(self)
        if origin and not allowed:
            self.send_error(403, "Origin non consentita")
            return
        self.send_response(200)
        if allowed:
            self.send_header("Access-Control-Allow-Origin", allowed)
            self.send_header("Vary", "Origin")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, X-Teoria-Token, Authorization")
        self.send_header("Access-Control-Max-Age", "86400")
        self.end_headers()

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        if not handle_get_api(self, parsed):
            super().do_GET()

    def do_POST(self):
        if self.path == "/salva":
            handle_salva_api(self)
        else:
            self.send_error(404, "Rotta non trovata")

    def log_message(self, format, *args):
        sys.stderr.write(f"[Server Editor Teoria] {self.address_string()} - {format % args}\n")


if __name__ == "__main__":
    server = http.server.ThreadingHTTPServer(("127.0.0.1", PORT), EditorHandler)
    print("=" * 66)
    print(f"[Server Editor Teoria] In ascolto su http://127.0.0.1:{PORT}")
    print(f"Cartella servita: {TEORIA_DIR}")
    print(f"Origin ammesse:   {', '.join(sorted(ALLOWED_ORIGINS))}")
    print(f"Token di sicurezza: {TOKEN}")
    print("-" * 66)
    print("Apri nel browser con token preimpostato:")
    print(f"  Locale:    http://127.0.0.1:{PORT}/index.html?token={TOKEN}")
    print(f"  Ufficiale: https://freeesta.github.io/mzlab/static/teoria/index.html?token={TOKEN}")
    print("Premi Ctrl+C per fermare il server.")
    print("=" * 66)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[Server Editor Teoria] Arrestato.")
