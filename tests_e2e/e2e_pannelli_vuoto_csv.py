"""Panel toolbars and menus: with no ticked file the work area shows the landing with the loading table (the panels come back as soon as a file is ticked), no visible table button
(T on the focused chart still opens the table), a CSV next to every Excel of a panel (UTF-8 with BOM, comma, provenance line first), the range button keeps its note
as a tooltip, the correction menu has no subtitle under «None» and «Subtract file».
Run from the repo root: python3 tests_e2e/e2e_pannelli_vuoto_csv.py"""
import sys, os, pathlib; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
D = pathlib.Path(__file__).resolve().parent / "shots" / "vuoto_tmp"; D.mkdir(parents=True, exist_ok=True)
ds.full_scan(D / "B_FullMass-t5.mzML", 5, np.random.default_rng(2)); ds.full_scan(D / "B_FullMass-t15.mzML", 15, np.random.default_rng(3))
r = Run(port=8910, wd="/tmp/wd_vuoto_csv")
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / "B_FullMass-t5.mzML"), str(D / "B_FullMass-t15.mzML")])
        pg.click("#opbtn"); ready(pg)
        def empty():
            assert pg.locator("#rmall").count() == 0, "no remove-all icon"
            W = "document.getElementById('v-workspace').dataset.work"
            assert pg.evaluate(W) == "panels", "a file is ticked: the panels"
            pg.evaluate("setVisOf(E.files, false)"); attendi(pg, W + " === 'landing'")
            assert pg.is_visible("#wtable") and pg.is_visible("#start") and pg.is_hidden("#dpanels"), "no file shown: the landing (drop area and the loading table) in place of the panels"
            pg.evaluate("setVisOf([E.files[0]], true)"); attendi(pg, W + " === 'panels'")
            assert pg.is_hidden("#wtable") and pg.is_visible("#dpanels")
        step("no ticked file: the landing with the loading table stands in for the panels, back when one is ticked", empty)
        def buttons():
            assert pg.locator(".pnl [data-a=tab]").count() == 0, "no table button"
            pg.evaluate("document.querySelector('.pnl.chrom canvas').focus()"); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]"); pg.keyboard.press("Escape")
        step("no visible table button; T on the chart still opens the table", buttons)
        def csv():
            pg.locator(".pnl.chrom [data-a=dl]").first.click()
            with pg.expect_download() as d: pg.locator("#ctx div", has_text="CSV").first.click()
            f = d.value; assert f.suggested_filename.endswith(".csv"), f.suggested_filename
            b = open(f.path(), "rb").read(); assert b[:3] == b"\xef\xbb\xbf", "BOM"
            lines = b[3:].decode("utf-8").split("\n"); assert lines[0].startswith("# mzLab "), lines[0][:40]
            assert "," in lines[1] and ";" not in lines[1] and len(lines) > 5, lines[1]
        step("CSV of a panel: BOM, provenance line, comma-separated", csv)
        def popover():
            b = pg.locator(".pnl.chrom [data-a=cpar]").first
            assert b.get_attribute("aria-description") and b.get_attribute("aria-description") in b.get_attribute("title"), "the note is the tooltip and the description"
            b.click()
            assert pg.locator(".pnl.chrom .cpop > .muted.sm").count() == 0, "no sentence above the fields"
        step("range popover: the sentence moved to the button", popover)
        def corr():
            pg.locator(".pnl.chrom .corrb").first.click()
            m = pg.locator(".pnl.chrom .corrm").first
            assert m.locator(".corri").first.locator(".sm").count() == 0, "no subtitle under None"
            assert m.locator(".corri").first.get_attribute("title")
        step("correction menu: no subtitle under None", corr)
finally:
    pass
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
r.close()
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
