"""Low- and high-resolution files together in the loading table: the world of the first file read opens, the files of the other world stay unticked in the list and a gentle note
(no error) offers «Switch»; the switch closes one world and opens the other; ticking a file of the other world leaves it aside."""
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
        world = lambda: pg.evaluate("OPENNOW.world()")
        def note():
            attendi(pg, "OPENNOW.busy() === 0 && UPBUSY === 0 && document.querySelector('#hrmix') && !document.querySelector('#hrmix').hidden")
            attendi(pg, "!document.querySelector('#fl-note').hidden && /«(Switch|Passa)/.test(document.querySelector('#fl-note').textContent)")      # the side list says where the other world is
            show_table(pg)
            w = world(); assert w is not None
            assert pg.get_attribute("#hrmix", "role") == "status" and pg.locator("#flist input[data-k=use]").count() == 3, "nothing lost from the list"
            assert ticked().count(True) == (1 if w else 2), ticked()           # the world of the first file read is open, the other is set aside
            assert pg.is_visible("#hrmix-lr" if w else "#hrmix-hr") and pg.is_hidden("#hrmix-hr" if w else "#hrmix-lr"), "one «Switch» button, to the other world"
            assert pg.evaluate("E.files.filter(f => !f.gone).length") >= (1 if w else 2)
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), pg.evaluate("JSON.stringify(window.MZLAB_LR_MISS)")
        step("mixed files: the first world read opens, the other is set aside with «Switch»", note)
        def switch():
            w = world(); pg.click("#hrmix-lr" if w else "#hrmix-hr")
            attendi(pg, f"OPENNOW.busy() === 0 && OPENNOW.world() === {'false' if w else 'true'}", 60000)
            assert ticked().count(True) == (2 if w else 1), ticked()
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), pg.evaluate("JSON.stringify(window.MZLAB_LR_MISS)")
        step("«Switch» closes the open world and opens the other one", switch)
        def retick():
            show_table(pg); w = world(); i = ticked().index(False)
            pg.locator("#flist input[data-k=use]").nth(i).click()
            attendi(pg, f"!document.querySelectorAll('#flist input[data-k=use]')[{i}].checked")
            assert world() == w, "a file of the other world does not join the open one"
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), pg.evaluate("JSON.stringify(window.MZLAB_LR_MISS)")
        step("ticking a file of the other world leaves it aside", retick)
        def load():
            open_first(pg); ready(pg)
            assert pg.locator("#gomsg .fail").count() == 0, pg.inner_text("#gomsg")
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), pg.evaluate("JSON.stringify(window.MZLAB_LR_MISS)")
        step("the open world gives no error", load)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
