"""Tabui «Dig» progress and «Cancel» with the Rust engine (TP_Mine/files/finder-lr.js, crates/mzlab-feat/src/legacy/progress.rs): the 38 synthetic files of tests/data/gruppi go in through the
page; during the Dig the page stays responsive (a 50 ms timer never stalls for more than 300 ms), the phases come in order with «file k of N» for the detection, «Cancel» stops the run
and gives the panel back without losing the files (a second Dig then finishes), and the long tasks of the page are recorded with their phase (DIAG.LONG, «Copy diagnostics»).
Builds the site (python3 tools/build_site.py) and static/wasm-feat; SKIP without cargo or wasm-bindgen. MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses one (the TP_Mine files are
copied over it). Run from the repo root: python3 tests_e2e/e2e_tpmine_progresso.py"""
import os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build the .wasm of mzFinder"); sys.exit(0)
tmp = Path(tempfile.mkdtemp(prefix="tpmine_progresso_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    from lib import build_feat; build_feat(site / "static", dict(os.environ, RUNNER_TEMP=str(tmp / "rt")))
else:
    for f in (ROOT / "TP_Mine" / "files").glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
FILES = sorted(str(p) for p in (ROOT / "tests" / "data" / "gruppi").glob("*.mzML"))
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []
# a 50 ms timer, and what the panel showed (the «k of N» of the detection, the phase list) while the Dig ran
WATCH = """() => { window.__w = {gaps: [], counts: [], t0: performance.now()}; let last = performance.now();
  window.__tm = setInterval(() => { const n = performance.now(); window.__w.gaps.push(Math.round(n - last)); last = n; }, 50);
  new MutationObserver(() => { const c = document.querySelector('#tp-live .pc'); if (c && window.__w.counts[window.__w.counts.length - 1] !== c.textContent) window.__w.counts.push(c.textContent); })
    .observe(document.getElementById('tp-right'), {subtree: true, childList: true, characterData: true}); }"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); pg = b.new_context(viewport={"width": 1500, "height": 1400}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');localStorage.setItem('qqq.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#nav", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        pg.set_input_files("#tp-in", FILES)
        pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 9", timeout=600000)
        pg.fill("#tp-smi", "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O")
        pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('241.0641')", timeout=60000)
        pg.wait_for_timeout(1500)

        def dig():
            pg.evaluate(WATCH); pg.evaluate("window.__t = performance.now()")
            pg.click("#tp-go"); pg.wait_for_selector("#tp-t", timeout=300000)
            w = pg.evaluate("(clearInterval(window.__tm), window.__w)")
            assert max(w["gaps"]) <= 300, ("the page stalled", sorted(w["gaps"])[-5:])
            assert len(w["gaps"]) >= 10, w["gaps"]
            ks = [c.strip() for c in w["counts"]]
            n = int(ks[-1].split()[-1]) if ks else 0
            assert n >= len(FILES) - 3 and ks[-1] == f"{n} of {n}", ("no «k of N» for the detection", ks[-3:])
            nums = [int(c.split()[0]) for c in ks]
            assert nums == sorted(nums) and len(nums) >= 10, ("the count went back or was not shown file by file", nums)
        step("Dig: the page stays responsive and the detection shows «k of N»", dig)

        def order():
            seq = [f for f in pg.evaluate("DIAG.LONG.marks.map(m => m.fase)") if f not in ("start", "render")]
            dedup = [f for i, f in enumerate(seq) if i == 0 or f != seq[i - 1]][-6:]
            assert dedup == ["calibration", "candidates", "xic", "unexpected", "ms2", "tables"], seq
        step("Dig: the phases come in order", order)

        def cancel():
            pg.click("#tp-go"); pg.wait_for_function("window.TPLIVE && TPLIVE.phases() === 'xic'", timeout=120000)
            assert pg.locator("#tp-live-cancel").is_visible()
            pg.keyboard.press("Escape") if pg.evaluate("document.activeElement && document.activeElement.id") == "tp-live-cancel" else pg.click("#tp-live-cancel")
            pg.wait_for_function("document.getElementById('tp-msg').textContent === 'Cancelled.'", timeout=30000)
            assert not pg.locator("#tp-go").is_disabled() and pg.locator("#tp-live-cancel").count() == 0
            assert pg.evaluate("document.querySelectorAll('#tp-files tr').length") >= 9, "the files are kept"
            assert pg.locator("#tp-t").count() == 1, "the previous results are back"
            pg.click("#tp-go"); pg.wait_for_function("document.getElementById('tp-msg').textContent === 'Done.'", timeout=300000)
        step("Cancel stops the run, keeps the files and the previous results; a new Dig finishes", cancel)

        def diag():
            t = pg.evaluate("DIAG.testoLento('9.9.9', 0)")
            assert t.startswith("mzLab 9.9.9\ncores: "), t
            assert pg.evaluate("typeof DIAG.LONG.righe(0).length") == "number"
        step("diagnostics: the text to copy names the cores and the long tasks", diag)
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
