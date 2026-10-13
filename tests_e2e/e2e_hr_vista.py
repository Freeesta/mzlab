"""HR view: MS1 at full width with its «MS1» mark, the MS2 panel under it, no marks in the rows of the files, formula -> combined XIC with its table.
Synthetic Orbitrap DDA file; in low resolution the entry of the menu and the endpoint do not exist."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_vista.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrvista")
r = Run(wd="/tmp/wd_hrvista")
ION = "C14H14F4N3O2S"
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1800})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2)
        pg.wait_for_timeout(2500)
        def panels():
            v = pg.evaluate("(()=>{const s1=E.panels.find(p=>p.type==='spec'&&p.duo!=null),s2=E.panels.find(p=>p.dda!=null);return {w1:s1.w,w2:s2.w,host:hostWidth(),y1:s1.y,h1:s1.h,y2:s2.y,l1:s1.el.querySelector('.mslv').textContent,l2:s2.el.querySelector('.mslv').textContent}})()"); print(v)
            assert v["l1"] == "MS1" and v["l2"] == "MS2", v
            assert abs(v["w1"] - v["host"]) <= 2 and abs(v["w2"] - v["host"]) <= 2 and v["y2"] >= v["y1"] + v["h1"], v
        step("MS1 at full width with its mark, the MS2 panel under it with «MS2»", panels)

        def extra():
            v = pg.evaluate("""(()=>{const s1=E.panels.find(p=>p.type==='spec'&&p.duo!=null);DDA.addMs2(s1);DDA.addMs2(s1);fitHost();return 1})()""")
            pg.wait_for_function("E.panels.filter(p=>p.dda!=null&&p.el&&p.el.querySelector('.mslv')).length===3", timeout=15000)
            v = pg.evaluate("""(()=>{const s1=E.panels.find(p=>p.type==='spec'&&p.duo!=null);
              const r=E.panels.filter(p=>p.dda===s1.id).map(p=>({x:p.x,w:p.w,y:p.y,l:p.el.querySelector('.mslv').textContent}));return {r,W:hostWidth(),w1:s1.w,y1:s1.y,h1:s1.h}})()""")
            print(v)
            assert len(v["r"]) == 3 and all(q["l"] == "MS2" and q["y"] == v["r"][0]["y"] and q["y"] >= v["y1"] + v["h1"] - 1 for q in v["r"]), v
            assert sorted(q["x"] for q in v["r"]) == [q["x"] for q in sorted(v["r"], key=lambda q: q["x"])] and len({q["x"] for q in v["r"]}) == 3, v
            assert abs(sum(q["w"] for q in v["r"]) - v["W"]) <= 2 and v["w1"] == v["W"], v
        step("two more MS2 panels sit side by side under the full-width MS1", extra)
        def no_rows():
            assert pg.locator("#flst .hrb").count() == 0
            t = pg.inner_text("#flst"); assert "centroidi" not in t and "profilo" not in t, repr(t)
        step("no marks in the rows of the files", no_rows)
        def sub():
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'))")
            pg.click("#hrbar [data-hb=add]"); pg.click("#ctx >> text=XIC da formula"); pg.wait_for_selector("#bs-f")
            pg.fill("#bs-f", ION); pg.click("#bs-go"); pg.wait_for_selector("#bs-out table tbody tr", timeout=30000)
            heads = pg.inner_text("#bs-out thead"); print(heads)
            for h in ("Sottoformula", "m/z teorica", "Δppm", "Intensità", "RT apice", "r con la formula intera"): assert h in heads, heads
            txt = pg.inner_text("#bs-out").lower(); assert "framment" not in txt and "propost" not in txt, txt[:200]
            assert pg.locator("#bs-out tbody tr", has_text=ION).count() == 1
            pg.fill("#bs-ppm", "2"); pg.click("#bs-go"); pg.wait_for_timeout(1500)
            n2 = pg.locator("#bs-out tbody tr").count(); pg.fill("#bs-ppm", "5"); pg.click("#bs-go"); pg.wait_for_timeout(1500)
            assert pg.locator("#bs-out tbody tr").count() >= n2 > 0
            pg.fill("#bs-min", "300"); pg.click("#bs-go"); pg.wait_for_timeout(1500)
            mins = pg.evaluate("[...document.querySelectorAll('#bs-out tbody tr')].map(r=>+r.cells[1].textContent)"); assert mins and min(mins) >= 299.99, mins
            pg.fill("#bs-min", "100"); pg.click("#bs-go"); pg.wait_for_timeout(1500)
            pg.click("#bs-add"); pg.wait_for_timeout(2500)
            v = pg.evaluate("(()=>{const c=E.panels.find(p=>p.ranges&&p.ranges.some(r=>r.kind==='sub'));return c&&{n:c._a.sr.length,y:Math.max(...c._a.sr[0].y),lab:c.ranges[0].label}})()"); print(v)
            assert v and v["n"] >= 1 and v["y"] > 0 and "Σ" in v["lab"], v
        step("formula -> XIC: table of measures only, ppm and lower limit change it, the XIC enters the cell", sub)
        def bad():
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'))")
            pg.click("#hrbar [data-hb=add]"); pg.click("#ctx >> text=XIC da formula"); pg.wait_for_selector("#bs-f")
            pg.fill("#bs-f", "Zz9"); pg.click("#bs-go"); pg.wait_for_timeout(800)
            assert pg.inner_text("#bs-err").strip() != ""
            pg.click("#bigx")
        step("a wrong formula gives a message", bad)
        pg.evaluate("fetch('api/new',{method:'POST',body:JSON.stringify({fresh:true})})"); pg.reload(); pg.wait_for_timeout(1500)
        load(pg, [D / "B_FullMass-t0.mzML"], 4000, 1)
        def lr():
            assert not pg.is_visible("#hrbar"), "no bench in low resolution"
            e = pg.evaluate("fetch('api/subxic?k=0&f=C10H12N2O3').then(r=>r.json())"); assert e["error_key"] == "err.subxic.lr", e
        step("low resolution: no entry in the menu, the endpoint refuses", lr)
        r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
