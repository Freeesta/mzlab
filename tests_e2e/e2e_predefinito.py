"""Desktop files opened from outside and the notice «make mzLab the default app?» (mzlab/web/predefinito.js) with a simulated Tauri:
the queue goes into the Data view, the dialog (Yes / Not now / Don't ask again, Esc, focus, labels), the guide for the system, the event of a second launch, the settings row."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_predefinito.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
FAKE = """window.MZLAB_DESKTOP = true; window.__calls = []; window.__open = {files: [], ask: false}; window.__handlers = {}; window.__ups = [];
window.MZLAB_TOFILE = f => ({ fake: f.name, id: f.id });
window.__TAURI__ = { core: { invoke: async (c, a) => { window.__calls.push(a && a.answer ? c + ':' + a.answer : c);
    if (c === 'take_opened') { const o = window.__open; window.__open = {files: [], ask: false}; return o; }
    return JSON.stringify({}); } },
  event: { listen: async (n, f) => { window.__handlers[n] = f; return () => {}; } } };"""
r = Run(port=8927, wd="/tmp/wd_predefinito")
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        ctx = r.b.new_context(viewport={"width": 1400, "height": 900})
        pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.upd.last',String(Date.now()));localStorage.setItem('mzlab.rel.last',String(Date.now()))}catch(e){}")
        pg.add_init_script(FAKE)
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_function("window.DESKDEF && typeof window.upload === 'function'", timeout=20000)
        pg.evaluate("() => { window.upload = (files, into) => { window.__ups.push([files.map(f => f.fake), into]); }; }")
        F = lambda *n: [{"id": i + 1, "name": x, "size": 10} for i, x in enumerate(n)]
        def arrive(files, ask, via_event=False):
            pg.evaluate("() => { window.__calls = []; window.__ups = []; }")
            pg.evaluate("o => { window.__open = o; }", {"files": files, "ask": ask})
            if via_event: pg.evaluate("() => window.__handlers['mzlab-open']({ payload: null })")
            else: pg.evaluate("DESKDEF.take()")
            pg.wait_for_function("window.__calls.includes('take_opened')")
        def dlg(): return pg.query_selector("#predefdlg[open]")
        def opened_into_data():
            arrive(F("a.mzML", "b.raw"), False)
            pg.wait_for_function("window.__ups.length === 1")
            assert pg.evaluate("window.__ups") == [[["a.mzML", "b.raw"], "data"]], pg.evaluate("window.__ups")
            assert dlg() is None
        step("a file opened from outside goes into the Data view, no question when it is not due", opened_into_data)
        def nothing():
            arrive([], True); pg.wait_for_timeout(200)
            assert pg.evaluate("window.__ups") == [] and dlg() is None
        step("an empty queue does nothing, not even the dialog", nothing)
        def event():
            arrive(F("c.mzML"), False, via_event=True); pg.wait_for_function("window.__ups.length === 1")
            assert pg.evaluate("window.__ups[0][0]") == ["c.mzML"]
        step("a second launch (event mzlab-open) takes the files into the open window", event)
        def labelled():
            pg.focus("body"); arrive(F("d.mzML"), True); pg.wait_for_selector("#predefdlg[open]")
            d = dlg()
            assert pg.evaluate("document.getElementById('predef-t').textContent") == pg.evaluate("I18N.t('predef.title')")
            assert d.get_attribute("aria-labelledby") == "predef-t" and d.get_attribute("aria-describedby") == "predef-d"
            names = pg.evaluate("[...document.querySelectorAll('#predefdlg button')].map(b => b.textContent)")
            assert names == [pg.evaluate(f"I18N.t('{k}')") for k in ("predef.yes", "predef.later", "predef.never")], names
            assert pg.evaluate("document.activeElement.textContent") == names[0], "focus starts on the first button"
            assert all(b["width"] >= 24 and b["height"] >= 24 for b in pg.evaluate("[...document.querySelectorAll('#predefdlg button')].map(b => b.getBoundingClientRect().toJSON())"))
        step("the dialog is modal, labelled, with the focus inside and targets of at least 24 px", labelled)
        def tab_stays():
            for _ in range(6):
                pg.keyboard.press("Tab"); assert pg.evaluate("document.activeElement === document.body || !!document.activeElement.closest('#predefdlg')")
        step("Tab never leaves the modal dialog for the page behind", tab_stays)
        def later():
            pg.click("#predefdlg button:nth-child(2)"); pg.wait_for_function("!document.getElementById('predefdlg')")
            assert "assoc_answer:no" in pg.evaluate("window.__calls")
        step("Not now answers «no» and closes", later)
        def esc():
            arrive(F("e.mzML"), True); pg.wait_for_selector("#predefdlg[open]"); pg.keyboard.press("Escape"); pg.wait_for_function("!document.getElementById('predefdlg')")
            assert "assoc_answer:no" in pg.evaluate("window.__calls")
        step("Esc is «Not now»", esc)
        def never():
            arrive(F("f.mzML"), True); pg.wait_for_selector("#predefdlg[open]"); pg.click("#predefdlg button:nth-child(3)"); pg.wait_for_function("!document.getElementById('predefdlg')")
            assert "assoc_answer:mai" in pg.evaluate("window.__calls")
        step("Don't ask again answers «mai»", never)
        def yes():
            arrive(F("g.mzML"), True); pg.wait_for_selector("#predefdlg[open]"); pg.click("#predefdlg button:nth-child(1)")
            pg.wait_for_function("document.querySelectorAll('#predefdlg button').length === 1")
            assert "assoc_answer:si" in pg.evaluate("window.__calls")
            txt = pg.inner_text("#predefdlg"); assert "xdg-mime default mzlab.desktop" in txt, txt     # headless Chromium reports Linux
            assert pg.evaluate("document.activeElement.textContent") == pg.evaluate("I18N.t('predef.done')")
            pg.keyboard.press("Enter"); pg.wait_for_function("!document.getElementById('predefdlg')")
        step("Yes records «si» and shows the guide of the system, with a Close button that has the focus", yes)
        def settings_row():
            pg.click("#np-set"); pg.wait_for_selector("#uipset"); pg.evaluate("document.getElementById('uip-g-app').open = true")
            pg.wait_for_selector("#uip-predef"); pg.evaluate("window.__calls = []"); pg.click("#uip-predef")
            pg.wait_for_selector("#predefdlg[open]")
            assert "assoc_reset" in pg.evaluate("window.__calls")
        step("the setting asks again at once and forgets the answer", settings_row)
        def lang_en():
            pg.goto(f"http://127.0.0.1:{r.port}/?lang=en"); pg.wait_for_function("window.DESKDEF", timeout=20000)
            pg.evaluate("DESKDEF.ask()"); pg.wait_for_selector("#predefdlg[open]")
            assert "default app" in pg.inner_text("#predefdlg") and "Don't ask again" in pg.inner_text("#predefdlg")
        step("the texts come from the English catalog too", lang_en)
finally:
    r.close()
for s in steps: print(s)
print("ERRORS", [e for e in r.errs if e[0] != "console.warning"][:5])
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
