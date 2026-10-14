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
finally:
    r.close(); r.report()
print("\nSTEPS")
for s in steps: print("  ", s)
