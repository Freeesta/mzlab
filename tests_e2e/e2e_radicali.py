"""Disegno, radical mark (disegno-radicale.js, item «Radical •» of the charge menu of the drop-down bar): with the tool on, a click on an atom puts the
mark (one H less, the label of the structure gets • and the mass of the radical) and a second click takes it off; Esc ends the tool; from the keyboard
the item marks the selected atoms; the mark is one undo step and goes out in the molfile (M  RAD) and in the SMILES (bracket atom with its H)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
LABELS = "[...document.querySelector('#kframe').contentDocument.querySelectorAll('svg g[id$=labels] text')].map(t=>t.textContent).join(' | ')"
IN = lambda pg, js: pg.evaluate("(() => { const d = document.querySelector('#kframe').contentDocument; return (" + js + ")(d); })()")
RADS = f"[...{KJ}.editor.struct().atoms.values()].filter(a=>a.radical===2).map(a=>a.label+a.implicitH).join(',')"
r = Run(port=9137, wd="/tmp/wd_radicali")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        wait_for_js(pg, "!!document.querySelector('#kframe').contentDocument.querySelector('#tp-bar [data-tp-own]')", 60000)
        pg.select_option("#lb-dec", "4")
        fr = pg.frame_locator("#kframe")
        def atom_pt(i):                                                   # page point of the i-th atom of the drawing
            return pg.evaluate(f"""(()=>{{const K={KJ},R=K.editor.render,o1=R.page2obj({{clientX:0,clientY:0}}),o2=R.page2obj({{clientX:100,clientY:100}});
              const f=document.querySelector('#kframe').getBoundingClientRect(),a=[...K.editor.struct().atoms.values()][{i}].pp;
              return [f.left+(a.x-o1.x)*100/(o2.x-o1.x),f.top+(a.y-o1.y)*100/(o2.y-o1.y)]}})()""")
        pressed = lambda: IN(pg, "d => d.querySelector('#tp-bar [data-tp=charge]').getAttribute('aria-pressed')")
        tool_on = lambda: IN(pg, "d => d.documentElement.classList.contains('tp-rad') && d.querySelector('#tp-bar [data-tp-own]').classList.contains('tp-sel')")
        def click_mark():
            pg.evaluate(f"{KJ}.setMolecule('CC')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 2")
            fr.locator("#tp-bar [data-tp=charge]").click()
            item = fr.locator("#tp-bar [data-tp-own]")
            box = item.bounding_box(); assert box["width"] >= 24 and box["height"] >= 24, box
            assert "Radicale" in item.get_attribute("aria-label")
            item.click(); assert pressed() == "true" and tool_on()
            x, y = atom_pt(0); pg.mouse.click(x, y)
            wait_for_js(pg, f"{RADS} === 'C2'")
            wait_for_js(pg, f"/C2H5•.*29\\.0391/.test({LABELS})", timeout=10000)
            assert pg.evaluate(f"{KJ}.editor.struct().atoms.size") == 2, "the click marks, it does not draw"
        step("with the tool on, a click on an atom puts the radical mark and the label shows C2H5• 29.0391", click_mark)
        def click_again():
            x, y = atom_pt(0); pg.mouse.click(x, y)
            wait_for_js(pg, f"{RADS} === ''")
            wait_for_js(pg, f"/C2H6.*30\\.0470/.test({LABELS}) && !/•/.test({LABELS})", timeout=10000)
        step("a second click takes the mark off", click_again)
        def esc():
            pg.keyboard.press("Escape"); wait_for_js(pg, "!document.querySelector('#kframe').contentDocument.documentElement.classList.contains('tp-rad')")
            x, y = atom_pt(1); pg.mouse.click(x, y)
            assert pg.evaluate(RADS) == "", "after Esc a click does not mark"
        step("Esc ends the tool", esc)
        def other_tool():
            pg.evaluate(f"{KJ}.editor.selection(null)")                     # the click of the step above selected the atom
            fr.locator("#tp-bar [data-tp=charge]").click(); fr.locator("#tp-bar [data-tp-own]").click(); assert tool_on()
            fr.locator("#tp-bar [data-tp=charge]").click(); fr.locator("#tp-bar [role=menuitem]").nth(10).click()          # charge −
            wait_for_js(pg, "!document.querySelector('#kframe').contentDocument.documentElement.classList.contains('tp-rad')")
            x, y = atom_pt(1); pg.mouse.click(x, y); pg.keyboard.press("Escape")
            assert pg.evaluate(RADS) == "", "the charge tool, not the mark"
            pg.evaluate("TPDraw.undo()"); wait_for_js(pg, f"![...{KJ}.editor.struct().atoms.values()].some(a=>a.charge)")
        step("another tool of the menu (charge −) ends the radical tool", other_tool)
        def keyboard():
            pg.evaluate(f"(()=>{{const K={KJ};K.editor.selection({{atoms:[[...K.editor.struct().atoms.keys()][1]]}})}})()")
            IN(pg, "d => d.querySelector('#tp-bar [data-tp=charge]').focus()")
            fr.locator("#tp-bar [data-tp=charge]").press("ArrowDown")
            fr.locator("#tp-bar [data-tp=charge] ~ [role=menu] [role=menuitem]").first.press("End")
            wait_for_js(pg, "document.querySelector('#kframe').contentDocument.activeElement.hasAttribute('data-tp-own')")
            fr.locator("#tp-bar [data-tp-own]").press("Enter")
            wait_for_js(pg, f"{RADS} === 'C2'")
            assert not tool_on(), "with a selection the item marks the atoms and leaves the tool off"
        step("from the keyboard the item marks the selected atoms", keyboard)
        def exports():
            mf = pg.evaluate(f"{KJ}.getMolfile()"); assert "M  RAD  1" in mf, mf[-200:]
            smi = pg.evaluate("TPDraw.smiles()"); assert "[CH2]" in smi, smi
            pg.evaluate("TPDraw.undo()"); wait_for_js(pg, f"{RADS} === '' && {KJ}.editor.struct().atoms.size === 2")
        step("the mark goes out as M  RAD and [CH2] and is one undo step", exports)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
