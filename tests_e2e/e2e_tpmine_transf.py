"""mzFinder, the list of transformations in Rust (crates/mzlab-feat/src/legacy/transf.rs, `finder_transformations`): a line added to the list of the page becomes a rule of
the hypotheses of the Dig, the Dig runs in Rust (no Pyodide), and a wrong line is refused with the text of the Python. A change that the default list only explains with two
steps (hydroxylation + methylation) is named by the new one-step rule.
Builds the site (python3 tools/build_site.py) and static/wasm-feat (tools/build_feat_wasm.sh); SKIP without cargo, wasm-bindgen or numpy. MZLAB_PIPE_SITE=<built site with
static/wasm-feat> reuses a site (the TP_Mine files are copied over it). Run from the repo root: python3 tests_e2e/e2e_tpmine_transf.py"""
import os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)

tmp = Path(tempfile.mkdtemp(prefix="tpmine_transf_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    subprocess.run(["tools/build_feat_wasm.sh", str(site / "static")], cwd=ROOT, check=True, env=dict(os.environ, RUNNER_TEMP=str(tmp / "rt")), capture_output=True)
else:                                                      # fresh sources of mzFinder over the site that was built
    for f in (ROOT / "TP_Mine" / "files").glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
mzml = sorted(str(f) for f in ser.glob("*.mzML"))
names = [Path(f).name for f in mzml]
PARENT = "C21H32N2O2"

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []; reqs = []
# SHA-256 of every .mzidx of OPFS, by name ({} when there is none)
SHAS = """async () => { const out = {}; try { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('idx');
  for await (const [n, h] of d.entries()) { if (n.endsWith('.part')) continue; const b = await (await h.getFile()).arrayBuffer();
    out[n] = [...new Uint8Array(await crypto.subtle.digest('SHA-256', b))].map(x => x.toString(16).padStart(2, '0')).join(''); } } catch (e) {} return out; }"""
TABLE = "[...document.querySelectorAll('#hr-t tr.clk')].map(r => r.textContent.replace(/\\s+/g, ' ').trim())"


def rows_ready(pg):
    pg.wait_for_function(f"document.querySelectorAll('#tp-files tr').length >= {len(names) + 1}", timeout=240000)


def settr(pg, t):
    pg.evaluate("document.querySelector('#tp-tr').closest('details').open = true"); pg.fill("#tp-tr", t)


def dig(pg, tr=None):
    if tr is not None: settr(pg, tr)
    pg.fill("#tp-mol", PARENT); pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
    return pg.evaluate(TABLE)


INIT = "try{localStorage.setItem('qqq.mzfinder','1');localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script(INIT)
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.context.on("request", lambda r: reqs.append(r.url))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("typeof window.TPCALL === 'function'", timeout=120000)
        pg.set_input_files("#pick", mzml)
        for _ in range(240):
            if len(pg.evaluate(SHAS)) == len(names): break
            pg.wait_for_timeout(500)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        pg.click("#tp-page"); rows_ready(pg)
        base = {}

        def default_list():
            base["text"] = pg.input_value("#tp-tr"); base["rows"] = dig(pg)
            assert any("hydroxylation" in r and "methylation" in r for r in base["rows"]), base["rows"]
        step("the default list: a product is explained by two steps", default_list)

        def extra_line():
            rows = dig(pg, base["text"] + "\nzzz custom change;+CH2O;a note")
            assert any("zzz custom change" in r for r in rows), rows
            assert not any("hydroxylation" in r and "methylation" in r for r in rows), "the one-step rule must name the change"
        step("a line added to the list names the change in the Dig", extra_line)

        def same_list():
            assert dig(pg, base["text"]) == base["rows"], "the default text gives the default Dig"
        step("the default text again: the same Dig", same_list)

        def wrong_line():
            settr(pg, base["text"] + "\nbroken line without a semicolon"); pg.fill("#tp-mol", PARENT); pg.click("#tp-go")
            pg.wait_for_function("(document.getElementById('tp-msg') || {}).textContent.includes('invalid line (expected')", timeout=120000)
        step("a wrong line is refused with the text of the Python", wrong_line)

        def no_pyodide():
            py = [u for u in reqs if "pyodide" in u.lower()]
            assert not py, py[:5]
        step("no pyodide* request", no_pyodide)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
