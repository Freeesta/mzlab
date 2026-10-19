"""Tabui, hide/show the loading panel and the Prev. Fukui indices in the summary (TP_Mine/js/20_tpmine.js; docs/agenti/mzfinder.md).
The 38 synthetic files of tests/data/gruppi are dug from Tabui in administrator mode. «#tp-sb» hides and shows the loading panel (aria-pressed, aria-label, target >= 24 px, keyboard), the table
column (#tp-right) gets wider, the choice survives a reload (mzlab.tabui.side); once the reactivity is calculated the summary (#tp-fuk) lists the highest Prev. f(-) heavy atoms, always
labelled «Prev.», with no Schymanski level and no verdict; axe is clean on the new controls. Builds the site and static/wasm-feat; SKIP without cargo and wasm-bindgen.
MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses one; AXE_JS=<axe.min.js>. Run from the repo root: python3 tests_e2e/e2e_tpmine_pannello.py"""
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
        def dig():
            pg.wait_for_function("!document.getElementById('tp-go').disabled && !document.getElementById('tp-live')")
            pg.click("#tp-go")
            pg.wait_for_function("document.getElementById('tp-msg').textContent === 'Done.'", timeout=300000)
        width = lambda: pg.evaluate("document.getElementById('tp-right').getBoundingClientRect().width")
        key = lambda: pg.evaluate("localStorage.getItem('mzlab.tabui.side')")

        def toggle():
            b = pg.locator("#tp-sb"); box = b.bounding_box()
            assert box["width"] >= 24 and box["height"] >= 24, box
            assert b.get_attribute("aria-pressed") == "false" and "Hide" in b.get_attribute("aria-label"), b.get_attribute("aria-label")
            assert pg.is_visible("#tp-left"); w0 = width()
            b.focus(); pg.keyboard.press("Enter")
            assert b.get_attribute("aria-pressed") == "true" and "Show" in b.get_attribute("aria-label")
            assert not pg.is_visible("#tp-left"), "the panel is still visible"
            w1 = width(); assert w1 > w0 + 200, (w0, w1)
            assert key() == "0", key()
            pg.keyboard.press("Space")
            assert pg.is_visible("#tp-left") and b.get_attribute("aria-pressed") == "false" and abs(width() - w0) < 2 and key() == "1"
            pg.keyboard.press("Enter")                                  # hidden again for the next steps
        step("the button hides and shows the loading panel (keyboard, aria, >= 24 px) and the right column fills the space", toggle)

        def reload():
            pg.reload(); pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go", state="attached")
            assert not pg.is_visible("#tp-left") and pg.get_attribute("#tp-sb", "aria-pressed") == "true", "the state was not kept"
            pg.click("#tp-sb"); assert pg.is_visible("#tp-left") and key() == "1"
        step("the state is kept after a reload", reload)

        def fukui():
            pg.set_input_files("#tp-in", FILES)
            pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 9", timeout=600000)
            pg.fill("#tp-smi", SMILES)
            pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('241.0641')", timeout=60000)
            dig()
            pg.wait_for_function("window.TPREATTAUTO && window.TPREATTAUTO.info().phase === 'done'", timeout=180000)
            pg.wait_for_function("document.querySelector('#tp-fuk .hc')", timeout=10000)
            t = pg.inner_text("#tp-fuk"); assert t.startswith("Prev. f(") and t.count(" · ") == 2, t
            hero = pg.inner_text(".hero").lower()
            for bad in ("schymanski", "level 2", "level 3", "confirmed", "identified"): assert bad not in hero, bad
            pg.click("#tp-sb"); assert pg.is_visible("#tp-fuk") and width() > 900, "the summary or the table is not wide"
            pg.click("#tp-sb")
        step("the summary lists the Prev. f(-) atoms after the reactivity is calculated, no level and no verdict", fukui)

        AXE = os.environ.get("AXE_JS") or lib.axe_path()
        def axe():
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("""axe.run('.tp-tools, .hero', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length, v.nodes.map(n => n.target.join(' ')).join(' | ').slice(0, 300)]))""")
            assert not v, v
        step("axe: the button and the summary have no violation", axe) if AXE else print("  SKIP axe-core not available (AXE_JS)")
        step("window.MZLAB_LR_MISS is empty", lambda: (_ for _ in ()).throw(AssertionError(pg.evaluate("window.MZLAB_LR_MISS"))) if pg.evaluate("(window.MZLAB_LR_MISS || []).length") else None)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
