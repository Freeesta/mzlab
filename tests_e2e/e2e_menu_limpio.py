"""Spectrum menu of an MS2 spectrum: no download, no «Add to the laboratory contaminants», «Options…» with a gear (scan markers, contaminants and lists live there),
isotope entries only with a pattern drawn; the downloads are in the button of the panel; unavailable entries are not shown and no separator is left first, last or doubled."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
r = Run(port=8874, wd="/tmp/wd_menu_limpio")
try:
    with sync_playwright() as p:
        pg = r.page(p); D = synth("menu_limpio")
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); pg.evaluate("setTab('full')"); ready(pg)
        MS2 = "E.panels.find(p=>p.type==='spec'&&p.level===2&&p._a)"
        pg.evaluate("(async()=>{const D=await DDA.get(E.panels.find(p=>p.level===1&&p.type==='spec').k);await DDA.selectMs2(E.panels.find(p=>p.level===2),D.sid[0]);})(); 0")
        attendi(pg, f"!!({MS2})")
        def rows(has=True):
            pg.evaluate(f"""(()=>{{const p={MS2},a=p._a;window.__c={{vista:'spec',p,a,m:a.data[0].d.mz[0],has:{str(has).lower()},pol:1,lab:'',e:{{clientX:50,clientY:50,preventDefault(){{}}}},px:50,py:50}};
              menu({{preventDefault(){{}},clientX:50,clientY:50}},[{{label:'m/z',dim:true}},'-',...AZIONI.per(__c)])}})()""")
            out = pg.evaluate("[...document.querySelectorAll('#ctx > *')].map(d=>d.tagName==='HR'?'-':(d.dataset.azione||d.textContent))")
            pg.evaluate("document.querySelector('#ctx').hidden=true"); return out
        def no_downloads():
            ids = rows()
            for bad in ("spec.exportMgf", "spec.exportMsp", "spec.exportMassbank", "spec.exportSirius", "spec.addLab", "spec.marker", "spec.noCont", "spec.contLists"):
                assert bad not in ids, (bad, ids)
        step("no download, no «Add to the laboratory contaminants», no view toggles in the menu", no_downloads)
        def options():
            assert "spec.disp" in rows()
            pg.evaluate(f"menu({{preventDefault(){{}},clientX:50,clientY:50}},AZIONI.per(__c))")
            t = pg.evaluate("document.querySelector('#ctx [data-azione=\"spec.disp\"]').textContent.trim()"); assert t == "Opzioni…", t
            assert pg.evaluate("!!document.querySelector('#ctx [data-azione=\"spec.disp\"] .ic svg circle')"), "gear icon"
            pg.evaluate("document.querySelector('#ctx').hidden=true")
        step("«Options…» with the gear", options)
        def isotopes():
            ids = rows(); assert "spec.isoRes" not in ids and "spec.isoOff" not in ids, ids
            pg.evaluate(f"{MS2}.iso = {{formula:'C10H10O', ad:'[M+H]+'}}; 0"); ids = rows()
            assert "spec.isoOff" in ids, ids
            pg.evaluate(f"{MS2}.iso = null; 0")
        step("isotope entries only while a pattern is drawn", isotopes)
        def hidden():
            ids = rows(False)
            assert "spec.xic" not in ids and "spec.copy" not in ids, ids      # no point under the pointer: not shown, not grey
            assert ids[-1] != "-" and "- -" not in " ".join(ids), ids
            assert pg.evaluate("document.querySelectorAll('#ctx [aria-disabled=true]').length") == 0
        step("unavailable entries are not shown; no orphan or doubled separator", hidden)
        def dl_button():
            pg.evaluate(f"{MS2}.el.scrollIntoView({{block:'center'}})")
            pg.evaluate(f"{MS2}.el.querySelector('[data-a=dl]').click()"); pg.wait_for_selector("#ctx")
            t = pg.inner_text("#ctx"); assert "MGF" in t.upper() or ".mgf" in t, t
            pg.evaluate("document.querySelector('#ctx').hidden=true")
        step("the downloads are in the download button of the panel", dl_button)
        def lists():
            t = pg.evaluate(f"(()=>{{const b=document.createElement('div');DISP.panel(b,E.panels.find(p=>p.type==='spec'&&p.level===1&&p._a));return b.textContent}})()")
            assert "contaminanti" in t.lower() and "marker" in t.lower() and "togli" in t.lower(), t
        step("scan markers, contaminants and their lists are options of the spectrum", lists)
finally:
    r.close(); r.report()
print("\nSTEPS")
for n, s in steps: print(" ", n, s)
