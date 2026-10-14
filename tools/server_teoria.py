"""Mini-server locale per l'editor visivo della Teoria di mzLab.
Ascolta su http://127.0.0.1:8765. Fornisce sia file statici che API per l'editor:
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
        if parsed.path == "/stato":
            self.send_json(200, {"ok": True, "servizio": "mzLab Editor Server"})
            return
        if parsed.path == "/carica":
            query = urllib.parse.parse_qs(parsed.query)
            filename = query.get("file", [""])[0]
            if not filename.endswith(".html") or "/" in filename or "\\" in filename:
                self.send_json(400, {"ok": False, "error": "Nome file non valido"})
                return
            target_file = TEORIA_DIR / filename
            if not target_file.exists():
                self.send_json(404, {"ok": False, "error": f"File {filename} non trovato"})
                return
            content = target_file.read_text(encoding="utf-8")
            self.send_json(200, {"ok": True, "file": filename, "html": content})
            return
        super().do_GET()

    def do_POST(self):
        if self.path != "/salva":
            self.send_error(404, "Rotta non trovata")
            return

        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length).decode("utf-8")
        try:
            data = json.loads(body)
            filename = data.get("file", "")
            html_content = data.get("html", "")
            if not filename.endswith(".html") or "/" in filename or "\\" in filename:
                raise ValueError("Nome file non valido")

            target_file = TEORIA_DIR / filename
            target_file.write_text(html_content, encoding="utf-8")

            # Controllo con controlla_teoria.py
            check_res = subprocess.run(
                [sys.executable, str(ROOT / "tools" / "controlla_teoria.py"), "--path", str(target_file)],
                capture_output=True,
                text=True
            )
            if check_res.returncode != 0:
                self.send_json(400, {
                    "ok": False,
                    "error": f"Errore nei controlli della Teoria:\n{check_res.stdout}\n{check_res.stderr}"
                })
                return

            # Verifica modifiche git
            status_res = subprocess.run(
                ["git", "-C", str(ROOT), "status", "--porcelain", str(target_file)],
                capture_output=True,
                text=True
            )
            if not status_res.stdout.strip():
                self.send_json(200, {
                    "ok": True,
                    "msg": "Nessuna nuova modifica: il file salvato è identico alla versione git corrente.",
                    "commit": "Nessun commit necessario"
                })
                return

            # Git add, commit e push
            subprocess.run(["git", "-C", str(ROOT), "add", str(target_file)], check=True)
            msg = f"Teoria: aggiornato {filename} dall'editor visivo"
            commit_res = subprocess.run(["git", "-C", str(ROOT), "commit", "-m", msg], capture_output=True, text=True)
            push_res = subprocess.run(["git", "-C", str(ROOT), "push", "origin", "HEAD:main"], capture_output=True, text=True)

            if push_res.returncode != 0:
                self.send_json(200, {
                    "ok": True,
                    "msg": "Capitolo salvato e committato localmente. Il push su GitHub ha restituito un avviso:\n" + push_res.stderr,
                    "commit": commit_res.stdout,
                    "warning": push_res.stderr
                })
                return

            self.send_json(200, {
                "ok": True,
                "msg": "Capitolo salvato e pubblicato con successo su GitHub!",
                "commit": commit_res.stdout
            })
        except Exception as e:
            self.send_json(500, {"ok": False, "error": str(e)})

    def send_json(self, status, payload):
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(json.dumps(payload).encode("utf-8"))

    def log_message(self, format, *args):
        sys.stderr.write(f"[Server Editor Teoria] {self.address_string()} - {format % args}\n")


if __name__ == "__main__":
    server = http.server.ThreadingHTTPServer(("127.0.0.1", PORT), EditorHandler)
    print(f"[Server Editor Teoria] In ascolto su http://127.0.0.1:{PORT}")
    print("Premi Ctrl+C per fermare il server.")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[Server Editor Teoria] Arrestato.")
