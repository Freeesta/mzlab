"""Group names are unique whatever the spelling («Full_scan» = «FULL SCAN» = an experiment-type group): setting, renaming and a new group with a taken name are refused with a note;
a file dragged (real mouse) into a user group, out onto its own type header, and refused onto a foreign type header (a file never changes experiment type)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
ES = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
def lab(pg, stem): return pg.evaluate("(s) => E.files.find(f => fileBase(f.file).startsWith(s))?.ugrp || null", stem)
def note(pg): return pg.evaluate("(() => { const n = document.getElementById('fl-note'); return n && !n.hidden ? n.textContent : '' })()")
def centre(loc):
    b = loc.bounding_box(); assert b, "not in the page"
    return b["x"] + b["width"] / 2, b["y"] + b["height"] / 2
def row(pg, stem):
    k = pg.evaluate("(s) => E.files.findIndex(f => fileBase(f.file).startsWith(s))", stem)
    return pg.locator(f'#flst .nm[data-k="{k}"]').locator("xpath=ancestor::div[contains(@class,'fl')][1]")
def drag(pg, stem, target):
    x, y = centre(row(pg, stem)); pg.mouse.move(x, y); pg.mouse.down(); pg.mouse.move(x + 4, y + 4); pg.mouse.move(x + 8, y + 8)
    tx, ty = centre(pg.locator(target)); pg.mouse.move(tx, ty, steps=10); pg.mouse.move(tx + 1, ty, steps=2); pg.mouse.up(); ready(pg)
def setg(pg, stem, g): return pg.evaluate("async ([s, g]) => { await carica('gruppi.js'); return GRUPPI.set([fileBase(E.files.find(f => fileBase(f.file).startsWith(s)).file)], g) }", [stem, g])
r = Run(port=8903, wd="/tmp/wd_gruppi_nomi")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1600})
        stage(pg, [str(ES / "MS2_t15.mzML")] + [str(ES / f"FullScan_t{t}.mzML") for t in (0, 5, 15, 30)])
        pg.click("text=Carica dati"); ready(pg); drawer(pg)
        def sets():
            assert setg(pg, "FullScan_t0", "Blanks") is True and lab(pg, "FullScan_t0") == "Blanks"
            for bad in ("blanks", "BLANKS ", "Full_scan", "full scan", "ms2(production)"):
                assert setg(pg, "FullScan_t15", bad) is False and lab(pg, "FullScan_t15") is None, bad
            assert note(pg), "no note"
            assert setg(pg, "FullScan_t15", "Blanks") is True and lab(pg, "FullScan_t15") == "Blanks"      # the same name joins the group
        step("set: another spelling or an experiment-type name is refused, the same name joins", sets)
        def renames():
            assert setg(pg, "FullScan_t15", "Other") and lab(pg, "FullScan_t15") == "Other"
            def ren(old, new):
                pg.evaluate("async (o) => { await carica('gruppi.js'); GRUPPI.rename(o) }", old)
                pg.wait_for_selector("#askdlg[open]"); pg.fill("#askin", new); pg.click("#askok"); ready(pg)
            ren("Other", "b_lanks"); assert lab(pg, "FullScan_t15") == "Other", "renamed onto a taken name"
            ren("Other", "Samples"); assert lab(pg, "FullScan_t15") == "Samples"
            ren("Samples", "SAMPLES"); assert lab(pg, "FullScan_t15") == "Samples" or lab(pg, "FullScan_t15") == "SAMPLES"   # the group's own name may change case
        step("rename onto a taken name is refused, a free one works", renames)
        def names():
            n = pg.evaluate("async () => { await carica('gruppi.js'); return GRUPPI.newName(['FullScan_t0.mzML']) }")
            assert n.replace("_", "").replace(" ", "").lower() not in ("fullscan", "blanks", "samples"), n
        step("a proposed name is free", names)
        def dnd():
            setg(pg, "FullScan_t15", ""); setg(pg, "FullScan_t0", ""); ready(pg)
            setg(pg, "FullScan_t0", "Blanks"); setg(pg, "FullScan_t5", "Blanks"); ready(pg)
            drag(pg, "FullScan_t15", '#flst .fgh[data-ug="Blanks"]'); assert lab(pg, "FullScan_t15") == "Blanks", "drag in"
            drag(pg, "FullScan_t15", '#flst .fgh[data-gt="full"]')
            assert lab(pg, "FullScan_t15") is None, "drag out onto its own type header"
            drag(pg, "FullScan_t5", '#flst .fgh[data-gt="ms2"]'); assert lab(pg, "FullScan_t5") == "Blanks", "foreign type header must refuse"
        step("drag a file into a user group, out onto its type header; a foreign type header refuses", dnd)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
