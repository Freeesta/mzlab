"""File list: the selected file is marked (aria-current, not only colour); a click on a hidden MRM file leaves no empty panels
(only the blocks of the types with a visible file); a spectrum that follows a chromatogram shares a side bar and a «→ n» / «← n» tag with it."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_file_selezione.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8866, wd="/tmp/wd66")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz(f) for f in FILES])
        pg.click("text=Carica dati"); ready(pg)
        attendi(pg, "E.files.filter(f=>f&&f.kind==='mrm').length>=1&&E.panels.some(p=>p.type==='chrom'&&p.el)")
        def marked():
            assert pg.evaluate("document.querySelectorAll('#flst .fl[aria-current=true]').length") == 1
            assert pg.evaluate("document.querySelector('#flst .fl[aria-current=true]').classList.contains('cur')")
            assert pg.evaluate("getComputedStyle(document.querySelector('#flst .fl.cur')).boxShadow") != "none"
            k = pg.evaluate("E.files.filter(f=>f&&f.kind==='full')[1].k")
            pg.click(f"#flst .fl[data-k='{k}'] .nm")
            attendi(pg, f"document.querySelector('#flst .fl[aria-current=true]')?.dataset.k==='{k}'&&document.querySelectorAll('#flst .fl[aria-current=true]').length===1")
        step("the selected file has aria-current and the marker", marked)
        def pairs():
            n = pg.evaluate("E.panels.filter(p=>p.type==='spec'&&p.link!=null).length")
            assert n >= 1
            assert pg.evaluate("E.panels.filter(p=>p.el&&p.el.dataset.pair).length") >= 2 * n
            assert pg.evaluate("[...document.querySelectorAll('.pnl[data-pair] .ppair')].every(x=>/^[\u2192\u2190] \\d+$/.test(x.textContent))")
            pg.evaluate("document.querySelector('.pnl[data-pair]').dispatchEvent(new MouseEvent('mouseenter'))")
            assert pg.evaluate("document.querySelectorAll('.pnl.pair-hi').length") == 1
        step("a spectrum and its chromatogram are paired (bar, tag, hover)", pairs)
        def mrm_click():
            pg.evaluate("E.files.forEach(f=>{if(f)f.vis=false});cleanupOrphanPanels();renderFileList();redrawAll()")
            attendi(pg, "E.panels.length===0")
            k = pg.evaluate("E.files.find(f=>f&&f.kind==='mrm').k")
            pg.click(f"#flst .fl[data-k='{k}'] .nm")
            attendi(pg, "E.panels.some(p=>p.tab==='mrm'&&p.el)")
            assert pg.evaluate("E.panels.every(p=>E.files.some(f=>f&&f.kind===p.tab&&f.vis&&!f.gone))")
            assert pg.evaluate("E.panels.every(p=>p.tab==='mrm')")
        step("a click on a hidden MRM file: only MRM panels, none empty", mrm_click)
        def multi():
            ks = pg.evaluate("E.files.filter(f=>f&&f.kind==='full').map(f=>f.k)"); assert len(ks) >= 3
            pg.evaluate("renderFileList()")
            pg.dblclick(f"#flst .fl[data-k='{ks[0]}'] .nm")
            attendi(pg, f"E.files.filter(f=>f&&f.vis).length===1&&E.files[{ks[0]}].vis&&document.querySelector('#flst .fl[aria-current=true]')?.dataset.k==='{ks[0]}'")
            assert pg.evaluate("E.panels.length>0&&E.panels.every(p=>p.tab==='full')"), "double click: only this file, panels of its type"
            pg.click(f"#flst .fl[data-k='{ks[2]}'] .nm", modifiers=["ControlOrMeta"])
            attendi(pg, f"E.files[{ks[2]}].vis&&E.files.filter(f=>f&&f.vis).length===2&&document.querySelector('#flst .fl[aria-current=true]')?.dataset.k==='{ks[0]}'")
            pg.click(f"#flst .fl[data-k='{ks[1]}'] > input")
            attendi(pg, f"E.files[{ks[1]}].vis")
            pg.click(f"#flst .fl[data-k='{ks[0]}'] > input")
            attendi(pg, f"!E.files[{ks[0]}].vis")
            pg.click(f"#flst .fl[data-k='{ks[2]}'] > input", modifiers=["Shift"])
            attendi(pg, f"!E.files[{ks[2]}].vis&&!E.files[{ks[1]}].vis")
            pg.click(f"#flst .fl[data-k='{ks[1]}'] .nm")
            attendi(pg, f"E.files[{ks[1]}].vis&&document.querySelector('#flst .fl[aria-current=true]')?.dataset.k==='{ks[1]}'")
        step("double click = only this file; Ctrl/Cmd+click adds; Shift+click on a box = range; click on a hidden row shows and activates it", multi)
        def derived():
            assert pg.evaluate("!document.querySelector('#fmode')"), "no mode buttons: the list is the only control"
            ks = pg.evaluate("E.files.filter(f=>f&&f.kind==='full').map(f=>f.k)")
            pg.dblclick(f"#flst .fl[data-k='{ks[0]}'] .nm")
            attendi(pg, f"E.browse&&E.cur==={ks[0]}&&E.files.filter(f=>f&&f.vis).length===1")
            pg.click(f"#flst .fl[data-k='{ks[1]}'] .nm", modifiers=["ControlOrMeta"])
            attendi(pg, f"!E.browse&&E.cur==={ks[0]}")                            # two ticked: overlaid, the first stays the active one
            pg.click("#fnext")
            attendi(pg, f"E.cur==={ks[1]}&&E.files[{ks[0]}].vis&&E.files[{ks[1]}].vis")      # the arrows walk through the ticked files
            attendi(pg, "[...document.querySelectorAll('.pscan')].length>=1")
            assert pg.evaluate("[...document.querySelectorAll('.pscan')].every(x=>x.textContent.includes(E.files[E.cur].label))")
            pg.click(f"#flst .fl[data-k='{ks[1]}'] > input")                       # unticking the active file hands the role to a ticked one
            attendi(pg, f"E.browse&&E.cur==={ks[0]}")
            pg.click("#fnext")
            attendi(pg, f"E.cur!=={ks[0]}&&E.browse&&E.files.filter(f=>f&&f.vis).length===1")      # one file shown: the arrows replace it
        step("the list drives the view: browse follows the ticked files, arrows, «scan of» tag", derived)
finally:
    r.close(); r.report()
print("\nSTEPS")
for s in steps: print("  ", s)
