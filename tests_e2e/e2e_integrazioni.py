"""Block 1 (prompt 3): integrations never overlap, select/delete one peak, Ctrl+Z, border drag stops at the neighbour, table x button."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_integrazioni.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
XP = "E.panels.find(p=>p.type==='xic')"
r = Run(port=8883, wd="/tmp/wd83")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_selector("text=Carica dati")
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); attendi(pg, f"!!({XP}&&{XP}._a&&{XP}._a.sr&&{XP}._a.sr.length)")
        pg.evaluate(f"(()=>{{const p={XP};setActive(p);p.imode=null;p.intf=p._a.sr[0].key;p.el.scrollIntoView({{block:'center'}})}})()"); frames(pg, 2)
        def n(): return pg.evaluate(f"{XP}.ints.length")
        def add(a, b): return pg.evaluate(f"(()=>{{const p={XP};return addInt(p,p._a.sr.find(s=>s.key==='{KEY}')||p._a.sr[0],{a},{b})}})()")
        KEY = pg.evaluate(f"{XP}._a.sr[0].key")
        def xy(x, below=25):
            return pg.evaluate(f"(()=>{{const p={XP},a=p._a,s=a.sr[0],q=p.cv.getBoundingClientRect(),i=nearIdx(s.x,{x});return {{x:q.left+a.X({x}),y:Math.min(q.top+a.H-M.b-4,q.top+a.Y(a.U(s,s.ys[i]))+{below}),yb:q.top+a.H-M.b-4}}}})()")
        def free():
            assert add(13.9, 14.6) is True and n() == 1
        step("first integration accepted", free)
        def refused():
            assert add(14.3, 15.0) is False and n() == 1, "overlap must be refused"
            t = pg.evaluate("document.querySelector('#qnote')&&!document.querySelector('#qnote').hidden?document.querySelector('#qnote').textContent:''")
            assert "già un picco integrato" in t, t
        step("overlapping integration refused with a notice", refused)
        def auto_in():
            pg.evaluate(f"(()=>{{const p={XP};autoInt(p,p._a.sr[0],14.3)}})()"); assert n() == 1
        step("automatic integration inside an integrated peak refused", auto_in)
        def touch():
            assert add(14.6, 15.2) is True and n() == 2
            assert pg.evaluate(f"(()=>{{const i={XP}.ints;return Math.abs(i[0].b-i[1].a)<1e-9}})()"), "shared border"
        step("adjacent peak accepted, shared border", touch)
        def drag():
            assert add(15.6, 16.2) is True
            c = xy(15.6); y = c["yb"] - 30
            pg.mouse.move(c["x"], y); pg.mouse.down()
            t = xy(14.0); pg.mouse.move(t["x"], y, steps=8); pg.mouse.up(); frames(pg, 3)
            a = pg.evaluate(f"{XP}.ints[2].a"); print("C.a after drag", a)
            assert abs(a - 15.2) < 1e-6, a
        step("dragging a border stops at the neighbouring peak", drag)
        def sel_del():
            c = xy(14.3, 30); pg.mouse.click(c["x"], c["yb"] - 6); attendi(pg, f"{XP}.isel!=null")
            sid = pg.evaluate(f"{XP}.isel"); bid = pg.evaluate(f"{XP}.ints[0].id"); assert sid == bid, (sid, bid)
            pg.keyboard.press("Delete"); attendi(pg, f"{XP}.ints.length===2")
            assert n() == 2 and pg.evaluate(f"{XP}.ints.every(i=>i.id!=={bid})")
            pg.keyboard.press("Control+z"); attendi(pg, f"{XP}.ints.length===3"); assert n() == 3
        step("click selects a peak, Delete removes it, Ctrl+Z brings it back", sel_del)
        def order():
            pg.evaluate(f"(()=>{{const p={XP};pushZh(p);p.zoom=[13,17];draw(p);p.isel=p.ints[0].id}})()"); pg.keyboard.press("Backspace"); attendi(pg, f"{XP}.ints.length===2")
            assert n() == 2 and pg.evaluate(f"{XP}.zoom") == [13, 17], "Backspace with a selected peak removes it, not the zoom"
            pg.keyboard.press("Control+z"); attendi(pg, f"{XP}.ints.length===3"); assert n() == 3 and pg.evaluate(f"{XP}.zoom") == [13, 17]
            pg.keyboard.press("Control+z"); attendi(pg, f"{XP}.zoom==null"); assert pg.evaluate(f"{XP}.zoom") is None
        step("Ctrl+Z undoes the last action, integration or zoom", order)
        def menu():
            labs = pg.evaluate(f"(()=>{{const p={XP};return intMenuItems(p,14.9,null,0).map(i=>i.label||i)}})()")
            assert labs[0] == "Elimina questa integrazione" and "Elimina tutte le integrazioni di questo pannello" in labs, labs
        step("context menu: delete this one first, then all", menu)
        def table():
            pg.evaluate("showInts()"); attendi(pg, "document.querySelectorAll('button[data-del]').length===3")
            assert pg.locator("button[data-del]").count() == 3
            pg.locator("button[data-del]").first.click(); attendi(pg, "document.querySelectorAll('button[data-del]').length===2")
            assert pg.locator("button[data-del]").count() == 2 and n() == 2
        step("table: × deletes the row and the peak from the graph", table)
        def legacy():
            pg.evaluate(f"(()=>{{const p={XP},i=p.ints[0];p.ints.push({{...i,id:E.seq++,a:i.a+0.1,b:i.b+0.1}});showInts()}})()"); attendi(pg, "document.querySelector('#bigbody').innerText.includes('sovrapposto')")
            assert "sovrapposto" in pg.inner_text("#bigbody")
        step("old overlapping integrations load and are marked", legacy)
        pg.evaluate("document.querySelector('#bigdlg')&&document.querySelector('#bigdlg').open&&document.querySelector('#bigdlg').close()")
        pg.screenshot(path="tests_e2e/shots/integrazioni.png")
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
