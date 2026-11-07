"""DDA trio, the ways to an MS2: the Full Scan reached with the arrows keeps its scan id (markers and «open the MS2» work while walking),
the MS2 panel reads its scans from the file of the Full Scan above it (two DDA files open), the markers of a second DDA file appear without a reload,
an empty MS2 panel says why it is empty (precursors outside the m/z zoom, averaged Full Scan), with the same words for the screen reader."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_dda_ms2_vie.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("ddavie")
r = Run(port=8977, wd="/tmp/wdddavie")
TRIO = "(()=>{const s1=E.panels.find(p=>p.type==='spec'&&p.duo!=null),c=s1&&E.panels.find(p=>p.id===s1.link),s2=s1&&E.panels.find(p=>p.id===s1.duo);return {c,s1,s2}})()"
D0 = "(()=>{const t=" + TRIO + ";const a=t.s1._a;return a&&a.data&&a.data[0]&&a.data[0].d})()"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML", D / "HR_DDA-Fusion-t30.mzML"], 6000, 4)
        attendi(pg, "(()=>{const t=" + TRIO + ";return t.s1&&t.s1._a&&t.s2&&DDA.ready(t.s1.k)})()", 60000); ready(pg)
        def arrows():
            pg.evaluate("(()=>{const t=" + TRIO + ";t.c.el.scrollIntoView({block:'center'})})()")
            b = pg.evaluate("(()=>{const t=" + TRIO + ";const a=t.c._a,r=t.c.cv.getBoundingClientRect();return {x:r.left+M.l+(t.c.cur-a.x0)/(a.x1-a.x0)*(a.W-M.l-M.r),y:r.top+a.H/2}})()")
            pg.mouse.click(b["x"], b["y"]); ready(pg)
            for _ in range(3):
                pg.keyboard.press("ArrowRight"); ready(pg)
            attendi(pg, "(()=>{const d=" + D0 + ";return d&&d.scans===1})()")
            v = pg.evaluate("(()=>{const t=" + TRIO + ";const d=" + D0 + ",D=DDA.ready(t.s1.k);return {sid:d.sid,want:D.ms1.sid[D.ms1.rt.findIndex(x=>Math.abs(x-t.c.cur)<1e-6)],markN:t.s1._a.markN}})()"); print(v)
            assert v["sid"] is not None and v["sid"] == v["want"], v
            assert v["markN"] is not None, v
        step("arrows on the chromatogram: the Full Scan keeps its scan id, the DDA markers are computed", arrows)
        WHY = "(()=>{const t=" + TRIO + ";return {why:t.s2._why,say:t.s2._say,sr:TDATI_P.text(t.s2)}})()"
        def reasons():
            pg.evaluate("(async()=>{const t=" + TRIO + ";const D=await DDA.get(t.s1.k);const j=D.sid.findIndex((_,i)=>D.parent[i]!=null&&(D.byParent.get(D.parent[i])||[]).length>1);await DDA.selectMs2(t.s2,D.sid[j]);window._kids=D.byParent.get(D.parent[j]).map(i=>D.tgt[i]??D.prec[i])})()"); ready(pg)
            attendi(pg, "(()=>{const t=" + TRIO + ";return t.s2._scan&&t.s1._a&&t.s1._a.flags&&t.s1._a.flags.length>0})()")
            pg.evaluate("(()=>{const t=" + TRIO + ";const m=Math.max(..._kids);t.s1.zoom=[m+5,m+6];t.s2.sid=null;t.s2.ion=null;t.s2.avg=null;draw(t.s1);draw(t.s2)})()"); ready(pg)
            want = pg.evaluate("I18N.t('dda.why.zoom',{n:_kids.length})")
            attendi(pg, "(()=>{const t=" + TRIO + ";return t.s2._why===" + json.dumps(want) + "})()")
            v = pg.evaluate(WHY); print(v)
            assert v["why"] == want and v["say"] == want and v["sr"] == want, (v, want)
            pg.evaluate("(()=>{const t=" + TRIO + ";t.s1.zoom=null;draw(t.s1)})()"); ready(pg)
            attendi(pg, "(()=>{const t=" + TRIO + ";return t.s2._why===I18N.t('dda.click')})()")
            pg.evaluate("(()=>{const t=" + TRIO + ";t.c.el.scrollIntoView({block:'center'})})()")
            xy = lambda rt: pg.evaluate("(rt=>{const t=" + TRIO + ";const a=t.c._a,r=t.c.cv.getBoundingClientRect();return {x:r.left+M.l+(rt-a.x0)/(a.x1-a.x0)*(a.W-M.l-M.r),y:r.top+a.H/2}})", rt)
            a, b = xy(8.0), xy(8.4)                     # a few MS1 scans in the middle of the run, a drag wider than the click slop
            pg.mouse.move(a["x"], a["y"]); pg.mouse.down(); pg.mouse.move(b["x"], b["y"], steps=5); pg.mouse.up(); ready(pg)
            attendi(pg, "(()=>{const d=" + D0 + ";return d&&d.scans>1})()")
            n = pg.evaluate("(()=>{const d=" + D0 + ";return d.scans})()")
            head = pg.evaluate("n=>I18N.t('dda.why.avg0',{n}).split(':')[0]", n)
            attendi(pg, "(()=>{const t=" + TRIO + ";return (t.s2._why||'').startsWith(" + json.dumps(head) + ")})()")
            v = pg.evaluate(WHY); print(v)
            assert v["say"] == v["why"] and v["sr"] == v["why"], v
            c = xy(8.2); pg.mouse.click(c["x"], c["y"]); ready(pg)          # back to one scan for the next step
            attendi(pg, "(()=>{const d=" + D0 + ";return d&&d.scans===1})()")
        step("empty MS2 panel: the reason (outside the m/z zoom, averaged Full Scan), the same words for the screen reader", reasons)
        def other_file():
            pg.locator("#flst .fl", has_text="HR_DDA-Fusion-t30").first.click(); ready(pg)
            attendi(pg, "(()=>{const t=" + TRIO + ";return t.s1.k===E.files.find(f=>f.label==='HR_DDA-Fusion-t30').k&&t.s1._a&&t.s1._a.markN!=null})()", 30000)
            pg.evaluate("(async()=>{const t=" + TRIO + ";const D=await DDA.get(t.s1.k);await DDA.selectMs2(t.s2,D.sid[5])})()"); ready(pg)
            attendi(pg, "(()=>{const t=" + TRIO + ";return t.s2._scan&&t.s2._scan.filter})()")
            v = pg.evaluate("(()=>{const t=" + TRIO + ";return {k2:t.s2.k,file:E.files[t.s2.k].file,f1:E.files[t.s1.k].file,filter:t.s2._scan.filter,leg:t.s2.leg.textContent}})()"); print(v)
            assert v["file"].split("#")[0] == v["f1"].split("#")[0], v
            assert "ITMS" in v["filter"] and "cid35" in v["filter"] and "HCD" not in v["leg"], v
        step("second DDA file: its markers appear and the MS2 panel shows ITS scan (Fusion CID), not one of the first file", other_file)
finally:
    r.close()
report = [(n, s) for n, s in steps]
for n, s in report: print(("PASS " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else " -- " + s))
bad = [e for e in r.errs if e[0] in ("pageerror", "console.error")]
for e in bad: print("ERR", e)
sys.exit(1 if any(s != "ok" for _, s in steps) or bad else 0)
