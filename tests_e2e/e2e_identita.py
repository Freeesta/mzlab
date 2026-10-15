"""mzFinder, feature tables: «group by identity» (engine in Pyodide, identity.py). The features of one compound (adducts, isotope) become ONE expandable row with the neutral mass, the level of
the best member, the evidence of every link and a small network; two co-eluting features with an unexplained difference stay apart; the table goes back to one row per feature.
Needs the built site (python3 tools/build_site.py). Run from the repo root: python3 tests_e2e/e2e_identita.py"""
import json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, attendi

PORT = 8852
errs, steps = [], []
def step(name, fn):
    try: fn(); print("  ", (name, "ok")); steps.append(1)
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:500])); steps.append(0)

if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python3 tools/build_site.py")); sys.exit(0)
mf = ROOT / "site" / "static" / "mzfinder"
for f in (ROOT / "TP_Mine" / "js").glob("*.js"): shutil.copyfile(f, mf / f.name)
idx = json.loads((mf / "indice.json").read_text()); idx["js"] = sorted(f.name for f in (ROOT / "TP_Mine" / "js").glob("*.js")); (mf / "indice.json").write_text(json.dumps(idx))
for f in ("libreria-worker.js", "libreria.js", "livelli.js", "livelli-1.js"): shutil.copyfile(ROOT / "mzlab" / "web" / f, ROOT / "site" / "static" / f)

P, M0 = 1.00727646688, 300.1
RAMP, OTHER = [1.0, 2.5, 4.0, 3.0, 6.0], [5.0, 1.0, 4.0, 6.0, 0.5]
def row(i, mz, rt, prof, k=1e5): return f"{i},{mz:.5f},{rt}," + ",".join(str(int(k * v)) for v in prof)
HEAD = "id,mz,rt," + ",".join(f"datafile:S{i}_pos.mzML:area" for i in range(1, 6)) + "\n"
ROWS = [row(1, M0 + P, 5.20, RAMP), row(2, M0 + 22.98921, 5.21, [v * 0.3 for v in RAMP]), row(3, M0 + 18.03382, 5.20, [v * 0.1 + (0.05 if i % 2 else 0) for i, v in enumerate(RAMP)]),
        row(4, M0 + P + 1.003355, 5.20, [v * 0.2 for v in RAMP]), row(5, 410.2, 7.0, OTHER), row(6, 426.195, 7.0, [v * 0.7 for v in OTHER]), row(7, 505.3, 3.0, RAMP[:2] + [0, 0, 0])]
tmp = Path(tempfile.mkdtemp()); (tmp / "tabella_pos.csv").write_text(HEAD + "\n".join(ROWS) + "\n")

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); pg = b.new_context(viewport={"width": 1500, "height": 1200}).new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.on("response", lambda r: errs.append(("http%d" % r.status, r.url)) if r.status >= 400 and "logo.svg" not in r.url and "/vendor/" not in r.url else None)
        def boot():
            pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000); pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-unt", timeout=60000)
            attendi(pg, "!!window.UNTUI", 120000); pg.click("#tp-unt"); pg.wait_for_selector("#unt[open]", timeout=10000)      # the button does nothing until the table module is registered
            pg.set_input_files("#unt-in", [str(tmp / "tabella_pos.csv")]); pg.wait_for_function("document.querySelector('#unt-sum').textContent.includes('7 features')", timeout=20000)
            pg.click("#unt-go"); pg.wait_for_selector("#unt-res:not([hidden])", timeout=60000)
            assert pg.evaluate("document.querySelectorAll('#unt-t tr.row1').length") == 7
        step("a table of 7 features (5 samples) is read and matched", boot)
        def group():
            pg.check("#unt-grp"); pg.wait_for_function("document.querySelectorAll('#unt-t tr.row1').length === 4 && document.querySelector('#unt-t').textContent.includes('ID')", timeout=240000)
            t = pg.inner_text("#unt-t"); assert "4 features" in t and "300.1000" in t, t
            assert "4 identities" in pg.inner_text("#unt-cnt") and "7 features" in pg.inner_text("#unt-cnt")
        step("group by identity: 4 rows, the compound's 4 features are one (neutral mass 300.1000); the co-eluted pair with an unexplained difference stays apart", group)
        def expand():
            tg = "#unt-t button.tg"; assert pg.get_attribute(tg, "aria-expanded") == "false"
            pg.click(tg); pg.wait_for_function("document.querySelectorAll('#unt-t tr.mem').length === 4")
            assert pg.get_attribute(tg, "aria-expanded") == "true" and "aria-label" in pg.evaluate("document.querySelector('#unt-t button.tg').outerHTML")
            roles = pg.evaluate("[...document.querySelectorAll('#unt-t tr.mem')].map(tr => tr.children[2].textContent).join('|')")
            assert "[M+H]+" in roles and "[M+Na]+" in roles and "[M+NH4]+" in roles and "isotope" in roles, roles
        step("the row expands to its members with their role", expand)
        def detail():
            pg.click("#unt-t tr.row1[data-i]:has-text('4 features')"); pg.wait_for_function("document.querySelector('#unt-det').textContent.includes('Evidence of the links')")
            d = pg.inner_text("#unt-det"); assert "Neutral mass 300.1000" in d and "r 1.00 over 5 samples" in d and "Level and candidate come from the best member" in d, d
            assert pg.evaluate("document.querySelectorAll('#unt-det svg line').length") >= 3 and "Network of identity" in pg.evaluate("document.querySelector('#unt-det svg').getAttribute('aria-label')")
            low = d.lower(); assert not any(w in low for w in ("identified", "confirmed", "proved")), d
            pg.click("#unt-t tr.mem[data-k='POS:2']"); pg.wait_for_function("document.querySelector('#unt-det').textContent.includes('Feature 2')")
        step("click → neutral-mass hypothesis, evidence of every link, network; a member opens the feature detail", detail)
        def back():
            pg.uncheck("#unt-grp"); pg.wait_for_function("document.querySelectorAll('#unt-t tr.row1').length === 7")
            pg.check("#unt-grp"); pg.wait_for_function("document.querySelectorAll('#unt-t tr.row1[data-i]').length === 4")          # the graph is kept: no new call
        step("back to one row per feature and again", back)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
bad = [e for e in errs if not (isinstance(e, tuple) and "wasm-lib" in str(e[1]))]          # the .wasm of the library engine exists only in the CI build
print("  ", ("browser errors", "ok" if not bad else "FAIL " + str(bad[:3])))
