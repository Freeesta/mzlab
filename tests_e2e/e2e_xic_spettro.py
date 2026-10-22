"""V-1: button «XIC» of the spectrum (low and high resolution): chosen peak, extraction into the panel under the spectrum (same window as api/xic, E2), ion and
tolerance written by hand, scan line and apex, keyboard only, accessibility."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
AXE = axe_path()
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0,3).map(n => n.target.join(' ') + ' :: ' + (n.failureSummary||'').split('\\n').slice(0,3).join(' | '))]))"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_xic_spettro.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
XP = "E.panels.find(p=>p.type==='xic')"
r = Run(port=8878, wd="/tmp/wd78")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        K = pg.evaluate("E.files[0].k")
        def peak(m):      # screen position of the labelled peak nearest to m/z m
            return pg.evaluate(f"(()=>{{const p={SPEC},q=p.cv.getBoundingClientRect(),a=p._a,d=a.data[0].d;let b=0;d.mz.forEach((v,i)=>{{if(Math.abs(v-{m})<0.6&&(Math.abs(d.mz[b]-{m})>=0.6||d.y[i]>d.y[b]))b=i}});return {{mz:d.mz[b],x:q.left+a.X(d.mz[b]),y:q.top+a.Y(d.y[b])+6}}}})()")
        def xic_api(mzc, tol):
            return pg.evaluate(f"J('api/xic?k={K}&mz={mzc}&tol={tol}&level=1').then(j=>j.traces[0])")
        def xic_panel():      # the series the XIC panel drew, as the engine gave them
            pg.wait_for_function(f"(()=>{{const p={XP};return p&&p._a&&p._a.sr&&p._a.sr.length}})()", timeout=20000)
            return pg.evaluate(f"(()=>{{const s={XP}._a.sr.find(q=>q.k=={K});return {{x:s.x,y:s.y}}}})()")
        def pressed(): return pg.evaluate(f"document.querySelector('[data-a=xicb]').getAttribute('aria-pressed')")
        def lr_extract():
            pk = peak(364.4)
            pg.click(f"{SPEC and ''}[data-a=xicb]"); assert pressed() == "true"        # no peak chosen: the mode «choose a peak»
            pg.mouse.click(pk["x"], pk["y"]); ready(pg); assert pressed() == "false"
            lab = pg.inner_text("[data-a=xicl]"); print(lab); assert lab.startswith("m/z ") and "–" in lab, lab
            n = pg.evaluate("E.panels.filter(p=>p.type==='xic').length"); assert n == 1, n
            a, b = pg.evaluate(f"xicWin({pk['mz']},true)"); c, w = round((a + b) / 2, 2), round((b - a) / 2, 2)
            want = xic_api(c, w); got = xic_panel(); print(list(want.keys()))
            assert got["y"] == want["y"] and got["x"] == (want.get("x") or want.get("rt")), "XIC differs from api/xic"      # E2: same numbers
            assert pg.evaluate(f"{XP}.el.getBoundingClientRect().top >= {SPEC}.el.getBoundingClientRect().bottom - 2"), "XIC is under the spectrum"
        step("LR: the button waits for a peak, the click extracts, same traces as api/xic", lr_extract)
        def lr_replace():
            pk = peak(194.2); pg.mouse.click(pk["x"], pk["y"]); ready(pg)
            pg.click("[data-a=xicb]"); ready(pg)
            assert pg.evaluate("E.panels.filter(p=>p.type==='xic').length") == 1
            assert pg.evaluate(f"{XP}.traces.length") == 1
            pk = peak(152.2); pg.mouse.click(pk["x"], pk["y"]); pg.keyboard.down("Shift"); pg.click("[data-a=xicb]"); pg.keyboard.up("Shift"); ready(pg)
            assert pg.evaluate(f"{XP}.traces.length") == 2
        step("LR: the same panel is reused; Shift adds a trace", lr_replace)
        def lr_cursor():
            ap = xic_panel(); assert pg.evaluate(f"{XP}.cur != null")
            rt0 = pg.evaluate(f"{XP}.cur")
            pg.evaluate(f"stepScan({SPEC}.link!=null?E.panels.find(q=>q.id=={SPEC}.link):E.panels[0],3)"); ready(pg); pg.wait_for_timeout(600)
            rt1 = pg.evaluate(f"{XP}.cur"); assert rt1 != rt0, (rt0, rt1)
            txt = pg.inner_text(f"xpath=//*[contains(@class,'xb-info')]"); print(txt)
            assert "RT" in txt or "picco" in txt, txt
            assert pg.evaluate(f"{XP}.ints.length") == 0       # unit resolution: no area until the student integrates
        step("scan line follows the spectrum, apex text, no area in LR", lr_cursor)
        def typed():
            pg.click("[data-a=xicl]"); pg.fill("[data-f=mz]", "194,2"); pg.fill("[data-f=tol]", "0,3"); pg.press("[data-f=tol]", "Enter"); ready(pg)
            lab = pg.inner_text("[data-a=xicl]"); print(lab); assert "193.9" in lab and "194.5" in lab, lab
            want = xic_api(194.2, 0.3); got = xic_panel(); assert got["y"] == want["y"]
            pg.click("[data-a=xicl]"); pg.fill("[data-f=mz]", "abc"); pg.press("[data-f=mz]", "Enter")
            assert pg.get_attribute("[data-f=mz]", "aria-invalid") == "true"
            pg.keyboard.press("Escape"); assert pg.evaluate("!document.querySelector('.xb-pop')")
        step("typed ion and tolerance (comma ok); not a number: aria-invalid, Esc closes", typed)
        def keys():
            pg.evaluate(f"{SPEC}.cv.focus()"); pg.keyboard.press("."); pg.wait_for_timeout(300)
            m1 = pg.evaluate("XICBTN.chosen(" + SPEC + ")"); pg.keyboard.press(","); pg.wait_for_timeout(300)
            m2 = pg.evaluate("XICBTN.chosen(" + SPEC + ")"); assert m2 < m1, (m1, m2)
            pg.keyboard.press("."); pg.wait_for_timeout(300); assert pg.evaluate("XICBTN.chosen(" + SPEC + ")") == m1
            pg.focus("[data-a=xicb]"); pg.keyboard.press("Enter"); ready(pg)
            assert pg.evaluate(f"{XP}.traces.length") == 1
            pg.focus("[data-a=xicb]"); pg.keyboard.press(" "); ready(pg)
            assert pg.evaluate("document.querySelector('.xb-info')") is not None
        step("keyboard only: , and . choose, Enter and Space on the button extract", keys)
        def esc():
            pg.evaluate(f"XICBTN.chosen({SPEC})"); pg.evaluate(f"(()=>{{const p={SPEC};}})()")
        def a11y():
            sz = pg.evaluate("(()=>{const b=document.querySelector('[data-a=xicb]').getBoundingClientRect();return [b.width,b.height]})()")
            assert sz[0] >= 24 and sz[1] >= 24, sz
            assert pg.get_attribute("[data-a=xicb]", "aria-label")
        step("target size and labels", a11y)
        def esc():
            pg.evaluate(f"(()=>{{const p={SPEC};XICBTN.chosen(p)}})()")
            pg.evaluate(f"{SPEC}.el.querySelector('[data-a=xicl]').click()"); assert pg.evaluate("!!document.querySelector('.xb-pop')")
            assert pg.evaluate("getComputedStyle(document.querySelector('.xb-pop')).animationName") == "none"      # prefers-reduced-motion
            pg.keyboard.press("Escape"); assert pg.evaluate("!document.querySelector('.xb-pop')")
            assert pg.evaluate("document.activeElement&&document.activeElement.dataset.a")=="xicl"
        step("Esc closes the pop-up and gives the focus back; no animation with reduced motion", esc)
        def axe():
            if not AXE: return
            pg.add_script_tag(path=AXE)
            pg.evaluate(f"{SPEC}.el.querySelector('[data-a=xicl]').click()")
            v = pg.evaluate(RUN % "'#dpanels'"); assert not v, v
            pg.keyboard.press("Escape")
        step("axe: spectrum, XIC and the pop-up have no serious or critical violation", axe)

        # ---- high resolution (Orbitrap-like DDA file): ppm window, area at once, tolerance 5 -> 10 ppm, MS2 spectrum
        D = synth("xicbtn")
        pg2 = r.page(p)
        load(pg2, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); pg2.evaluate("setTab('full')"); ready(pg2)
        S1 = "E.panels.find(p=>p.type==='spec'&&p.level===1)"
        KH = pg2.evaluate("E.files.find(f=>f.file.endsWith('#MS1')).k")
        def hr_peak():
            return pg2.evaluate(f"(()=>{{const p={S1},q=p.cv.getBoundingClientRect(),a=p._a,d=a.data[0].d;let b=0;d.mz.forEach((v,i)=>{{if(d.y[i]>d.y[b])b=i}});return {{mz:d.mz[b],x:q.left+a.X(d.mz[b]),y:q.top+a.Y(d.y[b])+6}}}})()")
        def hr_extract():
            pk = hr_peak(); pg2.click(f"{S1}.el".replace(S1 + ".el", "[data-a=xicb]") if False else "[data-a=xicb] >> nth=0") if False else None
            pg2.evaluate(f"{S1}.el.querySelector('[data-a=xicb]').click()"); pg2.mouse.click(pk["x"], pk["y"]); ready(pg2); pg2.wait_for_timeout(800)
            lab = pg2.evaluate(f"{S1}.el.querySelector('[data-a=xicl]').textContent"); print(lab)
            assert "\u00b1 5 ppm" in lab and "m/z" in lab, lab
            t = pg2.evaluate(f"{XP}.traces[0]"); assert t["ion"] and abs(t["mz"] - pk["mz"]) < 1e-4, t
            print(pg2.evaluate(f"[document.querySelector('.xb-info')?.textContent, PK.method, {XP}.ints.length, {XP}._a.sr.map(s=>[s.k,s.key,s.x.length]), {S1}.r0]"))
            assert pg2.evaluate(f"{XP}.ints.length") >= 1, "HR: the area comes with the XIC"
        step("HR: ion with ppm window, area at once", hr_extract)
        def hr_tol():
            pg2.evaluate(f"{S1}.el.querySelector('[data-a=xicl]').click()"); pg2.fill("[data-f=tol]", "10"); pg2.press("[data-f=tol]", "Enter"); ready(pg2); pg2.wait_for_timeout(800)
            t = pg2.evaluate(f"{XP}.traces[0]"); assert t["ppm"] == 10 and t["label"].endswith("\u00b1 10 ppm"), t
            assert pg2.evaluate(f"{XP}.traces.length") == 1
            a = pg2.evaluate(f"HR.xicArgs(E.files[{KH}],{XP}.traces[0],{XP})"); assert abs(a[1] - t["mz"] * 10e-6) < 1e-9, a
            want = pg2.evaluate(f"J('api/xic?k={KH}&mz='+{a[0]}+'&tol='+{a[1]}+'&level=1').then(j=>j.traces[0])")
            got = pg2.evaluate(f"{XP}._a.sr.find(q=>q.k=={KH}).y"); assert got == want["y"]
        step("HR: tolerance 10 ppm gives the api/xic trace of that window", hr_tol)
        def hr_ms2():
            S2 = "E.panels.find(p=>p.type==='spec'&&p.level===2)"
            has = pg2.evaluate(f"!!({S2}&&{S2}._a&&{S2}._a.lbls&&{S2}._a.lbls.length)")
            if not has: return
            pg2.evaluate(f"{S2}.cv.focus()"); pg2.keyboard.press("."); pg2.wait_for_timeout(300)
            assert pg2.evaluate(f"XICBTN.chosen({S2})") is not None
        step("HR: the keys choose a peak of the MS2 spectrum", hr_ms2)
        def esc_mode():
            fresh = pg2.evaluate("(()=>{const p=E.panels.find(q=>q.type==='spec'&&q.level===2);return p.id})()")
            pg2.evaluate(f"E.panels.find(q=>q.id=={fresh}).el.querySelector('[data-a=xicb]').click()")
            assert pg2.evaluate(f"E.panels.find(q=>q.id=={fresh}).el.querySelector('[data-a=xicb]').getAttribute('aria-pressed')") == "true"
            pg2.keyboard.press("Escape")
            assert pg2.evaluate(f"E.panels.find(q=>q.id=={fresh}).el.querySelector('[data-a=xicb]').getAttribute('aria-pressed')") == "false"
        step("Esc leaves the mode «choose a peak»", esc_mode)
        step("STATO.verifica() is empty", lambda: (lambda v: v == [] or v is None or (_ for _ in ()).throw(AssertionError(v)))(pg2.evaluate("window.STATO&&STATO.verifica?STATO.verifica():[]")))
        assert not pg.evaluate("window.MZLAB_LR_MISS&&window.MZLAB_LR_MISS.length"), pg.evaluate("window.MZLAB_LR_MISS")
        print([e for e in r.errs if e[0] != "console.warning"])
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
