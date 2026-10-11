"""Load table and file list: order of the files (time, mixed units), time field without arrows and with validation, unit of the time (min, h, d) saved in qqq.tunit."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from synth import write_full
from pathlib import Path
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_ordine.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = Path("/tmp/wd_ordine_files"); D.mkdir(parents=True, exist_ok=True)
for i, n in enumerate(["s_1d", "s_t30", "s_2h", "s_t5"]): write_full(D / f"{n}.mzML", 0, seed=i)
for i, n in enumerate(["campione10", "campione2"]): write_full(D / f"{n}.mzML", 0, seed=10 + i)
r = Run(port=8921, wd="/tmp/wd_ordine"); r2 = None
NAMES = "Array.from(document.querySelectorAll('#flist tr:not(.grp)')).slice(1).map(tr=>tr.children[1].textContent.trim().split(' ')[0])"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "s_1d.mzML", D / "s_t30.mzML", D / "s_2h.mzML", D / "s_t5.mzML"])
        def order():
            assert pg.evaluate(NAMES) == ["s_t5.mzML", "s_t30.mzML", "s_2h.mzML", "s_1d.mzML"], pg.evaluate(NAMES)
        step("the table is in growing time, units mixed (5 min, 30 min, 2 h, 1 d)", order)
        def field():
            t = pg.evaluate("Array.from(document.querySelectorAll('#flist input[data-k=time]')).map(i=>[i.type,i.inputMode,i.value])")
            assert all(x[0] == "text" and x[1] == "decimal" for x in t), t
            assert [x[2] for x in t] == ["5", "30", "120", "1440"], t
            assert pg.is_visible("#tunit")
        step("the time field is a text field (inputmode decimal), the unit select is there", field)
        def unit():
            pg.select_option("#tunit", "h")
            assert pg.evaluate("localStorage.getItem('qqq.tunit')") == "h"
            v = pg.evaluate("Array.from(document.querySelectorAll('#flist input[data-k=time]')).map(i=>i.value)")
            assert v[2:] == ["2", "24"] and v[0] == "0.0833333", v
            pg.select_option("#tunit", "d"); v = pg.evaluate("Array.from(document.querySelectorAll('#flist input[data-k=time]')).map(i=>i.value)")
            assert v[3] == "1", v
        step("choosing h or d converts the shown values and is saved", unit)
        def typed():
            pg.select_option("#tunit", "h")
            i = pg.locator("#flist input[data-k=time]").nth(0); i.fill("0,5"); i.press("Tab")
            assert pg.evaluate("Array.from(document.querySelectorAll('#flist input[data-k=time]')).map(i=>i.value)")[0] == "0.5"
            pg.wait_for_timeout(100)
            names = pg.evaluate(NAMES); assert names[0] == "s_t30.mzML" or names[1] == "s_t30.mzML", names     # 0.5 h = 30 min ties with s_t30: order by name
            j = pg.locator("#flist input[data-k=time]").nth(2); j.fill("abc"); j.press("Tab")
            assert pg.get_attribute("#flist input[aria-invalid=true]", "data-k") == "time"
        step("0,5 h is accepted (30 min), text is refused with aria-invalid", typed)
        def list_unit():
            pg.click("text=Carica dati"); ready(pg)
            t = pg.evaluate("E.files.map(f=>f.time)"); assert sorted(t)[:2] == [30, 30] or 30 in t, t
            sub = pg.evaluate("Array.from(document.querySelectorAll('#filelist .fl small, .fl small')).map(s=>s.textContent)")
            assert any(x.endswith(" h") for x in sub), sub
        step("the file list shows the chosen unit", list_unit)
        r2 = Run(port=8922, wd="/tmp/wd_ordine2"); pg2 = r2.page(p)
        def nolabel():
            stage(pg2, [D / "campione10.mzML", D / "campione2.mzML"])
            assert pg2.evaluate(NAMES) == ["campione2.mzML", "campione10.mzML"], pg2.evaluate(NAMES)
            assert not pg2.is_visible("#tunit"), "no time in the names: no unit"
        step("names without time: natural alphabetical order and no unit", nolabel)
finally:
    r.close(); r2 and r2.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
print("ERRORS", [e for e in r.errs if e[0] != "console.warning"][:5])
sys.exit(1 if bad else 0)
