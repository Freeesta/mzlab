"""Flow, the Raw files node (flow/lazy/flow-raw-files.js): the files open in Data as a table (type, MS levels, polarity, instrument, size), «Explore» shows the File and instrument, Scans and
Precursor tree tabs of the Data bench for the file, «Export mzML» saves it as <stem>.mzML. The files are synthetic: the MSn infusion of TP_Mine/tests/msn_synth.py and an HR DDA run (Orbitrap Fusion look-alike).
Needs the site (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_flow_rawfiles.py"""
import re, sys
from pathlib import Path
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
from lib import *
from lib_hr import synth
sys.path.insert(0, str(ROOT))
from TP_Mine.tests.msn_synth import write_msn, caffeine_nodes
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))

D = synth("flowrawfiles")
INF = write_msn(D / "MSn_synth_t01.mzML", caffeine_nodes(), n_scans=12)
LC = D / ds.HR_NAMES["fusion"]

def to_flow(pg):
    pg.evaluate("localStorage.setItem('mzlab.admin','1')"); pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
    pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
    pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web", timeout=120000)
    (None if pg.locator('.flx-tpl.show').count() else pg.click('button[aria-label="Start from a template"]')); pg.click('.flx-tc[data-tpl="msn"]')
    attendi(pg, "document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow && document.querySelector('.flx-list').__flow.plan.steps.length === 3")
    pg.click('.fln[data-key^="file:"]'); attendi(pg, "!!document.querySelector('.fl-insp .fl-data')")

r = Run(port=8982, wd="/tmp/wd_flowrawfiles")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        to_flow(pg)
        with pg.expect_file_chooser() as fc: pg.click(".fl-insp button:has-text('Add files')")
        fc.value.set_files(str(INF))
        attendi(pg, "FLOW.dataFiles().length === 1 && !!document.querySelector('.fl-rf table')", 180000)
        with pg.expect_file_chooser() as fc: pg.click(".fl-insp button:has-text('Add files')")
        fc.value.set_files(str(LC))
        attendi(pg, "FLOW.dataFiles().length === 2 && document.querySelectorAll('.fl-rf tbody tr').length === 2", 180000)
        def table():
            t = pg.locator(".fl-rf table")
            assert t.locator("caption").count() == 1 and t.locator("th[scope=col]").count() >= 6, t.inner_html()[:300]
            rows = pg.evaluate("[...document.querySelectorAll('.fl-rf tbody tr')].map(tr => [...tr.children].map(c => c.textContent.trim()).slice(0, 6))")
            assert [x[0] for x in rows] == sorted(x[0] for x in rows) and len(rows) == 2, rows
            inf = next(x for x in rows if x[0].startswith("MSn_synth")); lc = next(x for x in rows if x[0] == LC.name)
            assert inf[1] == "Infusion" and re.fullmatch(r"MS1–MS\d", inf[2]) and inf[3], inf
            assert lc[1] == "DDA" and lc[2] == "MS1–MS2" and lc[5], lc
            assert lc[4] == "Orbitrap Fusion", lc
        step("a row per file, sorted by name, with type, MS levels, polarity, instrument and size", table)
        def explore():
            pg.click("button[aria-label='Explore MSn_synth_t01.mzML']")
            attendi(pg, "document.querySelector('.fl-rf-body .fi-kv')", 30000)
            assert [x.strip() for x in pg.locator(".fl-rf-tabs button").all_inner_texts()] == ["File and instrument", "Scans", "Precursor tree"]
            ref = pg.evaluate("""async () => { const f = E.files.find(x => String(x.file).startsWith('MSn_synth')); const j = await (await fetch('api/fileinfo?k=' + f.k)).json(); return [j.instrument, j.scans]; }""")
            txt = pg.inner_text(".fl-rf-body")
            assert str(ref[1]) in txt and (ref[0] or "").lower() in txt.lower(), (ref, txt[:300])
            pg.click(".fl-rf-tabs button:has-text('Scans')"); attendi(pg, "document.querySelector('.fl-rf-body .hri-list .r')", 30000)
            pg.click(".fl-rf-tabs button:has-text('Precursor tree')"); attendi(pg, "document.querySelector('.fl-rf-body #inf-tree tbody tr')", 30000)
            assert pg.locator("#hri-list").count() == 0 or pg.locator("[id=hri-list]").count() <= 1
        step("Explore shows the File and instrument, Scans and Precursor tree tabs of the file", explore)
        def one_at_a_time():
            pg.click(f"button[aria-label='Explore {LC.name}']")
            attendi(pg, "document.querySelector('.fl-rf-body .fi-kv')", 30000)
            assert pg.locator(".fl-rf-body").count() == 1 and pg.locator(".fl-rf-tabs button").count() == 2, pg.locator(".fl-rf-tabs button").all_inner_texts()
            pg.click(f"button[aria-label='Explore {LC.name}']")
            assert pg.locator(".fl-rf-body").count() == 0
        step("one exploration at a time; a file without MSn has no Precursor tree tab", one_at_a_time)
        def export():
            with pg.expect_download() as d: pg.click("button[aria-label='Export MSn_synth_t01.mzML as mzML']")
            assert d.value.suggested_filename == "MSn_synth_t01.mzML", d.value.suggested_filename
            assert Path(d.value.path()).stat().st_size == INF.stat().st_size
        step("Export mzML downloads <stem>.mzML as it is", export)
        def keyboard():
            pg.focus("button[aria-label='Explore MSn_synth_t01.mzML']"); pg.keyboard.press("Enter")
            attendi(pg, "document.querySelector('.fl-rf-body .fi-kv')", 30000)
        step("Explore works from the keyboard", keyboard)
        errs = [e for e in r.errs if e[0] in ("pageerror",)]
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
        miss = pg.evaluate("(window.MZLAB_LR_MISS||[]).slice()")
        step("no request left unanswered by the engine", lambda: (_ for _ in ()).throw(AssertionError(miss)) if miss else None)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
