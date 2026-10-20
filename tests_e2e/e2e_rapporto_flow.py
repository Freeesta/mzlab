"""Report page of a Flow run (mzlab/web/rapporto-flow.js): the sections, the tables under the figure, the same HTML from the same JSON, the XSS escape, the toggles,
the JSON download, printing to PDF, axe-core and the time with 5000 features. The module is served alone (no engine needed) with a copy of the colours of the app."""
import sys, os, json, tempfile, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

ROOT = Path(__file__).resolve().parents[1]
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_rapporto_flow.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:1500]))

def corsa(n_steps=3, n_params=2, rows=4):
    hx = lambda k: ("%064x" % k)
    st = [{"instance": f"s{i}", "step": "features.detect@1", "algorithm": "masstrace@1", "status": "error" if i == 2 else "ok", "ms": 100.5 * (i + 1),
           "params_sha256": hx(i), "params": [{"name": f"p{j}", "value": j * 1.5, "state": "estimated" if j == 0 else "fixed", **({"proof": {"hist": [1, 2]}} if j == 0 else {})} for j in range(n_params)]}
          for i in range(n_steps)]
    if n_steps > 2: st[2]["error_key"] = "err.x"
    return {"schema": "corsa/1", "engine": {"name": "mzLab", "version": "1.0", "platform": "native"}, "plan_sha256": hx(99),
            "inputs": [{"name": "<img src=x onerror=window.PWNED=1>.mzML", "bytes": 2048, "sha256": hx(1), "type": "mzml"}], "lists": [{"name": "lib.msp", "bytes": 10, "sha256": hx(2), "type": "library"}],
            "steps": st, "total_ms": round(sum(s["ms"] for s in st), 1), "levels": {"files": 1, "rows": 4, "auto": {"2a": 2, "3a": 1}, "other": {"1": 1}},
            "report": {"schema": "report/1", "righe": rows, "file": 2, "celle": {"rilevate": 5, "mirate": 0, "riempite": 1, "assenti": 2},
                       "qc": {"schema": "qc/1", "file": [{"file": "a.mzML", "rilevati": 3, "mirati": 0, "riempiti": 1, "assenti": 0, "area_totale": 1e6}],
                              "gruppi": [{"tipo": "sample", "gruppo": "g", "tempo_min": 0, "file": 2, "righe": 4, "cv_mediano": 0.12}], "standard_cv": None, "scartate": [{"regola": "blank", "righe": 1}]}}}

HOST = """<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="icon" href="data:,"><title>host</title><style>:root{--ink:#1a1a1a;--line:#999;--accent:#0072B2;--bad:#D55E00}body{background:#fff;margin:0}</style></head>
<body><main id="m"></main><script src="rapporto-flow.js"></script></body></html>"""
tmp = Path(tempfile.mkdtemp(prefix="rf_"))
shutil.copy(ROOT / "mzlab/web/rapporto-flow.js", tmp / "rapporto-flow.js"); (tmp / "index.html").write_text(HOST)
port = free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).with_name("servi.py")), str(port), str(tmp)], cwd=str(ROOT), stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
try:
    import urllib.request
    for _ in range(100):
        try: urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=1).read(); break
        except Exception: time.sleep(0.1)
    errs = []
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_page(accept_downloads=True)
        pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("console", lambda m: errs.append(m.text) if m.type in ("error", "warning") else None)
        pg.goto(f"http://127.0.0.1:{port}/")
        R = "RAPPORTO_FLOW.render(document.getElementById('m'), %s)"
        pg.evaluate(R % json.dumps(corsa()))
        def sections():
            ids = pg.evaluate("[...document.querySelectorAll('section')].map(s => s.id)")
            assert ids == ["rf-summary", "rf-inputs", "rf-plan", "rf-timing", "rf-counts", "rf-qc"], ids
            assert pg.evaluate("document.querySelectorAll('h1').length") == 1
        step("the six sections, one column, one h1", sections)
        def tables():
            assert pg.evaluate("[...document.querySelectorAll('svg')].every(s => s.nextElementSibling && s.nextElementSibling.tagName === 'TABLE')")
            assert pg.evaluate("[...document.querySelectorAll('table')].every(t => t.querySelector('caption') && t.querySelector('th[scope=col]'))")
            assert pg.evaluate("document.querySelector('svg').getAttribute('role') === 'img' && !!document.querySelector('svg').getAttribute('aria-label')")
        step("every figure has its data table right under it; tables have caption and headers", tables)
        def content():
            t = pg.inner_text("#rf-counts"); assert "auto" in t and "2a" in t, t
            t = pg.inner_text("#rf-plan"); assert "estimated" in t and "fixed" in t and "err.x" in t, t
            t = pg.inner_text("#rf-qc"); assert "Median CV" in t and "blank" in t, t
        step("levels counted as auto, estimated/fixed parameters, the error key and the QC tables", content)
        def escape():
            assert pg.evaluate("window.PWNED") is None and "<img" in pg.inner_text("#rf-inputs")
        step("file names are escaped", escape)
        def same():
            a = pg.evaluate("document.getElementById('m').innerHTML")
            pg.evaluate(R % json.dumps(corsa())); c = pg.evaluate("document.getElementById('m').innerHTML")
            norm = lambda s: " ".join(s.split())
            assert norm(a) == norm(c) and len(a) > 2000
            pg.evaluate(R % json.dumps({**corsa(), "report": None})); assert "no table report" in pg.inner_text("#rf-qc")
            pg.evaluate(R % json.dumps(corsa()))
        step("same JSON, same HTML (normalized innerHTML); a run with no table report says so", same)
        def toggle():
            pg.uncheck("input[data-sec=timing]"); assert pg.evaluate("document.getElementById('rf-timing').hidden") and not pg.is_visible("#rf-timing")
            pg.check("input[data-sec=timing]"); assert pg.is_visible("#rf-timing")
            pg.evaluate("RAPPORTO_FLOW.render(document.getElementById('m'), %s, {sections: {qc: false}})" % json.dumps(corsa())); assert not pg.is_visible("#rf-qc")
            pg.evaluate(R % json.dumps(corsa()))
        step("each section can be turned off and on", toggle)
        def download():
            with pg.expect_download() as d: pg.click("button[data-rf=json]")
            f = d.value; data = json.loads(Path(f.path()).read_text()); assert f.suggested_filename == "corsa.json" and data["schema"] == "corsa/1"
        step("Download JSON gives the run as it was given", download)
        def pdf():
            out = tmp / "r.pdf"; pg.pdf(path=str(out)); assert out.stat().st_size > 2000
            pg.emulate_media(media="print"); assert not pg.is_visible(".rf .bar"); pg.emulate_media(media="screen")
        step("printing to PDF works and the toolbar is left out of the print", pdf)
        AXE = axe_path()
        def axe():
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("axe.run('#m', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length]))")
            assert not v, v
        if AXE: step("axe-core: no violation", axe)
        else: print("SKIP axe-core not available")
        def small():
            r = pg.evaluate("[...document.querySelectorAll('.rf button, .rf label')].map(e => e.getBoundingClientRect()).filter(r => r.width < 24 || r.height < 24).length"); assert r == 0, r
        step("WCAG 2.5.8: buttons and labels at least 24 x 24 px", small)
        def speed():
            big = corsa(50, 100, 5000); t0 = time.time()
            pg.evaluate(R % json.dumps(big)); ms = pg.evaluate("performance.now()") and (time.time() - t0) * 1000
            assert ms < 3000, ms
            print(f"MISURA rapporto_flow: 50 steps x 100 parameters (5000), 5000 parameter rows -> {ms:.0f} ms")
        step("render in under 3 s with 5000 features", speed)
        b.close()
    assert not errs, errs
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
