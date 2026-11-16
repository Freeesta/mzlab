"""«There is already an integrated peak here» shows only for a click inside ANOTHER integration, never on the peak just made, its label or its handles; Esc and moving away dismiss it."""
# covers: mzlab/web/picchi.js, mzlab/web/area-copia.js, mzlab/web/eventi.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_int_avviso.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
XP = "E.panels.find(p=>p.type==='xic')"
r = Run(port=8892, wd="/tmp/wd92")
try:
    with sync_playwright() as p:
        pg = r.page(p, motion=True)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_selector("text=Carica dati")
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); wait_for_js(pg, f"!!({XP}&&{XP}._a&&{XP}._a.sr&&{XP}._a.sr.length)")
        pg.evaluate(f"(()=>{{const p={XP};setActive(p);p.imode='auto';p.intf=p._a.sr[0].key;p.el.scrollIntoView({{block:'center'}})}})()"); frames(pg, 2)
        def note(): return pg.evaluate("(()=>{const d=document.querySelector('#qnote');return d&&!d.hidden?d.textContent:''})()")
        def clear(): pg.evaluate("(()=>{const d=document.querySelector('#qnote');if(d)d.hidden=true})()")
        def xy(x, dy=-30): return pg.evaluate(f"(()=>{{const p={XP},a=p._a,q=p.cv.getBoundingClientRect();return {{x:q.left+a.X({x}),y:q.top+a.H-M.b+({dy})}}}})()")
        def lab(): return pg.evaluate(f"(()=>{{const p={XP},l=p._a.lbls.find(b=>b.copy!=null),q=p.cv.getBoundingClientRect();return {{x:q.left+l.x+l.w/2,y:q.top+l.y+l.h/2,y1:q.top+l.y+l.h}}}})()")
        c = xy(14.29); pg.mouse.click(c["x"], c["y"]); frames(pg, 4)
        def created():
            assert pg.evaluate(f"{XP}.ints.length") == 1 and note() == "", note()
        step("no warning right after creating an integration", created)
        def label_false():
            l = lab(); pg.mouse.move(l["x"], l["y"]); frames(pg, 3); assert note() == "", note()
            pg.mouse.click(l["x"], l["y"]); frames(pg, 3); assert note() == "", note()
            pg.mouse.click(l["x"], l["y1"] + 4); frames(pg, 3); assert note() == "", "click just under the label: " + note()
        step("no warning on label hover, label click or the area under it", label_false)
        def inside_same():
            pg.mouse.click(c["x"], c["y"]); frames(pg, 3); assert note() == "", note()
            assert pg.evaluate(f"{XP}.ints.length") == 1
        step("clicking the peak just made only selects it", inside_same)
        def handles():
            e = xy(pg.evaluate(f"{XP}.ints[0].a")); pg.mouse.move(e["x"], e["y"]); pg.mouse.down(); pg.mouse.up(); frames(pg, 3); assert note() == "", note()
        step("no warning on the edge handles", handles)
        def real():
            n = pg.evaluate(f"(()=>{{const p={XP},s=p._a.sr[0];p._intCur=null;return addInt(p,s,15.6,16.2)}})()"); assert n is True; frames(pg, 3); clear()
            pg.mouse.click(c["x"], c["y"]); frames(pg, 3)   # 14.29 is inside the first peak, which is not the one last made
            assert "già un picco integrato" in note(), note()
            assert pg.get_attribute("#qnote", "role") == "status"
        step("a click inside another integration warns (role=status)", real)
        def dismiss():
            pg.keyboard.press("Escape"); frames(pg, 2); assert note() == ""
            pg.mouse.click(c["x"], c["y"]); frames(pg, 3); assert note() != ""
            pg.mouse.move(c["x"] + 80, c["y"] + 80, steps=4); frames(pg, 2); assert note() == ""
        step("Esc and moving the pointer away dismiss it", dismiss)
        pg.screenshot(path="tests_e2e/shots/int_avviso.png")
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
