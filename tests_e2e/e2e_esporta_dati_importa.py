"""«Import my data» round trip (settings, group «Dati e privacy»): export, change the settings, import the same ZIP from the keyboard, check the preview names the
overwritten keys, confirm, and check the page reloads with the exported state and without any secret; a file that is not an export is refused.
Build first: python tools/prepara_sito.py"""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import json
from lib import *

AXE = axe_path()
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).replace("\n", " ")[:600]))
AXE_RUN = """() => axe.run(document.querySelector('#uipset'), {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))"""
LS = "() => ({ tunit: localStorage.getItem('mzlab.tunit'), prefs: JSON.parse(localStorage.getItem('mzlab.prefs') || '{}'), token: localStorage.getItem('mzlab.libro.token') })"
r = Run(port=8872, wd="/tmp/wd_importa")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.click("#np-set"); pg.wait_for_selector("#uipset", state="visible")
        pg.check("input[name='uip-th'][value='dark']", force=True); pg.wait_for_function("JSON.parse(localStorage.getItem('mzlab.prefs') || '{}').theme === 'dark'")
        pg.evaluate("localStorage.setItem('mzlab.tunit', 's')")
        pg.focus("#uip-g-priv > summary"); pg.keyboard.press("Enter"); pg.wait_for_selector("#uip-export", state="visible")
        pg.focus("#uip-export")
        with pg.expect_download(timeout=30000) as dl:
            pg.keyboard.press("Enter")
        zpath = "/tmp/wd_importa/export.zip"; dl.value.save_as(zpath)
        bad = "/tmp/wd_importa/bad.zip"; open(bad, "wb").write(b"not a zip at all")
        def refused():
            pg.set_input_files("#uip-import-file", bad)
            pg.wait_for_function("document.querySelector('#uip-import-st').textContent.length > 0")
            assert pg.is_hidden("#uip-import-box"), "preview shown for a bad file"
        step("a file that is not an export is refused with a message", refused)
        def change():
            pg.evaluate("() => { localStorage.setItem('mzlab.tunit', 'min'); localStorage.setItem('mzlab.libro.token', 'ghp_KEEPLOCAL'); const o = JSON.parse(localStorage.getItem('mzlab.prefs')); o.theme = 'light'; localStorage.setItem('mzlab.prefs', JSON.stringify(o)); }")
        step("settings changed after the export", change)
        def preview():
            pg.set_input_files("#uip-import-file", zpath)
            pg.wait_for_selector("#uip-import-go", state="visible")
            assert "mzlab.tunit" in pg.text_content("#uip-import-box"), pg.text_content("#uip-import-box")
            assert pg.evaluate("document.activeElement.id") == "uip-import-go"
            if AXE:
                pg.add_script_tag(path=AXE)
                v = pg.evaluate(AXE_RUN); assert not v, v
            h = pg.evaluate("document.querySelector('#uip-import-go').getBoundingClientRect().height"); assert h >= 24, h
            assert pg.evaluate(LS)["tunit"] == "min", "state changed before the confirmation"
        step("preview names the overwritten keys, focus on Ripristina, axe clean, nothing written yet", preview)
        def restore():
            with pg.expect_navigation(timeout=30000):
                pg.keyboard.press("Enter")
            pg.wait_for_selector("#np-set", state="visible")
            s = pg.evaluate(LS)
            assert s["tunit"] == "s" and s["prefs"]["theme"] == "dark", s
            assert s["token"] == "ghp_KEEPLOCAL", "a secret kept locally must not be touched: %s" % s
        step("confirm from the keyboard: the page reloads with the exported state", restore)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
