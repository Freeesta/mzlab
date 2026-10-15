"""Saved memory (localStorage + notebook) survives a reload, a damaged copy never stops the page, two tabs do not overwrite each other,
and a direct address (#/disegno, Cmd+Shift+R) shows the editor with the saved options (formula and mass under the structure)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_memoria.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
LAB = "(()=>{const g=document.querySelector('#kframe').contentDocument.querySelector('#qqq-labels');return g?g.childNodes.length:-1})()"
KEYS = ["qqq.a11y", "qqq.banco.info", "qqq.barra.hr", "qqq.barra.lato", "qqq.barra.lr", "qqq.bianco", "qqq.comp", "qqq.contaminanti",
        "qqq.contaminanti.esclusi", "qqq.contaminanti.on", "qqq.convopen", "qqq.disegno.riquadri", "qqq.disegno.strumenti", "qqq.fair", "qqq.flgrp",
        "qqq.gradiente.ultimo", "mzlab.lang", "qqq.lib.cartella", "qqq.lib.soglie", "qqq.liste.attive", "qqq.lr", "qqq.mappa", "qqq.pca",
        "qqq.picchi", "qqq.pool", "qqq.prefs", "qqq.profilo", "qqq.qaqc", "qqq.riferimento", "qqq.tunit", "qqq.unopen", "qqq.upd.last", "qqq-labels", "qqq-theme"]
BAD = ["{", "null", "[]", "123", '"x"', '{"a":1}', "[1,2,3]", "undefined", "x" * 5000]
r = Run(port=8997, wd="/tmp/wd_memoria")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        stage(pg, [mz(f) for f in FILES[:2]]); pg.click("text=Carica dati"); ready(pg)
        def drawing_back():
            pg.click("#nav [data-v=draw]"); pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
            pg.evaluate(f"{KJ}.addFragment('CCO')"); pg.wait_for_timeout(1500)
            pg.uncheck("#lb-m"); pg.wait_for_timeout(1200)                   # only the formula stays on
            pg.reload(); pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000); pg.wait_for_timeout(1500)
            assert pg.evaluate("S.view") == "draw" and not pg.evaluate("document.querySelector('#v-draw').hidden")
            assert pg.is_checked("#lb-f") and not pg.is_checked("#lb-m"), "the saved options are applied at start"
            assert pg.evaluate(f"{KJ}.editor.struct().atoms.size") == 3, "the drawing is back"
            assert pg.evaluate(LAB) == 1, pg.evaluate(LAB)                   # one structure, one line (formula only)
        step("reload on #/disegno: editor, drawing, saved options and labels are back", drawing_back)
        def quick_draw_labels():
            pg.evaluate(f"{KJ}.setMolecule('')"); pg.wait_for_timeout(800)
            pg.check("#lb-m"); pg.fill("#ex-smi", "CCO"); pg.click("#ex-load"); pg.wait_for_timeout(1500)
            assert pg.evaluate(LAB) == 2, pg.evaluate(LAB)                   # formula and mass appear at once, without clicking the options again
        step("Quick draw: the labels under the new structure appear by themselves", quick_draw_labels)
        def damaged_keys():
            n = 0
            for bad in BAD:
                pg.evaluate("([ks,v])=>{ks.forEach(k=>{try{localStorage.setItem(k,v)}catch(_){}})}", [KEYS, bad])
                errs0 = len(r.errs); pg.reload(); pg.wait_for_function("window.qqReady===true", timeout=90000); pg.wait_for_timeout(600)
                new = [e for e in r.errs[errs0:] if e[0] == "pageerror"]
                assert not new, (bad[:20], new[:2]); n += 1
            pg.evaluate("([ks])=>{ks.forEach(k=>localStorage.removeItem(k))}", [KEYS])
        step("every saved key damaged in nine ways: the page still starts, no script error", damaged_keys)
        def damaged_notebook():
            for bad in ["[1,2]", "null", '"x"', '{"ui":{"panels":5,"files":"a"},"session":"x"}', '{"session":[1,null,{"file":3}]}']:
                pg.evaluate("b=>fetch('api/notebook',{method:'POST',body:b})", bad); pg.wait_for_timeout(300)
                errs0 = len(r.errs); pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_function("window.qqReady===true", timeout=90000); pg.wait_for_timeout(600)
                new = [e for e in r.errs[errs0:] if e[0] == "pageerror"]
                assert not new, (bad[:30], new[:2])
        step("a damaged notebook never stops the page", damaged_notebook)
        def two_tabs():
            pg.evaluate("fetch('api/notebook',{method:'POST',body:'{}'})")
            ctx = r.b.new_context(viewport={"width": 1440, "height": 900})
            ctx.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
            a, q = ctx.new_page(), ctx.new_page()
            for t in (a, q): t.goto(f"http://127.0.0.1:{r.port}/#/disegno"); t.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
            a.fill("#ex-name", "tab_a"); a.dispatch_event("#ex-name", "change"); a.wait_for_timeout(1500)
            q.select_option("#lb-dec", "2"); q.wait_for_timeout(1500)       # the second tab never saw the name written by the first
            nb = a.evaluate("fetch('api/notebook').then(r=>r.json())")
            assert nb.get("drawName") == "tab_a" and nb.get("labDec") == 2, nb
            ctx.close()
        step("two tabs: each one's change is kept, the other tab's is not undone", two_tabs)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
