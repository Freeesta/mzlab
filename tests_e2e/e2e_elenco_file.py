"""File list panel: «Remove all files» (one «Removed · Undo» notice, no question, the undo takes every file back) and files dropped on the panel: they are added at once
without the loading screen (drop zone visible, announced in a status note), files of another resolution stay out, no duplicates."""
import sys, os, base64; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import pathlib
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds, numpy as np
EX = ROOT / "mzlab" / "web" / "esempi"
D = pathlib.Path(__file__).resolve().parent / "shots" / "elenco_tmp"; D.mkdir(parents=True, exist_ok=True)
ds.full_scan(D / "B_FullMass-t5.mzML", 5, np.random.default_rng(2)); ds.hr_dda(D / "HR_DDA-Exploris-t0.mzML", np.random.default_rng(3))
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
LIVE = "E.files.filter(f => !f.gone).length"
def drop(pg, paths):
    """Fire the drag events of a real drop on the side panel with the files given."""
    data = [[p.name, base64.b64encode(p.read_bytes()).decode()] for p in paths]
    pg.evaluate("""async (data) => {
        const dt = new DataTransfer(); for (const [n, b] of data) dt.items.add(new File([Uint8Array.from(atob(b), c => c.charCodeAt(0))], n));
        const el = document.querySelector('#dfiles'); window.__dz = [];
        el.dispatchEvent(new DragEvent('dragenter', { dataTransfer: dt, bubbles: true, cancelable: true }));
        window.__dz.push(!document.querySelector('#fl-dz').hidden);
        const seen = new MutationObserver(() => { if (!document.querySelector('#loading').hidden) window.__ldSeen = true; }); window.__ldSeen = false;
        seen.observe(document.querySelector('#loading'), { attributes: true });
        el.dispatchEvent(new DragEvent('drop', { dataTransfer: dt, bubbles: true, cancelable: true }));
    }""", data)
r = Run(port=8886, wd="/tmp/wd86")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 950})
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")])
        pg.click("#opbtn"); ready(pg)
        def remove_all():
            assert pg.is_visible("#rmall") and pg.get_attribute("#rmall", "aria-label")
            n = pg.evaluate(LIVE); assert n >= 2
            pg.click("#rmall"); attendi(pg, f"{LIVE} === 0 && !!document.querySelector('#tolto-box .tolto')")
            assert pg.evaluate("document.querySelectorAll('dialog[open]').length") == 0, "no question"
            assert pg.locator("#tolto-box .tolto").count() == 1 and pg.is_disabled("#rmall")
            pg.click("#tolto-box .tolto button"); attendi(pg, f"{LIVE} === {n}")
            assert pg.locator("#tolto-box .tolto").count() == 0 and not pg.is_disabled("#rmall")
        step("remove all: one notice, no question, Undo puts every file back", remove_all)
        def drop_lr():
            n = pg.evaluate(LIVE)
            drop(pg, [D / "B_FullMass-t5.mzML"])
            assert pg.evaluate("window.__dz")[0] is True, "drop zone visible while dragging"
            attendi(pg, f"{LIVE} === {n + 1}", timeout=60000); ready(pg)
            assert pg.evaluate("window.__ldSeen") is False, "no loading screen"
            assert pg.get_attribute("#fl-note", "role") == "status" and pg.is_hidden("#fl-dz")
            assert not pg.evaluate("document.querySelector('#v-data').hidden") and pg.evaluate("S.adding") is False
        step("drop on the panel: the file joins the open ones, no loading screen, zone shown then hidden", drop_lr)
        def drop_dup():
            n = pg.evaluate(LIVE); drop(pg, [D / "B_FullMass-t5.mzML"]); ready(pg)
            assert pg.evaluate(LIVE) == n, "the same file is not added twice"
        step("dropping the same file again adds nothing", drop_dup)
        def drop_hr():
            n = pg.evaluate(LIVE); drop(pg, [D / "HR_DDA-Exploris-t0.mzML"])
            attendi(pg, "!document.querySelector('#fl-note').hidden && /LR|HR|risoluzione|resolution/.test(document.querySelector('#fl-note').textContent)", timeout=60000); ready(pg)
            assert pg.evaluate(LIVE) == n, "a file of the other resolution is not opened"
            assert pg.locator("#gomsg .fail").count() == 0
        step("a file of the other resolution is left out with a gentle note", drop_hr)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
