"""Garbage in the fields of the Statistica sections and of the QA/QC (campi.js gate, lib_campi.sweep): blank subtraction, difference from the reference,
PCA, PFAS, time profile and QA/QC; each field refuses letters, 1e999, empty where a value is due, out-of-bound numbers and decimals where a whole number
is due with one note and the old value, a refused field blocks its form, «auto» is taken as the pseudocount and a comma decimal is taken."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_campi import sweep, typed
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_analisi_garbage.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
# five samples and a blank: enough for the PCA section to show its form
D = HERE / "shots" / "analisi_garbage_tmp"; D.mkdir(parents=True, exist_ok=True)
IONS = [(305.1234, 6.0), (350.2000, 7.0), (410.3000, 8.0)]
for i, t in enumerate((0, 10, 30, 60, 90)):
    ds.hr_series(D / f"G_t{t}.mzML", np.random.default_rng(20 + i), [(m, rt, 0.06, 1e5 * (1 + j + t / 30)) for j, (m, rt) in enumerate(IONS)])
ds.hr_series(D / "G_Blank_1.mzML", np.random.default_rng(9), [(IONS[2][0], IONS[2][1], 0.06, 3e5)])
FILES = sorted(D.glob("*.mzML"))
urls = []
r = Run(port=8953, wd="/tmp/wd_analisi_garbage")
try:
    with sync_playwright() as p:
        pg = r.page(p, qaqc=True); pg.set_viewport_size({"width": 1500, "height": 1500})
        pg.on("request", lambda q: urls.append(q.url))
        pg.set_input_files("#pick", [str(f) for f in FILES])
        pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length>={len(FILES)}", timeout=120000)
        open_first(pg); pg.wait_for_function(f"typeof E!=='undefined' && E.files.length>={len(FILES)}", timeout=60000); ready(pg)
        drawer(pg, "stat"); pg.wait_for_selector("#feat-list")
        pg.fill("#feat-list", "\n".join(f"{m}; {rt}" for m, rt in IONS)); pg.click("#feat-go")
        pg.wait_for_selector("#feat-tab tbody tr", timeout=120000)
        wait_for_js(pg, "!!document.querySelector('#pca-form') && !!document.querySelector('#ref-form')", 60000)
        def stat():
            for sec in ("blank", "reference", "pca", "pfas", "profile"):
                pg.evaluate(f"document.querySelector('details.stat-sec[data-s={sec}]').open = true")
                sweep(pg, f"#stat-{sec}")
        step("blank, reference, PCA, PFAS and time profile: every number refuses garbage", stat)
        def form_blocked():
            pg.click("#blank-go"); wait_for_js(pg, "!!BIANCO.result", 30000)
            r0 = pg.evaluate("JSON.stringify(BIANCO.result.r)")
            x = pg.locator("#blank-n"); x.focus(); x.press("ControlOrMeta+a"); x.press_sequentially("abc"); x.press("Enter")
            wait_for_js(pg, "document.querySelector('#blank-n').getAttribute('aria-invalid') === 'true'", 5000)
            assert pg.input_value("#blank-n") != "abc" and pg.evaluate("JSON.stringify(BIANCO.result.r)") == r0, "a refused field runs nothing"
        step("Enter in a refused field runs nothing: the old value is back", form_blocked)
        def taken():
            typed(pg, "#blank-f", "2,5"); pg.click("#blank-go"); wait_for_js(pg, "BIANCO.result && BIANCO.result.r.factor === 2.5", 30000)
            fresh = "(RIFERIMENTO.result.r.old = 1, 0)"   # a new run gives a new result object
            typed(pg, "#ref-p", "auto"); assert pg.input_value("#ref-p") == "auto" and pg.get_attribute("#ref-p", "aria-invalid") is None
            pg.evaluate(fresh); pg.click("#ref-go"); wait_for_js(pg, "RIFERIMENTO.result && !RIFERIMENTO.result.r.old && RIFERIMENTO.result.r.params.pseudo_source !== 'given'", 30000)
            pg.evaluate(fresh); typed(pg, "#ref-p", "1000,5"); pg.click("#ref-go"); wait_for_js(pg, "RIFERIMENTO.result && !RIFERIMENTO.result.r.old && RIFERIMENTO.result.r.params.pseudo === 1000.5", 30000)
        step("«2,5» is taken as the blank factor; «auto» and «1000,5» as the pseudocount", taken)
        def qaqc():
            drawer(pg, "qaqc"); pg.wait_for_selector("#qq-win")
            sweep(pg, "#sb-panel-qaqc", "localStorage.getItem('mzlab.qaqc')")
            typed(pg, "#qq-win", "0,4"); wait_for_js(pg, "JSON.parse(localStorage.getItem('mzlab.qaqc')).win === 0.4", 5000)
        step("QA/QC: retention times and window refuse garbage; «0,4» is taken as the window", qaqc)
        def clean():
            bad = [u for u in urls if any(s in u for s in ("NaN", "Infinity", "undefined"))]
            assert not bad, bad[:3]
        step("no request carried NaN, Infinity or undefined", clean)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
