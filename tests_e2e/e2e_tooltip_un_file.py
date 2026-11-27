"""Hover box of a chromatogram names the file only when two or more files are loaded: with one file it shows the colour dash, the ion or channel label when several
traces share the panel, and the intensity (never the file name). One shared rule (hovName in grafici.js), read from the loaded files of the session, not from the panel."""
# covers: mzlab/web/grafici.js
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from pathlib import Path
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds; import numpy as np
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tooltip_un_file.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = HERE / "shots" / "tooltip_un_file_tmp"; D.mkdir(parents=True, exist_ok=True)
ds.full_scan(D / "un_a.mzML", 5, np.random.default_rng(1)); ds.full_scan(D / "un_b.mzML", 15, np.random.default_rng(2))
r = Run(port=8919, wd="/tmp/wd_tooltip_un_file")
def hover(pg, kind):
    """Moves the mouse over the panel of a kind and returns the text and the number of colour dashes of its hover box."""
    xp = f"E.panels.find(p=>p.type==='{kind}')"
    wait_for_js(pg, f"!!({xp}&&{xp}._a&&{xp}._a.sr&&{xp}._a.sr.length)")
    pg.evaluate(f"{xp}.el.scrollIntoView({{block:'center'}})"); frames(pg, 3)
    x, y = pg.evaluate(f"(() => {{ const p = {xp}, a = p._a, q = p.cv.getBoundingClientRect(), s = a.sr[0], k = s.y.indexOf(Math.max(...s.y)); return [q.left + a.X(s.x[k]), q.top + M.t + (a.H - M.t - M.b) * 0.5]; }})()")
    pg.mouse.move(x - 20, y); pg.mouse.move(x, y); frames(pg, 3)
    return pg.evaluate(f"(() => {{ const t = {xp}.tip; return {{ hidden: t.hidden, text: t.innerText, dashes: t.querySelectorAll('i').length, rows: t.querySelectorAll('div').length }}; }})()")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / "un_a.mzML")]); pg.wait_for_selector("text=Load data"); pg.click("text=Load data"); ready(pg)
        lab = pg.evaluate("E.files.filter(f => !f.gone)[0].label")
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'},{id:E.seq++,mz:500.2,w:0.5,label:'m/z 500'}]})")
        def one_file():
            for kind in ("chrom", "xic"):
                h = hover(pg, kind); assert not h["hidden"], (kind, "no hover box")
                assert lab not in h["text"], (kind, "the file name is in the box of a single-file session", h["text"])
                assert "min" in h["text"] and h["dashes"] >= 1, (kind, "time or colour dash missing", h)
            xt = " ".join(hover(pg, "xic")["text"].split())
            assert "m/z 364" in xt or "m/z 500" in xt, ("the ion label of a trace must stay", xt)      # two traces of one file: the label tells them apart
        step("one file loaded: no file name in the hover box of the TIC or the XIC, the ion label stays", one_file)
        def two_files():
            pg.set_input_files("#pick", [str(D / "un_b.mzML")]); wait_for_js(pg, "E.files.filter(f => !f.gone).length === 2", 60000); ready(pg)
            for kind in ("chrom", "xic"):
                h = hover(pg, kind); assert not h["hidden"], (kind, "no hover box")
                assert lab in h["text"], (kind, "the file name is missing with two files loaded", h["text"])
        step("two files loaded: the file name is in the hover box (as before)", two_files)
        def back():
            pg.evaluate("E.files[E.files.length - 1].gone = true"); frames(pg, 2); pg.evaluate("draw(E.panels.find(p=>p.type==='chrom'))"); frames(pg, 3)
            h = hover(pg, "chrom"); assert lab not in h["text"], ("the rule follows the loaded files: one left, no file name", h["text"])
        step("back to one loaded file: the file name goes away again", back)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
