"""Tabui, help and field errors (TP_Mine/js/40_tabui_testi.js, 41_tabui_aiuto.js, 41_tabui_campi.js, 20_tpmine.js, 43_trasformazioni.js):
(1) the help popover of a «?» button opens with the mouse, with the keyboard focus and with the touch, closes with Esc leaving the focus on the button, and speaks Italian
or English with the page; without qqq.admin there is no TT; (2) the fields of the form have short placeholders and labels; (3) the error «Enter a formula or SMILES.» marks the
formula and the SMILES together (aria-invalid, red double border, message linked with aria-describedby) and both return to normal on a click or a key in either; the same rule on
the fields of the list of transformations. axe: no serious or critical violation (SKIP part without axe-core).
Run from the repo root: python3 tests_e2e/e2e_tabui_aiuto.py"""
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
        pg0 = fresh(False)
        step("without qqq.admin there is no TT and no TPHELP", lambda: (lambda r: r == [False, False] or (_ for _ in ()).throw(AssertionError(r)))(pg0.evaluate("[typeof window.TT === 'function', typeof window.TPHELP === 'object']")))
        pg0.context.close()
        pg = fresh(True, "en")
        def mount():                                                         # the help of a column header, mounted by hand in #tp-right
            pg.evaluate("""() => { const d = document.createElement('div'); d.id = 'tph-test'; d.innerHTML = '<span>Level</span>' + TPHELP.button('ft.col.level', 'Level'); document.querySelector('#tp-right').prepend(d); TPHELP.attach(d); }""")
            assert pg.evaluate("TT('ft.col.level').startsWith('Identification level')")
            bb = pg.locator("#tph-test .tph-b").bounding_box(); assert bb["width"] >= 24 and bb["height"] >= 24, bb
        step("TPHELP.button is at least 24x24 px", mount)
        def mouse():
            pg.hover("#tph-test .tph-b"); pg.wait_for_selector(POP + ":not([hidden])", timeout=3000)
            assert "Identification level" in pg.inner_text(POP) and pg.get_attribute("#tph-test .tph-b", "aria-expanded") == "true"
            pg.hover(POP); lib.hold(pg, 400); assert pg.is_visible(POP), "the popover must stay open under the pointer"
            pg.mouse.move(5, 5); pg.wait_for_selector(POP, state="hidden", timeout=3000)
        step("the popover opens on hover, stays under the pointer, closes when it leaves", mouse)
        def keyboard():
            pg.focus("#tph-test .tph-b"); pg.wait_for_selector(POP + ":not([hidden])", timeout=3000)
            assert pg.get_attribute("#tph-test .tph-b", "aria-describedby") == "tph-pop"
            pg.keyboard.press("Escape"); assert not pg.is_visible(POP)
            assert pg.evaluate("document.activeElement.classList.contains('tph-b')"), "the focus must stay on the button"
            pg.keyboard.press("Enter"); pg.wait_for_selector(POP + ":not([hidden])", timeout=3000)
            pg.mouse.click(5, 5); pg.wait_for_selector(POP, state="hidden", timeout=3000)
        step("keyboard: focus opens, Esc closes and keeps the focus, a click outside closes", keyboard)
        pg.context.close()
        pt = fresh(True, "en", touch=True)
        def touch():
            pt.evaluate("""() => { const d = document.createElement('div'); d.id = 'tph-test'; d.innerHTML = TPHELP.button('ft.col.level', 'Level'); document.querySelector('#tp-right').prepend(d); TPHELP.attach(d); }""")
            pt.tap("#tph-test .tph-b"); pt.wait_for_selector(POP + ":not([hidden])", timeout=3000)
            pt.tap("#tp-name"); pt.wait_for_selector(POP, state="hidden", timeout=3000)
        step("touch: a tap opens the popover, a tap outside closes it", touch)
        pt.context.close()
        pi = fresh(True, "it")
        step("Italian page, Italian help", lambda: pi.evaluate("TT('ft.col.rt')") == "Tempo di ritenzione dell'apice, in minuti." or (_ for _ in ()).throw(AssertionError(pi.evaluate("TT('ft.col.rt')"))))
        pi.context.close()
        pg = fresh(True, "en")
        def short():
            ph = pg.evaluate("[...document.querySelectorAll('#v-tpmine .tp input[placeholder]')].map(i => i.placeholder)")
            assert all(len(x) <= 12 for x in ph), ph
            lab = pg.evaluate("[...document.querySelectorAll('#tp-left label')].map(l => l.textContent.trim())")
            assert all(len(x) <= 24 for x in lab), lab
            assert pg.get_attribute("#tp-mol", "placeholder") == "Formula" and pg.get_attribute("#tp-name", "placeholder") == "Name"
            assert not pg.query_selector("#tp-left [title*='empty = automatic']")
        step("placeholders and labels of the form are a few words", short)
        def hover_field():
            pg.hover("#tp-left .tph-b[data-tt='ft.f.formula']"); pg.wait_for_selector(POP + ":not([hidden])", timeout=3000)
            assert "C10H12N2O3S" in pg.inner_text(POP); pg.mouse.move(5, 5)
        step("the explanation of a field is in its help", hover_field)
        INV = "[document.querySelector('#tp-mol').getAttribute('aria-invalid'), document.querySelector('#tp-smi').getAttribute('aria-invalid')]"
        def field_err():
            pg.click("#tp-go")
            assert pg.inner_text("#tp-msg") == "Enter a formula or SMILES.", pg.inner_text("#tp-msg")
            assert pg.evaluate(INV) == ["true", "true"], pg.evaluate(INV)
            for s in ("#tp-mol", "#tp-smi"):
                assert pg.evaluate(f"getComputedStyle(document.querySelector('{s}')).borderTopWidth") == "2px" and pg.evaluate(f"getComputedStyle(document.querySelector('{s}')).boxShadow") != "none"
                assert pg.get_attribute(s, "aria-describedby") == "tp-msg"
        step("an empty parent marks the formula and the SMILES together", field_err)
        if AXE:
            def axe1():
                pg.add_script_tag(path=AXE); r = pg.evaluate(RUN % "document.querySelector('#tp-left')"); assert not r, r
            step("axe: the form in the error state has no serious violation", axe1)
        def clear_click():
            pg.click("#tp-smi"); assert pg.evaluate(INV) == [None, None], pg.evaluate(INV)
            assert pg.inner_text("#tp-msg") == "" and pg.get_attribute("#tp-mol", "aria-describedby") is None
        step("a click in one of the two fields returns both to normal", clear_click)
        def clear_type():
            pg.click("#tp-go"); assert pg.evaluate(INV) == ["true", "true"]
            pg.fill("#tp-mol", "C2H6O"); assert pg.evaluate(INV) == [None, None], pg.evaluate(INV)
        step("typing returns them to normal", clear_type)
        def tr_err():
            pg.fill("#tp-mol", ""); pg.wait_for_selector(".tr-add", state="attached", timeout=60000); pg.evaluate("document.querySelector('#tp-tr').closest('details').open = true")
            pg.click(".tr-go"); inv = pg.evaluate("[...document.querySelectorAll('.tr-add input')].map(i => i.getAttribute('aria-invalid'))")
            assert inv[:2] == ["true", "true"] and "Name and change" in pg.inner_text(".tr-msg"), inv
            pg.fill(".tr-n", "x"); inv = pg.evaluate("[...document.querySelectorAll('.tr-add input')].map(i => i.getAttribute('aria-invalid'))")
            assert inv[:2] == [None, None], inv
        step("the fields of the list of transformations follow the same rule", tr_err)
        b.close()
finally:
    srv.terminate()
    if not reuse: shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
