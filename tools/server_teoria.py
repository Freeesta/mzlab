"""Mini-server locale per l'editor visivo della Teoria di mzLab.
Ascolta su http://127.0.0.1:8765. Fornisce file statici e API per l'editor:
1. GET /stato -> verifica che il server sia attivo
2. GET /carica?file=<nome>.html -> restituisce il codice HTML pulito del file
3. POST /salva -> salva mzlab/web/teoria/<nome>.html, esegue controlla_teoria.py e fa commit + push

Uso da terminale:
    python3 tools/server_teoria.py
"""
import http.server
import json
import subprocess
import sys
import urllib.parse
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TEORIA_DIR = ROOT / "mzlab" / "web" / "teoria"
PORT = 8765


def send_json(handler, status, payload):
    handler.send_response(status)
    handler.send_header("Content-Type", "application/json; charset=utf-8")
    handler.send_header("Access-Control-Allow-Origin", "*")
    handler.end_headers()
    handler.wfile.write(json.dumps(payload).encode("utf-8"))


def handle_get_api(handler, parsed):
    if parsed.path == "/stato":
        send_json(handler, 200, {"ok": True, "servizio": "mzLab Editor Server"})
        return True
    if parsed.path == "/carica":
        query = urllib.parse.parse_qs(parsed.query)
        filename = query.get("file", [""])[0]
        if not filename.endswith(".html") or "/" in filename or "\\" in filename:
            send_json(handler, 400, {"ok": False, "error": "Nome file non valido"})
            return True
        target_file = TEORIA_DIR / filename
        if not target_file.exists():
            send_json(handler, 404, {"ok": False, "error": f"File {filename} non trovato"})
            return True
        send_json(handler, 200, {"ok": True, "file": filename, "html": target_file.read_text(encoding="utf-8")})
        return True
    return False


def handle_salva_api(handler):
    content_length = int(handler.headers.get("Content-Length", 0))
    body = handler.rfile.read(content_length).decode("utf-8")
    try:
        data = json.loads(body)
        filename, html_content = data.get("file", ""), data.get("html", "")
        if not filename.endswith(".html") or "/" in filename or "\\" in filename:
            raise ValueError("Nome file non valido")
        target_file = TEORIA_DIR / filename
        target_file.write_text(html_content, encoding="utf-8")
        chk = subprocess.run([sys.executable, str(ROOT / "tools" / "controlla_teoria.py"), "--path", str(target_file)], capture_output=True, text=True)
        if chk.returncode != 0:
            send_json(handler, 400, {"ok": False, "error": f"Errore nei controlli della Teoria:\n{chk.stdout}\n{chk.stderr}"})
            return
        st = subprocess.run(["git", "-C", str(ROOT), "status", "--porcelain", str(target_file)], capture_output=True, text=True)
        if not st.stdout.strip():
            send_json(handler, 200, {"ok": True, "msg": "Nessuna nuova modifica.", "commit": "Invariato"})
            return
        subprocess.run(["git", "-C", str(ROOT), "add", str(target_file)], check=True)
        msg = f"Teoria: aggiornato {filename} dall'editor visivo"
        c_res = subprocess.run(["git", "-C", str(ROOT), "commit", "-m", msg], capture_output=True, text=True)
        p_res = subprocess.run(["git", "-C", str(ROOT), "push", "origin", "HEAD:main"], capture_output=True, text=True)
        send_json(handler, 200, {"ok": True, "msg": "Capitolo salvato e pubblicato su GitHub!", "commit": c_res.stdout, "warning": p_res.stderr if p_res.returncode != 0 else None})
    except Exception as e:
        send_json(handler, 500, {"ok": False, "error": str(e)})


class EditorHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)

    def end_headers(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        super().end_headers()

    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
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
    print(f"[Server Editor Teoria] In ascolto su http://127.0.0.1:{PORT}\nPremi Ctrl+C per fermare il server.")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[Server Editor Teoria] Arrestato.")
