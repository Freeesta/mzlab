"""V-5: lens of the spectrum (low and high resolution): hovering a peak shows the XIC of that ion (one file) with the same numbers as api/xic (E1), one request in
flight and the last peak wins on a fast sweep, Esc closes, click or Enter makes it the XIC panel of the button, keyboard only, data table, axe, preference off."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
AXE = axe_path()
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length]))"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_xic_lente.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
XP = "E.panels.find(p=>p.type==='xic')"
PEAKS = "(()=>{const p=%s,q=p.cv.getBoundingClientRect(),a=p._a,d=a.data[0].d,out=[];(a.lbls||[]).forEach(l=>{if(l.m==null)return;let b=0;d.mz.forEach((v,i)=>{if(Math.abs(v-l.m)<Math.abs(d.mz[b]-l.m))b=i});if(!out.some(o=>o.mz==l.m))out.push({mz:l.m,x:q.left+a.X(d.mz[b]),y:q.top+a.Y(d.y[b])+8})});return out.sort((u,v)=>u.x-v.x)})()"
LENS = ".pnl .xl:not([hidden])"
TRACK = "(()=>{if(window.__xl)return;const o=window.fetch;window.__xl={n:0,cur:0,max:0};window.fetch=function(u){const k=String(u&&u.url||u);if(k.includes('xicbin')){__xl.n++;__xl.cur++;__xl.max=Math.max(__xl.max,__xl.cur);return o.apply(this,arguments).finally(()=>{__xl.cur--})}return o.apply(this,arguments)}})()"
def lens_trace(pg, S):
    return pg.evaluate(f"(()=>{{const p={S},t=XICLENS.trace(p);return t&&{{rt:t.rt,y:t.y,m:XICLENS.shown(p)}}}})()")
def api_trace(pg, S, m, K):
    return pg.evaluate(f"(()=>{{const p={S},r=XICBTN.lensArgs(p,{m});return J('api/xic?k='+r.f.k+'&mz='+r.args[0]+'&tol='+r.args[1]+'&level='+r.f.lv).then(j=>j.traces[0])}})()")
def same(a, b): return a["y"] == b["y"] and a["rt"] == (b.get("rt") or b.get("x"))
r = Run(port=8879, wd="/tmp/wd79")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        K = pg.evaluate("E.files[0].k")
        pg.evaluate(TRACK)
        def pk(): return pg.evaluate(PEAKS % SPEC)
        def hover_lr():
            ps = pk(); assert len(ps) >= 5, len(ps)
            t = ps[2]; pg.mouse.move(t["x"] - 40, t["y"]); pg.mouse.move(t["x"], t["y"], steps=3)
            pg.wait_for_selector(LENS, timeout=10000); frames(pg, 3)
            got = lens_trace(pg, SPEC); assert got and got["m"] == t["mz"], (got and got["m"], t["mz"])
            assert same(got, api_trace(pg, SPEC, t["mz"], K)), "the lens differs from api/xic"
            box = pg.evaluate("(()=>{const b=document.querySelector('.pnl .xl:not([hidden])').getBoundingClientRect();return [b.width,b.height]})()")
            assert box[0] >= 160 and box[1] >= 90, box
            assert pg.evaluate(f"(()=>{{const l=document.querySelector('.pnl .xl').getBoundingClientRect();return l.left>{t['x']}+2||l.right<{t['x']}-2}})()"), "the lens covers the peak under the pointer"
        step("LR: hovering a peak shows the lens, same traces as api/xic (Object.is), size, not over the peak", hover_lr)
        def sweep():
            ps = pk(); pg.keyboard.press("Escape"); pg.mouse.move(2, 2); frames(pg, 3)
            pg.evaluate("window.__xl.max=0;window.__xl.n=0")
            order = (ps * 3)[:20]
            for t in order: pg.mouse.move(t["x"], t["y"])
            last = order[-1]
            pg.wait_for_function(f"XICLENS.shown({SPEC})==={last['mz']}&&XICLENS.trace({SPEC})", timeout=10000)
            st = pg.evaluate("window.__xl"); print(st)
            assert st["max"] <= 1, st
            assert same(lens_trace(pg, SPEC), api_trace(pg, SPEC, last["mz"], K))
        step("LR: a fast sweep over 20 peaks: one request in flight, the last peak wins", sweep)
        def esc():
            pg.keyboard.press("Escape"); frames(pg, 2); assert not pg.query_selector(LENS)
            ps = pk(); t = ps[2]; pg.mouse.move(t["x"] + 1, t["y"]); frames(pg, 4); assert not pg.query_selector(LENS), "Esc: the same peak does not reopen it"
            pg.mouse.move(2, 2); frames(pg, 2); pg.mouse.move(t["x"], t["y"], steps=2); pg.wait_for_selector(LENS, timeout=10000)
        step("Esc closes the lens; it comes back on leaving and entering the peak", esc)
        def wheel_closes():
            pg.mouse.wheel(0, 120); frames(pg, 3); assert not pg.query_selector(LENS), "the wheel closes it"
        step("scroll closes the lens", wheel_closes)
        def commit():
            ps = pk(); t = ps[3]; pg.mouse.move(t["x"] - 30, t["y"]); pg.mouse.move(t["x"], t["y"], steps=3); pg.wait_for_selector(LENS, timeout=10000)
            want = api_trace(pg, SPEC, t["mz"], K)
            b = pg.evaluate("(()=>{const b=document.querySelector('.pnl .xl:not([hidden])').getBoundingClientRect();return [b.left+b.width/2,b.top+b.height/2]})()")
            pg.mouse.move(b[0], b[1], steps=4); frames(pg, 2); assert pg.query_selector(LENS), "the lens stays while the pointer is on it"
            pg.mouse.click(b[0], b[1]); ready(pg)
            pg.wait_for_function(f"(()=>{{const p={XP};return p&&p._a&&p._a.sr&&p._a.sr.length}})()", timeout=20000)
            assert not pg.query_selector(LENS)
            s = pg.evaluate(f"{XP}._a.sr.find(q=>q.k=={K})"); assert s["y"] == want["y"], "the panel differs from the lens"
            assert pg.evaluate(f"XICBTN.chosen({SPEC})") == t["mz"]
        step("a click on the lens makes the XIC panel of the button (same trace)", commit)
        def key_only():
            pg.evaluate(f"{SPEC}.cv.focus()"); pg.keyboard.press("."); frames(pg, 4)
            pg.wait_for_selector(LENS, timeout=10000)
            m = pg.evaluate(f"XICBTN.chosen({SPEC})"); assert pg.evaluate(f"XICLENS.shown({SPEC})") == m
            live = pg.evaluate("[...document.querySelectorAll('[role=status]')].map(e=>e.textContent).join('|')"); assert "XIC" in live, live
            pg.keyboard.press("Tab"); assert pg.evaluate("document.activeElement.classList.contains('xl')"), pg.evaluate("document.activeElement.outerHTML.slice(0,80)")
            assert pg.get_attribute(".pnl .xl", "aria-label")
            pg.keyboard.press("t"); pg.wait_for_selector("dialog#tdati", timeout=5000)
            rows = pg.evaluate("document.querySelectorAll('#tdati tbody tr').length"); t = lens_trace(pg, SPEC)
            assert rows == len(t["rt"]), (rows, len(t["rt"]))
            first = pg.evaluate("[...document.querySelectorAll('#tdati tbody tr')[0].children].map(c=>c.textContent)"); assert first[1] == str(float(t["y"][0])).rstrip("0").rstrip(".") or float(first[1]) == t["y"][0], first
            pg.keyboard.press("Escape"); frames(pg, 3)
            pg.wait_for_selector(LENS, timeout=3000); pg.focus(".pnl .xl")
            if AXE:
                pg.add_script_tag(path=AXE); v = pg.evaluate(RUN % "'#dpanels'"); assert not v, v
            n0 = pg.evaluate("E.panels.filter(p=>p.type==='xic').length")
            pg.keyboard.press("Enter"); ready(pg); frames(pg, 4)
            assert not pg.query_selector(LENS) and pg.evaluate("E.panels.filter(p=>p.type==='xic').length") == n0
            assert pg.evaluate(f"{XP}.traces.length") == 1
        step("keyboard only: the key chooses, the lens is reached with Tab, T gives the data table, Enter opens the panel; axe", key_only)
        def pref():
            pg.evaluate("UIP.xicLens=false"); ps = pk(); t = ps[1]
            pg.mouse.move(2, 2); pg.mouse.move(t["x"], t["y"], steps=3); frames(pg, 5); assert not pg.query_selector(LENS)
            pg.evaluate("UIP.xicLens=true")
        step("the preference switches the lens off", pref)
        def touch():
            ps = pk(); t = ps[4]; pg.mouse.move(2, 2); frames(pg, 2)
            pg.mouse.click(t["x"], t["y"]); pg.wait_for_selector(LENS, timeout=10000)
            h = pg.evaluate("(()=>{const b=document.querySelector('.pnl .xl:not([hidden])').getBoundingClientRect();return [b.width,b.height]})()"); assert h[0] >= 40 and h[1] >= 40, h
            pg.keyboard.press("Escape")
        step("a tap (mouse events of a tap) shows the lens; target >= 40 px", touch)
        def mem():
            n = pg.evaluate("XICLENS.cacheSize()"); assert n <= 64, n
        step("the cache keeps at most 64 traces", mem)
        assert not pg.evaluate("window.MZLAB_LR_MISS&&window.MZLAB_LR_MISS.length"), pg.evaluate("window.MZLAB_LR_MISS")

        # ---- high resolution
        D = synth("xicbtn")
        pg2 = r.page(p)
        load(pg2, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); pg2.evaluate("setTab('full')"); ready(pg2)
        S1 = "E.panels.find(p=>p.type==='spec'&&p.level===1)"
        KH = pg2.evaluate("E.files.find(f=>f.file.endsWith('#MS1')).k")
        def hr_lens():
            ps = pg2.evaluate(PEAKS % S1); assert ps, "no labelled peak"
            t = max(ps, key=lambda o: -o["y"]); pg2.mouse.move(t["x"] - 30, t["y"]); pg2.mouse.move(t["x"], t["y"], steps=3)
            pg2.wait_for_selector(LENS, timeout=15000); frames(pg2, 3)
            got = lens_trace(pg2, S1); assert got and got["m"] == t["mz"]
            assert same(got, api_trace(pg2, S1, t["mz"], KH)), "HR: the lens differs from api/xic"
            lab = pg2.inner_text(".pnl .xl .xl-t"); print(lab); assert "ppm" in lab, lab
        step("HR: the lens shows the XIC with the ppm window of the profile, same numbers as api/xic", hr_lens)
        print([e for e in r.errs if e[0] != "console.warning"])
        assert not pg2.evaluate("window.MZLAB_LR_MISS&&window.MZLAB_LR_MISS.length")
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
