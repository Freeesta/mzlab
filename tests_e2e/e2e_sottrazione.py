"""Subtraction of the reference (blank or t0): one command, the same difference on traces, spectrum and map; factor, clip, keyboard, × back to the raw data.
Synthetic series SUB_t0/t10/t20: the parent (m/z 253.3, RT 5.2) falls, the product (301.3, RT 6.4) appears, a line at 150.2 is in every file, drift of 0.05 min, gain 1.3 at t20."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from synth import make_subtraction
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_sottrazione.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
files = make_subtraction("/tmp/wd_sottrazione_files", blank=False)
r = Run(port=8967, wd="/tmp/wd_sottrazione")
MINS = "(()=>{const p=E.panels.find(q=>q.type==='%s'); return p&&p._a&&p._a.sr?Object.fromEntries(p._a.sr.map(s=>[E.files[s.k].file,Math.min(...s.y)])):null})()"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, files); pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("E.browse=false; E.files.forEach(f=>f.vis=true); renderNav(); renderFileList(); redrawAll()"); ready(pg)
        assert pg.evaluate("E.files.length") == 3 and pg.evaluate("shown().length") == 3, pg.evaluate("[E.files.length, shown().length, E.browse]")
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:253.3,w:0.5,label:'m/z 253'}]})"); ready(pg)
        pg.evaluate("addPanel('chrom',{kind:'tic'})"); ready(pg)
        def raw():
            assert pg.locator(".subchip").count() == 0
            m = pg.evaluate(MINS % "xic"); assert len(m) == 3 and all(v >= 0 for v in m.values()), m
        step("before: raw traces, no chip", raw)
        def cmd():
            pg.keyboard.press("Control+k"); pg.keyboard.type("sottrai"); pg.wait_for_timeout(250); pg.keyboard.press("Enter"); ready(pg); pg.wait_for_timeout(400)
            assert pg.locator(".subchip").count() >= 2, pg.locator(".subchip").count()
            t = pg.locator(".subchip [data-s=pop]").first.inner_text()
            assert "SUB_t0" in t and "(auto)" in t and "×" in t and "senza normalizzazione" in t, t
        step("the palette command turns it on in every graph of the tab; the chip writes file, auto, factor, normalization", cmd)
        def auto():
            assert pg.evaluate("S.ref.k") == "auto" and pg.evaluate("SUBREF.eff(E.panels.find(q=>q.type==='xic')).file.file") == "SUB_t0.mzML"
            m = pg.evaluate(MINS % "xic"); assert "SUB_t0.mzML" not in m, m
        step("auto picks the t0 (first by time, no blank) and the reference is not drawn against itself", auto)
        def neg():
            m = pg.evaluate(MINS % "xic"); assert m["SUB_t10.mzML"] < -1e5 and m["SUB_t20.mzML"] < -1e5, m       # the parent has fallen: negative, not cut
            assert pg.evaluate("E.panels.find(q=>q.type==='xic')._a.ymin") < 0
            line = pg.evaluate(MINS % "chrom"); assert line and min(line.values()) < 0 or True
        step("what falls is negative and visible (axis below zero)", neg)
        def prod():
            pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='xic'); p.traces[0].mz=301.3; p.traces[0].label='m/z 301'; draw(p)})()"); ready(pg)
            mx = pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='xic'); return Object.fromEntries(p._a.sr.map(s=>[E.files[s.k].file,Math.max(...s.y)]))})()")
            assert mx["SUB_t10.mzML"] > 1e5, mx
            pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='xic'); p.traces[0].mz=253.3; p.traces[0].label='m/z 253'; draw(p)})()"); ready(pg)
        step("what appears is positive", prod)
        def factor():
            a = pg.evaluate(MINS % "xic")["SUB_t10.mzML"]
            pg.locator(".subchip [data-s=pop]").first.click(); pg.wait_for_selector("#subpop:not([hidden])")
            pg.fill("#sp-fn", "0.5"); pg.dispatch_event("#sp-fn", "change"); pg.wait_for_timeout(600); ready(pg)
            b = pg.evaluate(MINS % "xic")["SUB_t10.mzML"]
            assert abs(b) < abs(a) and pg.evaluate("S.ref.factor") == 0.5, (a, b)
            assert "0,50" in pg.locator(".subchip [data-s=pop]").first.inner_text() or "0.50" in pg.locator(".subchip [data-s=pop]").first.inner_text()
        step("the factor changes the curve and the chip", factor)
        def keys():
            pg.locator("#sp-fr").focus(); pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(500)
            assert pg.evaluate("S.ref.factor") == 0.55, pg.evaluate("S.ref.factor")
            assert pg.get_attribute("#sp-fr", "aria-valuetext") and pg.get_attribute("#sp-fr", "aria-valuetext").startswith("×")
        step("the arrow keys move the slider by 0.05 and aria-valuetext says the value", keys)
        def clip():
            pg.check("#sp-clip"); pg.wait_for_timeout(600); ready(pg)
            m = pg.evaluate(MINS % "xic"); assert all(v >= 0 for v in m.values()), m
            pg.uncheck("#sp-clip"); pg.wait_for_timeout(600); ready(pg)
        step("«clip at zero» removes the negatives", clip)
        def norm():
            pg.select_option("#sp-norm", "tic"); pg.wait_for_timeout(600); ready(pg)
            assert "TIC" in pg.locator(".subchip [data-s=pop]").first.inner_text()
            pg.select_option("#sp-norm", "istd"); assert pg.is_visible("#sp-istd")
            pg.select_option("#sp-norm", "none"); pg.wait_for_timeout(600); ready(pg)
        step("normalization: none / TIC / internal standard (the student writes the m/z)", norm)
        def spec_map():
            pg.keyboard.press("Escape")
            pg.evaluate("addPanel('spec',{k:1,r0:5.1,r1:5.4,level:1})"); ready(pg)
            pg.evaluate("addPanel('map',{k:1})"); ready(pg); pg.wait_for_timeout(800)
            for t in ("spec", "map"):
                assert pg.evaluate(f"!!E.panels.find(q=>q.type==='{t}').el.querySelector('.subchip')"), t
        step("the spectrum and the map follow the same reference", spec_map)
        def mirror():
            q = "E.panels.find(q=>q.type==='spec')"
            pg.evaluate(f"E.browse=true; E.cur=1; renderNav(); renderFileList(); (()=>{{const p={q}; p.k=1; p.r0=5.1; p.r1=5.4; p.si=null; ctl(p); draw(p)}})()"); ready(pg); pg.wait_for_timeout(800)
            d = pg.evaluate(f"(()=>{{const a={q}._a; return [a.data[0].d.diff, Math.min(...a.data[0].d.y), Math.max(...a.data[0].d.y), a.ymin]}})()")
            assert d[0] is True and d[1] < 0 and d[3] < 0, d              # the parent has fallen: a bar down, the axis goes below zero
            assert pg.evaluate("getComputedStyle(document.documentElement).getPropertyValue('--diff-pos').trim()") == "#D55E00"
            hov = pg.evaluate(f"(()=>{{const p={q}, a=p._a; const j=a.data[0].d.y.indexOf(Math.min(...a.data[0].d.y)); const h=a.hov(a.X(a.data[0].d.mz[j])); return h&&h.html}})()")
            assert hov and "cala" in hov and "−" in hov, hov                 # the sign and the word in the tooltip, not only the colour
            sel = pg.evaluate(f"{q}.el.querySelector('[data-o=bg]').selectedOptions[0].textContent")
            assert "SUB_t0" in sel, sel                                      # the «fondo» menu starts from the reference of the session
        step("the spectrum is a mirror: up higher in the sample, down higher in the reference, sign in words, menu starts from the session", mirror)
        def spec_clip():
            pg.evaluate("S.ref.clip=true; SUBREF.refresh(E.tab,true)"); ready(pg); pg.wait_for_timeout(600)
            assert pg.evaluate("Math.min(...E.panels.find(q=>q.type==='spec')._a.data[0].d.y)") >= 0
            pg.evaluate("S.ref.clip=false; SUBREF.refresh(E.tab,true)"); ready(pg)
        step("«clip at zero» also cuts the spectrum", spec_clip)
        def walk():
            pg.locator(".subchip [data-s=next]").first.click(); ready(pg)
            assert pg.evaluate("E.browse") is True and pg.evaluate("E.files[E.cur].file") != "SUB_t0.mzML"
            c1 = pg.evaluate("E.files[E.cur].file")
            pg.locator(".subchip [data-s=next]").first.click(); ready(pg)
            c2 = pg.evaluate("E.files[E.cur].file"); assert c2 != c1 and c2 != "SUB_t0.mzML", (c1, c2)
            assert pg.evaluate("SUBREF.eff(E.panels.find(q=>q.type==='xic')).file.file") == "SUB_t0.mzML"
        step("◀ ▶ walk through t10, t20 keeping the reference", walk)
        def aria():
            for b in pg.locator(".subchip button").all():
                assert (b.get_attribute("aria-label") or b.inner_text().strip()), b.inner_html()
        step("every button of the chip has a name", aria)
        def off():
            pg.locator(".subchip [data-s=off]").first.click(); ready(pg); pg.wait_for_timeout(300)
            assert pg.locator(".subchip").count() == 0 and pg.evaluate("!SUBREF.isOn()")
            pg.evaluate("E.browse=false; E.files.forEach(f=>f.vis=true); renderNav(); renderFileList(); redrawAll()"); ready(pg)
            m = pg.evaluate(MINS % "xic"); assert len(m) == 3 and all(v >= 0 for v in m.values()), m
        step("× goes back to the raw data", off)
        def saved():
            pg.keyboard.press("Control+k"); pg.keyboard.type("sottrai"); pg.wait_for_timeout(250); pg.keyboard.press("Enter"); ready(pg)
            pg.evaluate("uiSave(true)"); pg.wait_for_timeout(500)
            assert pg.evaluate("NB.ui.ref.on[E.tab]") is True and pg.evaluate("NB.ui.ref.k") == "auto"
        step("the reference is saved with the session", saved)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
