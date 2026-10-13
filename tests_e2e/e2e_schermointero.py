"""Block 2 (prompt 3): full screen = only that panel exists (the rest is inert), every tool works inside, the panels underneath do not change."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_schermointero.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
CH = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full')"
r = Run(port=8885, wd="/tmp/wd85")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate(f"{CH}.el.scrollIntoView({{block:'start'}})"); pg.wait_for_timeout(400)
        snap = lambda: pg.evaluate(f"(()=>{{const s={SPEC},c={CH};return JSON.stringify({{sz:s.zoom,sr:[s.r0,s.r1],sk:s.k,si:s.si,cc:c.cur,cz:c.zoom,cur:E.cur,n:E.panels.length,files:E.files.map(f=>f.vis)}})}})()")
        # zoom the chromatogram a little, put the cursor, then go full screen on the SPECTRUM
        pg.evaluate(f"(()=>{{const c={CH};c.cur=14.3;c.zoom=[13,16];draw(c);pushLinked(c,14.25,14.35,c.k??0)}})()"); pg.wait_for_timeout(1200)
        before = snap()
        def enter():
            pg.evaluate(f"{SPEC}.el.querySelector('[data-a=max]').click()"); pg.wait_for_timeout(900)
            assert pg.evaluate(f"{SPEC}.el.classList.contains('max')")
            inert = pg.evaluate("[...document.querySelectorAll('header,#dfiles,#tools,#dpanels>.pnl')].filter(e=>!e.classList.contains('max')).every(e=>e.hasAttribute('inert'))")
            assert inert, "everything else is inert"
            assert not pg.evaluate(f"{SPEC}.el.hasAttribute('inert')")
        step("full screen: the rest is inert", enter)
        def zoom_and_dbl():
            c = pg.evaluate(f"(()=>{{const p={SPEC},q=p.cv.getBoundingClientRect(),a=p._a;return {{x0:q.left+a.X(300),x1:q.left+a.X(380),y:q.top+q.height/2}}}})()")
            pg.mouse.move(c["x0"], c["y"]); pg.mouse.down(); pg.mouse.move(c["x1"], c["y"], steps=5); pg.mouse.up(); pg.wait_for_timeout(600)
            z = pg.evaluate(f"{SPEC}.zoom"); assert z and z[1] - z[0] < 100, z
            pg.mouse.dblclick((c["x0"] + c["x1"]) / 2, c["y"]); pg.wait_for_timeout(600)
            assert pg.evaluate(f"{SPEC}.zoom") is None, "double click = whole view, also in full screen"
        step("tools inside: zoom with a box, double click = whole view", zoom_and_dbl)
        def keys():
            for k in ("ArrowRight", "ArrowRight", "ArrowLeft", "1", "2"): pg.keyboard.press(k)
            pg.wait_for_timeout(500)
            assert pg.evaluate(f"{SPEC}.el.classList.contains('max')")
            now = pg.evaluate(f"JSON.parse({snap.__name__ and 'null'})") if False else None
            after = snap(); b, a = __import__("json").loads(before), __import__("json").loads(after)
            assert a["cur"] == b["cur"] and a["cc"] == b["cc"] and a["sr"] == b["sr"] and a["cz"] == b["cz"] and a["n"] == b["n"], (b, a)
        step("arrows and number keys do not touch the other panels or the files", keys)
        def menu_inside():
            c = pg.evaluate(f"(()=>{{const p={SPEC},q=p.cv.getBoundingClientRect(),a=p._a,d=a.data[0].d;let b=0;d.y.forEach((v,i)=>{{if(v>d.y[b])b=i}});return {{x:q.left+a.X(d.mz[b]),y:q.top+a.Y(d.y[b])+3}}}})()")
            pg.mouse.click(c["x"], c["y"], button="right"); pg.wait_for_timeout(300)
            vis = pg.evaluate("(()=>{const m=document.querySelector('#ctx');if(!m||m.hidden)return null;const r=m.getBoundingClientRect();return [r.left,r.top,r.right,r.bottom,innerWidth,innerHeight, document.elementFromPoint(r.left+10,r.top+10).closest('#ctx')!=null]})()")
            assert vis and vis[6] and vis[2] <= vis[4] and vis[3] <= vis[5], vis
            pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
            assert pg.evaluate(f"{SPEC}.el.classList.contains('max')"), "Esc closes the menu first"
        step("the right-click menu opens on top of the full-screen panel", menu_inside)
        def exit_():
            pg.keyboard.press("Escape"); pg.wait_for_timeout(700)
            assert not pg.evaluate(f"{SPEC}.el.classList.contains('max')")
            assert pg.evaluate("document.querySelectorAll('[inert]').length") == 0
            a = __import__("json").loads(snap()); b = __import__("json").loads(before)
            assert a["cz"] == b["cz"] and a["cc"] == b["cc"] and a["sr"] == b["sr"] and a["cur"] == b["cur"], (b, a)
        step("leaving restores everything as it was", exit_)
        def chrom_fs():
            pg.evaluate(f"{CH}.el.querySelector('[data-a=max]').click()"); pg.wait_for_timeout(900)
            n0 = pg.evaluate("E.panels.length")
            c = pg.evaluate(f"(()=>{{const p={CH},q=p.cv.getBoundingClientRect(),a=p._a;return {{x0:q.left+a.X(14),x1:q.left+a.X(15),y:q.top+q.height/2}}}})()")
            pg.mouse.move(c["x0"], c["y"]); pg.mouse.down(); pg.mouse.move(c["x1"], c["y"], steps=5); pg.mouse.up(); pg.wait_for_timeout(300)
            pg.mouse.dblclick(c["x0"] + 20, c["y"]); pg.wait_for_timeout(600)
            assert pg.evaluate("E.panels.length") == n0, "no new spectrum is made behind the full-screen panel"
            assert pg.evaluate(f"{CH}.zoom") is None
            pg.screenshot(path=SH + "schermointero.png")
            pg.keyboard.press("Escape"); pg.wait_for_timeout(500)
        step("chromatogram in full screen: double click = whole view, no new panel", chrom_fs)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
