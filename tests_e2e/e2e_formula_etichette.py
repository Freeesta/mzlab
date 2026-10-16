"""Tabui never shows a bare «Formula»: the form says «Neutral formula» (placeholder, label, aria-label, help) and no label, header or placeholder of the tab is just «formula» (case-insensitive).
Run from the repo root: python3 tests_e2e/e2e_formula_etichette.py"""
import os, shutil, subprocess, sys, time, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:600]))

tmp = Path(tempfile.mkdtemp()); site = tmp / "site"
reuse = os.environ.get("MZLAB_PIPE_SITE")
if reuse: site = Path(reuse)
else:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import urllib.request
for _ in range(100):                                                  # the server answers: no fixed pause
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).read(); break
    except Exception: __import__("threading").Event().wait(0.1)
errs = []
AXE = lib.axe_path()
TAB = '#nav button[data-v="tpmine"]'
POP = "#tph-pop"
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length, v.nodes.slice(0,2).map(n => n.target.join(' '))]))"
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh(admin=True, lang="en", touch=False):
            c = b.new_context(viewport={"width": 1400, "height": 1000}, has_touch=touch, reduced_motion="reduce"); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            if admin: pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/?lang={lang}"); pg.wait_for_selector("#drop", timeout=120000)
            if admin:
                pg.wait_for_selector(TAB, timeout=30000); pg.wait_for_function("window.QTOOLS.list().some(t => t.id === 'tpmine')", timeout=60000)
                pg.click(TAB); pg.wait_for_selector("#tp-go")
            return pg
        pg = fresh(True, "en")
        def form():
            assert pg.get_attribute("#tp-mol", "placeholder") == "Neutral formula"
            assert "formula" in (pg.get_attribute("#tp-mol", "aria-label") or "").lower()
        step("the parent field says «Neutral formula»", form)
        def scan():
            bad = pg.evaluate("""() => { const out = []; const bare = s => /^\\s*formulas?\\s*$/i.test(s || '');
              for (const e of document.querySelectorAll('#v-tpmine label, #v-tpmine th, #v-tpmine summary, #v-tpmine h2, #v-tpmine h3, #v-tpmine h4')) if (bare(e.textContent.replace(/\\?$/, ''))) out.push(e.tagName + ':' + e.textContent);
              for (const e of document.querySelectorAll('#v-tpmine [placeholder], #v-tpmine [aria-label]')) if (bare(e.getAttribute('placeholder')) || bare(e.getAttribute('aria-label'))) out.push(e.id || e.tagName);
              return out; }""")
            assert not bad, bad
        step("no label, header or placeholder of the tab is a bare «Formula»", scan)
        def hb():
            t = pg.evaluate("[...document.querySelectorAll('#tp-left .tph-b')].map(b => b.getAttribute('aria-label') || '')")
            assert not [x for x in t if x.strip().lower() in ("formula", "help: formula")], t
        step("the help buttons do not name a bare «Formula»", hb)
        b.close()
finally:
    srv.terminate()
    if not reuse: shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
