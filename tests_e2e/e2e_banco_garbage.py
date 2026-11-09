"""Garbage in the fields of the HR bench and of the subtraction (campi.js gate, lib_campi.sweep): the isotope pattern (charge states, points, resolution,
concentration), «XIC from formula» (ppm, minimum intensity), a new XIC trace (ppm) and the subtraction popover (internal standard m/z, RT tolerance); each
field refuses letters, 1e999, empty where a value is due, out-of-bound numbers and decimals where a whole number is due with one note and the old value, the
state does not change and no request carries NaN, Infinity or undefined; a comma decimal is taken."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
from lib_campi import sweep, typed
from synth import make_subtraction
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_banco_garbage.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = synth("banco_garbage")
SUB = make_subtraction("/tmp/wd_banco_garbage_files", blank=False)
r, r2 = Run(port=8951, wd="/tmp/wd_banco_garbage"), Run(port=8952, wd="/tmp/wd_banco_garbage2")
RANGES = "JSON.stringify(E.panels.map(q => q.ranges || []))"
urls = []
def clean(pg):
    def requests():
        bad = [u for u in urls if any(s in u for s in ("NaN", "Infinity", "undefined"))]
        assert not bad, bad[:3]
    return requests
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1500})
        pg.on("request", lambda q: urls.append(q.url))
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); ready(pg)
        def iso():
            pg.evaluate("BARRA.setTab('iso')"); wait_for_js(pg, "!!document.querySelector('#hri-z0') && !!document.querySelector('#hri-z0').offsetParent", 15000)
            sweep(pg, "#hri-body")
            typed(pg, "#hri-z0", "2,5"); assert pg.input_value("#hri-z0") == "2,5" and pg.get_attribute("#hri-z0", "aria-invalid") is None
        step("isotope pattern: charge states, points, resolution and concentration refuse garbage; «2,5» is taken", iso)
        def sub_formula():
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'))")
            pg.click("#hrbar [data-hb=add]"); pg.click("#ctx >> text=XIC da formula"); pg.wait_for_selector("#bs-ppm")
            sweep(pg, "#bigdlg")
            typed(pg, "#bs-ppm", "2,5"); assert pg.input_value("#bs-ppm") == "2,5"
            pg.evaluate("document.querySelector('#bigdlg').close()")
        step("«XIC from formula»: ppm and minimum intensity refuse garbage; «2,5» ppm is taken", sub_formula)
        def new_trace():
            pg.evaluate("BANCO.addRange(E.panels.find(p=>p.type==='chrom'))"); pg.wait_for_selector("#br-kind")
            pg.select_option("#br-kind", "xic"); wait_for_js(pg, "!!document.querySelector('#br-ppm').offsetParent", 5000)
            sweep(pg, "#bigdlg", RANGES)
            pg.fill("#br-ion", "250,1"); typed(pg, "#br-ppm", "2,5"); pg.click("#br-ok")
            wait_for_js(pg, "E.panels.some(q => (q.ranges || []).some(x => x.kind === 'xic' && x.ppm === 2.5))", 10000)
        step("a new XIC trace: the ppm refuses garbage; «2,5» ppm is taken", new_trace)
        step("no request carried NaN, Infinity or undefined", clean(pg))
        pg2 = r2.page(p)
        pg2.on("request", lambda q: urls.append(q.url))
        stage(pg2, SUB); pg2.click("text=Carica dati"); ready(pg2)
        pg2.evaluate("E.browse=false; E.files.forEach(f=>f.vis=true); renderNav(); renderFileList(); redrawAll()"); ready(pg2)
        pg2.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:253.3,w:0.5,label:'m/z 253'}]})"); ready(pg2)
        def subtraction():
            pg2.keyboard.press("Control+k"); pg2.keyboard.type("sottrai"); frames(pg2, 3); pg2.keyboard.press("Enter"); ready(pg2)
            wait_for_js(pg2, "document.querySelectorAll('.subchip').length >= 1", 15000)
            if not pg2.locator(".corrbar:not([hidden])").count(): pg2.locator("[data-o=cinfo]").first.click()
            pg2.locator(".subchip [data-s=pop]").first.click(); pg2.wait_for_selector("#subpop #sp-norm", state="visible")
            pg2.select_option("#sp-norm", "istd"); wait_for_js(pg2, "!!document.querySelector('#sp-istd').offsetParent", 5000)
            sweep(pg2, "#subpop", "JSON.stringify(S.ref)")
            typed(pg2, "#sp-rt", "0,5"); wait_for_js(pg2, "S.ref.rttol === 0.5", 5000)
            typed(pg2, "#sp-istd", "150,2"); wait_for_js(pg2, "S.ref.istd === '150.2' && S.ref.norm === 'istd'", 5000)
        step("subtraction: the internal standard m/z and the RT tolerance refuse garbage; «0,5» and «150,2» are taken", subtraction)
        step("no request carried NaN, Infinity or undefined (subtraction)", clean(pg2))
    r.close(); r2.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    for x in (r, r2):
        try: x.close()
        except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
