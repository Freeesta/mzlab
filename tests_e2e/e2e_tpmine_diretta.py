"""mzFinder «Dig» live: while the engine works the right-hand side shows the phases (at least one done), the candidates that pass the criteria and, in HR, the tree
growing. Synthetic LR demo and synthetic HR series on the built site. Run from the repo root: python3 tests_e2e/e2e_tpmine_diretta.py   (env: PYODIDE_DIR)"""
import os, subprocess, sys, time, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
SRC = ROOT / "TP_Mine"
PORT = 8847
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:400]))
tmp = Path(tempfile.mkdtemp())
cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")]
if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
subprocess.run(cmd, check=True)
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []
# records what the live panel showed before the results replaced it
WATCH = """() => { window.__lv = {done: 0, cands: 0, nodes: 0, sr: [], finished: false};
  const R = document.getElementById('tp-right');
  new MutationObserver(() => {
    const L = document.getElementById('tp-live');
    if (!L) { if (document.querySelector('#tp-view .gem, #hr-t')) window.__lv.finished = true; return; }
    const v = window.__lv; v.done = Math.max(v.done, L.querySelectorAll('li.ph[data-s=done]').length);
    v.cands = Math.max(v.cands, L.querySelectorAll('.cand').length); v.nodes = Math.max(v.nodes, L.querySelectorAll('#tp-live-t .nd').length);
    const s = document.getElementById('tp-live-sr').textContent; if (s && v.sr[v.sr.length - 1] !== s) v.sr.push(s);
  }).observe(R, {subtree: true, childList: true, characterData: true, attributes: true}); }"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1500, "height": 1400}, reduced_motion="reduce").new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_timeout(3200)
        for _ in range(5): pg.click("header img.logo")
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        def lr():
            sys.path[:0] = [str(SRC / "py"), str(ROOT)]
            from tpmine.demo import make_demo
            fl = make_demo(Path(tempfile.mkdtemp()))
            pg.set_input_files("#tp-in", [f["path"] for f in fl])
            pg.wait_for_function("document.querySelector('#tp-files table') && document.querySelectorAll('#tp-files tr').length >= 9", timeout=240000)
            pg.fill("#tp-smi", "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O")
            pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('241.0641')", timeout=60000)
            pg.evaluate(WATCH)
            pg.click("#tp-go"); pg.wait_for_selector("#tp-view .gem", timeout=300000)
            v = pg.evaluate("window.__lv")
            assert v["done"] >= 1 and v["cands"] >= 1 and v["finished"], v
            assert len(v["sr"]) >= 2 and len(v["sr"]) <= 7, v["sr"]                   # announced once per phase, not per update
        step("LR: phases with a done one and a candidate row before the end", lr)
        def hr():
            sys.path[:0] = [str(SRC / "tests"), str(SRC / "py"), str(ROOT)]
            import lc_synth, msn_synth
            d = Path(tempfile.mkdtemp()); fs = lc_synth.write_series(d, hidden_isf=True); mf = msn_synth.write_msn(d / "cafe_msn.mzML", msn_synth.caffeine_nodes())
            pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.set_input_files("#tp-in", [f["path"] for f in fs] + [str(mf)])
            pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 8", timeout=300000)
            pg.fill("#tp-smi", "Cn1cnc2c1c(=O)n(C)c(=O)n2C"); pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('195.0877')", timeout=60000)
            pg.evaluate(WATCH)
            pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
            v = pg.evaluate("window.__lv")
            assert v["done"] >= 1 and v["cands"] >= 1 and v["nodes"] >= 2 and v["finished"], v
        step("HR: tree grows node by node, candidates and phases before the results", hr)
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
