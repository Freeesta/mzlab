"""Disegno, Quick draw with a compound name: Enter on a name draws its first suggestion, a name that is not found draws nothing and shows a note (never junk query atoms),
a pasted name on the canvas shows the same note; SMILES and InChI keep drawing. Needs the built site (tools/build_site.py).
Run from the repo root: python3 tests_e2e/e2e_disegno_veloce_nomi.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))

r = Run(port=8938, wd="/tmp/wd_dveloce")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1300, "height": 900})
        errors = []
        pg.on("pageerror", lambda e: errors.append(str(e)))
        pg.evaluate("() => { location.hash = '#/draw'; }")
        attendi(pg, "!!(window.TPDraw && TPDraw.ready()) && document.getElementById('kframe').classList.contains('kwait') === false", timeout=90000)
        attendi(pg, "document.getElementById('ex-smi').getAttribute('role') === 'combobox'", timeout=30000)
        smiles = lambda: pg.evaluate("async () => String(await TPDraw.smiles())")
        note = lambda: pg.evaluate("() => { const w = document.getElementById('smi-warn'); return w.hidden ? '' : w.textContent; }")
        def clear():
            pg.evaluate("() => document.getElementById('kframe').contentWindow.ketcher.editor.clear()")
        def type_into(text):
            pg.fill("#ex-smi", ""); pg.focus("#ex-smi"); pg.keyboard.type(text, delay=20)

        def name_enter():
            clear(); type_into("caffeina"); pg.keyboard.press("Enter")
            attendi(pg, "document.getElementById('ex-smi').value === ''", timeout=60000)
            s = smiles()
            assert "n" in s.lower() and "." not in s and len(s) > 10, s
        step("a name plus Enter draws the first suggestion", name_enter)

        def name_button():
            clear(); type_into("aspirina"); pg.click("#ex-load")
            attendi(pg, "!document.getElementById('smi-warn').hidden", timeout=30000)
            assert "SMILES" in note() and "invalid" not in note().lower() and "non valido" not in note().lower(), note()
            s = smiles()
            assert s == "" and "*" not in s, s
            assert pg.input_value("#ex-smi") == "aspirina"
        step("an unknown name draws nothing and shows the note", name_button)

        def plain():
            clear(); pg.fill("#ex-smi", "CCO"); pg.focus("#ex-smi"); pg.keyboard.press("Enter")
            attendi(pg, "document.getElementById('ex-smi').value === ''", timeout=30000)
            assert smiles() == "CCO", smiles()
            clear(); pg.fill("#ex-smi", "InChI=1S/CH4/h1H4"); pg.click("#ex-load")
            attendi(pg, "document.getElementById('ex-smi').value === ''", timeout=30000)
            assert smiles() == "C", smiles()
        step("SMILES and InChI still draw", plain)

        def paste():
            clear()
            pg.evaluate("""() => { const d = document.getElementById('kframe').contentDocument, dt = new DataTransfer(); dt.setData('text/plain', 'aspirina');
              (d.activeElement || d.body).dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true, cancelable: true })); }""")
            attendi(pg, "!document.getElementById('smi-warn').hidden", timeout=15000)
            assert smiles() == "", smiles()
        step("a name pasted on the canvas shows the note", paste)
        step("no page error", lambda: (_ for _ in ()).throw(AssertionError(errors[:2])) if errors else None)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s[1][:4].ljust(5), s[0], "" if s[1] == "ok" else s[1])
print("FAIL" if bad else "OK")
sys.exit(1 if bad else 0)
