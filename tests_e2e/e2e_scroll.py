"""Scan-by-scan walk with the arrow keys: frozen axes (lock), no blank frames, last key wins, Maiusc = 5 scans, Space = play, MS2 skips empty scans."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_scroll.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.link!=null)"
r = Run(port=8860, wd="/tmp/wd60")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        def lockclick():             # the click on the lock makes the spectrum the active panel: give the chromatogram the focus back
            pg.click(".pnl.spec [data-a=lock]"); pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'&&p.tab==='full'))")
        st = lambda: pg.evaluate(f"(()=>{{const s={SPEC};return {{si:s.si,x0:s._a&&s._a.x0,x1:s._a&&s._a.x1,ymax:s._a&&s._a.ymax,lock:!!s.lock,r0:s.r0,r1:s.r1,cur:E.active&&E.active.cur,leg:s.leg.textContent}}}})()")
        # cursor a little before the peak (14.3 min) in the first full-scan chromatogram
        def put_cursor(rt=14.0):
            c = pg.evaluate("(rt)=>{const p=E.panels.find(p=>p.type==='chrom'&&p.tab==='full'),r=p.cv.getBoundingClientRect();return {px:r.left+p._a.X(rt),py:r.top+r.height/2}}", rt)
            pg.mouse.click(c["px"], c["py"]); pg.wait_for_timeout(900)
        put_cursor()
        def prepare():
            assert pg.evaluate(f"!!({SPEC})"), "no linked spectrum"
            assert pg.evaluate("!!E.active && E.active.cur != null")
            a = st(); assert a["si"] is None and not a["lock"], a                   # a click is not a walk: free axes, window request
        step("click puts the cursor, spectrum is free", prepare)
        def axes_fixed():
            pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(500)
            a0 = st(); assert not a0["lock"] and a0["si"] is not None, a0               # the lock is open by default and the arrows never close it
            lockclick(); pg.wait_for_timeout(500); a0 = st(); assert a0["lock"], a0
            assert pg.evaluate(f"document.querySelector('.pnl.spec [data-a=lock]').classList.contains('on')")
            ys = [a0["ymax"]]
            for i in range(30):
                pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(110)
                a = st(); assert a["x0"] == a0["x0"] and a["x1"] == a0["x1"], (i, a0["x0"], a["x0"], a0["x1"], a["x1"])
                assert a["si"] == a0["si"] + i + 1, (i, a["si"], a0["si"]); ys.append(a["ymax"])
            assert all(b >= a - 1e-9 for a, b in zip(ys, ys[1:])), ys                # the y axis only grows
            print("   ymax over 30 steps:", [round(v) for v in ys[::6]])
        step("30 steps: x axis identical, y never shrinks, one scan per press", axes_fixed)
        def last_wins():
            a0 = st()
            for i in range(20): pg.keyboard.press("ArrowLeft")
            pg.wait_for_timeout(1500)
            a = st(); assert a["si"] == a0["si"] - 20, (a0["si"], a["si"])
            want = pg.evaluate(f"(()=>{{const p=E.active,s=p._a.sr.find(x=>x.k==={SPEC}.k);return s.x[{a['si']}]}})()")
            assert abs((a["r0"] + a["r1"]) / 2 - want) < 0.01 and abs(a["cur"] - want) < 1e-9, (a, want)
        step("20 quick presses: the last scan is the one shown", last_wins)
        def shift5():
            a0 = st(); pg.keyboard.press("Shift+ArrowRight"); pg.wait_for_timeout(500)
            assert st()["si"] == a0["si"] + 5, (a0["si"], st()["si"])
            pg.keyboard.press("Shift+ArrowLeft"); pg.wait_for_timeout(500); assert st()["si"] == a0["si"]
        step("Maiusc + arrow = 5 scans", shift5)
        def held():
            a0 = st(); pg.keyboard.down("ArrowRight"); hold(pg, 1350); pg.keyboard.up("ArrowRight"); hold(pg, 300)
            n = st()["si"] - a0["si"]; assert 8 <= n <= 18, n                        # ~1 s of repeat at ~15 scans/s after a short delay (+ the first step)
            a1 = st(); pg.wait_for_timeout(500); assert st()["si"] == a1["si"], "must stop when the key goes up"
        step("held key: ~15 scans/s, stops when released", held)
        def play():
            a0 = st(); pg.keyboard.press(" "); pg.wait_for_timeout(1100); a1 = st(); assert a1["si"] - a0["si"] >= 6, (a0["si"], a1["si"])
            pg.keyboard.press(" "); pg.wait_for_timeout(300); a2 = st(); pg.wait_for_timeout(600); assert st()["si"] == a2["si"], "Space again must stop"
            pg.keyboard.press(" "); pg.wait_for_timeout(500); pg.keyboard.press("Escape"); pg.wait_for_timeout(300); a3 = st(); pg.wait_for_timeout(500); assert st()["si"] == a3["si"], "Esc must stop"
        step("Space plays and pauses, Esc stops", play)
        def play_sel():
            # a selection on the chromatogram: the play goes back and forth inside it
            pg.evaluate("()=>{const p=E.active;p.sel=[13.95,14.05];p.cur=13.95;draw(p)}"); pg.wait_for_timeout(300)
            pg.keyboard.press(" "); pg.wait_for_timeout(2500); cs = pg.evaluate("E.active.cur"); pg.keyboard.press(" ")
            assert 13.9 <= cs <= 14.12, cs
            assert pg.evaluate("!!E.active.sel"), "the selection stays during play"
        step("play inside a selection", play_sel)
        def lock_open_close():
            put_cursor(14.4); a = st(); assert not a["lock"], a
            pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(500); assert not st()["lock"], "arrows never close the lock by themselves"
            lockclick(); pg.wait_for_timeout(300); assert st()["lock"]
            pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(300); assert st()["lock"], "stays closed while walking"
            lockclick(); pg.wait_for_timeout(300); assert not st()["lock"]
            pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(500); assert not st()["lock"], "unlocked by hand stays unlocked"
            lockclick(); pg.wait_for_timeout(300); assert st()["lock"]
            c = pg.evaluate(f"(()=>{{const s={SPEC},r=s.cv.getBoundingClientRect();return {{x:r.left+r.width/2,y:r.top+r.height/2}}}})()")
            pg.mouse.dblclick(c["x"], c["y"]); pg.wait_for_timeout(400); assert not st()["lock"], "double click on the spectrum unlocks"
            pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(500); assert not st()["lock"], "and the arrows do not lock it again"
            lockclick(); pg.wait_for_timeout(300); assert st()["lock"]
            # a zoom made by the student redefines the locked axes; "Vista intera" unlocks
            a0 = st(); pg.evaluate(f"()=>{{const s={SPEC};s.zoom=[s._a.x0+(s._a.x1-s._a.x0)*0.3,s._a.x0+(s._a.x1-s._a.x0)*0.6];draw(s)}}"); pg.wait_for_timeout(500)
            a1 = st(); assert a1["lock"] and a1["x0"] > a0["x0"] and a1["x1"] < a0["x1"], (a0, a1)
            for i in range(5): pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(120)
            a2 = st(); assert a2["x0"] == a1["x0"] and a2["x1"] == a1["x1"], (a1, a2)
            pg.click(".pnl.spec [data-a=fit]"); pg.wait_for_timeout(400); assert not st()["lock"], "Vista intera unlocks"
            lockclick(); pg.wait_for_timeout(300); assert st()["lock"]
            put_cursor(14.0); assert not st()["lock"], "a new click on the chromatogram unlocks"
        step("lock: only the student closes it; opens with click / double click / Vista intera / new click", lock_open_close)
        def blanks():
            pg.evaluate("""()=>{window.BL={blank:0,frames:0};const s=E.panels.find(p=>p.type==='spec'&&p.link!=null);
              const pr=document.createElement('canvas');pr.width=60;pr.height=12;const pc=pr.getContext('2d',{willReadFrequently:true});
              const loop=()=>{BL.frames++;pc.clearRect(0,0,60,12);pc.drawImage(s.cv,0,0,60,12);const d=pc.getImageData(0,0,60,12).data;let n=0;for(let i=3;i<d.length;i+=4)n+=d[i];if(n===0)BL.blank++;requestAnimationFrame(loop)};loop()}""")
            for i in range(60): pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(33)
            pg.wait_for_timeout(500); b = pg.evaluate("BL"); print("   blank frames:", b); assert b["blank"] == 0, b
        step("no blank frame while walking 60 scans at 30/s", blanks)
        def cursor_overlay():
            ok = pg.evaluate("(()=>{const p=E.active,c=p.cl;return !c.hidden&&Math.abs(parseFloat(c.style.left)-(p.cv.offsetLeft+p._a.X(p.cur)))<1})()"); assert ok
        step("cursor line is an overlay following the cursor", cursor_overlay)
        pg.screenshot(path=SH + "scroll_full.png")
        def ms2():
            pg.evaluate("window.setTab('ms2')"); pg.wait_for_timeout(3500)
            info = pg.evaluate("""()=>{const p=E.panels.find(p=>p.type==='chrom'&&p.tab==='ms2');return !!p&&!!p._a}"""); assert info, "ms2 chromatogram"
            c = pg.evaluate("()=>{const p=E.panels.find(p=>p.type==='chrom'&&p.tab==='ms2'),s=p._a.sr[0];let j=s.y.indexOf(Math.max(...s.y)),r=p.cv.getBoundingClientRect();return {px:r.left+p._a.X(s.x[j]),py:r.top+r.height/2,j}}")
            pg.mouse.click(c["px"], c["py"]); pg.wait_for_timeout(900)
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'&&p.tab==='ms2'))")
            got = []
            for i in range(25):
                pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(150)
                got.append(pg.evaluate("""()=>{const p=E.active,s=p._a.sr[0],i=s.x.findIndex(v=>Math.abs(v-p.cur)<1e-9);return [i,s.y[i]]}"""))
            assert all(y > 0 for i, y in got), got                                   # never an empty scan
            idx = [i for i, y in got]; assert idx == sorted(set(idx)) and len(idx) >= 5, idx
            sp = pg.evaluate("(()=>{const s=E.panels.find(p=>p.type==='spec'&&p.link!=null&&p.tab==='ms2');return s&&s._a&&s._a.data[0].d.mz.length})()"); assert sp, "MS2 spectrum drawn"
        step("MS2: arrows jump over empty scans, with the cache", ms2)
finally:
    r.close(); r.report()
print("\nSTEPS")
for s in steps: print("  ", s)
