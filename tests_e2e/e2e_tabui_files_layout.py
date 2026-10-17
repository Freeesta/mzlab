"""Tabui «1 · Files» card: the table stays inside the panel (2, 8 and 40 files; 360 px and 1500 px wide), the page never scrolls sideways,
long names are cut with the full name in the tooltip, and «Choose folder...» no longer exists. Built site + synthetic mzML. Run from the repo root: python3 tests_e2e/e2e_tabui_files_layout.py"""
import os, shutil, subprocess, sys, time, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401
from lib import free_port; PORT = free_port()
SRC = ROOT / "TP_Mine"
tmp = Path(tempfile.mkdtemp())
cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")]
subprocess.run(cmd, check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import socket
while True:                                   # wait for the server on its port (no fixed pause)
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []; steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:400]))
MEASURE = """() => { const f = document.getElementById('tp-files'), c = f.closest('.card'), cr = c.getBoundingClientRect(), t = f.querySelector('table').getBoundingClientRect();
  const nav = document.getElementById('nav'), nd = nav ? nav.style.display : ''; if (nav) nav.style.display = 'none'; const de = document.documentElement; return {pageOver: de.scrollWidth - de.clientWidth, cardR: cr.right, tableR: t.right, filesOver: f.scrollWidth - f.clientWidth,
    cardOver: c.scrollWidth - c.clientWidth, vw: de.clientWidth, rows: f.querySelectorAll('tr').length, _: nav ? (nav.style.display = nd) : 0}; }"""
try:
    sys.path[:0] = [str(ROOT)]
    from finder_demo import make_demo
    base = [Path(f["path"]) for f in make_demo(Path(tempfile.mkdtemp()))]
    def mk(n):
        d = Path(tempfile.mkdtemp()); out = []
        for i in range(n):
            q = d / f"very_long_sample_name_for_the_degradation_series_replicate_{i:02d}_{base[i % len(base)].stem}.mzML"
            shutil.copy(base[i % len(base)], q); out.append(str(q))
        return out
    with sync_playwright() as p:
        b = p.chromium.launch()
        for w in (360, 1500):
            pg = b.new_context(viewport={"width": w, "height": 1400}, reduced_motion="reduce").new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            def nofolder():
                assert pg.locator("#tp-pickdir").count() == 0 and pg.locator("#tp-dir").count() == 0
                assert "Choose folder" not in pg.inner_text("#tp-left")
                assert pg.locator("#tp-page").count() == 1 and pg.locator("#tp-pick").count() == 1
            step(f"{w}px: no «Choose folder...»", nofolder)
            for n in (2, 8, 40):
                def go(n=n):
                    pg.reload(); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
                    pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
                    pg.set_input_files("#tp-in", mk(n))
                    pg.wait_for_function(f"document.querySelectorAll('#tp-files tr').length > {min(n, 6)}", timeout=240000)
                    if n > 6: pg.click("#tp-more"); lib.attendi(pg, "document.getElementById('tp-more').getAttribute('aria-expanded') === 'true'")
                    m = pg.evaluate(MEASURE)
                    assert m["pageOver"] <= 0, m                                   # the page does not scroll sideways
                    assert m["tableR"] <= m["cardR"] + 1 or m["filesOver"] > 0, m  # the table fits, or scrolls inside its own box
                    assert m["cardOver"] <= 1, m                                   # nothing is cut by the card
                    assert pg.locator("#tp-files td[title]").first.get_attribute("title").startswith("very_long"), "tooltip with the full name"
                step(f"{w}px, {n} files: the table stays in the panel", go)
            pg.close()
        b.close()
finally:
    srv.terminate()
for n, r in steps: print(("PASS " if r == "ok" else "FAIL ") + n + ("" if r == "ok" else ": " + r))
if errs: print("pageerror:", errs[:3])
sys.exit(1 if any(r != "ok" for _, r in steps) or errs else 0)
