"""One integration table: the toolbar button of the former «Peak integrations» dialog is gone, and the Manual integrations card carries everything the dialog had
(S/N, w1/2, plates, tailing, quality switches; noise-zone pick; CSV and Excel export)."""
# covers: mzlab/web/integrazioni-manuali.js, mzlab/web/pannelli.js, mzlab/web/picchi.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
XP = "E.panels.find(p=>p.type==='xic')"
r = Run(port=8884, wd="/tmp/wd84")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=2", timeout=60000)
        pg.click("text=Load data"); ready(pg)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.wait_for_function(f"(()=>{{const p={XP};return !!(p && p._a && p._a.sr && p._a.sr.length)}})()", timeout=60000)
        pg.evaluate(f"(()=>{{const p={XP};p.sr=p._a.sr;addInt(p,p._a.sr[0],13.9,14.9)}})()")
        wait_for_js(pg, "!!document.querySelector('#manint-h')", timeout=60000)
        pg.evaluate("(h => { if (h.getAttribute('aria-expanded') !== 'true') h.click(); })(document.querySelector('#manint-h'))")
        wait_for_js(pg, "!!document.querySelector('#manint-b > table tbody tr')", timeout=60000)
        def gone():
            assert pg.locator("[data-a=itab]").count() == 0, "the toolbar button of the integrations dialog is gone"
            assert pg.evaluate("typeof showInts") == "undefined" and pg.evaluate("typeof allInts") == "undefined"
        step("no integrations button on the panels, no dialog function", gone)
        def options():
            assert pg.locator("#manint input[data-opt]").count() == 3, "S/N, parameters and quality switches are on the card"
            for k in ("sn", "par", "q"): pg.evaluate("k => { const i = document.querySelector(`#manint input[data-opt=${k}]`); if (!i.checked) i.click(); }", k)
            wait_for_js(pg, "['N (plates)', 'Gaussian similarity'].every(h => document.querySelector('#manint').innerText.includes(h))", timeout=60000)
        step("the card has the switches and the columns they add", options)
        def exports():
            with pg.expect_download() as d: pg.click("#manint [data-dl]")
            assert d.value.suggested_filename.endswith(".csv"), d.value.suggested_filename
            with pg.expect_download() as d: pg.click("#manint [data-xlsx]")
            hd = [c[1] for c in xlsx_rows(d.value.path())[0] if c]; assert any("plates" in x for x in hd), hd
        step("CSV and Excel export work and carry the optional columns", exports)
        step("no page errors", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] in ("pageerror", "console.error")]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
