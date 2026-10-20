"""Flow with the REAL engines (docs/agenti/flow-grafica.md): e2e_flow.py stubs the attributio pool, here the synthetic campaign folder (tests/data/attributio/campagna_sintetica) goes through the
«Confidence level» node of Flow and the .wasm `attributio-all`: 15 features with the levels of livelli_attesi.csv, the settings digest, an engine export, the Review screen on a real feature
and the MS2 panel; MZLAB_LR_MISS stays empty. Needs the built site (site/static/wasm and wasm-feat, as the CI step «e2e on the built site»); SKIP without them.
Run from the repo root: python3 tests_e2e/e2e_flow_motore.py"""
import csv, json, subprocess, sys
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi

site = ROOT / "site"
if not ((site / "static" / "wasm-feat" / "manifest.json").exists() and (site / "static" / "wasm" / "mzlab_wasm.js").exists()):
    print("SKIP the built site with static/wasm and static/wasm-feat is needed (tools/build_site.py, tools/build_feat_wasm.sh site/static)"); sys.exit(0)
FOLDER = ROOT / "tests" / "data" / "attributio" / "campagna_sintetica"
want = {}
with open(FOLDER / "livelli_attesi.csv", newline="", encoding="utf-8") as f:
    for r in csv.DictReader(f, delimiter=";"): want[(r["_pol"], r["_id"])] = r["level"] + r["level_sub"]
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:1500]))

PORT = free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import socket
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        pg = b.new_context(viewport={"width": 1400, "height": 1000}, reduced_motion="reduce", accept_downloads=True).new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_selector("#flx-add"); pg.click("#flx-add"); pg.click('.flx-pop button:has-text("Confidence level")')
        pg.click('.fln:has-text("Confidence level")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Confidence level'")

        def cascade():
            pg.set_input_files('.fl-insp input[type=file][webkitdirectory]', str(FOLDER))
            try: attendi(pg, "document.querySelectorAll('.flx-res .tp tbody tr.clk').length > 0", 180000)
            except Exception: raise AssertionError("no rows: " + pg.inner_text(".fl-insp .fl-note") + " " + str(errs[:3]))
            n = pg.evaluate("document.querySelectorAll('.flx-res .tp tbody tr.clk').length")
            assert n == 15, n
            assert pg.evaluate("window.MZLAB_LR_MISS || []") == [], pg.evaluate("window.MZLAB_LR_MISS")
        step("the synthetic folder goes through the real cascade: 15 features", cascade)

        def levels():
            counts = {}
            for lv in want.values(): counts[lv[0]] = counts.get(lv[0], 0) + 1
            got = pg.evaluate("[...document.querySelectorAll('.flx-res .tp tbody tr.clk .lvb')].map(e => e.textContent[1])")
            seen = {}
            for g in got: seen[g] = seen.get(g, 0) + 1
            assert seen == counts, (seen, counts)
        step("the levels shown are those of livelli_attesi.csv", levels)

        def digest():
            pg.click('.flx-res button:has-text("Settings digest")')
            attendi(pg, "(document.querySelector('.flw-digest code') || {textContent: ''}).textContent.length >= 8", 120000)
        step("the settings digest comes from the real evidence export", digest)

        def export():
            pg.click('.flx-res #at-export')
            with pg.expect_download(timeout=120000) as dl: pg.click('.flx-res #at-export-menu button[data-kind=schymanski]')
            assert dl.value.suggested_filename, dl.value
        step("the Schymanski export is built by the real engine", export)

        def review():
            pg.click('.flx-res button:has-text("Review")')
            attendi(pg, "document.querySelector('.flw-rv h3') || document.querySelector('.flw-review .fl-note')", 120000)
            attendi(pg, "!/Loading the proofs/.test(document.querySelector('.flw-review').textContent) && document.querySelectorAll('.flw-rv-acts button').length > 0", 120000)
            txt = pg.inner_text(".flw-review")
            assert "Review" in txt and pg.locator(".flw-rv-acts button").count() >= 3, txt[:300]
            pg.keyboard.press("Escape")
        step("the Review screen opens on a real feature", review)

        def ms2():
            assert pg.locator('.flx-res .flw-ms2 #ms2-insilico').count() == 1
        step("the MS2 panel is mounted", ms2)

        step("no page error and no request left to the missing engine", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
        b.close()
finally:
    srv.terminate()
bad = [s for s in steps if s[1] != "ok"]
for n, r in steps: print(("  ok " if r == "ok" else "  FAIL "), n, "" if r == "ok" else r)
print("page errors:", errs)
sys.exit(1 if bad else 0)
