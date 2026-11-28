"""e2e_pbar_gruppi: the kind-specific buttons live in the shared panel bar #pbar, in the labelled groups View, Analyse, Extract, Output.
A chromatogram has all four (integration set in Analyse, XIC in Extract), a spectrum has no integration (peak picking is its Analyse, the XIC button its Extract), a SIM/MRM panel adds the area toggle to View,
the RT x m/z map has View (fit, zoom in and out, gestures) and Output only; an action that is not available is hidden, never greyed; a group with nothing to show is not drawn; the buttons work on the selected panel."""
# covers: mzlab/web/azioni-barra.js, mzlab/web/pbar.js, mzlab/web/sim-ui.js, mzlab/web/mappa.js, tests_e2e/lib_barra.py
import json, os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=9151, wd="/tmp/wd_pbar_gruppi")
GROUPS = "[...document.querySelectorAll('#pbar .pbtools .tbg')].filter(g=>!g.hidden).map(g=>g.dataset.pg+':'+g.querySelector('.tbl').textContent)"
IDS = "(g=>[...document.querySelectorAll('#pbar .pbtools .tbg[data-pg='+g+'] [data-a]')].filter(b=>!b.hidden).map(b=>b.dataset.a))"
def pick(pg, sel):
    """selects the first panel of this selector (what a click on it does) and waits for the bar to follow"""
    pg.evaluate("s => setActive(E.panels.find(q => q.el && q.el.matches(s)))", sel)
    wait_for_js(pg, f"E.active && E.active.el.matches({json.dumps(sel)}) && document.querySelector('#pbar .pbchip') && document.querySelector('#pbar .pbchip').textContent.startsWith(E.active.num + ' ')")
def fresh(p, f):
    pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 1000})
    pg.add_init_script("try{localStorage.setItem('mzlab.pbar','1');}catch(_){}")
    pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#pick", state="attached")
    pg.set_input_files("#pick", [str(EX / f)]); pg.wait_for_selector("#opbtn", state="visible"); open_first(pg); ready(pg)
    wait_for_js(pg, "!document.querySelector('#pbar').hidden")
    return pg
try:
    with sync_playwright() as p:
        pg = fresh(p, "FullScan_t0.mzML")
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.click("#np-map"); ready(pg)
        wait_for_js(pg, "E.panels.some(q=>q.type==='spec') && E.panels.some(q=>q.type==='map') && E.panels.some(q=>q.type==='xic')")
        def chrom():
            pick(pg, ".pnl.chrom")
            assert pg.evaluate(GROUPS) == ["v:View", "a:Analyse", "x:Extract", "o:Output"], pg.evaluate(GROUPS)
            a = pg.evaluate(IDS + "('a')"); assert a[:3] == ["iauto", "iall", "iman"], a
            assert pg.evaluate(IDS + "('x')") == ["xic", "ticbg"], pg.evaluate(IDS + "('x')")   # the TIC background switch is an extraction of the trace, like the XIC
            o = pg.evaluate(IDS + "('o')"); assert o == ["dl", "max"], o
            v = pg.evaluate(IDS + "('v')"); assert v[:2] == ["izoom", "fit"] and "smooth" in v and "log" in v, v
        step("chromatogram: View, Analyse (integration), Extract (XIC), Output, labelled", chrom)
        def ticbg():
            pick(pg, ".pnl.chrom")
            assert pg.evaluate("E.active.kind==='tic'"), "the first chromatogram of the example is the TIC"
            B = "#pbar [data-a=ticbg]"; assert pg.evaluate(f"document.querySelector('{B}').closest('.tbg').dataset.pg") == "x"
            assert pg.evaluate("E.active.el.querySelector('[data-a=ticbg]').offsetParent") is None, "the header twin is not shown with the bar on"
            pg.locator(B).click(button="right")
            pg.wait_for_selector(".pnl.chrom .tb-pop", timeout=10000)
            got = pg.evaluate("""() => { const p = E.active, q = p.el.getBoundingClientRect(), o = p.el.querySelector('.tb-pop').getBoundingClientRect(), h = p.el.querySelector('.hd').getBoundingClientRect();
              return { top: o.top - h.bottom, bottom: q.bottom - o.bottom, left: o.left - q.left, right: q.right - o.right, bar: document.querySelector('#pbar [data-a=ticbg]').getAttribute('aria-expanded'), hd: p.el.querySelector('[data-a=ticbg]').getAttribute('aria-expanded') }; }""")
            assert got["top"] >= 0 and got["left"] >= 0 and got["right"] >= 0 and got["bottom"] >= 0, ("the options are under the header, inside the panel", got)
            assert got["bar"] == "true" and got["hd"] == "true", ("aria-expanded on both twins", got)
            pg.keyboard.press("Escape"); pg.wait_for_selector(".pnl.chrom .tb-pop", state="detached", timeout=5000)
            assert pg.evaluate(f"document.activeElement === document.querySelector('{B}')"), "Esc puts the focus back on the bar button"
            assert pg.evaluate("[...document.querySelectorAll('[data-a=ticbg]')].every(b => b.getAttribute('aria-expanded') === 'false')")
            pg.keyboard.press("ArrowDown"); pg.wait_for_selector(".pnl.chrom .tb-pop", timeout=5000); pg.keyboard.press("Escape")
            pg.wait_for_selector(".pnl.chrom .tb-pop", state="detached", timeout=5000)
        step("TIC: the background switch is in Extract; its options open under the panel header, Esc returns to the bar button, aria-expanded on both twins", ticbg)
        def xic():
            pick(pg, ".pnl.xic")
            assert pg.evaluate(GROUPS) == ["v:View", "a:Analyse", "o:Output"], pg.evaluate(GROUPS)
        step("XIC panel: no Extract group (it has no XIC button), never a grey gap", xic)
        def spec():
            pick(pg, ".pnl.spec")
            g = pg.evaluate(GROUPS); assert g == ["v:View", "a:Analyse", "x:Extract", "o:Output"], g
            assert pg.evaluate(IDS + "('a')") == ["par"], pg.evaluate(IDS + "('a')")
            assert pg.evaluate(IDS + "('x')")[:1] == ["xicb"], pg.evaluate(IDS + "('x')")
            assert pg.evaluate(IDS + "('v')") == ["fit", "rul", "rel", "lock"], pg.evaluate(IDS + "('v')")
            assert pg.evaluate("!document.querySelector('#pbar [data-a=iauto], #pbar [data-a=tlink], #pbar [data-a=smooth], #pbar [data-a=log]')"), "a spectrum has no integration, link, smoothing or log"
        step("spectrum: peak picking in Analyse, XIC in Extract, ruler, %, lock in View; no integration", spec)
        def spec_work():
            pick(pg, ".pnl.spec")
            was = pg.evaluate("!!specRel(E.active)"); bar(pg, ".pnl.spec", "rel"); wait_for_js(pg, f"!!specRel(E.active)===({str(not was).lower()})")
            wait_for_js(pg, f"document.querySelector('#pbar [data-a=rel]').getAttribute('class').includes('on')==={str(not was).lower()}")
            bar(pg, ".pnl.spec", "rel"); wait_for_js(pg, f"!!specRel(E.active)===({str(was).lower()})")
            bar(pg, ".pnl.spec", "par"); pg.wait_for_selector(".pnl.spec .sp-pop"); pg.keyboard.press("Escape")
        step("the spectrum buttons of the bar act on the spectrum (%, peak picking)", spec_work)
        def chrom_work():
            pick(pg, ".pnl.chrom")
            bar(pg, ".pnl.chrom", "iauto"); wait_for_js(pg, "E.active.imode==='auto'"); bar(pg, ".pnl.chrom", "iauto"); wait_for_js(pg, "E.active.imode!=='auto'")
            bar(pg, ".pnl.chrom", "xic"); wait_for_js(pg, "document.querySelector('#xicdlg') && document.querySelector('#xicdlg').open"); pg.keyboard.press("Escape")
        step("integration and XIC buttons of the bar act on the chromatogram", chrom_work)
        def hidden_not_grey():
            pick(pg, ".pnl.chrom")
            pg.evaluate("(()=>{const p=E.active;p._tab0=p.tab;p.tab=ESP.ms2;AZIONI.refresh({view:p.type,p})})()")   # a chromatogram of the MS2 tab: no log, no XIC (ctl.noMs2)
            wait_for_js(pg, "(()=>{const b=document.querySelector('#pbar [data-a=log]');return !b||b.hidden})()")
            assert pg.evaluate("!document.querySelector('#pbar .pbtools [data-a=xic]:not([hidden])')"), "XIC is hidden on an MS2 chromatogram"
            assert pg.evaluate(IDS + "('x')") == ["ticbg"], pg.evaluate(IDS + "('x')")   # Extract keeps only the TIC background (it does not depend on the tab)
            pg.evaluate("(()=>{const p=E.active;p.tab=p._tab0;AZIONI.refresh({view:p.type,p})})()")
            wait_for_js(pg, "!document.querySelector('#pbar [data-a=log]').hidden && !document.querySelector('#pbar [data-a=xic]').hidden")
        step("an action that is not available is hidden, and a group with nothing in it is not drawn", hidden_not_grey)
        def map2d():
            pick(pg, ".pnl.map")
            assert pg.evaluate(GROUPS) == ["v:View", "o:Output"], pg.evaluate(GROUPS)
            assert pg.evaluate(IDS + "('v')") == ["fit", "mapzin", "mapzout", "maphelp"], pg.evaluate(IDS + "('v')")
            assert pg.evaluate(IDS + "('o')") == ["dl", "max"]
            assert pg.evaluate("!!document.querySelector('.pnl.map .mznav') && document.querySelector('.pnl.map .mznav').offsetParent===null"), "the header nav group is hidden; the map keeps its own logic"
            z0 = pg.evaluate("JSON.stringify(E.active.zoom||null)"); bar(pg, ".pnl.map", "mapzin"); wait_for_js(pg, f"JSON.stringify(E.active.zoom||null)!=={json.dumps(z0)}")
            bar(pg, ".pnl.map", "maphelp"); wait_for_js(pg, "document.getElementById('bigdlg').open"); pg.keyboard.press("Escape")
        step("map: View (fit, zoom, gestures) and Output through the same renderer; the buttons work", map2d)
        pg.context.close()
        pg = fresh(p, "MRM_std_0.6ppm.mzML")
        def mrm():
            wait_for_js(pg, "E.panels.some(q=>q.type==='mrm')"); pick(pg, ".pnl.mrm")
            v = pg.evaluate(IDS + "('v')"); assert "area" in v, v
            on = pg.evaluate("E.active.area!==false"); bar(pg, ".pnl.mrm", "area"); wait_for_js(pg, f"(E.active.area!==false)===({str(not on).lower()})")
        step("SIM/MRM: the area toggle sits in View and works", mrm)
        pg.context.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
