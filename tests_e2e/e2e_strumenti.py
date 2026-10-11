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
    cut: btns.filter(b => [...b.querySelectorAll('.tl')].some(l => l.offsetParent && l.scrollWidth > l.clientWidth + 1)).map(b => b.dataset.cmd), n: btns.length, lab: bar.classList.contains('tb-lab'), wrapped: btns.filter(b => b.getBoundingClientRect().height > 48).length};
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
            assert ids == ["file", "calc", "losses", "adducts", "lists", "ptable", "more"], ids
            lab = pg.evaluate("[...document.querySelectorAll('#stbar > button[data-cmd]')].filter(b=>!b.hidden).map(b=>b.innerText.trim())")
            assert lab == ["File", "Calcolatrice", "Perdite", "Addotti", "Liste", "Tavola", "Altro"], lab
            for b in pg.query_selector_all("#stbar > button[data-cmd]"):
                if b.is_visible(): assert b.get_attribute("aria-label") and b.get_attribute("title") and b.query_selector("svg"), b.get_attribute("data-cmd")
            assert pg.evaluate("document.querySelectorAll('header button#np-pc, header button#np-bt').length") == 0, "PubChem and BioTransformer only in «Altro»"
            assert not pg.is_visible("#dfiles"), "the drawer is closed until a tool is chosen"
        step("barra in the header (role toolbar), file/calc/losses/adducts/lists/table in this order, words under the icons, no rail, drawer closed", barra)

        def altro():
            pg.click("#tb-more"); pg.wait_for_selector("#stmenu")
            t = pg.inner_text("#stmenu"); assert "PubChem" in t and "BioTransformer" in t, t
            assert pg.get_attribute("#stmenu", "role") == "menu" and pg.get_attribute("#tb-more", "aria-expanded") == "true"
            pg.keyboard.press("Escape"); assert pg.locator("#stmenu").count() == 0 and pg.evaluate("document.activeElement.id") == "tb-more"
        step("«Altro» menu: PubChem and BioTransformer, Esc closes and gives the focus back", altro)

        def apri_chiudi():
            w0 = pg.evaluate("E.panels[0].w")
            pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(300)
            assert pg.is_visible("#dfiles") and pg.is_visible("#flst"), "file list in the drawer"
            assert pg.get_attribute("#stbar [data-cmd=file]", "aria-pressed") == "true" and "on" in pg.get_attribute("#stbar [data-cmd=file]", "class")
            assert pg.evaluate("E.panels[0].w") == w0, "the drawer floats over the charts: it does not narrow them"
            assert pg.evaluate("document.querySelector('#dfiles').contains(document.activeElement)"), "focus moves into the panel"
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
            assert not pg.is_visible("#dfiles") and pg.get_attribute("#stbar [data-cmd=file]", "aria-pressed") == "false"
            assert pg.evaluate("document.activeElement.dataset.cmd") == "file", "Esc gives the focus back to the button"
            pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(300); assert pg.is_visible("#dfiles")
            pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(300); assert not pg.is_visible("#dfiles"), "the same button closes it"
            pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(300)
            pg.mouse.click(700, 600); pg.wait_for_timeout(300); assert not pg.is_visible("#dfiles"), "click outside closes it"
            pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(300); pg.click("#sb-x"); pg.wait_for_timeout(300); assert not pg.is_visible("#dfiles")
        step("drawer: opens over the charts, Esc / click outside / same button / × close it, focus in and back", apri_chiudi)

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
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300); assert not pg.is_visible("#dfiles")
        step("one tool at a time in the drawer; the periodic table stays the window #refdlg", cambia)

        def fissa():
            pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(300)
            w0 = pg.evaluate("Q('#dpanels').clientWidth")
            pg.click("#sb-pin"); pg.wait_for_timeout(500)
            assert pg.get_attribute("#sb-pin", "aria-pressed") == "true"
            w1 = pg.evaluate("Q('#dpanels').clientWidth"); assert w1 < w0 - 150, (w0, w1)
            assert pg.evaluate("E.panels.filter(p=>p.full).every(p=>Math.abs(p.w - Q('#dpanels').clientWidth) <= 2)"), "the charts follow the narrower page"
            assert pg.evaluate("getComputedStyle(document.querySelector('#dfiles')).position") == "sticky"
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300); assert pg.is_visible("#dfiles"), "pinned: Esc does not close it"
            pg.mouse.click(900, 500); pg.wait_for_timeout(300); assert pg.is_visible("#dfiles"), "pinned: click outside does not close it"
            pg.click("#sb-pin"); pg.wait_for_timeout(500); assert pg.evaluate("Q('#dpanels').clientWidth") == w0
            pg.click("#sb-x"); pg.wait_for_timeout(300)
        step("pin: the panel goes next to the charts (narrows the grid), unpin gives the page back", fissa)

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
            assert pg.evaluate("localStorage.getItem('qqq.barra.lato')") == "right"
            pg.keyboard.press("Alt+Shift+ArrowLeft"); pg.wait_for_timeout(300)
            assert pg.evaluate("BARRA.side") == "left" and pg.evaluate("document.querySelector('#dfiles').getBoundingClientRect().left") < 20
            pg.keyboard.press("Alt+Shift+ArrowRight"); pg.wait_for_timeout(200); assert pg.evaluate("BARRA.side") == "right"
            pg.click("#sb-pin"); pg.wait_for_timeout(400)
            w_dp = pg.evaluate("Q('#dpanels').getBoundingClientRect().left")
            b = pg.locator("#sb-title").bounding_box(); x0, y0 = b["x"] + 4, b["y"] + b["height"] / 2
            pg.mouse.move(x0, y0); pg.mouse.down(); pg.mouse.move(100, 300, steps=6); pg.mouse.up(); pg.wait_for_timeout(500)
            assert pg.evaluate("BARRA.side") == "left"
            assert pg.evaluate("Q('#dpanels').getBoundingClientRect().left") > w_dp + 150, "the charts move to the free side"
            assert pg.evaluate("E.panels.filter(p=>p.full).every(p=>Math.abs(p.w - Q('#dpanels').clientWidth) <= 2)")
            pg.click("#sb-pin"); pg.click("#sb-x"); pg.wait_for_timeout(300)
        step("side by drag and drop of the header (drop zones, preview, charts move) and Alt+Shift+arrows; remembered in qqq.barra.lato", lato)

        def ricorda():
            pg.click("#stbar [data-cmd=adducts]"); pg.wait_for_timeout(300); pg.click("#sb-pin"); pg.wait_for_timeout(300)
            pg.evaluate("BARRA.setWidth(320)"); pg.evaluate("BARRA.setSide('right')")
            st = pg.evaluate("JSON.parse(localStorage.getItem('qqq.barra.lr'))"); assert st["pinned"] and st["w"] == 320 and st["tab"] == "adducts" and st["open"], st
            pg.reload(); ready(pg); pg.wait_for_timeout(500)
            assert pg.is_visible("#dfiles") and pg.evaluate("BARRA.pinned && BARRA.side === 'right' && BARRA.width === 320 && BARRA.curTab === 'adducts'")
            pg.click("#sb-pin"); pg.click("#sb-x"); pg.evaluate("BARRA.setSide('left')"); pg.reload(); ready(pg); pg.wait_for_timeout(400)
            assert not pg.is_visible("#dfiles"), "unpinned: after a reload the drawer is closed"
            pg.evaluate("localStorage.removeItem('qqq.barra.lato'); localStorage.setItem('qqq.barra.lr', JSON.stringify({tab:'calc', side:'right', pinned:true, w:300}))"); pg.reload(); ready(pg); pg.wait_for_timeout(400)
            assert not pg.is_visible("#dfiles") and pg.evaluate("BARRA.side") == "right", "old state (no «open»): closed, and its side is respected"
            pg.evaluate("localStorage.removeItem('qqq.barra.lr'); localStorage.removeItem('qqq.barra.lato')"); pg.reload(); ready(pg)
        step("state remembered after reload (pinned, side, width, tool); an old state without «open» opens closed", ricorda)

        def larghezze():
            for w, h in ((1280, 800), (1440, 900), (1680, 1000), (1024, 1366), (1366, 1024)):
                pg.set_viewport_size({"width": w, "height": h}); pg.wait_for_timeout(500)
                f = pg.evaluate(FIT)
                assert f["rows"] <= 6 and f["hh"] < 70, (w, f)
                assert not f["clipped"] and not f["cut"] and f["wrapped"] == 0, (w, f)
                assert f["lab"] == (w >= 1440), (w, f)
            pg.set_viewport_size({"width": 1500, "height": 1000})
        step("no button wrapped, cut or clipped at 1280/1440/1680 and iPad 1024×1366 / 1366×1024; words only from 1440", larghezze)

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
            assert ids == ["file", "calc", "losses", "adducts", "lists", "ptable", "scan", "form", "idn", "more"], ids
            lab = pg.evaluate("[...document.querySelectorAll('#stbar button.tb')].filter(b=>!b.hidden&&b.offsetParent).map(b=>b.innerText.trim()).filter(Boolean)")
            assert lab[-4:] == ["Scansione", "Formule", "Librerie", "Altro"], lab
            assert "·" not in " ".join(lab) and not any(x.isupper() and len(x) == 3 for x in lab), lab
            assert pg.evaluate("document.querySelector('#stbar [data-hrgrp]').offsetParent !== null"), "separator before the HR group"
            for w in (1280, 1440, 1680):
                pg.set_viewport_size({"width": w, "height": 900}); pg.wait_for_timeout(500)
                f = pg.evaluate(FIT); assert f["rows"] <= 6 and not f["clipped"] and not f["cut"] and f["wrapped"] == 0, (w, f)
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
            pg.click("#stbar [data-cmd=adducts]"); pg.click("#sb-pin"); pg.evaluate("BARRA.setWidth(400)")
            hr = pg.evaluate("JSON.parse(localStorage.getItem('qqq.barra.hr'))"); lr = pg.evaluate("JSON.parse(localStorage.getItem('qqq.barra.lr')||'null')")
            assert hr["pinned"] and hr["w"] == 400 and hr["tab"] == "adducts" and hr["last"]["scan"] == "lst" and hr["last"]["form"] == "iso", hr
            assert lr is None or lr.get("w") != 400, "LR and HR keep separate states"
            pg.click("#sb-pin"); pg.click("#sb-x")
        step("HR state in qqq.barra.hr (pinned, width, tool, last item of each menu), separate from LR", hr_stato)

        r.close()
except Exception as e:
    steps.append(("setup", "FAIL " + str(e)[:400]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for n, s in steps) else 0)
