"""Teoria editor page (mzlab/web/libro/teoria.html) with the REAL Rust converter (.wasm built here, checked against its manifest): token for the tab, chapter list, load (the converter
must reproduce the page on main), edit, preview, error of the converter with file:line, the headings guard, and the pull request (branch, source, page, auto-merge). GitHub is a mock
that serves the real chapter files; only api.github.com is called. Skipped without cargo, the wasm32 target or wasm-bindgen. Run from the repo root: python3 tests_e2e/e2e_teoria_editor.py"""
import base64, functools, hashlib, http.server, json, shutil, subprocess, sys, tempfile, threading
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
TOKEN = "github_pat_TESTTOKEN_0123456789"
CHAPTER = "03-cromatografia"
CORS = {"access-control-allow-origin": "*", "access-control-allow-headers": "*", "access-control-allow-methods": "GET,PUT,POST,OPTIONS"}
steps = []
calls = []


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
        import traceback; steps.append((name, "FAIL " + (str(e).split("\n")[0] or "")[:300] + " @line " + str(traceback.extract_tb(e.__traceback__)[-1].lineno)))


def build_site(out: Path):
    """libro/ and teoria/ as the site serves them, plus wasm-feat/ with the converter (web target) and its manifest."""
    shutil.copytree(ROOT / "mzlab/web/libro", out / "libro")
    shutil.copytree(ROOT / "mzlab/web/teoria", out / "teoria", ignore=shutil.ignore_patterns("src"))
    (out / "wasm-feat/libro").mkdir(parents=True)
    subprocess.run(["cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-libro-wasm"], cwd=ROOT, check=True, capture_output=True)
    subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(out / "wasm-feat/libro"), "target/wasm32-unknown-unknown/release/mzlab_libro_wasm.wasm"], cwd=ROOT, check=True, capture_output=True)
    sha = hashlib.sha256((out / "wasm-feat/libro/mzlab_libro_wasm_bg.wasm").read_bytes()).hexdigest()
    (out / "wasm-feat/manifest.json").write_text(json.dumps({"libro": {"js": "libro/mzlab_libro_wasm.js", "wasm": "libro/mzlab_libro_wasm_bg.wasm", "sha256": sha}}))


def mock_github(files, md_path, state):
    """Route handler for api.github.com: serves the real chapter files and records every write in state['writes']."""
    def gh(route):
        r = route.request
        if r.method == "OPTIONS": return route.fulfill(status=204, headers=CORS)
        calls.append((r.method, r.url, r.headers.get("authorization")))
        def ok(obj, status=200): route.fulfill(status=status, headers={**CORS, "content-type": "application/json"}, body=json.dumps(obj))
        u = r.url.split("api.github.com")[1]
        if u == "/graphql": state["writes"].append(("graphql", json.loads(r.post_data))); return ok({"data": {}})
        path = u.split("/repos/Freeesta/mzlab/")[1]
        if path.startswith("git/trees/"): return ok({"tree": [{"path": md_path}, {"path": "mzlab/web/teoria/src/LEGGIMI.md"}]})
        if path.startswith("pulls") and r.method == "GET": return ok(state["open"])
        if path.startswith("git/ref/heads/main"): return ok({"object": {"sha": "tip"}})
        if path == "git/refs": state["writes"].append(("ref", json.loads(r.post_data))); return ok({}, 201)
        if path == "pulls": state["writes"].append(("pr", json.loads(r.post_data))); return ok({"number": 9, "html_url": "https://github.com/Freeesta/mzlab/pull/9", "node_id": "PR_9"}, 201)
        p = path.split("contents/")[1].split("?")[0]
        from urllib.parse import unquote
        p = unquote(p)
        if r.method == "PUT": state["writes"].append(("put", p, json.loads(r.post_data))); return ok({"content": {"sha": "n"}, "commit": {"sha": "c"}})
        f = files.get(p)
        return ok({"content": base64.b64encode(f["text"].encode()).decode(), "sha": f["sha"]}) if f else ok({"message": "Not Found"}, 404)
    return gh


def edit_in_the_page(pg, src):
    """Editing in the preview: a paragraph lands in the source, a title is locked, bold and italic stay markup, a meaning-changing text is refused."""
    step("the page is shown by itself", lambda: until(pg, "!document.getElementById('preview').hidden"))
    fr = pg.frame_locator("#preview")
    until(pg, "document.getElementById('preview').contentDocument && document.getElementById('preview').contentDocument.querySelector('[data-src]')")
    def edit_first_paragraph():
        p1 = fr.locator("p[data-src]").first
        p1.click(); pg.keyboard.press("Control+End"); pg.keyboard.type(" Zeta."); pg.keyboard.press("Enter")
        until(pg, "document.getElementById('source').value.includes(' Zeta.')")
        now = pg.input_value("#source")
        assert len(now.split("\n")) == len(src.split("\n")) and now != src, "one line changed"
        assert pg.inner_text("#msg") == "", pg.inner_text("#msg")
    step("a paragraph is written in the page and lands in the source", edit_first_paragraph)
    def locked_block():
        fr.locator("h1").first.click()
        until(pg, "document.getElementById('msg').textContent.includes('cannot be changed in the page')")
    step("a title is locked with a pointer to the source box", locked_block)
    def inline_kept():
        until(pg, "document.getElementById('preview').contentDocument.querySelector('p[data-src] b, p[data-src] i')")
        el = fr.locator("p[data-src]:has(b, i)").first
        word = el.locator("b, i").first.inner_text()
        el.click(); pg.keyboard.press("Control+End"); pg.keyboard.type(" Eta."); pg.keyboard.press("Enter")
        until(pg, "document.getElementById('source').value.includes(' Eta.')")
        assert ("**" + word + "**" in pg.input_value("#source")) or ("*" + word + "*" in pg.input_value("#source")), word
    step("bold and italic stay markup in the source", inline_kept)
    def refused_text():
        before = pg.input_value("#source")
        el = fr.locator("p[data-src]").first
        el.click(); pg.keyboard.press("Control+End"); pg.keyboard.type(" *x*"); pg.keyboard.press("Enter")
        until(pg, "document.getElementById('msg').textContent.includes('edit it in the source box')")
        assert pg.input_value("#source") == before, "refused edit must not touch the source"
    step("text that would change meaning is refused", refused_text)


def edit_questions(pg, src):
    """The question and the answer of a ??? block are written in the page; the answer stays open while typing."""
    fr = pg.frame_locator("#preview")
    until(pg, "document.getElementById('preview').contentDocument.querySelector('summary[data-src]')")
    def question():
        s1 = fr.locator("summary[data-src]").first
        before = pg.input_value("#source")
        s1.click(); pg.keyboard.press("Control+End"); pg.keyboard.type(" con spazi")
        pg.keyboard.press("Enter")
        until(pg, "document.getElementById('source').value.includes(' con spazi')")
        now = pg.input_value("#source")
        line = next(l for l in now.split("\n") if l.endswith(" con spazi"))
        assert line.startswith("??? ") and len(now.split("\n")) == len(before.split("\n")), line
    step("a question is written in the page", question)
    def answer():
        until(pg, "document.getElementById('preview').contentDocument.querySelector('details[open] p[data-src]')")
        a1 = fr.locator("details p[data-src]").first
        a1.click(); pg.keyboard.press("Control+End"); pg.keyboard.type(" e anche qui")
        pg.keyboard.press("Enter")
        until(pg, "document.getElementById('source').value.includes(' e anche qui')")
        assert pg.inner_text("#msg") == "", pg.inner_text("#msg")
    step("the answer of the question is written in the page", answer)
    pg.fill("#source", src)


def history_and_draft(pg, src):
    """Undo/redo walk the changes made in the page, and an unsent draft survives a reload (restored, discarded, or only copyable when the chapter has moved on)."""
    val = lambda: pg.input_value("#source")
    until(pg, "!document.getElementById('btn-undo').disabled")
    last = val()
    def undo_redo():
        pg.click("#btn-undo")
        until(pg, "!document.getElementById('source').value.includes(' Eta.')")
        assert " Zeta." in val() and val() != last
        pg.click("#btn-redo")
        until(pg, "document.getElementById('source').value.includes(' Eta.')")
        assert val() == last
    step("undo and redo walk the changes", undo_redo)
    def keys():
        pg.focus("#source"); pg.keyboard.press("Control+z")
        until(pg, "!document.getElementById('source').value.includes(' Eta.')")
        pg.keyboard.press("Control+Shift+z")
        until(pg, "document.getElementById('source').value.includes(' Eta.')")
    step("Ctrl+Z and Ctrl+Shift+Z do the same", keys)
    def reload_offers_draft():
        pg.reload()
        until(pg, "!document.getElementById('draft').hidden")
        assert "unsent draft" in pg.inner_text("#draft") and pg.is_visible("text=Restore the draft")
    step("a reload offers the unsent draft", reload_offers_draft)
    def restore():
        pg.click("text=Restore the draft")
        until(pg, "document.getElementById('source').value.includes(' Eta.')")
        assert val() == last, "the draft is the last text"
    step("the draft is restored as it was", restore)
    def discard():
        pg.reload()
        until(pg, "!document.getElementById('draft').hidden")
        pg.click("text=Discard the draft")
        pg.reload()
        until(pg, "document.getElementById('source').value.length > 100")
        assert pg.is_hidden("#draft") and val() == src, ("no draft is offered after discarding", pg.is_hidden("#draft"), len(val()), len(src), pg.inner_text("#draft")[:80])
    step("a discarded draft does not come back", discard)
    def stale():
        key = pg.evaluate("() => Object.keys(localStorage).find(k => k.startsWith('mzlab.libro.teoria.draft.')) || ('mzlab.libro.teoria.draft.' + document.querySelector('#sections button').textContent + '.md')")
        pg.evaluate("([k, t]) => localStorage.setItem(k, JSON.stringify({text: t, base: 'old-sha', summary: '', saved: '2026-01-01T00:00:00Z'}))", [key, src + " Old."])
        pg.reload()
        until(pg, "!document.getElementById('draft').hidden")
        assert "cannot be put back" in pg.inner_text("#draft") and pg.is_hidden("text=Restore the draft")
        pg.click("text=Discard the draft")
    step("a draft of an older chapter is only offered as a copy", stale)
    step("the draft never holds the token", lambda: not pg.evaluate("() => Object.values(localStorage).some(v => v.includes('github_pat_'))") or (_ for _ in ()).throw(AssertionError("token in storage")))


def retitle_and_send(pg, w):
    """A section title is written in the page: its anchor is written out, and the pull request carries the rebuilt search index as a third file."""
    pg.reload()
    until(pg, "document.getElementById('source').value.length > 100")
    until(pg, "document.getElementById('preview').contentDocument && document.getElementById('preview').contentDocument.querySelector('h2[data-src]')")
    first = len(w)
    def retitle():
        h = pg.frame_locator("#preview").locator("h2[data-src]").first
        h.click(); pg.keyboard.press("Control+End"); pg.keyboard.type(" Zzz")
        pg.keyboard.press("Enter")
        until(pg, "document.getElementById('source').value.includes(' Zzz {#')")
        assert pg.inner_text("#msg") == "", pg.inner_text("#msg")
    step("a section title is written in the page and keeps its anchor", retitle)
    def send_with_index():
        pg.fill("#summary", "retitle a section"); pg.click("#btn-send")
        until(pg, "document.getElementById('result').textContent.includes('pull/9')")
        kinds = [x[0] for x in w[first:]]
        assert kinds == ["ref", "put", "put", "put", "pr", "graphql"], kinds
        index = [x for x in w[first:] if x[0] == "put" and x[1].endswith("indice-ricerca.js")]
        assert index and " Zzz" in base64.b64decode(index[0][2]["content"]).decode(), "the index has the new title"
    step("the pull request carries the rebuilt search index", send_with_index)


def main():
    if not all(shutil.which(x) for x in ("cargo", "wasm-bindgen")):
        print("SKIP e2e_teoria_editor: cargo or wasm-bindgen not available"); return 0
    tmp = Path(tempfile.mkdtemp())
    build_site(tmp)
    md_path, html_path = f"mzlab/web/teoria/src/{CHAPTER}.md", f"mzlab/web/teoria/{CHAPTER}.html"
    files = {md_path: {"text": (ROOT / md_path).read_text(encoding="utf-8"), "sha": "m1"}, html_path: {"text": (ROOT / html_path).read_text(encoding="utf-8"), "sha": "h1"},
             "tools/libro/importati.txt": {"text": (ROOT / "tools/libro/importati.txt").read_text(encoding="utf-8"), "sha": "i1"},
             "mzlab/web/teoria/indice-ricerca.js": {"text": (ROOT / "mzlab/web/teoria/indice-ricerca.js").read_text(encoding="utf-8"), "sha": "x1"}}
    state = {"open": [], "writes": []}

    gh = mock_github(files, md_path, state)
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(tmp))
    handler.log_message = lambda *a, **k: None
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{srv.server_address[1]}/libro/teoria.html"
    csp, other = [], []
    with sync_playwright() as p:
        b = p.chromium.launch()
        pg = b.new_page()
        pg.on("console", lambda m: csp.append(m.text) if "Content Security Policy" in m.text else None)
        pg.on("request", lambda r: other.append(r.url) if r.url.startswith("http") and not r.url.startswith(("http://127.0.0.1", "https://api.github.com")) else None)
        pg.route("https://api.github.com/**", gh)
        pg.goto(url)
        step("the token dialog opens first, with the one-line hint", lambda: "passphrase is a word you invent" in pg.inner_text("#dlg-token") or (_ for _ in ()).throw(AssertionError("hint")))
        pg.fill("#in-token", "my passphrase"); pg.click("#t-tab")
        step("a text that is not a token is refused in red", lambda: "must start with github_pat_" in pg.inner_text("#t-msg") or (_ for _ in ()).throw(AssertionError(pg.inner_text("#t-msg"))))
        pg.fill("#in-token", TOKEN); pg.click("#t-tab")
        step("connects and lists the chapters", lambda: pg.wait_for_selector("#sections button") and None)
        step("the converter version is in the status tooltip", lambda: until(pg, "document.getElementById('status').title.includes('mzlab-libro/3')"))
        step("only the chapter files, not the guide", lambda: pg.locator("#sections button").count() == 1 or (_ for _ in ()).throw(AssertionError("buttons")))
        step("status says Connected", lambda: until(pg, "document.getElementById('status').textContent === 'Connected'"))
        step("the first chapter opens by itself", lambda: until(pg, "document.getElementById('source').value.length > 100"))
        step("loads the source (the converter reproduces the page on main)", lambda: until(pg, "document.getElementById('source').value.length > 100 && !document.getElementById('source').disabled"))
        step("no message after the load", lambda: pg.inner_text("#msg") == "" or (_ for _ in ()).throw(AssertionError(pg.inner_text("#msg"))))
        src = pg.input_value("#source")
        edit_in_the_page(pg, src)
        history_and_draft(pg, src)
        edit_questions(pg, src)
        pg.fill("#source", src)
        lines = src.split("\n")
        k = next(i for i, l in enumerate(lines) if l[:1].isalpha() and not l.startswith(("Tabella", "script")))
        lines[k] += " Prova."
        pg.fill("#source", "\n".join(lines))
        step("send needs a summary", lambda: pg.is_disabled("#btn-send") or (_ for _ in ()).throw(AssertionError("enabled")))
        pg.fill("#summary", "add a test sentence")
        step("preview shows the page", lambda: (pg.click("#btn-preview"), until(pg, "!document.getElementById('preview').hidden"), None)[2])
        step("preview text has the edit", lambda: "Prova." in pg.frame_locator("#preview").locator("main").inner_text() or (_ for _ in ()).throw(AssertionError("no edit in preview")))
        step("converter error shows file and line", lambda: (pg.fill("#source", src + "\n\n[x](ftp://bad)\n"), pg.click("#btn-preview"), until(pg, f"document.getElementById('msg').textContent.includes('{CHAPTER}.md:')"), None)[3])
        pg.fill("#source", src.replace("\n", "\n\n## Nuova sezione\n", 1) if "\n# " not in src else src + "\n\n## Nuova sezione\n\nTesto.\n")
        pg.click("#btn-send")
        step("a heading change is refused", lambda: until(pg, "document.getElementById('msg').textContent.includes('added, removed')"))
        pg.fill("#source", "\n".join(lines))
        pg.click("#btn-send")
        step("pull request opened with auto-merge", lambda: until(pg, "document.getElementById('result').textContent.includes('pull/9') && document.getElementById('result').textContent.includes('merge by itself')"))
        w = state["writes"]
        step("one branch, source, page, pull request, auto-merge, in order", lambda: [x[0] for x in w] == ["ref", "put", "put", "pr", "graphql"] or (_ for _ in ()).throw(AssertionError([x[0] for x in w])))
        step("source committed as typed", lambda: base64.b64decode(w[1][2]["content"]).decode() == "\n".join(lines) or (_ for _ in ()).throw(AssertionError("source differs")))
        step("page committed is the converter output with the edit", lambda: ("Prova." in base64.b64decode(w[2][2]["content"]).decode() and w[2][1] == html_path) or (_ for _ in ()).throw(AssertionError("page")))
        step("branch name", lambda: w[0][1]["ref"].startswith(f"refs/heads/libro/teoria-{CHAPTER}-") or (_ for _ in ()).throw(AssertionError(w[0][1])))
        retitle_and_send(pg, w)
        step("token only as a header to api.github.com", lambda: all(c[2] == "Bearer " + TOKEN for c in calls) and not other or (_ for _ in ()).throw(AssertionError(other)))
        step("no CSP violation", lambda: not csp or (_ for _ in ()).throw(AssertionError(csp[:2])))
        b.close()
    srv.shutdown(); shutil.rmtree(tmp, ignore_errors=True)
    bad = [s for s in steps if s[1] != "ok"]
    for n, r in steps: print(f"{r:5} {n}" if r == "ok" else f"{r} -- {n}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
