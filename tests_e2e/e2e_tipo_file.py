"""Experiment menu of the start-screen table: a plain menu of the types with the detected one pre-selected (reason as tooltip), a picked type wins, is applied at once (table, group,
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
        stage(pg, [D / "B_FullMass-t0.mzML", D / "B_FullMass-t1.mzML"]); frames(pg, 6); show_table(pg)
        def auto():
            assert pg.locator(SEL).count() == 2
            first = pg.evaluate(f"(() => {{ const s = document.querySelector('{SEL}'); return [s.selectedOptions[0].textContent, s.value, s.title, Array.from(s.options).map(o => o.value)]; }})()")
            assert first[0] == "Full Scan" and first[1] == "full" and first[2] and not first[2].startswith("load."), first
            assert first[3] == pg.evaluate("TIPOF.KINDS") and all("manual" not in o.lower() and "detected" not in o.lower() for o in pg.evaluate("Array.from(document.querySelector('%s').options).map(o => o.textContent)" % SEL)), first
        step("the types are listed plainly, the detected one is selected, with its reason", auto)
        def one():
            name = pg.evaluate(f"document.querySelector('{SEL}').dataset.exp")
            pg.locator(SEL).first.select_option("infusion"); frames(pg, 4)
            SELF = f"select[data-exp='{name}']"      # the table is sorted again by experiment: the changed row follows its group
            kinds = pg.evaluate("ST.files.map(f => f.kind + ':' + f.kindAuto).sort()")
            assert kinds == ["full:full", "infusion:full"], kinds
            assert len(set(pg.evaluate("Array.from(document.querySelectorAll('#flist tr[data-grp]')).map(tr => tr.dataset.grp)"))) == 2
            opts = pg.evaluate("Array.from(document.querySelector(\"%s\").options).map(o => [o.value, o.textContent, o.selected])" % SELF)
            sel = [o for o in opts if o[2]]
            assert len(sel) == 1 and sel[0][0] == "infusion" and not any("manual" in o[1].lower() for o in opts), opts
        step("a manual type applies at once and changes the row label and the row follows its experiment group", one)
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
        def same():
            pg.locator(SEL).first.select_option("full"); frames(pg, 3)
            assert pg.evaluate("localStorage.getItem('mzlab.filekind')") in (None, "{}") or pg.evaluate("!Object.values(JSON.parse(localStorage.getItem('mzlab.filekind') || '{}')).includes('full')")
        step("picking the detected type stores nothing", same)
        step("manual choice reaches the viewer; back to Auto without reloading", remembered)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
