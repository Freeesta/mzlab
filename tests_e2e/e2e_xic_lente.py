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
        pg.evaluate("window.__ok = XICBTN.lens.ok; XICBTN.lens.ok = () => true; 0")          # the lens belongs to HR MS1 spectra only: these steps check the lens itself on the low-resolution synthetic file, which has many peaks; the gate is checked apart
        pg.evaluate(TRACK)
        def pk(): return pg.evaluate(PEAKS % SPEC)
        def hover_lr():
            ps = pk(); assert len(ps) >= 4, len(ps)
            t = ps[2]; pg.mouse.move(t["x"] - 40, t["y"]); pg.mouse.move(t["x"], t["y"], steps=3)
            pg.wait_for_selector(LENS, timeout=10000); frames(pg, 3)
            got = lens_trace(pg, SPEC); assert got and got["m"] == t["mz"], (got and got["m"], t["mz"])
            assert same(got, api_trace(pg, SPEC, t["mz"], K)), "the lens differs from api/xic"
            box = pg.evaluate("(()=>{const b=document.querySelector('.pnl .xl:not([hidden])').getBoundingClientRect();return [b.width,b.height]})()")
            assert box[0] >= 160 and box[1] >= 90, box
            assert pg.evaluate(f"(()=>{{const l=document.querySelector('.pnl .xl').getBoundingClientRect();return l.left>{t['x']}+2||l.right<{t['x']}-2||l.bottom<{t['y']}-8}})()"), "the lens covers the peak under the pointer"
        step("LR: hovering a peak shows the lens, same traces as api/xic (Object.is), size, not over the peak", hover_lr)
        def fixed_place():
            ps = pk(); rects = []
            for t in (ps[1], ps[len(ps) - 2], ps[2]):
                pg.keyboard.press("Escape"); pg.mouse.move(2, 2); frames(pg, 2)
                pg.mouse.move(t["x"] - 30, t["y"]); pg.mouse.move(t["x"], t["y"], steps=3); pg.wait_for_selector(LENS, timeout=10000); frames(pg, 3)
                r = pg.evaluate("(()=>{const l=document.querySelector('.pnl .xl:not([hidden])').getBoundingClientRect(),c=XICLENS&&E.panels.find(p=>p.type==='spec'&&p.tab==='full').cv.getBoundingClientRect();return {l:l.left,r:l.right,t:l.top,b:l.bottom,ct:c.top,cl:c.left,cr:c.right}})()")
                assert abs(r["t"] - r["ct"]) < 6, ("the lens is not at the top edge of the plot", r)
                assert not (r["l"] - 1 <= t["x"] <= r["r"] + 1 and r["t"] - 1 <= t["y"] <= r["b"] + 1), ("the lens is over the pointer", r, t)
                assert not (r["l"] - 1 <= t["x"] <= r["r"] + 1 and t["y"] - 8 <= r["b"]), ("the lens is over the peak", r, t)
                rects.append(round(r["l"] - r["cl"]))
            lab = pg.evaluate("(()=>{const e=document.querySelector('.pnl .xl');return [e.title,e.getAttribute('aria-label'),!!e.querySelector('.xl-t,.xl-x')]})()")
            assert lab[0] and "\u00b7" in lab[0] and lab[0] in lab[1] and not lab[2], lab    # ion and file are the tooltip: no title row, no DOM axis
        step("the lens is in a fixed corner of the plot (the other one when the peak or the pointer is in it), with the file name as tooltip", fixed_place)
        def right_click():
            t = pk()[2]; pg.keyboard.press("Escape"); pg.mouse.move(2, 2); frames(pg, 2)
            pg.mouse.move(t["x"] - 30, t["y"]); pg.mouse.move(t["x"], t["y"], steps=3); pg.wait_for_selector(LENS, timeout=10000)
            pg.mouse.click(t["x"], t["y"], button="right"); pg.wait_for_selector("#ctx div", timeout=5000); frames(pg, 2)
            assert pg.evaluate("document.querySelectorAll('.pnl .xl:not([hidden])').length") == 0, "the lens stays under the context menu"
            pg.keyboard.press("Escape"); pg.mouse.move(2, 2); frames(pg, 2)
        step("a right click closes the lens: it never overlaps the context menu", right_click)
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
        last_m = [None]
        def esc():
            last_m[0] = pg.evaluate(f"XICLENS.shown({SPEC})"); assert last_m[0] is not None
            pg.keyboard.press("Escape"); frames(pg, 2); assert not pg.query_selector(LENS)
            ps = pk(); t = next(o for o in ps if o["mz"] == last_m[0]); pg.mouse.move(t["x"] + 1, t["y"]); frames(pg, 4); assert not pg.query_selector(LENS), "Esc: the same peak does not reopen it"
            pg.mouse.move(2, 2); frames(pg, 2); pg.mouse.move(t["x"], t["y"], steps=2); pg.wait_for_selector(LENS, timeout=10000)
        step("Esc closes the lens; it comes back on leaving and entering the peak", esc)
        def wheel_closes():
            pg.mouse.wheel(0, 120); frames(pg, 3); assert not pg.query_selector(LENS), "the wheel closes it"
        step("scroll closes the lens", wheel_closes)
        def commit():
            ps = pk(); t = ps[3]; pg.mouse.move(t["x"] - 30, t["y"]); pg.mouse.move(t["x"], t["y"], steps=3); pg.wait_for_function(f"XICLENS.shown({SPEC})==={t['mz']}&&XICLENS.trace({SPEC})", timeout=10000); frames(pg, 2)
            b = pg.evaluate("(()=>{const b=document.querySelector('.pnl .xl:not([hidden])').getBoundingClientRect();return [b.left+b.width/2,b.top+b.height/2]})()")
            pg.mouse.move(b[0], b[1], steps=4); frames(pg, 2); assert pg.query_selector(LENS), "the lens stays while the pointer is on it"
            # the lens is in a corner: the way to it may cross other peaks, the click opens what the lens shows
            pg.wait_for_function(f"XICLENS.trace({SPEC})", timeout=10000)
            t = {"mz": pg.evaluate(f"XICLENS.shown({SPEC})")}
            want = api_trace(pg, SPEC, t["mz"], K)
            pg.mouse.click(b[0], b[1]); ready(pg)
            pg.wait_for_function(f"(()=>{{const p={XP};return p&&p._a&&p._a.sr&&p._a.sr.length}})()", timeout=20000)
            assert not pg.query_selector(LENS)
            s = pg.evaluate(f"{XP}._a.sr.find(q=>q.k=={K})"); print(pg.evaluate(f"[{XP}.traces, XICBTN.lensArgs({SPEC},{t['mz']}).args]"), len(s["y"]), len(want["y"]), max(s["y"]), max(want["y"]))
            assert s["y"] == want["y"], ("the panel differs from the lens", t["mz"], pg.evaluate(f"{XP}.traces"))
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
            pg.evaluate("XICBTN.setLens(false)"); ps = pk(); t = ps[1]
            pg.mouse.move(2, 2); pg.mouse.move(t["x"], t["y"], steps=3); frames(pg, 5); assert not pg.query_selector(LENS)
            pg.evaluate("XICBTN.setLens(true)")
        step("the preference switches the lens off", pref)
        pg.evaluate("XICBTN.lens.ok = window.__ok; E.panels.forEach(p => { if (p.type === 'spec' && p.el) XICBTN.sync(p) }); 0")          # the real resolution test again
        def gate():
            # the lens and its switch exist only on the MS1 spectrum of a high-resolution file: none on MS2 or low resolution
            assert pg.evaluate("E.panels.filter(p=>p.type==='spec'&&p.el).every(p=>p.el.querySelector('[data-a=xicp]').hidden&&!XICBTN.lens.ok(p))"), "a low-resolution spectrum shows the lens switch"
            ps = pk(); t = ps[1]; pg.mouse.move(2, 2); pg.mouse.move(t["x"], t["y"], steps=3); frames(pg, 6)
            assert not pg.query_selector(LENS), "the lens opened on a low-resolution spectrum"
        step("LR: no lens switch, no lens on hover", gate)
        pg.evaluate("XICBTN.lens.ok = () => true; E.panels.forEach(p => { if (p.type === 'spec' && p.el) XICBTN.sync(p) }); 0")
        def toggle():
            B = ".pnl [data-a=xicp]"; v = [pg.get_attribute(B, "aria-pressed"), pg.get_attribute(B, "title")]; assert v == ["true", pg.evaluate("I18N.t('xiclens.toggle')")], v
            pg.evaluate(f"document.querySelector('{B}').click()"); frames(pg, 2); assert pg.get_attribute(B, "aria-pressed") == "false" and not pg.evaluate("UIP.xicLens")
            ps = pk(); t = ps[1]; pg.mouse.move(2, 2); pg.mouse.move(t["x"], t["y"], steps=3); frames(pg, 5); assert not pg.query_selector(LENS)
            pg.evaluate(f"document.querySelector('{B}').click()"); frames(pg, 2); assert pg.get_attribute(B, "aria-pressed") == "true" and pg.evaluate("UIP.xicLens")
            pg.keyboard.press("Escape")
        step("the button of the bar switches the lens on and off (aria-pressed), on by default", toggle)
        def touch():
            ps = pk(); t = ps[0]; pg.mouse.move(2, 2); frames(pg, 2)
            pg.mouse.click(t["x"], t["y"]); pg.wait_for_selector(LENS, timeout=10000)
            h = pg.evaluate("(()=>{const b=document.querySelector('.pnl .xl:not([hidden])').getBoundingClientRect();return [b.width,b.height]})()"); assert h[0] >= 40 and h[1] >= 40, h
            pg.keyboard.press("Escape")
        step("a tap (mouse events of a tap) shows the lens; target >= 40 px", touch)
        def mem():
            n = pg.evaluate("XICLENS.cacheSize()"); assert n <= 64, n
        step("the cache keeps at most 64 traces", mem)
        assert not pg.evaluate("window.MZLAB_LR_MISS&&window.MZLAB_LR_MISS.length"), pg.evaluate("window.MZLAB_LR_MISS")

        def narrow_peak():
            # a 0.05 min peak on a 20 min axis at 160 px: a visible peak (tall, filled, 1.5 px line), round equidistant ticks, no "min" on them
            r = pg.evaluate("""(()=>{const n=4000,rt=[],y=[];for(let i=0;i<n;i++){const t=20*i/(n-1);rt.push(t);y.push(Math.exp(-(((t-14.3)/0.015)**2)/2)*1000+1)}
              const c=document.createElement('canvas');c.width=160;c.height=96;const g=c.getContext('2d');
              const d=XICDRAW.draw(g,160,96,{rt,y},{line:'#888',ink:'#000',muted:'#444',accent:'#d00',scanRt:null,label:true});
              const im=g.getImageData(0,0,160,96).data;let top=96,fill=0;
              for(let yy=0;yy<96;yy++)for(let xx=0;xx<160;xx++){const i=(yy*160+xx)*4;if(im[i]>150&&im[i+1]<150&&im[i+2]<150&&im[i+3]>60){if(yy<top)top=yy;fill++}}
              return {d,top,fill,ticks:XICDRAW.ticks(rt[0],rt[n-1])}})()""")
            h = (r["d"]["y1"] - r["top"]) / (r["d"]["y1"] - r["d"]["y0"]); assert h >= .6, ("the narrow peak is too low", h, r)
            assert r["fill"] >= 6, ("the peak is a hairline", r["fill"]); assert r["ticks"]["vals"] == [0, 5, 10, 15, 20], r["ticks"]
        step("a 0.05 min peak on 20 min at 160 px is drawn as a tall filled peak; the axis has round equidistant ticks", narrow_peak)

        # ---- high resolution
        D = synth("xicbtn")
        pg2 = r.page(p)
        load(pg2, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); pg2.evaluate("setTab('full')"); ready(pg2)
        S1 = "E.panels.find(p=>p.type==='spec'&&p.level===1)"
        KH = pg2.evaluate("E.files.find(f=>f.file.endsWith('#MS1')).k")
        def hr_gate():
            assert pg2.evaluate("XICBTN.lens.ok(E.panels.find(p=>p.type==='spec'&&p.level===1))") is True and pg2.evaluate("!E.panels.find(p=>p.type==='spec'&&p.level===1).el.querySelector('[data-a=xicp]').hidden")
            assert pg2.evaluate("XICBTN.lens.ok({...E.panels.find(p=>p.type==='spec'&&p.level===1),level:2})") is False, "an MS2 spectrum has no lens"
        step("HR MS1 keeps the lens switch; an MS2 spectrum has none", hr_gate)
        def hr_lens():
            ps = pg2.evaluate(PEAKS % S1); assert ps, "no labelled peak"
            t = max(ps, key=lambda o: -o["y"]); pg2.mouse.move(t["x"] - 30, t["y"]); pg2.mouse.move(t["x"], t["y"], steps=3)
            pg2.wait_for_selector(LENS, timeout=15000); frames(pg2, 3)
            got = lens_trace(pg2, S1); assert got and got["m"] == t["mz"]
            assert same(got, api_trace(pg2, S1, t["mz"], KH)), "HR: the lens differs from api/xic"
            lab = pg2.get_attribute(".pnl .xl", "title"); print(lab); assert "ppm" in lab, lab
        step("HR: the lens shows the XIC with the ppm window of the profile, same numbers as api/xic", hr_lens)
        print([e for e in r.errs if e[0] != "console.warning"])
        assert not pg2.evaluate("window.MZLAB_LR_MISS&&window.MZLAB_LR_MISS.length")
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
