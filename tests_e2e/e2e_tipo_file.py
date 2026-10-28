"""Experiment menu of the start-screen table: "Auto: <type>" with the reason as tooltip, a manual type wins, is applied at once (table, group,
viewer) and is remembered per file; the group select sets every file of the group."""
import sys, os, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth
steps = []
def all_to(pg, kind):
    # Rows keep their place until the first edit, then re-sort: pick whichever row is not yet of the wanted type.
    for _ in range(4):
        i = pg.evaluate("kind => ST.files.map(f => f.kind).findIndex(k => k !== kind)", kind)
        if i < 0: return
        name = pg.evaluate("ST.files[%d].name" % i)
        row = pg.locator("#flist tr", has_text=name).locator(SEL)
        row.select_option(kind); frames(pg, 4)
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tipo_file.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + repr(e)[:400]))
D = synth("tipo_file")
shutil.copy(D / "B_FullMass-t0.mzML", D / "B_FullMass-t1.mzML")
SEL = "select[data-exp]"
r = Run(port=8868, wd="/tmp/wd_tipo_file")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "B_FullMass-t0.mzML", D / "B_FullMass-t1.mzML"]); frames(pg, 6)
        def auto():
            assert pg.locator(SEL).count() == 2
            first = pg.evaluate(f"(() => {{ const s = document.querySelector('{SEL}'); return [s.options[0].textContent, s.value, s.title]; }})()")
            assert first[0] == "Auto: Full Scan" and first[1] == "" and first[2] and not first[2].startswith("load."), first
        step("Auto is the first option, selected, with its reason", auto)
        def one():
            pg.locator(SEL).first.select_option("infusion"); frames(pg, 4)
            kinds = pg.evaluate("ST.files.map(f => f.kind + ':' + f.kindAuto).sort()")
            assert kinds == ["full:full", "infusion:full"], kinds
            assert len(set(pg.evaluate("Array.from(document.querySelectorAll('#flist tr[data-grp]')).map(tr => tr.dataset.grp)"))) == 2
        step("a manual type applies at once and changes the row label, not its place", one)
        def grp():
            all_to(pg, "full")
            assert pg.locator("select[data-expg]").count() == 0 and pg.locator("tr.grp").count() == 0
            all_to(pg, "ms2")
            assert pg.evaluate("ST.files.every(f => f.kind === 'ms2')")
        step("no group line or group select; each row sets its own type", grp)
        def remembered():
            all_to(pg, "full")
            pg.locator(SEL).first.select_option("infusion"); frames(pg, 3)
            pg.click("text=Carica dati"); ready(pg)
            info = pg.evaluate("[E.files.map(f => [f.file, f.kind, f.kindAuto, f.manual]), TIPOF.manual(ST.files[0].name), ST.files.map(f=>f.name), localStorage.getItem('mzlab.filekind')]")
            assert pg.evaluate("E.files.some(f => f.kind === 'infusion' && f.kindAuto === 'full')"), info
            pg.evaluate("E.files.forEach(f => TIPOF.set(f.file, null)); TIPOF.viewer(E.files.map(f => f.file))"); frames(pg, 3)
            assert pg.evaluate("E.files.every(f => f.kind === 'full' && !f.manual)")
        step("manual choice reaches the viewer; back to Auto without reloading", remembered)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
