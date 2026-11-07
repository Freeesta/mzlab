"""DDA trio, the ways to an MS2: the Full Scan reached with the arrows keeps its scan id (markers and «open the MS2» work while walking),
the MS2 panel reads its scans from the file of the Full Scan above it (two DDA files open), the markers of a second DDA file appear without a reload,
an empty MS2 panel says why it is empty (precursors outside the m/z zoom, averaged Full Scan), with the same words for the screen reader,
the «N MS²» button of the Full Scan (one scan and an average: the count of the engine, the list with the keyboard) and the flags of an averaged Full Scan,
‹ › on the MS2 panel (the MS2 of the same precursor, Shift = in time) and ← → from an empty panel starting at the chromatogram cursor."""
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
        xy = lambda rt: pg.evaluate("(rt=>{const t=" + TRIO + ";const a=t.c._a,r=t.c.cv.getBoundingClientRect();return {x:r.left+M.l+(rt-a.x0)/(a.x1-a.x0)*(a.W-M.l-M.r),y:r.top+a.H/2}})", rt)
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
        BTN = "(()=>{const t=" + TRIO + ";const b=t.s1.el.querySelector('[data-a=\"ddaw\"]');return b&&{txt:b.textContent,dis:b.getAttribute('aria-disabled'),w:t.s1._dw}})()"
        def button():
            pg.evaluate("(async()=>{const t=" + TRIO + ";const D=await DDA.get(t.s1.k);const j=D.sid.findIndex((_,i)=>D.parent[i]!=null&&(D.byParent.get(D.parent[i])||[]).length>1);await DDA.selectMs2(t.s2,D.sid[j]);window._kn=D.byParent.get(D.parent[j]).length;t.s2.sid=null;t.s2.ion=null;t.s2.avg=null;draw(t.s2)})()"); ready(pg)
            attendi(pg, "(()=>{const v=" + BTN + ";return v&&v.w&&v.txt===I18N.t('dda.win.btn',{n:_kn})&&v.dis==='false'})()")
            pg.locator("[data-a='ddaw']").first.focus(); pg.keyboard.press("Enter")
            attendi(pg, "(()=>{const l=document.querySelector('.dw-pop[role=listbox]');return l&&document.activeElement===l&&l.querySelectorAll('[role=option]').length>0})()")
            pg.keyboard.press("End"); pg.keyboard.press("Enter"); ready(pg)
            v = pg.evaluate("(()=>{const t=" + TRIO + ";const g=t.s1._dw.groups;return {sid:t.s2.sid,want:g[g.length-1].sid,pop:!!document.querySelector('.dw-pop'),focus:document.activeElement&&document.activeElement.dataset.a}})()"); print(v)
            assert v["sid"] in v["want"] and not v["pop"] and v["focus"] == "ddaw", v
            a, b = xy(8.0), xy(8.4)
            pg.mouse.move(a["x"], a["y"]); pg.mouse.down(); pg.mouse.move(b["x"], b["y"], steps=5); pg.mouse.up(); ready(pg)
            attendi(pg, "(()=>{const t=" + TRIO + ";const d=" + D0 + ";return d&&d.scans>1&&t.s1._dw&&t.s1._a.flags.length>0})()")
            v = pg.evaluate("(()=>{const t=" + TRIO + ";const D=DDA.ready(t.s1.k),w=t.s1._dw;let m=0;D.ms1.sid.forEach((s,i)=>{if(D.ms1.rt[i]>=Math.min(t.s1.r0,t.s1.r1)-1e-4&&D.ms1.rt[i]<=Math.max(t.s1.r0,t.s1.r1)+1e-4)m+=(D.byParent.get(s)||[]).length});const b=t.s1.el.querySelector('[data-a=\"ddaw\"]');return {n:w.n,m,txt:b.textContent,f:t.s1._a.flags[0],r:t.s1.cv.getBoundingClientRect().toJSON()}})()"); print(v)
            assert v["n"] == v["m"] and v["txt"] == pg.evaluate("n=>I18N.t('dda.win.btn',{n})", v["n"]), v
            pg.mouse.click(v["r"]["x"] + v["f"]["px"], v["r"]["y"] + v["f"]["py"] - 8); ready(pg)
            attendi(pg, "(()=>{const t=" + TRIO + ";return t.s2.sid!=null&&t.s2._scan})()")
            v = pg.evaluate("(()=>{const t=" + TRIO + ";const d=" + D0 + ";return {scans:d.scans,sid:t.s2.sid,j:DDA.ready(t.s1.k).sid[t.s1._a.flags[0].j]}})()"); print(v)
            assert v["scans"] > 1 and v["sid"] == v["j"], v
            c = xy(8.2); pg.mouse.click(c["x"], c["y"]); ready(pg)
            attendi(pg, "(()=>{const d=" + D0 + ";return d&&d.scans===1})()")
        step("«N MS²» button: the count of the engine, the list with the keyboard; flags on an averaged Full Scan, a click keeps the average", button)
        NAV = "(()=>{const t=" + TRIO + ";const g=t.s2.el.querySelector('.ddanav');return g&&{n:g.querySelector('.ddann').textContent,prev:g.querySelector('[data-a=\"ddaprev\"]').getAttribute('aria-disabled'),sid:t.s2.sid}})()"
        def nav():
            pg.evaluate("(async()=>{const t=" + TRIO + ";const D=await DDA.get(t.s1.k),f=E.files[t.s1.k];let l=[];for(let i=0;i<D.sid.length;i++){if(D.prec[i]==null)continue;const tl=DDA._in.tolOf(f,D.prec[i]),q=[];for(let j=0;j<D.sid.length;j++)if(D.prec[j]!=null&&Math.abs(D.prec[j]-D.prec[i])<=tl)q.push(j);if(q.length>l.length)l=q}window._nv=l.map(i=>D.sid[i]);window._nx=D.sid[l[0]+1];await DDA.selectMs2(t.s2,D.sid[l[0]],{zoom:false})})()"); ready(pg)
            N = pg.evaluate("_nv.length"); assert N > 1, N
            attendi(pg, "(()=>{const v=" + NAV + ";return v&&v.n==='1/" + str(N) + "'&&v.prev==='true'})()")
            pg.evaluate("(()=>{const t=" + TRIO + ";t.s2.el.scrollIntoView({block:'center'})})()")
            pg.locator(".ddanav [data-a='ddanext']").first.click(); ready(pg)
            attendi(pg, "(()=>{const v=" + NAV + ";return v&&v.sid===_nv[1]&&v.n==='2/" + str(N) + "'})()")
            pg.locator(".ddanav [data-a='ddaprev']").first.focus(); pg.keyboard.press("Space"); ready(pg)
            attendi(pg, "(()=>{const v=" + NAV + ";return v&&v.sid===_nv[0]&&v.n==='1/" + str(N) + "'})()")
            pg.locator(".ddanav [data-a='ddanext']").first.click(modifiers=["Shift"]); ready(pg)
            attendi(pg, "(()=>{const v=" + NAV + ";return v&&v.sid===_nx})()")
            for d in (1, -1):                       # an empty panel: the walk starts at the chromatogram cursor, not at the start of the file
                v = pg.evaluate("(async d=>{const t=" + TRIO + ";const D=DDA.ready(t.s1.k),cur=D.rt[Math.floor(D.rt.length/2)]+1e-3;t.s2.sid=null;t.s2.ion=null;t.s2.avg=null;t.c.cur=cur;await DDA.step(t.s2,d);const want=d>0?D.rt.findIndex(r=>r>=cur):D.rt.length-1-[...D.rt].reverse().findIndex(r=>r<=cur);return {sid:t.s2.sid,want:D.sid[want],cur}})", d); print(v)
                assert v["sid"] == v["want"], v
        step("‹ › of the MS2 panel: the MS2 of the same precursor (i/N), Shift = next in time; ← → from an empty panel start at the cursor", nav)
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
