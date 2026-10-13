"""HR spectrum (DDA trio): triangles only under the pointer when the Marker switch is off, right-click menu of a peak (open the MS2 scan first, then formula and
display options, grouped, with aria-labels), «remove contaminants» (Keller list) undoable, palette commands. iPad viewport for the hover by touch."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_spettro_hr.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("spettrohr")
r = Run(port=8863, wd="/tmp/wd_spettrohr")
S1 = "(()=>{const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='full');return E.panels.find(p=>p.type==='spec'&&p.link===c.id)})()"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2); ready(pg)
        pg.evaluate(f"{S1}.el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(500)
        t = pg.evaluate("(()=>{const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='full');c.el.scrollIntoView({block:'center'});if(!c.ms2tri){c.ms2tri=true;draw(c)}const tr=c._a.tri.filter(q=>q.rt>13.5&&q.rt<15)[0];const r=c.cv.getBoundingClientRect();return [r.left+tr.px,r.top+M.t+4]})()")
        pg.mouse.click(t[0], t[1]); ready(pg); pg.wait_for_timeout(1500)
        pg.evaluate(f"{S1}.el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(400)
        def geo(): return pg.evaluate(f"(()=>{{const s={S1},r=s.cv.getBoundingClientRect();return {{n:s._a.flags.length,markN:s._a.markN,on:markOn(s),f:s._a.flags.map(q=>[r.left+q.px,r.top+q.py+14,q.j]),l:r.left,t:r.top}}}})()")
        def switch():
            g = geo(); assert g["n"] >= 1 and g["markN"] >= g["n"], g
            assert g["on"] is True, "few peaks: markers on by themselves"
            pg.evaluate(f"{S1}.mk = false; draw({S1})"); ready(pg)
            assert pg.evaluate(f"markOn({S1})") is False
            g = geo(); x, y, j = g["f"][0]; pg.mouse.move(x - 40, y); pg.mouse.move(x, y); pg.wait_for_timeout(300)
            assert pg.evaluate(f"{S1}._mk") == j, "hover shows the triangle of that peak"
            pg.mouse.move(x, 5); pg.wait_for_timeout(100); pg.mouse.move(g["l"] + 5, g["t"] + 5); pg.wait_for_timeout(300)
            assert pg.evaluate(f"{S1}._mk") is None, "and it goes away"
        step("Marker off: triangle only under the pointer", switch)
        def menu():
            pg.evaluate(f"{S1}.mk = null; draw({S1})"); ready(pg)
            g = geo(); x, y, j = g["f"][0]
            pg.mouse.click(x, y + 10, button="right"); pg.wait_for_selector("#ctx:not([hidden])")
            items = pg.evaluate("[...document.querySelectorAll('#ctx [role^=menuitem]')].map(e=>[e.textContent.trim(),e.getAttribute('aria-label')])")
            hrs = pg.evaluate("document.querySelectorAll('#ctx hr').length")
            labs = [i[0] for i in items]; print(labs)
            first = [i for i in items if not i[0].startswith(("m/z", "Ripristina"))]
            assert first[0][0].startswith("Apri la scansione MS²") and first[0][1], first[:2]
            assert "ormula" in first[1][0], first[:3]
            assert any("pzioni di visualizzazione" in i[0] for i in first[:5]) and hrs >= 3, (first, hrs)
            assert pg.evaluate(f"DDA.ms2For({S1}, {S1}._pm).length") >= 1
            pg.click("#ctx [role^=menuitem]:has-text('Apri la scansione MS²')"); ready(pg); pg.wait_for_timeout(800)
            assert pg.evaluate("E.panels.find(p=>p.dda!=null).sid") is not None
        step("right click on a peak: open the MS2 scan first, groups, aria-labels", menu)
        def cont():
            top = pg.evaluate(f"(()=>{{const d={S1}._a.data[0].d;let b=0;d.y.forEach((v,i)=>{{if(v>d.y[b])b=i}});return [d.mz[b],d.mz.length]}})()")
            pg.evaluate(f"window.__k=LISTE.keller; LISTE.keller=()=>m=>Math.abs(m-{top[0]})<0.01; {S1}.noCont=true; draw({S1})"); ready(pg); pg.wait_for_timeout(500)
            v = pg.evaluate(f"({{n:{S1}._nCont,mz:{S1}._a.data[0].d.mz.length,leg:{S1}.leg.textContent}})")
            assert v["n"] >= 1 and v["mz"] == top[1] - v["n"] and "contaminant" in v["leg"], v
            pg.evaluate(f"{S1}.noCont=false; draw({S1})"); ready(pg); pg.wait_for_timeout(400)
            assert pg.evaluate(f"{S1}._a.data[0].d.mz.length") == top[1], "undone: all the peaks are back"
            pg.evaluate("LISTE.keller=window.__k")
        step("remove contaminants takes the Keller peaks out and puts them back", cont)
        def pal():
            ids = pg.evaluate("BARRA.commands.filter(c=>!c.visible||c.visible()).map(c=>c.id)")
            assert "marker" in ids and "nocont" in ids, ids
        step("the palette has the new commands", pal)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
