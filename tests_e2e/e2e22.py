"""Settings gear (text size, theme, chart colours): saved in localStorage, survive a reload, old saved keys (tips, num, tutorial) are ignored.
Palettes and Nuova sessione: e2e25. Synthetic data, no real lab files."""
import sys, os, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e22.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8822 + 100, wd="/tmp/wd22")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.add_init_script("try{if(!localStorage.getItem('qqq.prefs'))localStorage.setItem('qqq.prefs',JSON.stringify({tips:false,num:false,font:90,theme:'auto'}))}catch(e){}")   # a pre-simplification value
        pg.reload(); pg.wait_for_timeout(1500)
        shutil.rmtree("/tmp/s22", ignore_errors=True)
        pg.set_input_files("#pick", synth.make_series("/tmp/s22", [0, 15, 60])); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        def legacy():
            assert pg.evaluate("UIP.font") is None and pg.evaluate("fz()") == 1 and not pg.evaluate("'tips' in UIP") and pg.evaluate("document.querySelectorAll('.pnum:not([hidden])').length") >= 1
        step("an old saved value (font, tips/num false) is read: the removed options are ignored", legacy)
        def gear():
            pg.click("#np-set"); pg.wait_for_timeout(200); assert pg.is_visible("#uipset")
            assert pg.locator("#uipset [data-f]").count() == 0 and "Dimensione testo" not in pg.inner_text("#uipset"), "no text size buttons any more"
            pg.select_option("#uip-th", "dark"); pg.wait_for_timeout(500)
            assert pg.evaluate("document.documentElement.dataset.theme") == "dark"
            saved = pg.evaluate("JSON.parse(localStorage.getItem('qqq.prefs'))"); print(saved)
            assert saved == {"theme": "dark", "v": 2, "pal": "file", "merge": True}, saved
        step("gear: theme, saved in localStorage (theme, pal, merge)", gear)
        def reload():
            pg.reload(); pg.wait_for_timeout(2500)
            assert pg.evaluate("document.documentElement.dataset.theme") == "dark" and pg.evaluate("UIP.font") is None
        step("settings survive a reload", reload)
        def reset():
            pg.click("#np-set"); pg.wait_for_timeout(200)
            pg.select_option("#uip-th", "auto"); pg.wait_for_timeout(400)
            assert pg.evaluate("!document.documentElement.dataset.theme")
        step("back to defaults", reset)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
