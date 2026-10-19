"""Low- and high-resolution files together in the loading table: a gentle note (no error) offers «LR only» / «HR only»; the choice ticks one world and unticks the other,
the loading is paused while both are ticked, the other files stay in the list for a second load."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import pathlib
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds, numpy as np
D = pathlib.Path(__file__).resolve().parent / "shots" / "lrhr_tmp"; D.mkdir(parents=True, exist_ok=True)
rng = np.random.default_rng
ds.full_scan(D / "B_FullMass-t0.mzML", 0, rng(1)); ds.full_scan(D / "B_FullMass-t5.mzML", 5, rng(2)); ds.hr_dda(D / "HR_DDA-Exploris-t0.mzML", rng(3))
r = Run(port=8885, wd="/tmp/wd85")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "B_FullMass-t0.mzML", D / "B_FullMass-t5.mzML", D / "HR_DDA-Exploris-t0.mzML"])
        ticked = lambda: pg.evaluate("[...document.querySelectorAll('#flist input[data-k=use]')].map(x=>x.checked)")
        def note():
            attendi(pg, "document.querySelector('#hrmix') && !document.querySelector('#hrmix').hidden")
            assert pg.get_attribute("#hrmix", "role") == "status" and "LR (2)" in pg.inner_text("#hrmix-lr") and "HR (1)" in pg.inner_text("#hrmix-hr")
            assert ticked().count(True) == 2 and pg.get_attribute("#hrmix-lr", "aria-pressed") == "true"      # the most numerous world is preselected
            assert pg.locator("#flist input[data-k=use]").count() == 3 and not pg.is_disabled("#opbtn")
        step("mixed files: a note with the counts, the most numerous world preselected, nothing lost from the list", note)
        def tick_hr():
            pg.click("#hrmix-hr"); assert ticked().count(True) == 1 and pg.get_attribute("#hrmix-hr", "aria-pressed") == "true" and not pg.is_disabled("#opbtn")
        step("«HR only» ticks the high-resolution file alone", tick_hr)
        def retick():
            pg.locator("#flist input[data-k=use]").first.check()
            attendi(pg, "document.querySelector('#opbtn').disabled"); assert pg.get_attribute("#hrmix-lr", "aria-pressed") == "false"
            pg.click("#hrmix-lr"); assert ticked().count(True) == 2 and not pg.is_disabled("#opbtn")
        step("ticking both worlds pauses the loading; «LR only» resolves it", retick)
        def load():
            pg.click("#opbtn"); ready(pg)
            assert pg.locator("#gomsg .fail").count() == 0, pg.inner_text("#gomsg")
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length")
        step("loading the chosen world gives no error", load)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
