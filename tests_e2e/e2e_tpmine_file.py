"""Tabui, step «1 · Files»: remove files (TP_Mine/js/20_tpmine.js; docs/agenti/mzfinder.md). The 38 synthetic files of tests/data/gruppi are loaded through the page. Every row has a «Remove» button
(aria-label, >= 24 px, keyboard) and «Remove all» sits next to «Choose files...»; a removal shows the notice «Removed ... · Undo» (no blocking question) and Undo puts the rows back in place;
the same sample read twice (the file again, or with a doubled .mzML) is one row, shown once without the extension; the help of a file name never covers «Conditions of the files»; axe is clean.
Builds the site and static/wasm-feat; SKIP without cargo and wasm-bindgen. MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses one; AXE_JS=<axe.min.js>. Run from the repo root: python3 tests_e2e/e2e_tpmine_file.py"""
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
tmp = Path(tempfile.mkdtemp(prefix="reatt_auto_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    from lib import build_feat; build_feat(site / "static", dict(os.environ, RUNNER_TEMP=str(tmp / "rt")))
else:
    for f in (ROOT / "TP_Mine" / "files").glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
    for f in (ROOT / "TP_Mine" / "lazy").glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / "lazy" / f.name)
FILES = sorted(str(p) for p in (ROOT / "tests" / "data" / "gruppi").glob("*.mzML"))
SMILES = "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"          # bentazone
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import urllib.request
_t0 = time.time()
while time.time() - _t0 < 30:
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: pass
errs = []; reqs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); pg = b.new_context(viewport={"width": 1500, "height": 1400}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("request", lambda r: reqs.append(r.url)); logs = []; pg.on("console", lambda m: logs.append(m.text[:200]))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#nav", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        rows = lambda: pg.locator("#tp-files tr").count() - 1
        names = lambda: pg.eval_on_selector_all("#tp-files .tp-fn", "e => e.map(x => x.textContent)")
        pg.set_input_files("#tp-in", FILES)
        pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 9", timeout=600000)
        pg.wait_for_function("document.querySelectorAll('#tp-files tr').length === %d" % (len(FILES) + 1), timeout=600000)
        N = len(FILES)

        def unique():
            ns = names(); assert len(ns) == N and len(set(ns)) == N, (len(ns), len(set(ns)))
            assert not any(n.lower().endswith(".mzml") for n in ns), ns[:3]
            dup = Path(tmp) / (Path(FILES[0]).stem + ".mzML.mzML"); shutil.copy(FILES[0], dup)
            pg.set_input_files("#tp-in", [FILES[0], str(dup)])
            pg.wait_for_function("!document.getElementById('tp-msg').textContent.startsWith('Reading')", timeout=120000)
            assert rows() == N and len(set(names())) == N, (rows(), N)
        step("a sample read twice (again, or with a doubled .mzML) is one row, shown once without the extension", unique)

        def one():
            b = pg.locator("#tp-files [data-rm]").first; box = b.bounding_box()
            assert box["width"] >= 24 and box["height"] >= 24 and b.get_attribute("aria-label").startswith("Remove "), (box, b.get_attribute("aria-label"))
            before = names(); b.focus(); pg.keyboard.press("Enter")
            assert rows() == N - 1 and names() == before[1:], "the row was not removed"
            assert pg.locator("#tolto-box .tolto").last.inner_text().startswith("Removed"), pg.inner_text("#tolto-box")
            assert pg.locator("#tp-fsum").inner_text().startswith(str(N - 1)), pg.inner_text("#tp-fsum")
            pg.locator("#tolto-box .tolto button").last.click()
            assert rows() == N and names() == before, "Undo did not put the row back in place"
        step("«Remove» on a row (keyboard) shows «Removed · Undo» and Undo restores it in place", one)

        def allgone():
            before = names(); pg.click("#tp-rmall")
            assert rows() == 0 and pg.is_hidden("#tp-rmall") and pg.locator("#tp-files tr").count() == 0
            assert f"{N} files" in pg.locator("#tolto-box .tolto").last.inner_text(), pg.inner_text("#tolto-box")
            pg.locator("#tolto-box .tolto button").last.click()
            assert names() == before and pg.is_visible("#tp-rmall")
        step("«Remove all» empties the list and Undo brings all the files back", allgone)

        def tip():
            assert pg.is_visible("#tp-cond"), "the conditions section is not shown"
            pg.locator("#tp-files .tp-fn").last.hover()
            pg.wait_for_selector("#tph-pop:not([hidden])", timeout=5000)
            r = pg.evaluate("""(() => { const a = document.getElementById('tph-pop').getBoundingClientRect(), b = document.querySelector('#tp-cond summary').getBoundingClientRect();
              return a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top; })()""")
            assert not r, "the help of the name covers «Conditions of the files»"
        step("the help of a file name does not cover «Conditions of the files»", tip)

        AXE = os.environ.get("AXE_JS") or lib.axe_path()
        def axe():
            pg.mouse.move(0, 0); pg.add_script_tag(path=AXE)
            v = pg.evaluate("""axe.run('#tp-left', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length, v.nodes.map(n => n.target.join(' ')).join(' | ').slice(0, 300)]))""")
            assert not v, v
        step("axe: the list of files and its buttons have no violation", axe) if AXE else print("  SKIP axe-core not available (AXE_JS)")
        step("window.MZLAB_LR_MISS is empty", lambda: (_ for _ in ()).throw(AssertionError(pg.evaluate("window.MZLAB_LR_MISS"))) if pg.evaluate("(window.MZLAB_LR_MISS || []).length") else None)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
