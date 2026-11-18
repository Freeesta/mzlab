"""Tabui is hidden from the navigation (TABUI_VISIBLE = false in mzlab/web/first-paint.js) but kept: (1) the nav shows no Tabui button, (2) the command palette has no
entry that opens Tabui, (3) the exact address #/tpmine still opens the tool, typed on a fresh page and changed on a loaded one, and Back leaves it.
Run from the repo root: python3 tests_e2e/e2e_tabui_hidden.py"""
import os, subprocess, sys, tempfile, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:600]))

tmp = Path(tempfile.mkdtemp()); site = tmp / "site"
reuse = os.environ.get("MZLAB_PIPE_SITE")
if reuse: site = Path(reuse)
else:
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
    from lib_sito import add_wasm; add_wasm(site)
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).read(); break
    except Exception: __import__("threading").Event().wait(0.1)
URL = f"http://127.0.0.1:{PORT}/"
errs = []
TAB = '#nav button[data-v="tpmine"]'
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        pg = b.new_context(viewport={"width": 1400, "height": 900}).new_page(); pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector(TAB, state="attached", timeout=60000); pg.wait_for_selector('#nav button[data-v="flow"]', state="attached", timeout=60000)

        def nav_has_no_tabui():
            seen = pg.evaluate("[...document.querySelectorAll('#nav button[data-v]')].filter(b => b.getBoundingClientRect().width > 0).map(b => b.dataset.v)")
            assert "tpmine" not in seen and "flow" in seen and "data" in seen, seen
            assert not pg.locator(TAB).is_visible()
        step("the nav shows no Tabui button (Data, Drawing and Flow stay)", nav_has_no_tabui)

        def palette_has_no_tabui():
            rows = pg.evaluate("BARRA.commands.filter(c => c.run && !c.items).map(c => [c.id, I18N.t('cmd.' + c.id + '.name'), I18N.t('cmd.' + c.id + '.lab')])")
            opening = [r for r in rows if "tabui" in " ".join(r).lower() and r[0] != "mapfeat"]    # mapfeat only draws the features Tabui found on the map
            assert not opening, opening
            pg.keyboard.press("Control+k"); pg.fill("input[role=combobox]", "tabui")
            pg.wait_for_function("document.querySelector('[role=listbox]')", timeout=10000)
            txt = pg.evaluate("[...document.querySelectorAll('[role=listbox] li')].map(li => li.textContent)")
            assert not [t for t in txt if t.lower().startswith(("open", "go to")) and "tabui" in t.lower()], txt
            pg.keyboard.press("Escape")
        step("the command palette has no entry that opens Tabui", palette_has_no_tabui)

        def link_on_loaded_page():
            pg.evaluate("location.hash = '#/tpmine'"); pg.wait_for_selector("#tp-go", timeout=60000)
            assert pg.evaluate("S.view") == "tpmine" and pg.evaluate("!document.getElementById('v-tpmine').hidden")
            nav_has_no_tabui()
        step("the exact address opens Tabui on a loaded page", link_on_loaded_page)

        def back_leaves():
            pg.evaluate("history.back()"); pg.wait_for_function("S.view === 'data'", timeout=30000)
            assert pg.evaluate("document.getElementById('v-tpmine').hidden")
        step("Back leaves Tabui", back_leaves)

        def link_on_fresh_page():
            q = b.new_context(viewport={"width": 1400, "height": 900}).new_page(); q.on("pageerror", lambda e: errs.append(str(e)))
            q.goto(URL + "#/tpmine"); q.wait_for_selector("#tp-go", timeout=120000)
            assert q.evaluate("S.view") == "tpmine" and q.evaluate("!document.getElementById('v-tpmine').hidden")
            assert not q.locator(TAB).is_visible()
            q.close()
        step("the exact address typed on a fresh page opens Tabui", link_on_fresh_page)
        b.close()
finally:
    srv.terminate()
for s in steps: print(" ", s)
assert not errs, errs
assert all(r == "ok" for _, r in steps), steps
