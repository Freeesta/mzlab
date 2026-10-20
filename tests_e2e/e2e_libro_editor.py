"""Libro editor page (mzlab/web/libro/editor.html): token flow (tab-only and sealed with a passphrase), section list, visual editing, save with the blob sha,
409 conflict dialog, history, and the privacy rules (only api.github.com is called, the token is only sent as a header, never stored in clear, no CSP violation).
GitHub is replaced by a mock in the test. Run from the repo root: python3 tests_e2e/e2e_libro_editor.py"""
import base64, functools, http.server, json, sys, threading
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
TOKEN = "github_pat_TESTTOKEN_0123456789"
BODY = "# S1 — Prova\n\nCiao mondo.\n"
F = {"Libro_Untargeted/A_algoritmi/A2_prova.md": {"text": BODY, "sha": "sha1"}}
STATE = {"puts": [], "calls": [], "conflict": False}
CORS = {"access-control-allow-origin": "*", "access-control-allow-headers": "*", "access-control-allow-methods": "GET,PUT,OPTIONS"}
steps = []


def until(pg, js, timeout=15000):
    """Poll a JS expression through evaluate: the page CSP forbids eval, which Playwright's own wait_for_function needs."""
    import time
    t0 = time.time()
    while time.time() - t0 < timeout / 1000:
        if pg.evaluate("() => !!(" + js + ")"): return
        pg.evaluate("() => new Promise(r => setTimeout(r, 60))")
    raise TimeoutError(js)


def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; steps.append((name, "FAIL " + (str(e).split("\n")[0] or "") [:300] + " @line " + str(traceback.extract_tb(e.__traceback__)[-1].lineno)))


def gh(route):
    r = route.request
    if r.method == "OPTIONS": return route.fulfill(status=204, headers=CORS)
    STATE["calls"].append((r.method, r.url, r.headers.get("authorization")))
    path = r.url.split("api.github.com/repos/Freeesta/mzlab-dati/")[1]
    def ok(obj, status=200): route.fulfill(status=status, headers={**CORS, "content-type": "application/json"}, body=json.dumps(obj))
    if path.startswith("git/trees/"): return ok({"tree": [{"path": p, "type": "blob"} for p in F] + [{"path": "Libro_Untargeted/LEGGIMI.md"}, {"path": "Libro_Untargeted/A_algoritmi/A2_prova.qa.md"}, {"path": "README.md"}]})
    if path.startswith("commits"): return ok([{"sha": "c1", "html_url": "https://github.com/x/c1", "commit": {"message": "Prima versione\n\ndettagli", "committer": {"date": "2026-10-20T09:00:00Z"}}}])
    p = path.split("contents/")[1].split("?")[0]
    if r.method == "PUT":
        b = json.loads(r.post_data)
        STATE["puts"].append((p, b))
        if STATE["conflict"] or (p in F and b.get("sha") != F[p]["sha"]): return ok({"message": "does not match"}, 409)
        F[p] = {"text": base64.b64decode(b["content"]).decode(), "sha": "sha%d" % (len(STATE["puts"]) + 1)}
        return ok({"content": {"sha": F[p]["sha"]}, "commit": {"sha": "cm"}})
    if p not in F: return ok({"message": "Not Found"}, 404)
    ref = path.split("ref=")[-1]
    txt = "# Vecchia versione\n" if ref == "c1" else F[p]["text"]
    return ok({"content": base64.b64encode(txt.encode()).decode(), "sha": F[p]["sha"]})


handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(ROOT / "mzlab" / "web"))
handler.log_message = lambda *a, **k: None
srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
threading.Thread(target=srv.serve_forever, daemon=True).start()
URL = f"http://127.0.0.1:{srv.server_port}/libro/editor.html"
with sync_playwright() as p:
    import os; b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
    ctx = b.new_context(viewport={"width": 1300, "height": 900}, reduced_motion="reduce")
    pg = ctx.new_page(); errs = []; hosts = set()
    pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.on("console", lambda m: errs.append(m.text) if m.type in ("error", "warning") else None)
    pg.on("request", lambda r: hosts.add(r.url.split("/")[2]))
    pg.route("https://api.github.com/**", gh)
    pg.goto(URL)
    status = lambda: pg.inner_text("#status")
    def token_dialog():
        pg.wait_for_selector("#dlg-token[open]"); assert "mzlab-dati" in pg.inner_text("#dlg-token") and "Contents: Read and write" in pg.inner_text("#dlg-token")
        pg.click("#t-tab"); assert "Paste the token" in pg.inner_text("#t-msg")
        pg.fill("#in-token", TOKEN); pg.click("#t-tab"); pg.wait_for_selector("#sections button")
        assert pg.locator("#sections button").count() == 1 and "A2 prova" in pg.inner_text("#sections")
    step("no token: the setup dialog explains what to create; tab-only token lists only the section files", token_dialog)
    def open_edit():
        pg.click("#sections button"); pg.wait_for_selector(".milkdown .ProseMirror")
        until(pg, "document.querySelector('.milkdown .ProseMirror').innerText.includes('Ciao mondo')")
        assert status() == "All changes saved" and pg.is_disabled("#btn-save")
        pg.click(".milkdown .ProseMirror p"); pg.keyboard.press("End"); pg.keyboard.type(" Nuovo testo.")
        until(pg, "document.getElementById('status').textContent === 'Unsaved changes'")
        assert not pg.is_disabled("#btn-save")
    step("opening a section shows the text; typing marks the file as changed", open_edit)
    def save():
        pg.keyboard.press("Control+s"); until(pg, "document.getElementById('status').textContent === 'All changes saved'")
        path, body = STATE["puts"][-1]
        assert path.endswith("A2_prova.md") and body["sha"] == "sha1" and body["branch"] == "main"
        assert "Nuovo testo." in base64.b64decode(body["content"]).decode() and "edit text from the editor" in body["message"]
        assert F["Libro_Untargeted/A_algoritmi/A2_prova.md"]["text"].startswith("# S1 — Prova") and F["Libro_Untargeted/A_algoritmi/A2_prova.md"]["text"].endswith("\n")
    step("Ctrl+S commits the file with the sha it was read with", save)
    def conflict():
        STATE["conflict"] = True
        pg.click(".milkdown .ProseMirror p"); pg.keyboard.press("End"); pg.keyboard.type(" Altro.")
        pg.keyboard.press("Control+s"); pg.wait_for_selector("#dlg-conflict[open]")
        assert "Altro." in pg.input_value("#c-mine") and "Nothing was overwritten" in pg.inner_text("#dlg-conflict")
        n = len(STATE["puts"]); STATE["conflict"] = False
        pg.click("#c-load"); until(pg, "document.getElementById('status').textContent === 'All changes saved'")
        assert len(STATE["puts"]) == n and "Altro." not in pg.inner_text(".milkdown .ProseMirror")
    step("a stale sha gives the conflict dialog with the user's text, and never overwrites", conflict)
    def history():
        pg.click("#btn-history"); pg.wait_for_selector("#history li"); assert "Prima versione" in pg.inner_text("#history") and pg.locator("#history a[rel~=noopener]").count() == 1
        pg.click("#history button"); until(pg, "document.querySelector('.milkdown .ProseMirror').innerText.includes('Vecchia versione')")
        assert status() == "Unsaved changes", "an old version is only loaded, saved when the user presses Save"
    step("history lists the commits; loading an old version does not save it", history)
    def tabs():
        n = len(STATE["puts"]); pg.once("dialog", lambda d: d.accept()); pg.click("#tab-notes")
        until(pg, "document.getElementById('path').textContent.includes('A2_prova.appunti.md — Nothing here yet')"); assert len(STATE["puts"]) == n
        pg.click("#tab-qa"); until(pg, "document.getElementById('path').textContent.includes('A2_prova.qa.md')")
    step("notes and Q&A are separate files; a missing one is only created on save", tabs)
    def sealed():
        pg.click("#btn-token"); pg.fill("#in-token", TOKEN); pg.fill("#in-pass", "frase segreta"); pg.click("#t-seal"); until(pg, "localStorage.getItem('mzlab.libro.token')")
        raw = pg.evaluate("localStorage.getItem('mzlab.libro.token')"); assert raw and TOKEN not in raw and "github_pat" not in raw
        pg.evaluate("sessionStorage.clear()"); pg.reload()
        pg.wait_for_selector("#dlg-unlock[open]"); pg.fill("#in-unlock", "sbagliata"); pg.click("#u-go"); until(pg, "document.getElementById('u-msg').textContent.includes('Wrong passphrase')")
        pg.fill("#in-unlock", "frase segreta"); pg.click("#u-go"); pg.wait_for_selector("#sections button")
    step("the sealed token is not in clear in localStorage; a wrong passphrase is refused, the right one connects", sealed)
    def privacy():
        assert hosts <= {f"127.0.0.1:{srv.server_port}", "api.github.com"}, hosts
        assert STATE["calls"] and all(a == "Bearer " + TOKEN for _, _, a in STATE["calls"]) and not any(TOKEN in u for _, u, _ in STATE["calls"])
        real = [e for e in errs if "404" not in e and "409" not in e]       # a file that does not exist yet (notes, Q&A) is a 404 and a stale sha a 409 from GitHub: the browser logs both
        assert not real, real
    step("only api.github.com is called, the token travels in the header only, no console error or CSP violation", privacy)
    b.close()
srv.shutdown()
bad = [s for s in steps if s[1] != "ok"]
for n, r in steps: print(("PASS " if r == "ok" else "FAIL ") + n + ("" if r == "ok" else "  -> " + r))
sys.exit(1 if bad else 0)
