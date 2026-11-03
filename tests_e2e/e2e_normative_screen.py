"""Regulatory screening in the run report: a native plan runs both paths (`screen.suspects`, `annot.regulatory`) on the invented 6-row package, `mzlab-cli flow report`
makes the `corsa/1`, and `rapporto-flow.js` lays out its «Regulatory lists» section: header with versions and SHA-256, coverage, the «Listed in» column, the
membership lines, the study disclaimer, no verdict word, axe-core clean; a run without lists has no such section. Needs cargo."""
import sys, os, json, re, tempfile, shutil, threading; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

ROOT = Path(__file__).resolve().parents[1]
if not shutil.which("cargo"):
    print("  ", ("normative", "SKIP cargo is needed to run the plan")); sys.exit(0)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_normative_screen.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:1500]))

tmp = Path(tempfile.mkdtemp(prefix="ns_"))
run_dir = tmp / "run"
subprocess.run(["cargo", "test", "--locked", "-q", "-p", "mzlab-cli", "--lib", "both_paths_run_in_one_plan_for_the_wasm_parity"], cwd=str(ROOT), check=True,
               capture_output=True, env={**os.environ, "MZLAB_PARITA_NORMATIVE": str(run_dir)})
for f in ("table.mzfeat", "normative.mzsus"): (run_dir / f).unlink()
corsa = json.loads(subprocess.run(["cargo", "run", "--locked", "-q", "-p", "mzlab-cli", "--", "flow", "report", str(run_dir)], cwd=str(ROOT), check=True, capture_output=True).stdout)
FORBIDDEN = re.compile(r"\b(detected|present|non-compliant|exceeds?|violations?|contaminated)\b", re.I)

HOST = """<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="icon" href="data:,"><title>host</title><style>:root{--ink:#1a1a1a;--line:#999;--accent:#0072B2;--bad:#D55E00}body{background:#fff;margin:0}</style></head>
<body><main id="m"></main><script src="rapporto-flow.js"></script></body></html>"""
shutil.copy(ROOT / "mzlab/web/rapporto-flow.js", tmp / "rapporto-flow.js"); (tmp / "index.html").write_text(HOST)
port = free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).with_name("servi.py")), str(port), str(tmp)], cwd=str(ROOT), stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
try:
    import urllib.request
    for _ in range(100):
        try: urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=1).read(); break
        except Exception: threading.Event().wait(0.1)
    errs = []
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_page()
        pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("console", lambda m: errs.append(m.text) if m.type in ("error", "warning") else None)
        pg.goto(f"http://127.0.0.1:{port}/")
        R = "RAPPORTO_FLOW.render(document.getElementById('m'), %s)"
        pg.evaluate(R % json.dumps(corsa))
        g = corsa["regulatory"]
        def section():
            t = pg.inner_text("#rf-regulatory")
            assert pg.inner_text("#rf-regulatory-h") == "Regulatory lists", t
            assert "Screened against:" in t and "version 2000-01-01" in t and "retrieved 2000-01-02" in t and "SHA-256" in t, t
            assert "4 lists in force, 1 proposal." in g["header"], g["header"]
            assert "not screened by mass." in t and g["disclaimer"] in t, t
        step("the section has the header with versions, the coverage and the disclaimer", section)
        def table():
            head = pg.eval_on_selector_all("#rf-regulatory th[scope=col]", "e => e.map(x => x.textContent)")
            assert "Listed in" in head, head
            t = pg.inner_text("#rf-regulatory table")
            assert "Candidate formula C14H20N2O4 matches 2 listed substances" in t and "Isomerone A" in t and "Isomerone B" in t, t
            assert "Candidate structure matches a listed substance (InChIKey)" in t, t
            assert '(MCL 4 ng/L), version of 2000-01-01' in t, t
        step("the table names every isomer, the lists and the list's own words", table)
        def words():
            t = pg.inner_text("#rf-regulatory"); m = FORBIDDEN.search(t); assert not m, m.group(0)
        step("no verdict word in the section", words)
        AXE = axe_path()
        def axe():
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("axe.run('#rf-regulatory', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length]))")
            assert not v, v
        if AXE: step("axe-core: no violation", axe)
        else: print("SKIP axe-core not available")
        def toggle():
            pg.uncheck("input[data-sec=regulatory]"); assert not pg.is_visible("#rf-regulatory")
        step("the toolbar turns the section off", toggle)
        def absent():
            c = {k: v for k, v in corsa.items() if k != "regulatory"}
            pg.evaluate(R % json.dumps(c)); assert pg.locator("#rf-regulatory").count() == 0 and pg.locator("input[data-sec=regulatory]").count() == 0
        step("a run without regulatory lists has no such section", absent)
        b.close()
    assert not errs, errs
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
