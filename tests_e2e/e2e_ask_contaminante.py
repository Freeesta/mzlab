"""Formula dialog of the isotope pattern (HR): the field takes a formula or the name of a contaminant of the list, with suggestions (combobox/listbox, keyboard).
Types a name, picks the suggestion with the keyboard, checks the formula used; a plain formula and an unknown text keep working. Synthetic data only."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_ask_contaminante.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("askcont")
r = Run(port=8974, wd="/tmp/wd_askcont")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        sp = pg.evaluate("(()=>{const s=E.panels.find(q=>q.type==='spec'&&q._a&&q._a.hrp);return s?s.id:null})()"); assert sp is not None
        def opened():
            pg.evaluate(f"void isoAsk(E.panels.find(q=>q.id=={sp}))")      # async since #2687: its promise ends when the dialog closes; pg.wait_for_selector("#askdlg[open]")
        def used(): return pg.evaluate(f"(()=>{{const s=E.panels.find(q=>q.id=={sp});return s.iso&&s.iso.formula+' '+s.iso.ad}})()")
        def reset(): pg.evaluate(f"(()=>{{E.panels.find(q=>q.id=={sp}).iso=null}})()")
        def by_name():
            opened(); pg.fill("#askin", "FTALATO"); pg.fill("#askin", "DIBUTILFTALÁTO")
            pg.wait_for_selector("#asksug li"); assert pg.get_attribute("#askin", "role") == "combobox" and pg.get_attribute("#asksug", "role") == "listbox"
            opts = pg.eval_on_selector_all("#asksug li", "e=>e.map(x=>x.textContent)")
            assert any("dibutilftalato" in o and "C16H22O4" in o for o in opts), opts
            hs = pg.evaluate("[...document.querySelectorAll('#asksug li')].every(e=>e.getBoundingClientRect().height>=24)"); assert hs
            pg.press("#askin", "ArrowDown"); assert pg.get_attribute("#asksug li", "aria-selected") == "true"
            pg.press("#askin", "Enter")                                   # picks, does not confirm the dialog
            v = pg.input_value("#askin"); assert v.startswith("C16H22O4 [M"), v
            assert pg.evaluate("document.querySelector('#askdlg').open")
            pg.click("#askok"); pg.wait_for_function(f"(()=>{{const s=E.panels.find(q=>q.id=={sp});return !!s.iso}})()", timeout=15000)
            assert used().startswith("C16H22O4 [M+H]+") or used().startswith("C16H22O4 [M"), used()
        step("a contaminant name, picked with the keyboard, fills the formula and OK uses it", by_name)
        def adduct_after_pick():
            reset(); opened(); pg.fill("#askin", "dibutilftalato"); pg.wait_for_selector("#asksug li"); pg.press("#askin", "ArrowDown"); pg.press("#askin", "Enter")
            pg.fill("#askin", pg.input_value("#askin").split(" ")[0] + " [M+Na]+"); pg.click("#askok")
            pg.wait_for_function(f"(()=>{{const s=E.panels.find(q=>q.id=={sp});return !!s.iso}})()", timeout=15000)
            assert used() == "C16H22O4 [M+Na]+", used()
        step("the adduct after a space still works after a picked name", adduct_after_pick)
        def full_name():
            reset(); opened(); pg.fill("#askin", "dimetilftalato (DMP)"); pg.click("#askok")
            pg.wait_for_function(f"(()=>{{const s=E.panels.find(q=>q.id=={sp});return !!s.iso}})()", timeout=15000)
            assert used().startswith("C10H10O4 "), used()
        step("a full name typed and confirmed (no pick) is read like the pick", full_name)
        def plain():
            reset(); opened(); pg.fill("#askin", "C9H10Cl2N2O [M+Na]+"); assert pg.locator("#asksug li").count() == 0
            pg.click("#askok"); pg.wait_for_function(f"(()=>{{const s=E.panels.find(q=>q.id=={sp});return !!s.iso}})()", timeout=15000)
            assert used() == "C9H10Cl2N2O [M+Na]+", used()
        step("a plain formula still works (no suggestions)", plain)
        def nothing():
            reset(); opened(); pg.fill("#askin", "zzqq nothing"); assert pg.locator("#asksug li").count() == 0
            pg.click("#askok"); pg.wait_for_function("document.querySelector('#asktxt').innerText.length>0 && !document.querySelector('#askno').offsetParent", timeout=15000)
            assert not used(), used()
            pg.click("#askok")
        step("text that is neither a name nor a formula shows the usual error", nothing)
        def esc():
            opened(); pg.fill("#askin", "ftalato"); pg.wait_for_selector("#asksug li"); pg.press("#askin", "Escape")
            assert pg.locator("#asksug li").count() == 0 and pg.evaluate("document.querySelector('#askdlg').open")
            pg.click("#askno")
        step("Esc closes the list first, the dialog stays", esc)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
