"""Chart colours: the default is «By file» (a different colour per file, continuous lines, never grey or black); «By time» without any time falls back to the same colours; an old saved «time» default migrates."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_colori_default.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
GREY = "['#4d4d4d','#cbd5e1','#8a8a8a','#9ca3af','#000000']"
r = Run(port=8994, wd="/tmp/wd_colori_default")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 900})
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        pg.click("text=Carica dati"); ready(pg); attendi(pg, "E.files.length === 2")
        def default():
            assert pg.evaluate("UIP.pal") == "file"
        step("the default palette is «file»", default)
        def per_file():
            pg.evaluate("E.files.forEach(f => { f.time = null; f.colorSet = false; }); E.files[0].type = 'standard'; paintFiles()")
            cols = pg.evaluate("E.files.map(f => f.color.toLowerCase())")
            assert len(set(cols)) == len(cols) and not set(cols) & set(eval(GREY)), cols
            assert pg.evaluate("E.files.every(f => fdash(f).length === 0)"), "a TIC must be a continuous line"
        step("a Full Scan standard without a time: own colour, continuous line", per_file)
        def by_time_no_time():
            pg.evaluate("setPal('time'); paintFiles()")
            cols = pg.evaluate("E.files.map(f => f.color.toLowerCase())")
            assert not set(cols) & set(eval(GREY)) and pg.evaluate("E.files.every(f => fdash(f).length === 0)"), cols
        step("«by time» without times falls back to a colour per file", by_time_no_time)
        def migrate():
            pg.evaluate("localStorage.setItem('mzlab.prefs', JSON.stringify({theme: 'auto', pal: 'time', merge: true}))"); pg.reload(); attendi(pg, "typeof UIP !== 'undefined' && E.files !== undefined")
            assert pg.evaluate("UIP.pal") == "file", "old saved default must become «file»"
            pg.evaluate("localStorage.setItem('mzlab.prefs', JSON.stringify({v: 2, theme: 'auto', pal: 'time', merge: true}))"); pg.reload(); attendi(pg, "typeof UIP !== 'undefined' && E.files !== undefined")
            assert pg.evaluate("UIP.pal") == "time", "an explicit choice is kept"
        step("saved «time» migrates to «file» unless chosen (v2)", migrate)
finally:
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
