"""The «already an integrated peak here» notice (lang key int.clash) shows only for a click inside ANOTHER integration: not on the peak just created or edge-dragged (also after a second press/release or click),
not on its label; it does show for an older peak (also right after pressing that peak's label), after Esc or 4 s; it is role=status, ends after 2.5 s, Esc or pointer movement, and never covers a label tooltip."""
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
        CLASH = pg.evaluate("I18N.t('int.clash')")          # the notice is compared through its lang key, never a literal
        def note(): return pg.evaluate("(()=>{const d=document.querySelector('#qnote');return d&&!d.hidden?d.textContent:''})()")
        def clash(): return note() == CLASH
        def clear(): pg.evaluate("(()=>{const d=document.querySelector('#qnote');if(d)d.hidden=true})()")
        def nints(): return pg.evaluate(f"{XP}.ints.length")
        def xy(x, dy=-30): return pg.evaluate(f"(()=>{{const p={XP},a=p._a,q=p.cv.getBoundingClientRect();return {{x:q.left+a.X({x}),y:q.top+a.H-M.b+({dy})}}}})()")
        def lab(i): return pg.evaluate(f"(()=>{{const p={XP},it=p.ints[{i}],l=p._a.lbls.find(b=>b.copy!=null&&b.int===it),q=p.cv.getBoundingClientRect();return {{x:q.left+l.x+l.w/2,y:q.top+l.y+l.h/2,y1:q.top+l.y+l.h}}}})()")
        def click(c): pg.mouse.click(c["x"], c["y"]); frames(pg, 3)
        def reset(): pg.evaluate(f"(()=>{{const p={XP};p.ints=[];p._intCur=null;p.isel=null}})()"); frames(pg, 2); clear()
        A, B = xy(14.29), xy(15.9)
        def create_then_second_press():
            reset(); click(A); assert nints() == 1 and note() == "", note()
            pg.mouse.down(); pg.mouse.up(); frames(pg, 3); assert note() == "", "the mouseup that follows the creating click: " + note()     # a second press/release on the peak just made
            pg.evaluate(f"{XP}.cv.dispatchEvent(new MouseEvent('click',{{bubbles:true,clientX:{A['x']},clientY:{A['y']}}}))"); frames(pg, 2); assert note() == "" and nints() == 1
        step("no notice after creating a peak, nor on a second press/release or click on it", create_then_second_press)
        def label_then_inside():
            reset(); click(A); l = lab(0)
            pg.mouse.move(l["x"], l["y"]); frames(pg, 3); pg.mouse.click(l["x"], l["y"]); frames(pg, 3); assert note() == "", note()
            click({"x": l["x"], "y": l["y1"] + 4}); assert note() == "", "just under the label: " + note()
            click(A); assert note() == "" and nints() == 1, note()
        step("no notice on the label of the peak just made, the area under it, or the peak itself", label_then_inside)
        def real_edge_drag():
            reset(); click(A); e = xy(pg.evaluate(f"{XP}.ints[0].b")); b0 = pg.evaluate(f"{XP}.ints[0].b")
            pg.mouse.move(e["x"], e["y"]); pg.mouse.down(); pg.mouse.move(e["x"] + 25, e["y"], steps=6); pg.mouse.up(); frames(pg, 3)
            assert pg.evaluate(f"{XP}.ints[0].b") != b0, "the border did not move"
            clear(); pg.evaluate(f"{XP}._intT=Date.now()")
            click(A); assert note() == "" and nints() == 1, "click inside after an edge drag: " + note()
        step("no notice on a click inside the peak just edge-dragged", real_edge_drag)
        def older_peak():
            reset(); click(A); click(B); assert nints() == 2
            click(A); assert clash(), "a click inside the older peak: " + note()
            clear(); l = lab(0); pg.mouse.click(l["x"], l["y"]); frames(pg, 3); assert note() == "", note()     # pressing its label only copies
            click(A); assert clash() and pg.get_attribute("#qnote", "role") == "status", "after pressing the older peak's label: " + note()
        step("a click inside an older peak warns, also right after pressing its label (role=status)", older_peak)
        def esc_and_age():
            reset(); click(A); pg.keyboard.press("Escape"); click(A); assert clash(), "after Esc: " + note()
            reset(); click(A); pg.evaluate(f"{XP}._intT-=4500"); click(A); assert clash(), "after 4 s: " + note()
        step("the peak just made is no longer current after Esc or 4 s", esc_and_age)
        def dismiss():
            reset(); click(A); pg.keyboard.press("Escape"); click(A); assert clash()
            pg.evaluate("window._t0=performance.now();window._tH=0"); assert clash()
            wait_for_js(pg, "(()=>{const d=document.querySelector('#qnote');if(d.hidden&&!window._tH)window._tH=performance.now();return d.hidden})()", 4000)
            dt = pg.evaluate("window._tH-window._t0"); assert 2200 < dt < 3300, f"auto-dismiss after {dt:.0f} ms (expected about 2500)"
            click(A); assert clash(); pg.keyboard.press("Escape"); frames(pg, 2); assert note() == ""
            click(A); assert clash(); pg.mouse.move(A["x"] + 80, A["y"] + 80, steps=4); frames(pg, 2); assert note() == ""
        step("the notice ends by itself (2.5 s), with Esc and when the pointer moves away", dismiss)
        def no_cover():
            reset(); click(A); l = lab(0); pg.mouse.move(l["x"] - 20, l["y"]); pg.mouse.move(l["x"], l["y"], steps=3)
            wait_for_js(pg, f"!{XP}.tip.hidden", 4000)
            pg.evaluate("nearMsg(I18N.t('int.clash'))"); frames(pg, 2)
            bx = pg.evaluate(f"(()=>{{const r=e=>{{const b=e.getBoundingClientRect();return [b.left,b.top,b.right,b.bottom]}};return [r({XP}.tip),r(document.querySelector('#qnote'))]}})()")
            t, n = bx; assert t[2] <= n[0] or n[2] <= t[0] or t[3] <= n[1] or n[3] <= t[1], f"notice {n} covers the tooltip {t}"
        step("the notice does not cover the label's tooltip", no_cover)
        pg.screenshot(path="tests_e2e/shots/int_avviso.png")
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
