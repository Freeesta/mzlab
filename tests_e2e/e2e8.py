"""Panel interaction: active panel + arrows = scan by scan, zoom feedback (rectangle, bar, 'Vista intera'),
PNG/CSV names and PNG metadata, right-click menu above panels after many clicks."""
import sys, os, struct, zlib; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:200]))
def pt(pg, idx, x=None):
    return pg.evaluate("""([i,x])=>{const p=E.panels[i],r=p.cv.getBoundingClientRect(),a=p._a;
        return {px:r.left+(x==null?(M.l+(a.W-M.r))/2:a.X(x)), py:r.top+r.height/2}}""", [idx, x])
r = Run(port=8818, wd="/tmp/wd8")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz(f) for f in FILES])
        pg.click("text=Carica dati"); ready(pg)
        rt = pg.evaluate("()=>{const s=E.panels[0]._a.sr[0];let m=0;s.ys.forEach((v,i)=>{if(v>s.ys[m])m=i});return s.x[m]}")
        def arrows():
            c = pt(pg, 0, rt); pg.mouse.click(c["px"], c["py"]); pg.wait_for_timeout(1500)
            assert pg.evaluate("E.active===E.panels[0] && E.panels[0].cur!=null")
            f0 = pg.evaluate("E.cur"); c0 = pg.evaluate("E.panels[0].cur")
            pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(800)
            c1 = pg.evaluate("E.panels[0].cur"); assert c1 > c0, (c0, c1)
            pg.keyboard.press("ArrowLeft"); pg.keyboard.press("ArrowLeft"); pg.wait_for_timeout(800)
            c2 = pg.evaluate("E.panels[0].cur"); assert c2 < c0, (c0, c2)
            assert pg.evaluate("E.cur") == f0, "file must not change"
            sp = pg.evaluate("({r0:E.panels[1].r0,r1:E.panels[1].r1})"); assert sp["r0"] < c2 < sp["r1"], sp
        step("active panel: arrows move scan by scan, spectrum follows", arrows)
        def files():
            pg.mouse.click(5, 5)           # outside panels (page header area) does not deactivate; click on the empty board does
            pg.evaluate("setActive(null)"); pg.click("#fmode [data-m=sel]"); f0 = pg.evaluate("E.cur")
            pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(600)
            assert pg.evaluate("E.cur") != f0
            pg.keyboard.press("ArrowLeft"); pg.wait_for_timeout(1500); assert pg.evaluate("E.cur") == f0
        step("no active panel: arrows change file", files)
        def zoom():
            i = pg.evaluate("E.panels.findIndex(p=>p.type==='spec')"); pg.evaluate(f"E.panels[{i}].zoom=null"); pg.wait_for_timeout(300)
            a = pt(pg, i, None); w = pg.evaluate(f"E.panels[{i}]._a.W")
            pg.mouse.move(a["px"] - 60, a["py"]); pg.mouse.down(); pg.mouse.move(a["px"] + 40, a["py"], steps=6)
            vis = pg.evaluate(f"!E.panels[{i}].el.querySelector('.zr').hidden"); assert vis, "rectangle not shown during drag"
            pg.screenshot(path=SH + "80_zoom_drag.png")
            pg.mouse.up(); pg.wait_for_timeout(800)
            assert pg.evaluate(f"E.panels[{i}].zoom!==null") and pg.evaluate(f"E.panels[{i}].el.querySelector('.zr').hidden")
            assert pg.evaluate(f"E.panels[{i}].el.querySelector('[data-a=fit]').disabled")==False
            pg.screenshot(path=SH + "81_zoomed.png")
            pg.locator(f".pnl.spec [data-a=fit]").first.click(); pg.wait_for_timeout(600)
            assert pg.evaluate(f"E.panels[{i}].zoom===null")
        step("spectrum zoom: rectangle while dragging, bar, Vista intera", zoom)
        def names():
            got = []
            for i in range(pg.evaluate("E.panels.length")):
                got.append(pg.evaluate(f"plotName(E.panels[{i}])"))
            print("names", got); assert all(n and len(n) <= 60 and " " not in n for n in got), got
            assert got[0].startswith("TIC_"), got
        step("plotName", names)
        def png():
            with pg.expect_download() as d: dlmenu(pg, ".pnl", "png")
            dd = d.value; path = "/tmp/w8.png"; dd.save_as(path); print("png file", dd.suggested_filename)
            b = open(path, "rb").read(); assert b[:8] == b"\x89PNG\r\n\x1a\n"
            i = 8; txt = {}
            while i < len(b):
                n, t = struct.unpack(">I4s", b[i:i+8]); body = b[i+8:i+8+n]; crc = struct.unpack(">I", b[i+8+n:i+12+n])[0]
                assert zlib.crc32(t + body) & 0xffffffff == crc, t
                if t == b"tEXt": k, v = body.split(b"\0", 1); txt[k.decode()] = v.decode("latin1")
                i += 12 + n
            print(txt); assert txt.get("Software") == "mzLab" and txt.get("Title") and txt.get("Source")
            assert dd.suggested_filename.endswith(".png") and "TIC" in dd.suggested_filename
        step("PNG name + valid tEXt metadata", png)
        def xlsn():
            with pg.expect_download() as d: dlmenu(pg, ".pnl.spec", "xlsx")
            print("xlsx file", d.value.suggested_filename); assert d.value.suggested_filename.startswith("spettro")
        step("Excel name", xlsn)
        def ctx():
            for _ in range(60):
                for i in range(pg.evaluate("E.panels.length")): pg.evaluate(f"front(E.panels[{i}].el)")
            si = pg.evaluate("E.panels.findIndex(p=>p.type==='spec')")
            top = pg.evaluate(f"(()=>{{const d=E.panels[{si}]._a.data[0].d;let m=0;d.y.forEach((v,i)=>{{if(v>d.y[m])m=i}});return d.mz[m]}})()")
            c = pt(pg, si, top); pg.mouse.click(c["px"], c["py"], button="right"); pg.wait_for_timeout(300)
            ok = pg.evaluate("""()=>{const m=document.querySelector('#ctx'),r=m.getBoundingClientRect();const e=document.elementFromPoint(r.left+r.width/2,r.top+10);return !m.hidden&&(e===m||m.contains(e))}""")
            assert ok, "menu is covered"
            pg.screenshot(path=SH + "82_ctx.png")
        step("right-click menu stays on top after many front() calls", ctx)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
