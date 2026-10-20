"""Barra strumenti dell'header e cassetto della vista Dati (LR e HR): un posto per strumento, cassetto chiuso/sovrapposto/fissato, Esc e fuoco,
lato per trascinamento e da tastiera, stato ricordato, menu HR da tastiera, nessun pulsante a capo o tagliato a 1280/1440/1680 e su iPad."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_strumenti.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))

EX = ROOT / "mzlab" / "web" / "esempi" / "FullScan_t15.mzML"
D = synth("strumenti")
r = Run(port=8957, wd="/tmp/wd_strumenti")

# a toolbar that fits: one row, every visible button inside the bar, no text cut
FIT = """(() => {
  const h = document.querySelector('header'), bar = document.querySelector('#stbar'), br = bar.getBoundingClientRect();
  const tops = [...h.children].filter(e => e.offsetParent && e.getBoundingClientRect().height).map(e => Math.round(e.getBoundingClientRect().top + e.getBoundingClientRect().height / 2));
  const btns = [...bar.querySelectorAll('.tb')].filter(b => !b.hidden && b.offsetParent);
  return {rows: Math.max(...tops) - Math.min(...tops), hh: h.getBoundingClientRect().height, clipped: btns.filter(b => { const q = b.getBoundingClientRect(); return q.right > br.right + 1 || q.left < br.left - 1; }).map(b => b.dataset.cmd || b.dataset.cmdMain || b.dataset.cmdCar),
    cut: btns.filter(b => [...b.querySelectorAll('.tl')].some(l => l.offsetParent && l.scrollWidth > l.clientWidth + 1)).map(b => b.dataset.cmd), n: btns.length, lab: bar.classList.contains('lab-all'), lvl: bar.classList.contains('lab-all') ? 'all' : bar.classList.contains('lab-main') ? 'main' : 'none', dropped: (document.querySelector('#tb-more').dataset.dropped || ''), wrapped: btns.filter(b => b.getBoundingClientRect().height > 48).length};
})()"""

try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1500, "height": 1000})
        stage(pg, [EX]); pg.click("text=Carica dati"); ready(pg)

        def barra():
            assert pg.get_attribute("#stbar", "role") == "toolbar" and pg.get_attribute("#stbar", "aria-label") == "Strumenti"
            assert pg.locator("#sidebar-tablist").count() == 0, "no rail"
            ids = pg.evaluate("[...document.querySelectorAll('#stbar > button[data-cmd]')].filter(b=>!b.hidden).map(b=>b.dataset.cmd)")
            assert ids == ["file", "calc", "losses", "adducts", "lists", "ptable", "more", "search"], ids
            lab = pg.evaluate("[...document.querySelectorAll('#stbar > button[data-cmd]')].filter(b=>!b.hidden).map(b=>b.innerText.trim())")
            assert lab == ["File", "Calcolatrice", "Perdite", "Addotti", "Liste", "Tavola", "Altro", "Cerca"], lab
            for b in pg.query_selector_all("#stbar > button[data-cmd]"):
                if b.is_visible(): assert b.get_attribute("aria-label") and b.get_attribute("title") and b.query_selector("svg"), b.get_attribute("data-cmd")
            assert pg.evaluate("document.querySelectorAll('header button#np-pc, header button#np-bt').length") == 0, "PubChem and BioTransformer only in «Altro»"
            assert pg.is_visible("#dfiles") and pg.evaluate("BARRA.curTab") == "file", "first start on a wide screen: the File panel is open"
        step("barra in the header (role toolbar), file/calc/losses/adducts/lists/table in this order, words under the icons, no rail, File panel open at first start", barra)

        def altro():
            pg.click("#tb-more"); pg.wait_for_selector("#stmenu")
            t = pg.inner_text("#stmenu"); assert "PubChem" in t and "BioTransformer" in t, t
            assert pg.get_attribute("#stmenu", "role") == "menu" and pg.get_attribute("#tb-more", "aria-expanded") == "true"
            pg.keyboard.press("Escape"); assert pg.locator("#stmenu").count() == 0 and pg.evaluate("document.activeElement.id") == "tb-more"
        step("«Altro» menu: PubChem and BioTransformer, Esc closes and gives the focus back", altro)

        def apri_chiudi():
            pg.click("#sb-x"); pg.wait_for_timeout(400)      # open at first start: close it, then reopen with the tool button
            w0 = pg.evaluate("E.panels[0].w")
            pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(500)
            assert pg.is_visible("#dfiles") and pg.is_visible("#flst"), "file list in the panel"
            assert pg.get_attribute("#stbar [data-cmd=file]", "aria-pressed") == "true" and "on" in pg.get_attribute("#stbar [data-cmd=file]", "class")
            assert pg.evaluate("E.panels[0].w") < w0 - 150, "the panel is docked: the charts narrow"
            assert pg.evaluate("document.querySelector('#dfiles').contains(document.activeElement)"), "focus moves into the panel"
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300); assert pg.is_visible("#dfiles"), "docked: Esc does not close it"
            pg.mouse.click(900, 500); pg.wait_for_timeout(300); assert pg.is_visible("#dfiles"), "docked: a click on the charts does not close it"
            pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(500)
            assert not pg.is_visible("#dfiles") and pg.get_attribute("#stbar [data-cmd=file]", "aria-pressed") == "false", "the same button hides it"
            assert pg.evaluate("document.activeElement.dataset.cmd") == "file", "the focus goes back to the button"
            assert pg.evaluate("E.panels[0].w") == w0, "the charts get the page back"
            pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(300); pg.click("#sb-x"); pg.wait_for_timeout(300); assert not pg.is_visible("#dfiles")
        step("panel: docks next to the charts (they narrow), the same button / × hide it, focus in and back", apri_chiudi)

        def cambia():
            pg.click("#stbar [data-cmd=calc]"); pg.wait_for_timeout(400)
            assert pg.is_visible("#calcdlg") and pg.inner_text("#sb-title") == "Calcolatrice m/z", pg.inner_text("#sb-title")
            pg.click("#stbar [data-cmd=adducts]"); pg.wait_for_timeout(300)
            assert pg.inner_text("#sb-title") == "Addotti ESI" and not pg.is_visible("#calcdlg")
            assert pg.get_attribute("#stbar [data-cmd=calc]", "aria-pressed") == "false" and pg.get_attribute("#stbar [data-cmd=adducts]", "aria-pressed") == "true"
            pg.click("#stbar [data-cmd=losses]"); pg.wait_for_timeout(300); assert pg.inner_text("#sb-title") == "Perdite neutre" and pg.is_visible("#sb-panel-losses")
            pg.click("#stbar [data-cmd=lists]"); pg.wait_for_timeout(500); assert pg.is_visible("#sb-panel-lists")
            pg.click("#stbar [data-cmd=ptable]"); pg.wait_for_timeout(500); assert pg.evaluate("document.querySelector('#refdlg').open"), "the table stays a window"
            pg.evaluate("document.querySelector('#refdlg').close()")
            pg.click("#sb-x"); pg.wait_for_timeout(300); assert not pg.is_visible("#dfiles")
        step("one tool at a time in the drawer; the periodic table stays the window #refdlg", cambia)

        def larghezza():
            pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(300)
            h = pg.locator("#sidebar-resize-handle")
            assert h.get_attribute("role") == "separator" and h.get_attribute("aria-orientation") == "vertical" and h.get_attribute("tabindex") == "0"
            pg.evaluate("BARRA.setWidth(300)"); w300 = pg.evaluate("Q('#dpanels').clientWidth")
            h.focus(); pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(300)
            assert pg.evaluate("BARRA.width") == 316 and pg.evaluate("Q('#dpanels').clientWidth") < w300, "left panel: → widens it, the charts narrow"
            pg.keyboard.press("Home"); pg.wait_for_timeout(100); assert pg.evaluate("BARRA.width") == 280
            pg.keyboard.press("End"); pg.wait_for_timeout(100); assert pg.evaluate("BARRA.width") == 560
            pg.keyboard.press("Enter"); pg.wait_for_timeout(100); assert pg.evaluate("BARRA.width") == 300
            pg.evaluate("BARRA.setWidth(1000)"); assert pg.evaluate("BARRA.width") == 560, "never wider than 560 px"
            assert pg.evaluate("E.panels.filter(p=>p.full).every(p=>Math.abs(p.w - Q('#dpanels').clientWidth) <= 2)"), "the charts follow the narrower page"
            h.dblclick(); pg.wait_for_timeout(200); assert pg.evaluate("BARRA.width") == 300, "double click = default"
            pg.click("#sb-x"); pg.wait_for_timeout(300)
        step("splitter (role separator): keyboard, Home/End, double click, 280-560 px", larghezza)

        def lato():
            assert pg.locator("#sb-side-tog").count() == 0, "the useless «move to the right» button is gone"
            pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(300)
            b = pg.locator("#sb-title").bounding_box(); x0, y0 = b["x"] + 4, b["y"] + b["height"] / 2
            pg.mouse.move(x0, y0); pg.mouse.down(); pg.mouse.move(x0 + 40, y0, steps=3)
            assert pg.is_visible(".sb-dz.l") and pg.is_visible(".sb-dz.r") and pg.is_visible(".sb-dz-prev"), "drop zones and preview while dragging"
            pg.mouse.move(1300, 300, steps=6); pg.wait_for_timeout(100)
            assert "hot" in pg.get_attribute(".sb-dz.r", "class") and "hot" not in pg.get_attribute(".sb-dz.l", "class")
            pg.mouse.up(); pg.wait_for_timeout(300)
            assert pg.evaluate("BARRA.side") == "right" and pg.evaluate("document.querySelector('#dfiles').getBoundingClientRect().right") > 1480, pg.evaluate("BARRA.side")
            assert pg.locator(".sb-dz").count() == 0
            assert pg.evaluate("localStorage.getItem('mzlab.barra.lato')") == "right"
            pg.keyboard.press("Alt+Shift+ArrowLeft"); pg.wait_for_timeout(300)
            assert pg.evaluate("BARRA.side") == "left" and pg.evaluate("document.querySelector('#dfiles').getBoundingClientRect().left") < 20
            pg.keyboard.press("Alt+Shift+ArrowRight"); pg.wait_for_timeout(200); assert pg.evaluate("BARRA.side") == "right"
            w_dp = pg.evaluate("Q('#dpanels').getBoundingClientRect().left")
            b = pg.locator("#sb-title").bounding_box(); x0, y0 = b["x"] + 4, b["y"] + b["height"] / 2
            pg.mouse.move(x0, y0); pg.mouse.down(); pg.mouse.move(100, 300, steps=6); pg.mouse.up(); pg.wait_for_timeout(500)
            assert pg.evaluate("BARRA.side") == "left"
            assert pg.evaluate("Q('#dpanels').getBoundingClientRect().left") > w_dp + 150, "the charts move to the free side"
            assert pg.evaluate("E.panels.filter(p=>p.full).every(p=>Math.abs(p.w - Q('#dpanels').clientWidth) <= 2)")
            pg.click("#sb-x"); pg.wait_for_timeout(300)
        step("side by drag and drop of the header (drop zones, preview, charts move) and Alt+Shift+arrows; remembered in mzlab.barra.lato", lato)

        def ricorda():
            pg.click("#stbar [data-cmd=adducts]"); pg.wait_for_timeout(300)
            pg.evaluate("BARRA.setWidth(320)"); pg.evaluate("BARRA.setSide('right')")
            st = pg.evaluate("JSON.parse(localStorage.getItem('mzlab.barra.lr'))"); assert st["w"] == 320 and st["tab"] == "adducts" and st["open"] and "pinned" not in st, st
            pg.reload(); ready(pg)
            assert pg.is_visible("#dfiles") and pg.evaluate("BARRA.side === 'right' && BARRA.width === 320 && BARRA.curTab === 'adducts'"), "open, tool, side and width come back"
            assert pg.is_visible("#sb-panel-adducts"), "the restored panel is painted"
            pg.click("#sb-x"); pg.evaluate("BARRA.setSide('left')"); pg.reload(); ready(pg)
            assert not pg.is_visible("#dfiles"), "hidden when it was left hidden"
            pg.evaluate("localStorage.setItem('mzlab.barra.lato','bogus'); localStorage.setItem('mzlab.barra.lr', JSON.stringify({tab:'calc', pinned:true, open:true, w:0}))"); pg.reload(); ready(pg)
            st = pg.evaluate("JSON.parse(localStorage.getItem('mzlab.barra.lr'))"); assert "pinned" not in st and st["w"] == 300 and pg.evaluate("localStorage.getItem('mzlab.barra.lato')") == "left", st
            pg.evaluate("localStorage.setItem('mzlab.barra.lr', JSON.stringify({tab:'file', open:true, w:99999}))"); pg.reload(); ready(pg)
            assert pg.evaluate("BARRA.width") <= 560, "a width out of range goes back to the default"
            pg.evaluate("localStorage.removeItem('mzlab.barra.lr'); localStorage.removeItem('mzlab.barra.lato')"); pg.reload(); ready(pg)
            assert pg.is_visible("#dfiles"), "first start: open"
        step("state remembered after reload (open, side, width, tool); old states (pinned, bad side, width 0 or huge) are cleaned", ricorda)

        def larghezze():
            # the richest level that fits: words on every button from 1280 px; a narrower bar loses words, then groups (into «Altro»), never a button cut or wrapped
            for panel in (False, True):
                pg.evaluate("BARRA.show('file',{noFocus:true})" if panel else "BARRA.close(false)")
                for w, h in ((1280, 800), (1440, 900), (1500, 1000), (1680, 1000), (1024, 1366), (1366, 1024), (900, 900), (820, 1180)):
                    pg.set_viewport_size({"width": w, "height": h}); pg.wait_for_timeout(500)
                    f = pg.evaluate(FIT)
                    assert f["rows"] <= 6 and f["hh"] < 70, (w, panel, f)
                    assert not f["clipped"] and not f["cut"] and f["wrapped"] == 0, (w, panel, f)
                    if w >= 1280: assert f["lvl"] == "all" and not f["dropped"], (w, panel, f)
                    assert pg.evaluate("document.documentElement.scrollWidth - document.documentElement.clientWidth") <= 1, (w, panel)
                    names = pg.evaluate("[...document.querySelectorAll('#stbar .tb')].filter(b=>!b.hidden&&b.offsetParent).map(b=>b.getAttribute('aria-label'))"); assert all(names), (w, names)
                    if f["dropped"]:
                        pg.click("#tb-more"); pg.wait_for_selector("#stmenu")
                        txt = pg.inner_text("#stmenu")
                        for d in f["dropped"].split(","): assert pg.evaluate(f"BARRA.name(BARRA.byId('{d}'))") in txt, (w, d, txt)
                        pg.keyboard.press("Escape")
            pg.evaluate("BARRA.close(false)"); pg.set_viewport_size({"width": 1500, "height": 1000})
        step("measured labels: words on every button from 1280 px, fewer words then fewer groups below, nothing wrapped or cut (with and without the panel, iPad included); the rest is in «Altro»", larghezze)

        def preferenza():
            pg.set_viewport_size({"width": 1500, "height": 1000}); pg.wait_for_timeout(400)
            assert pg.evaluate("typeof BARRA.setLabels") == "undefined" and pg.evaluate(FIT)["lvl"] == "all"
            pg.click("#np-set"); pg.wait_for_selector("#uipset"); assert pg.locator("#uip-etich").count() == 0, "no labels option in the settings"
            pg.keyboard.press("Escape")
            pg.evaluate("localStorage.setItem('mzlab.barra.etichette','hidden')"); pg.reload(); ready(pg)
            assert pg.evaluate(FIT)["lvl"] == "all", "an old saved value is ignored: the labels follow the width"
            pg.set_viewport_size({"width": 1024, "height": 800}); pg.wait_for_timeout(500)
            f = pg.evaluate(FIT); assert not f["clipped"] and f["wrapped"] == 0, f
            pg.evaluate("localStorage.removeItem('mzlab.barra.etichette')"); pg.set_viewport_size({"width": 1500, "height": 1000}); pg.wait_for_timeout(400)
            assert pg.evaluate("BARRA.fitStats.max") < 50
        step("labels are always automatic: no option in the settings, an old saved value is ignored", preferenza)

        def disegno():
            pg.click("#nav [data-v=draw]"); pg.wait_for_timeout(800)
            assert not pg.is_visible("#stbar [data-cmd=file]"), "no File outside the Dati view"
            pg.click("#np-ad"); pg.wait_for_timeout(500); assert pg.evaluate("document.querySelector('#refdlg').open"), "in Disegno the tools are the windows of before"
            pg.evaluate("document.querySelector('#refdlg').close()")
            pg.click("#np-calc2"); pg.wait_for_timeout(300); assert pg.is_visible("#calcdlg") and pg.evaluate("!BARRA.open")
            pg.click("#np-calc2"); pg.wait_for_timeout(200); assert not pg.is_visible("#calcdlg")
            pg.click("#nav [data-v=data]"); pg.wait_for_timeout(500); assert pg.is_visible("#stbar [data-cmd=file]")
        step("in Disegno the same buttons open the windows of before; File only in the Dati view", disegno)

        # ------------------------------------------------------------------ high resolution
        pg.evaluate("fetch('api/new',{method:'POST',body:JSON.stringify({fresh:true})})"); pg.reload(); pg.wait_for_timeout(1500)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)

        def hr_barra():
            ids = pg.evaluate("[...document.querySelectorAll('#stbar button[data-cmd],#stbar [data-cmd-main]')].filter(b=>!b.hidden&&b.offsetParent).map(b=>b.dataset.cmd||b.dataset.cmdMain)")
            assert ids == ["file", "calc", "losses", "adducts", "lists", "ptable", "scan", "form", "idn", "more", "search"], ids
            pg.set_viewport_size({"width": 1800, "height": 1000}); attendi(pg, "document.documentElement.dataset.barLab === 'all'")        # the words need room: the panel pill takes its own word from the bar
            lab = pg.evaluate("[...document.querySelectorAll('#stbar button.tb')].filter(b=>!b.hidden&&b.offsetParent).map(b=>b.innerText.trim()).filter(Boolean)")
            assert all(x in lab for x in ("Scansione", "Formule", "Librerie")), lab         # the HR group keeps its words as long as the main tools do
            assert "·" not in " ".join(lab) and not any(x.isupper() and len(x) == 3 for x in lab), lab
            assert pg.evaluate("document.querySelector('#stbar [data-hrgrp]').offsetParent !== null"), "separator before the HR group"
            for w in (1280, 1440, 1680):
                pg.set_viewport_size({"width": w, "height": 900}); pg.wait_for_timeout(500)
                f = pg.evaluate(FIT); assert f["rows"] <= 6 and not f["clipped"] and not f["cut"] and f["wrapped"] == 0, (w, f)
            for w in (900, 1024):
                pg.set_viewport_size({"width": w, "height": 900}); pg.wait_for_timeout(500)
                f = pg.evaluate(FIT); assert not f["clipped"] and not f["cut"] and f["wrapped"] == 0, (w, f)
            pg.set_viewport_size({"width": 1500, "height": 1000}); pg.wait_for_timeout(300)
        step("HR: same bar plus Scansione ▾, Formule ▾, Librerie after a separator, in words (no HDR/LST…), fits at every width", hr_barra)

        def hr_menu():
            pg.focus("#stbar [data-cmd-car=scan]"); pg.keyboard.press("ArrowDown"); pg.wait_for_selector("#stmenu")
            items = pg.evaluate("[...document.querySelectorAll('#stmenu [role^=menuitem]')].map(b=>b.innerText.trim())")
            assert items == ["Intestazione della scansione", "Elenco delle scansioni", "File e strumento"], items
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("Enter"); pg.wait_for_timeout(800)
            assert pg.is_visible("#hrinfo") and pg.evaluate("BARRA.curTab") == "lst" and pg.inner_text("#sb-title") == "Elenco delle scansioni"
            assert pg.evaluate("document.querySelector('#stbar [data-cmd-main=scan]').getAttribute('aria-label')") == "Scansione: Elenco delle scansioni"
            assert pg.get_attribute("#stbar [data-cmd-main=scan]", "aria-pressed") == "true"
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
            if pg.is_visible("#dfiles"): pg.click("#sb-x")
            pg.click("#stbar [data-cmd-main=scan]"); pg.wait_for_timeout(600)
            assert pg.is_visible("#hrinfo") and pg.evaluate("BARRA.curTab") == "lst", "a click on the button reopens the last chosen item"
            pg.click("#stbar [data-cmd-car=scan]"); pg.wait_for_selector("#stmenu")
            assert pg.evaluate("document.querySelector('#stmenu [aria-checked=true]').innerText.trim()") == "Elenco delle scansioni"
            pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
            pg.click("#stbar [data-cmd-car=form]"); pg.wait_for_selector("#stmenu")
            items = pg.evaluate("[...document.querySelectorAll('#stmenu [role^=menuitem]')].map(b=>b.innerText.trim())"); assert items == ["Composizione elementare", "Simulazione isotopica"], items
            pg.click("#stmenu [role^=menuitem] >> nth=1"); pg.wait_for_timeout(800)
            assert pg.evaluate("BARRA.curTab") == "iso" and pg.is_visible("#hrinfo")
            pg.click("#stbar [data-cmd=idn]"); pg.wait_for_timeout(600); assert pg.evaluate("BARRA.curTab") == "idn" and pg.is_visible("#hrinfo")
            pg.click("#stbar [data-cmd=idn]"); pg.wait_for_timeout(300); assert not pg.is_visible("#dfiles")
            for t in ("hdr", "fil", "cmp"):
                pg.evaluate(f"BARRA.setTab('{t}')"); pg.wait_for_timeout(500); assert pg.is_visible("#hrinfo") and pg.evaluate("BARRA.curTab") == t, t
            pg.click("#sb-x")
        step("HR menus from the keyboard (↓ opens, ↓ Enter chooses), last item kept on the button and reopened by a click, 6 tools in all", hr_menu)

        def hr_stato():
            pg.click("#stbar [data-cmd=adducts]"); pg.evaluate("BARRA.setWidth(400)")
            hr = pg.evaluate("JSON.parse(localStorage.getItem('mzlab.barra.hr'))"); lr = pg.evaluate("JSON.parse(localStorage.getItem('mzlab.barra.lr')||'null')")
            assert hr["open"] and hr["w"] == 400 and hr["tab"] == "adducts" and hr["last"]["scan"] == "lst" and hr["last"]["form"] == "iso", hr
            assert lr is None or lr.get("w") != 400, "LR and HR keep separate states"
            pg.click("#sb-x")
        step("HR state in mzlab.barra.hr (pinned, width, tool, last item of each menu), separate from LR", hr_stato)

        # ------------------------------------------------------------------ narrow windows, iPad, phone
        def stretta():
            pg.set_viewport_size({"width": 1500, "height": 1000}); pg.wait_for_timeout(300)
            pg.evaluate("BARRA.setTab('adducts',{noFocus:true})"); pg.wait_for_timeout(300)
            assert pg.evaluate("BARRA.open && !BARRA.sheet")
            pg.set_viewport_size({"width": 800, "height": 900})
            attendi(pg, "!BARRA.open")           # below 1100 px the side panel closes by itself; the tool button gives it back, as a sheet
            pg.evaluate("BARRA.setTab('adducts',{noFocus:true})"); attendi(pg, "BARRA.open && BARRA.sheet")
            assert pg.is_visible("#dfiles") and pg.evaluate("BARRA.sheet") and not pg.is_visible("#dpanels"), "below 900 px the panel is a sheet that replaces the workspace"
            assert pg.evaluate("document.querySelector('#dfiles').getBoundingClientRect().width") >= 800 - 30 and pg.is_visible("#sb-x")
            assert pg.evaluate("document.documentElement.scrollWidth - document.documentElement.clientWidth") <= 1
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
            assert not pg.is_visible("#dfiles") and pg.is_visible("#dpanels")
            pg.evaluate("localStorage.removeItem('mzlab.barra.hr')"); pg.set_viewport_size({"width": 1500, "height": 1000})
        step("under 900 px the panel is a sheet (replaces the workspace, × and Esc give it back)", stretta)

        def ipad():
            for w, h in ((1024, 1366), (1366, 1024)):
                ctx = r.b.new_context(viewport={"width": w, "height": h}, has_touch=True, is_mobile=True, locale="it-IT", reduced_motion="reduce")
                q = ctx.new_page(); q.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
                q.goto(f"http://127.0.0.1:{r.port}/"); q.wait_for_timeout(1500)
                try:
                    stage(q, [EX]); q.click("text=Carica dati"); ready(q); q.wait_for_function("typeof E !== 'undefined' && E.files.length>=1", timeout=60000); q.wait_for_timeout(800)     # the session lives in the browser: a new context opens the file itself
                    q.evaluate("window.scrollTo(0,0)")
                    f = q.evaluate(FIT); assert not f["clipped"] and not f["cut"] and f["wrapped"] == 0, (w, f)
                    small = q.evaluate("[...document.querySelectorAll('#stbar .tb')].filter(b=>!b.hidden&&b.offsetParent).filter(b=>{const r=b.getBoundingClientRect();return r.width<39.5||r.height<39.5}).map(b=>b.dataset.cmd||b.dataset.cmdMain||b.dataset.cmdCar)")
                    assert not small, (w, small)
                    q.evaluate("BARRA.setTab('file',{noFocus:true})"); q.wait_for_timeout(300)
                    assert q.is_visible("#dfiles") and (w > 900) == (not q.evaluate("BARRA.sheet")), (w, "iPad: docked when landscape, a sheet when portrait")
                    big = q.evaluate("[...document.querySelectorAll('#sb-header button')].filter(b=>b.offsetParent).filter(b=>{const r=b.getBoundingClientRect();return r.width<39.5||r.height<39.5}).length"); assert big == 0, big
                    q.set_viewport_size({"width": h, "height": w}); q.wait_for_timeout(600)           # rotation
                    ov = q.evaluate("[document.documentElement.scrollWidth - document.documentElement.clientWidth, E.panels.filter(p=>p.el&&!p.el.classList.contains('max')).map(p=>p.x+p.w-document.getElementById('dpanels').clientWidth).reduce((m,v)=>Math.max(m,v),-9)]")
                    assert ov[0] <= 1 and ov[1] <= 2, (w, ov)
                    f = q.evaluate(FIT); assert not f["clipped"] and not f["cut"] and f["wrapped"] == 0, (h, f)
                finally:
                    ctx.close()
        step("iPad 1024×1366 and 1366×1024 (touch): buttons ≥ 40 px, the panel is only a drawer, rotation leaves charts and bar whole", ipad)

        def telefono():
            ctx = r.b.new_context(viewport={"width": 390, "height": 844}, has_touch=True, is_mobile=True, locale="it-IT", reduced_motion="reduce")
            q = ctx.new_page(); q.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
            q.goto(f"http://127.0.0.1:{r.port}/?telefono=0"); q.wait_for_timeout(1500)
            try:
                stage(q, [EX]); q.click("text=Carica dati"); ready(q); q.wait_for_function("typeof E !== 'undefined' && E.files.length>=1", timeout=60000); q.wait_for_timeout(800)     # the session lives in the browser: a new context opens the file itself
                assert q.evaluate("document.documentElement.scrollWidth - document.documentElement.clientWidth") <= 1, "the page does not scroll sideways"
                vis = q.evaluate("[...document.querySelectorAll('#stbar > button[data-cmd],#stbar [data-cmd-main]')].filter(b=>!b.hidden&&b.offsetParent).map(b=>b.dataset.cmd||b.dataset.cmdMain)")
                assert vis == ["file", "more", "search"], vis
                q.click("#tb-more"); q.wait_for_selector("#stmenu")
                t = q.inner_text("#stmenu"); assert "Calcolatrice m/z" in t and "Perdite neutre" in t and "Tavola periodica" in t and "PubChem" in t, t
                q.locator("#stmenu button", has_text="Calcolatrice m/z").click(); q.wait_for_timeout(500)
                assert q.is_visible("#calcdlg") and q.evaluate("document.querySelector('#dfiles').getBoundingClientRect().width") <= 390 + 1
                q.evaluate("BARRA.close(false)")
                assert q.evaluate("document.documentElement.scrollWidth - document.documentElement.clientWidth") <= 1
            finally:
                ctx.close()
        step("phone (?telefono=0, 390 px): File, «Altro» and the search only, everything else under «Altro», page does not break", telefono)

        # ------------------------------------------------------------------ command palette and keyboard
        pg.set_viewport_size({"width": 1500, "height": 1000}); pg.wait_for_timeout(300)
        def paletta():
            pg.evaluate("document.activeElement.blur()"); pg.keyboard.press("Control+k"); pg.wait_for_selector("#paletta:not([hidden])")
            assert pg.get_attribute("#paletta", "role") == "dialog" and pg.get_attribute("#pal-in", "role") == "combobox" and pg.get_attribute("#pal-l", "role") == "listbox"
            assert pg.evaluate("document.activeElement.id") == "pal-in"
            pg.keyboard.type("isotop"); pg.wait_for_timeout(200)
            t = pg.inner_text("#pal-l"); assert "Simulazione isotopica" in t, t
            pg.keyboard.press("Control+a"); pg.keyboard.type("CALCOL"); pg.wait_for_timeout(200)
            assert pg.inner_text("#pal-l #pal-o-0").startswith("Calcolatrice") or "Calcolatrice m/z" in pg.inner_text("#pal-o-0"), pg.inner_text("#pal-l")
            pg.keyboard.press("Control+a"); pg.keyboard.type("perdita"); pg.wait_for_timeout(200); assert "Perdite neutre" in pg.inner_text("#pal-l")       # singular finds the plural entry through its keywords
            pg.keyboard.press("Control+a"); pg.keyboard.type("zzzzqq"); pg.wait_for_timeout(200); assert "Nessun risultato" in pg.inner_text("#paletta")
            pg.keyboard.press("Control+a"); pg.keyboard.type("metodo"); pg.wait_for_timeout(200); assert "Metodo di acquisizione" in pg.inner_text("#pal-l")
            pg.keyboard.press("Control+a"); pg.keyboard.type("pannello"); pg.wait_for_timeout(200)
            assert pg.locator("#pal-l li[role=option]").count() >= 3, "several matches: the side of the panel, pin"
            n0 = pg.evaluate("document.querySelector('#pal-in').getAttribute('aria-activedescendant')"); pg.keyboard.press("ArrowDown")
            assert pg.evaluate("document.querySelector('#pal-in').getAttribute('aria-activedescendant')") != n0 and pg.evaluate("document.querySelectorAll('#pal-l [aria-selected=true]').length") == 1
            pg.keyboard.press("Escape"); pg.wait_for_timeout(200); assert not pg.is_visible("#paletta")
            pg.click("#tb-search"); pg.wait_for_selector("#paletta:not([hidden])"); pg.keyboard.type("calcol"); pg.keyboard.press("Enter"); pg.wait_for_timeout(500)
            assert not pg.is_visible("#paletta") and pg.evaluate("BARRA.curTab") == "calc" and pg.is_visible("#calcdlg"), "Enter runs the command"
            pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
            if pg.is_visible("#dfiles"): pg.click("#sb-x")
        step("palette: Ctrl+K or 🔍, finds «isotop» and «calcol» without accents, partial words, ↑↓ Enter Esc, dialog/combobox/listbox", paletta)

        def paletta_hr():
            pg.keyboard.press("Control+k"); pg.keyboard.type("isotop"); pg.wait_for_timeout(200)
            assert "Simulazione isotopica" in pg.inner_text("#pal-l") and "Alta risoluzione" in pg.inner_text("#pal-l")
            pg.keyboard.press("Enter"); pg.wait_for_timeout(700)
            assert pg.evaluate("BARRA.curTab") == "iso" and pg.is_visible("#hrinfo")
            pg.click("#sb-x"); pg.keyboard.press("Control+k"); pg.keyboard.type("sinistra"); pg.keyboard.press("Enter"); pg.wait_for_timeout(300)
            assert pg.evaluate("BARRA.side") == "left"
            pg.keyboard.press("Control+k"); pg.keyboard.type("destra"); pg.keyboard.press("Enter"); pg.wait_for_timeout(300); assert pg.evaluate("BARRA.side") == "right"
            pg.evaluate("BARRA.setSide('left')")
        step("palette in HR: the HR items are there («Pannello a sinistra / a destra» too), Enter opens the panel", paletta_hr)

        def tastiera():
            pg.evaluate("BARRA.close(false)")
            pg.evaluate("document.activeElement.blur()"); pg.keyboard.press("Alt+0"); pg.wait_for_timeout(100)
            assert pg.evaluate("!!document.activeElement.closest('#stbar')"), "Alt+0 reaches the bar"
            a = pg.evaluate("document.activeElement.dataset.cmd||document.activeElement.dataset.cmdMain")
            pg.keyboard.press("ArrowRight"); b = pg.evaluate("document.activeElement.dataset.cmd||document.activeElement.dataset.cmdMain||document.activeElement.dataset.cmdCar"); assert a != b
            pg.keyboard.press("End"); assert pg.evaluate("document.activeElement.dataset.cmd") == "search"
            pg.keyboard.press("Home"); assert pg.evaluate("document.activeElement.dataset.cmd") == "file"
            assert pg.evaluate("document.querySelectorAll('#stbar .tb[tabindex=\"0\"]').length") == 1, "roving tabindex"
            pg.keyboard.press("Escape"); assert pg.evaluate("!document.activeElement.closest('#stbar')"), "Esc goes back where the focus was"
            pg.keyboard.press("F6"); assert pg.evaluate("!!document.activeElement.closest('#stbar')"), "F6 reaches the bar too"
            pg.keyboard.press("Escape")
            # the arrows of the chromatograms are not taken by the bar when the focus is on a graph
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'))")
            box = pg.locator(".pnl.chrom canvas").first.bounding_box(); pg.mouse.click(box["x"] + box["width"] * 0.4, box["y"] + box["height"] * 0.5); pg.wait_for_timeout(500)
            c0 = pg.evaluate("E.active.cur"); pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(300)
            assert pg.evaluate("E.active.cur") != c0 and not pg.evaluate("!!document.activeElement.closest('#stbar')"), "← → move the cursor of the chromatogram"
        step("keyboard: Alt+0 / F6 reach the bar, arrows Home End inside it (roving tabindex), Esc goes back; arrows of the chromatograms unchanged", tastiera)

        r.close()
except Exception as e:
    steps.append(("setup", "FAIL " + str(e)[:400]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for n, s in steps) else 0)
