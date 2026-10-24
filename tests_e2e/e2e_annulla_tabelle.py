"""Undo of the tables, the lists and the settings of the Dati view (comandi.js): metadata of the samples, file list, preferences, peak settings, XIC traces,
series of the legend, title of a panel, marks of the map, opening of a panel, isotopes and the background of the spectrum. Each case does the action and
takes it back with Ctrl/Cmd+Z, then does it again with the redo."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try:
        fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_annulla_tabelle.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
XIC = "E.panels.find(p=>p.type==='xic')"
r = Run(port=8886, wd="/tmp/wd86")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        tick = lambda: pg.evaluate("new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>setTimeout(r,0))))")
        stage(pg, [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        mod = "Meta" if pg.evaluate("/Mac/.test(navigator.platform)") else "Control"
        def undo(): pg.keyboard.press(f"{mod}+z"); tick()
        def redo(): pg.keyboard.press(f"{mod}+Shift+z"); tick()
        def fresh(): pg.evaluate("STORIA.svuota();document.activeElement&&document.activeElement.blur()")
        ev = pg.evaluate
        def files_view():
            fresh()
            assert ev("E.files[0].vis") is True
            pg.click("#flst input[data-k='0']"); tick()
            assert ev("E.files[0].vis") is False, "the box hides the file"
            assert ev("E.panels.length") == 0, "the panels of the hidden file are closed"
            undo(); assert ev("E.files[0].vis") is True, "Ctrl+Z shows it again"
            assert ev("E.panels.map(p=>p.type).sort().join()") == "chrom,spec", "the panels it closed come back"
            redo(); assert ev("E.files[0].vis") is False, "redo hides it"
            undo()
            n = ev("STORIA.pila.v.length")
            ev("fileViewDo(()=>{E.files[1].label='zz'})"); assert ev("STORIA.pila.v.length") == n + 1
            undo(); assert ev("E.files[1].label") != "zz", "Ctrl+Z gives the name back"
            first = ev("E.files.map(f=>f.file)")
            ev("reorderFiles(E.files[1].k,E.files[0].k,true)"); tick()
            assert ev("E.files.map(f=>f.file)") == first[::-1], "the files changed place"
            undo(); assert ev("E.files.map(f=>f.file)") == first, "Ctrl+Z gives the order back"
        step("files: visibility, name and order", files_view)
        def samples_meta():
            fresh()
            f = ev("E.files[0].file"); old = ev("E.files[0].conc ?? null")
            ev(f"campioniEdit([{{file:{f!r},conc:3.5,cunit:'mg/L'}}])"); tick()
            assert ev("E.files[0].conc") == 3.5
            undo(); assert ev("E.files[0].conc ?? null") == old, "Ctrl+Z gives the concentration back"
            redo(); assert ev("E.files[0].conc") == 3.5, "redo"
        step("sample metadata: edit, undo, redo", samples_meta)
        def settings():
            fresh()
            ev("uipOpen(document.getElementById('np-set'))"); tick()
            ev("document.querySelector('#uip-g-look').open=true"); tick()
            was = ev("UIP.theme")
            pg.click("#uipset input[name=uip-th][value=dark]", force=True); tick(); pg.wait_for_function("UIP.theme==='dark'")
            undo(); pg.wait_for_function(f"UIP.theme==={was!r}")
            redo(); pg.wait_for_function("UIP.theme==='dark'")
            undo(); ev("document.getElementById('uipset')&&document.getElementById('uipset').remove()")
        step("preferences: theme, undo, redo", settings)
        def peaks_settings():
            fresh()
            before = ev("JSON.stringify([PK,INTOPT])")
            ev("COMANDI.fai('peaks.settings',COMANDI.GLOBALE,()=>{PK.win=PK.win+1;INTOPT.sn=!INTOPT.sn})")
            assert ev("JSON.stringify([PK,INTOPT])") != before
            undo(); assert ev("JSON.stringify([PK,INTOPT])") == before, "Ctrl+Z gives the settings back"
        step("peak settings: change, undo", peaks_settings)
        def xic_traces():
            fresh()
            ev("openXic(null)") if False else None
            ev("xicDirect(364.4,null,null)"); tick(); pg.wait_for_function(f"!!{XIC}&&{XIC}.traces.length===1")
            n = ev("STORIA.pila.v.length"); xp = XIC
            ev(f"addTrace({xp},152.2)"); assert ev(f"{xp}.traces.length") == 2
            assert ev("STORIA.pila.v.length") == n + 1
            undo(); assert ev(f"{xp}.traces.length") == 1, "Ctrl+Z takes the trace out"
            redo(); assert ev(f"{xp}.traces.length") == 2, "redo"
            ev(f"COMANDI.fai('series.visibility',{xp},()=>{{{xp}.hid['x|0|364.4']=true}})")
            undo(); assert not ev(f"!!{xp}.hid['x|0|364.4']"), "Ctrl+Z shows the series"
            ev(f"COMANDI.fai('panel.rename',{xp},()=>{{{xp}.title='mine'}})")
            undo(); assert ev(f"{xp}.title") != "mine", "Ctrl+Z gives the title back"
        step("XIC: traces, series and title", xic_traces)
        def apri_chiudi():
            fresh()
            n = ev("E.panels.length")
            pg.click("#np-chrom"); tick()
            assert ev("E.panels.length") == n + 1
            undo(); assert ev("E.panels.length") == n, "Ctrl+Z closes the panel just opened"
            redo(); assert ev("E.panels.length") == n + 1, "redo opens it again"
            undo()
        step("panels: open, undo, redo", apri_chiudi)
        def isotopi_sfondo():
            fresh()
            sp = SPEC
            ev(f"COMANDI.fai('spec.isotopes',{sp},()=>{{{sp}.iso={{formula:'C10H14N',ad:'[M]+'}};{sp}.zoom=[100,200]}})")
            undo(); assert ev(f"{sp}.iso") is None and ev(f"{sp}.zoom") is None, "Ctrl+Z takes the pattern and its zoom back"
            ev(f"COMANDI.fai('spec.background',{sp},()=>{{{sp}.bg='w';{sp}.bw0=2;{sp}.bw1=3}})")
            undo(); assert ev(f"{sp}.bw0") is None and ev(f"{sp}.bg") in ("", None), "Ctrl+Z gives the background back"
        step("spectrum: isotopes and background", isotopi_sfondo)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1].startswith("FAIL") for s in steps): sys.exit(1)
