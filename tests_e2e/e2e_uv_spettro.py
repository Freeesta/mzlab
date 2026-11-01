"""UV spectrum of the DAD: a double click on the PDA map or on the PDA trace of a SCIEX .wiff opens the UV spectrum (nm, mAU) of that time under the graph, with its
table of the data; a file with only the PDA trace (no optical matrix) answers with a clear message and opens nothing. Synthetic pair (tools/wiff_riferimento.py); the demo FullScan_t10.mzML carries its matrix in the TWC chromatogram."""
import sys, os, math, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from pathlib import Path
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import wiff_riferimento as wr, dati_sintetici as ds
AXE = axe_path()
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length]))"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_uv_spettro.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = Path(tempfile.mkdtemp(prefix="uv_spettro_"))
w = wr.Writer(); blocks = [("scan", [(100 + k, 50 + k)], 1000.0 * k, 50.0 + k) for k in range(40)]
ts = [0.05 * k for k in range(40)]; rows = [[10 + 400 * math.exp(-((j - 60) / 12) ** 2) * math.exp(-((t - 1.0) / 0.2) ** 2) for j in range(201)] for t in ts]
w.sample(1, blocks, {"Experiment0/ExperimentHeader": wr.header(15, 30.0)}, dad=(ts, 200.0, 1.0, rows)); w.save(D, "uv")
T0 = D / "SOLOTRACCIA_t0.mzML"; ds.full_scan(T0, 0, np.random.default_rng(5))
UV = "E.panels.find(q=>q.type==='spec'&&q.uv)"
def dbl(pg, sel_js, rt):
    """double click on the panel `sel_js` (a JS expression) at the time `rt` of its x axis, in the middle of the plot"""
    box = pg.evaluate(f"(()=>{{const p={sel_js},b=p.cv.getBoundingClientRect(),a=p._a;return {{x:b.x+a.X({rt}),y:b.y+M.t+(a.H-M.t-M.b)/2}}}})()")
    pg.mouse.dblclick(box["x"], box["y"])
r = Run(port=8868, wd="/tmp/wd_uv_spettro")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / "uv.wiff"), str(D / "uv.wiff.scan")])
        pg.click("text=Carica dati"); ready(pg)
        def map_dbl():
            pg.click("#np-map"); ready(pg); attendi(pg, "E.files[0]._pdaMap === true")
            pg.select_option("body select[data-o=mapKind]", "pda"); ready(pg)
            attendi(pg, "E.panels.find(q=>q.type==='map')._a && E.panels.find(q=>q.type==='map')._a.pda")
            dbl(pg, "E.panels.find(q=>q.type==='map')", 1.0)
            attendi(pg, f"!!({UV} && {UV}._a && {UV}._a.uvd)")
            a = pg.evaluate(f"({{n: {UV}._a.uvd.nm.length, nm0: {UV}._a.uvd.nm[0], rt: {UV}._a.uvd.rt, mx: {UV}._a.uvd.nm[{UV}._a.uvd.y.indexOf(Math.max(...{UV}._a.uvd.y))]}})")
            assert a["n"] == 201 and a["nm0"] == 200 and abs(a["rt"] - 1.0) < 0.06 and abs(a["mx"] - 260) <= 1, a
        step("double click on the PDA map: the UV spectrum of that time, peak at 260 nm", map_dbl)
        def table():
            d = pg.evaluate(f"TDATI_P.data({UV})"); t = pg.evaluate(f"TDATI_P.text({UV})")
            assert d["head"] == ["nm", "Assorbanza (mAU)"] and len(d["rows"]) > 50, d["head"]
            assert "UV" in t and "m/z" not in t, t
        step("the table of the data has nm and mAU; the description speaks of the UV spectrum", table)
        LBU = f"({UV}._a.lbls || []).filter(l => l.etpk)"
        def uv_labels():
            attendi(pg, f"{LBU}.length >= 1")
            t = pg.evaluate(f"{LBU}.map(l => l.text)"); assert t == ["260 nm"], t
            b = pg.evaluate(f"{LBU}.map(l => [l.x, l.y, l.w, l.h])")[0]; cv = pg.evaluate(f"{UV}.cv.getBoundingClientRect().width")
            assert b[0] >= 0 and b[0] + b[2] <= cv and b[1] >= 0, b
        step("UV spectrum: the absorbance maximum is labelled with its wavelength (260 nm), inside the plot", uv_labels)
        def uv_labels_zoom():
            pg.evaluate(f"{UV}.zoom=[300,400]"); pg.evaluate(f"drawSoon({UV})"); frames(pg, 3)
            assert pg.evaluate(f"{LBU}.length") == 0, "the label of a maximum out of view stays"
            pg.evaluate(f"{UV}.zoom=null"); pg.evaluate(f"drawSoon({UV})"); attendi(pg, f"{LBU}.length >= 1")
        step("the wavelength labels follow the zoom", uv_labels_zoom)
        def uv_labels_off():
            pg.evaluate(f"COMANDI.fai('spec.display', {UV}, () => {{ {UV}.etpk = false }}); drawSoon({UV})"); frames(pg, 3)
            assert pg.evaluate(f"{LBU}.length") == 0
            pg.evaluate(f"COMANDI.fai('spec.display', {UV}, () => {{ {UV}.etpk = true }}); drawSoon({UV})"); attendi(pg, f"{LBU}.length >= 1")
        step("the wavelength labels can be hidden and shown again", uv_labels_off)
        def header_rt():
            t = pg.evaluate(f"{UV}.el.querySelector('.rtl').textContent"); assert __import__('re').fullmatch(r"(fermo a )?\d+\.\d\d min", t.strip()), t
        step("the header of the UV spectrum says the RT it comes from", header_rt)
        def sheet():
            s = pg.evaluate(f"plotSheets({UV})")
            assert s and s[0]["head"][0] == "nm" and s[0]["head"][1] == "Assorbanza (mAU)" and len(s[0]["rows"]) == 201, s and s[0]["head"]
        step("the export has the nm and mAU columns", sheet)
        def zoom_keys():
            pg.evaluate(f"{UV}.cv.focus()"); pg.keyboard.press("+"); frames(pg, 3)
            z = pg.evaluate(f"{UV}.zoom"); assert z and z[1] - z[0] < 200, z
        step("the keyboard zoom works on the UV spectrum", zoom_keys)
        def chrom_dbl():
            n0 = pg.evaluate("E.panels.filter(q=>q.uv).length")
            pick_kind(pg, "pda")
            ready(pg); attendi(pg, "E.panels.find(q=>q.type==='chrom')._a && E.panels.find(q=>q.type==='chrom').kind==='pda'")
            dbl(pg, "E.panels.find(q=>q.type==='chrom')", 0.5)
            attendi(pg, f"E.panels.filter(q=>q.uv&&q._a).length === {n0 + 1}")
            rt = pg.evaluate("(()=>{const c=E.panels.find(q=>q.type==='chrom'); return E.panels.find(q=>q.uv&&q.src===c.id)._a.uvd.rt})()"); assert abs(rt - 0.5) < 0.06, rt
        step("double click on the PDA trace: the UV spectrum of that time", chrom_dbl)
        def live_uv():
            c = "E.panels.find(q=>q.type==='chrom'&&q.kind==='pda')"; u = f"E.panels.filter(q=>q.uv&&q.src==={c}.id).pop()"
            attendi(pg, f"!!({u}._a && {u}.link === {c}.id)")
            assert pg.evaluate(f"{u}.link === {c}.id") and pg.evaluate(f"{c}.cur != null"), "linked and the vertical line is drawn"
            fol = pg.evaluate(f"(()=>{{const b={u}.el.querySelector('.fol'); return b && b.offsetParent ? b.textContent : null}})()"); assert fol and fol.startswith("\u2190"), fol
            lg = pg.evaluate(f"{u}.leg.textContent"); assert " \u00b7 RT " in lg, lg
            assert pg.evaluate(f"{u}.el.querySelector('.rtl').getAttribute('aria-live')") == "polite"
            assert pg.evaluate(f"{u}.el.querySelector('.rtl').textContent") == "%.2f min" % pg.evaluate(f"{u}._a.uvd.rt"), "rtl"
            pg.evaluate("(()=>{window.__n=0; if(!window.__f){window.__f=window.fetch; window.fetch=function(u,...a){ if(/api\\/(uvspec|map)/.test(String(u))) window.__n++; return window.__f.call(this,u,...a)}}})()")
            for rt in (1.0, 1.5):
                pg.evaluate(f"(()=>{{const c={c}; c.cur={rt}; pushLinked(c,{rt},{rt},0,true)}})()")
                attendi(pg, f"Math.abs({u}._a.uvd.rt - {rt}) < 0.06")
                assert pg.evaluate(f"{u}.el.querySelector('.rtl').textContent") == "%.2f min" % pg.evaluate(f"{u}._a.uvd.rt")
                assert ("RT %.2f" % pg.evaluate(f"{u}._a.uvd.rt")) in pg.evaluate(f"{u}.leg.textContent")
                tx = pg.evaluate(f"TDATI_P.text({u})"); assert ("RT %.2f" % pg.evaluate(f"{u}._a.uvd.rt")) in tx, tx
            assert pg.evaluate("window.__n") == 0, "no uvspec/map request while the line moves"
            pg.evaluate(f"spLinkFreeze({u})"); frames(pg, 2)
            assert pg.evaluate(f"{u}.link") is None and pg.evaluate(f"{u}.el.querySelector('.rtl').textContent").startswith("fermo a ")
            pg.evaluate(f"(()=>{{const c={c}; c.cur=0.5; pushLinked(c,0.5,0.5,0,true)}})()"); frames(pg, 3)
            assert abs(pg.evaluate(f"{u}._a.uvd.rt") - 1.5) < 0.06, "frozen: the UV spectrum does not follow"
            pg.evaluate(f"spLinkFollow({u}, {c})"); attendi(pg, f"{u}.link === {c}.id && Math.abs({u}._a.uvd.rt - 0.5) < 0.06")
        step("UV spectrum from the PDA trace: linked, follows the line without requests, freezes and follows again", live_uv)
        def fol_once():
            c = "E.panels.find(q=>q.type==='chrom'&&q.kind==='pda')"; u = f"E.panels.filter(q=>q.uv&&q.src==={c}.id).pop()"
            for i in range(5):
                rt = 0.4 + 0.2 * i
                pg.evaluate(f"(()=>{{const c={c}; c.cur={rt}; pushLinked(c,{rt},{rt},0,true)}})()"); attendi(pg, f"Math.abs({u}._a.uvd.rt - {rt}) < 0.06")
            pg.evaluate(f"spLinkFreeze({u})"); frames(pg, 2); pg.evaluate(f"spLinkFollow({u}, {c})"); frames(pg, 3)
            pg.evaluate(f"(()=>{{ctl({u}); ctl({u}); ctl({u})}})()"); frames(pg, 3)          # every redraw of the controls (new file, tab, level)
            assert pg.evaluate(f"{u}.el.querySelectorAll('.fol').length") == 1, "one «Follows» button, in the bar or in the «…» menu"
            xb = pg.evaluate(f"[...{u}.el.querySelectorAll('[data-a=xicb],[data-a=xicl]')].every(b => b.hidden || !b.offsetParent)"); assert xb, "no XIC button on a UV spectrum"
        step("UV spectrum: exactly one «Follows» button after the line moves and the source changes, and no XIC button", fol_once)
        def wl_panel():
            c = "E.panels.find(q=>q.type==='chrom')"
            assert pg.evaluate(f"{c}.el.querySelector('[data-a=xic]').textContent") == "λ"
            pg.evaluate(f"{c}.el.querySelector('[data-a=xic]').click()")
            attendi(pg, "E.panels.filter(q=>q.type==='chrom').length === 2 && E.panels.filter(q=>q.type==='chrom')[1]._a")
            w = "E.panels.filter(q=>q.type==='chrom')[1]"
            assert pg.evaluate(f"[{w}.kind, {w}.mz0, {w}.mz1]") == ["pda", 250, 260]
            assert pg.evaluate(f"{w}.el.querySelectorAll('input[data-o=mz0], input[data-o=mz1]').length") == 2
            pg.evaluate(f"(()=>{{const i={w}.el.querySelector('input[data-o=mz0]'); i.value='258'; i.dispatchEvent(new Event('change',{{bubbles:true}}))}})()")
            attendi(pg, f"{w}.mz0 === 258 && {w}._a")
            y = pg.evaluate("fetch('api/chrom?k=0&kind=pda&level=1&mz0=255&mz1=265').then(r=>r.json()).then(j=>Math.max(...j.y))"); assert y > 0, y
        step("PDA trace: the button is λ; it opens a trace by wavelength with an nm range that can be edited", wl_panel)
        def wl_menu():
            ids = pg.evaluate("(()=>{const c=E.panels.find(q=>q.type==='chrom'); return AZIONI.per({vista:'chrom', p:c, k:0, x:0.5}).map(a=>a.id)})()")
            assert "chrom.wl" in ids and "chrom.xic" not in ids, ids
        step("the right-click menu of a PDA trace offers λ and not XIC", wl_menu)
        def n_wl(): return pg.evaluate("E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').length")
        def menu_click(azione):
            pg.evaluate(f"(()=>{{const d=document.querySelector('#ctx [data-azione=\"{azione}\"]'); if(!d) throw new Error('no menu entry {azione}'); d.click()}})()")
        def wl_from_map():
            n0 = n_wl()
            m = "E.panels.find(q=>q.type==='map')"
            pg.evaluate(f"(()=>{{const p={m};p.zoom=null;p.cv.scrollIntoView({{block:'center'}})}})()"); frames(pg, 3)
            at = lambda: pg.evaluate(f"(()=>{{const p={m},b=p.cv.getBoundingClientRect(),a=p._a;return {{x:b.x+a.X(0.5),y:b.y+a.Y(260),t:b.y}}}})()")
            box = at(); pg.mouse.move(box["x"], box["y"]); frames(pg, 3); box = at()       # the hover can move the layout: measure again
            pg.mouse.click(box["x"], box["y"], button="right")
            attendi(pg, "!!document.querySelector('#ctx [data-azione=mapxic]')")
            want = pg.evaluate(f"{m}._a.mzAt({box['y'] - box['t']})")
            menu_click("mapxic")
            attendi(pg, f"E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').length === {n0 + 1}")
            w = pg.evaluate("(()=>{const c=E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').pop(); return [c.mz0, c.mz1]})()")
            assert abs(w[0] - want) < 10 and w[0] == w[1] and 200 <= w[0] <= 400, (w, want)
        step("right click on the PDA map, λ: a trace at the wavelength under the pointer", wl_from_map)
        def wl_from_uv():
            n0 = n_wl()
            pg.evaluate(f"({UV}).zoom=null"); frames(pg, 2)
            box = pg.evaluate(f"(()=>{{const p={UV},b=p.cv.getBoundingClientRect(),a=p._a;return {{x:b.x+a.X(255),y:b.y+M.t+(a.H-M.t-M.b)/2}}}})()")
            pg.mouse.click(box["x"], box["y"], button="right")
            attendi(pg, "!!document.querySelector('#ctx [data-azione=\"uv.wlAt\"]')")
            ids = pg.evaluate("[...document.querySelectorAll('#ctx [data-azione]')].map(d=>d.dataset.azione)")
            # without a zoom there is no visible range to trace: «X» (uv.wlView) is not offered here, the zoomed case is the next steps; no XIC either way
            assert "uv.wlAt" in ids and "uv.wlView" not in ids and "spec.xic" not in ids, ids
            menu_click("uv.wlAt")
            attendi(pg, f"E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').length === {n0 + 1}")
            w = pg.evaluate("(()=>{const c=E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').pop(); return [c.mz0, c.mz1]})()"); assert w[0] == w[1] and abs(w[0] - 255) < 3, w
        step("UV spectrum, right-click menu: λ at the pointer and no XIC", wl_from_uv)
        def wl_range():
            n0 = n_wl()
            pg.evaluate(f"({UV}).cv.focus()"); pg.keyboard.press("+"); frames(pg, 3)
            lo, hi = pg.evaluate(f"[{UV}._a.x0, {UV}._a.x1]"); assert hi - lo < 200, (lo, hi)
            pg.keyboard.press("x")
            attendi(pg, f"E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').length === {n0 + 1}")
            w = pg.evaluate("(()=>{const c=E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').pop(); return [c.mz0, c.mz1]})()")
            assert abs(w[0] - lo) < 0.06 and abs(w[1] - hi) < 0.06, (w, lo, hi)
        step("UV spectrum: zoom with the keys, then X traces the visible nm range (no dragging)", wl_range)
        def undo_wl():
            WL = "E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda')"
            n0 = pg.evaluate(f"{WL}.length")
            pg.evaluate(f"({UV}).cv.focus()"); pg.keyboard.press("Escape")
            pg.evaluate(f"({UV}).zoom=[230,300]"); pg.evaluate(f"draw({UV})"); frames(pg, 3)
            pg.keyboard.press("x"); attendi(pg, f"{WL}.length === {n0 + 1}")
            pg.keyboard.press("Control+z"); attendi(pg, f"{WL}.length === {n0}")      # the opened trace is one undo step
            pg.keyboard.press("Control+y"); attendi(pg, f"{WL}.length === {n0 + 1}")
            assert pg.evaluate(f"{WL}.pop().mz0") == 230
            w = f"{WL}[{n0}]"
            pg.evaluate(f"(()=>{{const i={w}.el.querySelector('input[data-o=mz0]'); i.value='240'; i.dispatchEvent(new Event('change',{{bubbles:true}}))}})()")
            attendi(pg, f"{w}.mz0 === 240")
            pg.evaluate(f"{w}.cv.focus()"); pg.keyboard.press("Control+z"); attendi(pg, f"{w}.mz0 === 230")      # the nm range is undone
        step("lambda trace: opening it and changing its nm range can be undone and redone", undo_wl)
        def undo_uv():
            n0 = pg.evaluate("E.panels.filter(q=>q.uv).length")
            pg.evaluate("(()=>{const c=E.panels.find(q=>q.type==='chrom'&&q.kind==='pda');c.cv.focus()})()")
            dbl(pg, "E.panels.find(q=>q.type==='chrom'&&q.kind==='pda')", 0.7)
            attendi(pg, f"E.panels.filter(q=>q.uv&&q._a).length === {n0 + 1}")
            pg.keyboard.press("Control+z"); attendi(pg, f"E.panels.filter(q=>q.uv).length === {n0}")
            pg.keyboard.press("Control+y"); attendi(pg, f"E.panels.filter(q=>q.uv).length === {n0 + 1}")
        step("UV spectrum: opening it can be undone and redone", undo_uv)
        def axe():
            if not AXE: steps.append(("axe-core not available (SKIP)", "ok")); return
            pg.add_script_tag(path=AXE)
            v = pg.evaluate(RUN % "'#dpanels'"); assert not v, v
        step("axe-core: no serious or critical violation with the UV spectrum open", axe)
        pg.close()
        pg = r.page(p)
        stage(pg, [T0]); pg.click("text=Carica dati"); ready(pg)
        def no_matrix():
            assert pg.evaluate("E.files[0].pda") is True
            pick_kind(pg, "pda"); ready(pg)
            attendi(pg, "E.panels.find(q=>q.type==='chrom')._a && E.panels.find(q=>q.type==='chrom').kind==='pda'")
            dbl(pg, "E.panels.find(q=>q.type==='chrom')", 10)
            attendi(pg, "/DAD/.test(document.body.innerText)")
            assert pg.evaluate("E.panels.filter(q=>q.uv).length") == 0
            pg.evaluate("E.panels.find(q=>q.type==='chrom').el.querySelector('[data-a=xic]').click()")
            attendi(pg, "/DAD/.test(document.body.innerText)")
            assert pg.evaluate("E.panels.filter(q=>q.type==='chrom').length") == 1
        step("a file with only the PDA trace: a clear message and no panel", no_matrix)
        pg.close()
        pg = r.page(p)
        stage(pg, [ROOT / "mzlab" / "web" / "esempi" / "FullScan_t10.mzML"]); pg.click("text=Carica dati"); ready(pg)
        CH = "E.panels.find(q=>q.type==='chrom')"
        UVS = "E.panels.filter(q=>q.uv&&q._a)"
        def demo_uv_at(rt, nm):
            dbl(pg, CH, rt)
            AT = f"{UVS}.filter(q=>Math.abs(q._a.uvd.rt - {rt}) < 0.15)"      # a new spectrum goes under the graph, above the older ones: pick the one of this time, not the last
            attendi(pg, f"{AT}.length > 0")
            a = pg.evaluate(f"(()=>{{const u={AT}[0]._a.uvd; return {{n: u.nm.length, nm0: u.nm[0], nm1: u.nm[u.nm.length-1], rt: u.rt, mx: u.nm[u.y.indexOf(Math.max(...u.y))]}}}})()")
            assert a["n"] == 201 and a["nm0"] == 200 and a["nm1"] == 400 and abs(a["rt"] - rt) < 0.1 and abs(a["mx"] - nm) <= 4, a
        def demo_open():
            pick_kind(pg, "pda"); ready(pg)
            attendi(pg, f"{CH}._a && {CH}.kind==='pda'")
            demo_uv_at(14.30, 224)
        step("FullScan_t10 demo: double click on the PDA trace at 14.30 min opens the UV spectrum, maximum near 224 nm", demo_open)
        step("FullScan_t10 demo: at 7.07 min the UV spectrum has its maximum near 248 nm", lambda: demo_uv_at(7.07, 248))
        def demo_wl():
            n0 = pg.evaluate("E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').length")
            UV1 = f"{UVS}.pop()"
            pg.evaluate(f"(()=>{{const u={UV1};u.zoom=[244,252];draw(u);u.cv.focus()}})()"); frames(pg, 3)
            pg.keyboard.press("x")
            attendi(pg, f"E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').length === {n0 + 1}")
            w = pg.evaluate("(()=>{const c=E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').pop(); return [c.mz0, c.mz1]})()")
            assert abs(w[0] - 244) < 0.06 and abs(w[1] - 252) < 0.06, w
            attendi(pg, "E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').pop()._a")
        step("FullScan_t10 demo: the trace at 248 +/- 4 nm is created without errors", demo_wl)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
