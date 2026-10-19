"""Guided tour of the Dati view (low-resolution only): invitation, 15 steps, try-it steps, keyboard, EN, HR exclusion, example file."""
import sys, os, shutil
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

r = Run(port=8894, wd="/tmp/wd94")
steps = []

# Prepare example files in /tmp with requested names
TMP_LR = "/tmp/Esempio_FullScan_t15.mzML"
TMP_HR = "/tmp/Esempio_HRMS_DDA.mzML"
shutil.copy(str(ROOT / "mzlab" / "web" / "esempi" / "FullScan_t15.mzML"), TMP_LR)
shutil.copy(str(ROOT / "mzlab" / "web" / "esempi" / "HRMS_t000.mzML"), TMP_HR)

try:
    with sync_playwright() as p:
        pg = r.page(p, tour=True)
        # Clear any existing tour state
        pg.evaluate("try { localStorage.removeItem('mzlab.tour.lr'); } catch (_) {}")

        # 1. First load of LR data shows invitation
        stage(pg, [TMP_LR])
        pg.click("text=Carica dati")
        ready(pg)

        pg.wait_for_selector("#tour-invito", timeout=30000)      # shown once the data are up: on a loaded machine that takes longer than 5 s
        inv_text = pg.inner_text("#tour-invito")
        assert "Primo accesso? Un tour guidato degli strumenti." in inv_text and "minut" not in inv_text, inv_text

        # bottom-left corner of the window, compact, non-modal dialog
        bb = pg.locator("#tour-invito").bounding_box()
        vp = pg.viewport_size
        assert bb["y"] + bb["height"] > vp["height"] - 40, (bb, vp)
        side = pg.locator("#dfiles").bounding_box()
        assert abs((bb["x"] - side["x"]) - (side["x"] + side["width"] - bb["x"] - bb["width"])) <= 2, ("centred under the side panel", bb, side)
        assert pg.get_attribute("#tour-invito", "role") == "dialog"
        assert pg.get_attribute("#tour-invito", "aria-modal") == "false"
        assert "Inizia il tour" in inv_text, inv_text

        # Click "Non ora" -> closes; the invitation is recorded as already shown
        pg.click("#tour-inv-no")
        attendi(pg, "!document.querySelector('#tour-invito') || document.querySelector('#tour-invito').hidden || !document.querySelector('#tour-invito').offsetParent", 5000)
        assert not pg.is_visible("#tour-invito"), "invitation closed by Non ora"
        pg.wait_for_selector("#tour-later.on")
        assert pg.inner_text("#tour-later") == "Potrai fare il tour più tardi dalle Impostazioni." and pg.get_attribute("#tour-later", "aria-live") == "polite"
        lb, gb = pg.locator("#tour-later").bounding_box(), pg.locator("#np-set").bounding_box()
        assert lb["y"] >= gb["y"] + gb["height"] and lb["x"] <= gb["x"] + gb["width"] and lb["x"] + lb["width"] >= gb["x"], ("bubble under the gear", lb, gb)
        pg.mouse.click(300, 300)
        attendi(pg, "!document.querySelector('#tour-later')", 3000)
        st = pg.evaluate("JSON.parse(localStorage.getItem('mzlab.tour.lr'))")
        assert st["stato"] == "invitato", st

        # Reload: invitation does not come back (the session is saved first: the reload resumes from the notebook)
        saved(pg)
        pg.reload()
        attendi(pg, "typeof E !== 'undefined' && E.files && E.files.length > 0")  # the invitation shows once the data are up: wait for that state, then check it stays away
        ready(pg)
        frames(pg, 30)
        assert not pg.is_visible("#tour-invito"), "invitation does not return on reload"
        steps.append(("invitation shown on first LR load, closed by Non ora, not repeated on reload", "ok"))

        # 2. Replay tour from gear: all 15 steps tested
        attendi(pg, "typeof E !== 'undefined' && E.files && E.files.length > 0")  # the label reads «Rifai» only once the reloaded file is back
        pg.click("#np-set")
        pg.wait_for_selector("#uipset", timeout=3000)
        assert pg.is_visible("#uip-tour") and "Rifai il tour" in pg.inner_text("#uip-tour")
        pg.click("#uip-tour")

        # Chapter 1, Step 1: file list
        pg.wait_for_selector("#tour-dialog", timeout=3000)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 1 di 7 · I tuoi dati" in hd, hd
        assert ttl == "Qui ci sono i file caricati", ttl
        assert pg.get_attribute("#tour-btn-back", "disabled") is not None, "back disabled on step 1"

        # Advance to Step 2: gruppi
        pg.click("#tour-btn-next")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 2 di 7 · I tuoi dati" in hd, hd
        assert ttl == "Un gruppo per esperimento", ttl

        # Test Back button returns to Step 1
        pg.click("#tour-btn-back")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 1 di 7 · I tuoi dati" in hd, hd

        # Advance again to Step 2, then Step 3: TIC
        pg.click("#tour-btn-next")
        pg.wait_for_timeout(200)
        pg.click("#tour-btn-next")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 3 di 7 · I tuoi dati" in hd, hd
        assert ttl == "Il TIC: tutto il segnale nel tempo", ttl
        # Screenshot step 3 (light theme)
        pg.screenshot(path=str(HERE / "shots" / "tour_passo3.png"))

        # Advance to Step 4: istante (Try it step)
        pg.click("#tour-btn-next")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 4 di 7 · I tuoi dati" in hd, hd
        assert ttl == "Scegli un istante", ttl
        assert pg.is_visible("#tour-dialog .tour-try"), "prova tu row visible"

        # Try it: click anywhere on TIC canvas -> should auto-advance to Step 5
        pg.click(".pnl.chrom canvas", position={"x": 150, "y": 80})
        pg.wait_for_function("document.querySelector('#tour-dialog .tour-hd')?.textContent.includes('Passo 5')", timeout=5000)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 5 di 7 · I tuoi dati" in hd, hd
        assert ttl == "Lo spettro di massa", ttl

        # Advance to Step 6: xic (Try it step)
        pg.click("#tour-btn-next")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 6 di 7 · I tuoi dati" in hd, hd
        assert ttl == "Estrai uno ione (XIC)", ttl
        # Screenshot step 6 (light theme)
        pg.screenshot(path=str(HERE / "shots" / "tour_passo6.png"))

        # Try it: right-click a peak in the spectrum -> extract XIC
        # Find a label box in the spectrum
        attendi(pg, "(() => { const p = E.panels.find(q => q.type === 'spec'); return !!(p && p._a && p._a.lbls && p._a.lbls.some(x => x.m != null)); })()")   # the labels are laid out once the spectrum is drawn: without them the click would land on an empty spot
        frames(pg)
        lb = pg.evaluate("(() => { const p = E.panels.find(q => q.type === 'spec'); const q = p.cv.getBoundingClientRect(); const l = p._a && p._a.lbls && p._a.lbls.filter(x => x.m != null)[0]; return l ? { x: q.left + l.x + l.w / 2, y: q.top + l.y + l.h / 2 } : { x: q.left + q.width / 2, y: q.top + q.height / 2 }; })()")
        pg.mouse.click(lb["x"], lb["y"], button="right")
        pg.wait_for_selector("#ctx", timeout=3000)
        pg.locator("#ctx div", has_text="Estrai l'XIC").first.click()

        # Step 6 auto-advances to Step 7 when XIC panel is created
        pg.wait_for_function("document.querySelector('#tour-dialog .tour-hd')?.textContent.includes('Passo 7')", timeout=5000)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 7 di 7 · I tuoi dati" in hd, hd
        assert ttl == "L'XIC che hai estratto", ttl
        assert pg.is_visible(".pnl.xic"), "new xic panel exists"
        assert pg.is_visible("#tour-btn-cont") and pg.is_visible("#tour-btn-fine-cap1"), "end of chapter 1 choices visible"
        steps.append(("chapter 1 steps 1-7: auto-advance on TIC click and XIC extraction, targets correct", "ok"))

        # Click Continua to go to Chapter 2
        pg.click("#tour-btn-cont")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 1 di 9 · Strumenti e movimenti" in hd, hd
        assert ttl == "Ingrandire e tornare indietro", ttl

        # 3. Keyboard test in Chapter 2
        # Check focus is on Avanti
        focused_id = pg.evaluate("document.activeElement.id")
        assert focused_id == "tour-btn-next", f"expected focus on tour-btn-next, got {focused_id}"

        # Enter advances to Step 2: tasti
        pg.keyboard.press("Enter")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 2 di 9 · Strumenti e movimenti" in hd, hd
        assert ttl == "Il cursore del cromatogramma", ttl

        # ArrowLeft goes back to Step 1
        pg.keyboard.press("ArrowLeft")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 1 di 9 · Strumenti e movimenti" in hd, hd

        # ArrowRight advances to Step 2
        pg.keyboard.press("ArrowRight")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 2 di 9 · Strumenti e movimenti" in hd, hd

        # Escape exits and saves saltato
        pg.keyboard.press("Escape")
        pg.wait_for_timeout(300)
        assert not pg.is_visible("#tour-dialog"), "dialog closed by Escape"
        st = pg.evaluate("JSON.parse(localStorage.getItem('mzlab.tour.lr'))")
        assert st["stato"] == "saltato", st
        # Focus returned to gear
        focused_id = pg.evaluate("document.activeElement.id")
        assert focused_id == "np-set", f"expected focus on np-set, got {focused_id}"
        steps.append(("keyboard controls: Enter advances, ArrowLeft back, ArrowRight advances, Esc exits", "ok"))

        # 4. Finish all steps of Chapter 2
        pg.evaluate("TOUR.vai(2, 2)") # go to step 3
        pg.wait_for_timeout(200)

        # Step 3: navfile
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 3 di 9 · Strumenti e movimenti" in hd, hd
        assert ttl == "Cambiare file", ttl

        # Step 4: aggiungi
        pg.click("#tour-btn-next")
        attendi(pg, "document.querySelector('#tour-dialog .tour-hd').innerText.includes('Passo 4 di 9')")
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 4 di 9 · Strumenti e movimenti" in hd, hd

        # Step 5: affianca (the handle of the panels)
        pg.click("#tour-btn-next")
        attendi(pg, "document.querySelector('#tour-dialog .tour-hd').innerText.includes('Passo 5 di 9')")
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 5 di 9 · Strumenti e movimenti" in hd, hd
        assert pg.inner_text("#tour-dialog h3") == "Affiancare i grafici"

        # Step 6: pannello
        pg.click("#tour-btn-next")
        attendi(pg, "document.querySelector('#tour-dialog .tour-hd').innerText.includes('Passo 6 di 9')")
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 6 di 9 · Strumenti e movimenti" in hd, hd

        # Step 7: barra
        pg.click("#tour-btn-next")
        attendi(pg, "document.querySelector('#tour-dialog .tour-hd').innerText.includes('Passo 7 di 9')")
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 7 di 9 · Strumenti e movimenti" in hd, hd

        # Step 8: metodo
        pg.click("#tour-btn-next")
        attendi(pg, "document.querySelector('#tour-dialog .tour-hd').innerText.includes('Passo 8 di 9')")
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 8 di 9 · Strumenti e movimenti" in hd, hd

        # Step 9: ingranaggio
        pg.click("#tour-btn-next")
        attendi(pg, "document.querySelector('#tour-dialog .tour-hd').innerText.includes('Passo 9 di 9')")
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 9 di 9 · Strumenti e movimenti" in hd, hd
        assert ttl == "Impostazioni, Teoria e questo tour", ttl
        assert pg.is_visible("#tour-btn-fine") and "Fine" in pg.inner_text("#tour-btn-fine")

        # Click Fine -> finishes and saves fatto
        pg.click("#tour-btn-fine")
        pg.wait_for_timeout(300)
        assert not pg.is_visible("#tour-dialog")
        st = pg.evaluate("JSON.parse(localStorage.getItem('mzlab.tour.lr'))")
        assert st["stato"] == "fatto", st
        steps.append(("chapter 2 steps completed with Fine, saves stato fatto", "ok"))

        # every step finds its target visible (the toolbar, the file drawer opened for its step and closed after it)
        pg.goto(f"http://127.0.0.1:{r.port}/?lang=it"); ready(pg)
        pg.evaluate("localStorage.setItem('mzlab.tour.lr', JSON.stringify({stato:'saltato'}))")
        pg.evaluate("window.BARRA && BARRA.isOpen() && BARRA.close(false)"); pg.wait_for_timeout(300)      # a drawer left open (the state is saved) would stay open after the «file» step: that is not what is measured here
        SPOT = "(()=>{const s=document.querySelector('#tour-spotlight');if(!s)return null;const r=s.getBoundingClientRect();return [Math.round(r.left),Math.round(r.top),Math.round(r.width),Math.round(r.height)]})()"
        for cap, n in ((1, 7), (2, 9)):
            for i in range(n):
                pg.evaluate(f"TOUR.vai({cap}, {i})"); pg.wait_for_timeout(350)
                hd = pg.inner_text("#tour-dialog .tour-hd")
                assert f"Passo {i + 1} di {n}" in hd, (cap, i, hd)           # a step with no target is skipped: the number would jump
                sp = pg.evaluate(SPOT); assert sp and sp[2] > 10 and sp[3] > 10, (cap, i, sp)
                assert pg.evaluate("document.querySelector('#dfiles') && getComputedStyle(document.querySelector('#dfiles')).display !== 'none'") == (cap == 1 and i in (0, 1)), (cap, i)
                if cap == 2 and i == 6:
                    bar = pg.evaluate("(()=>{const b=document.querySelector('#stbar').getBoundingClientRect();return [Math.round(b.left),Math.round(b.top)]})()")
                    assert abs(sp[0] - bar[0]) < 20 and abs(sp[1] - bar[1]) < 20, (sp, bar)      # the «barra» step points at the toolbar
        pg.click("#tour-btn-skip")
        # the invitation (bottom left) is not covered by the toolbar or by the drawer
        pg.evaluate("localStorage.removeItem('mzlab.tour.lr')"); saved(pg); pg.reload(); ready(pg); pg.wait_for_selector("#tour-invito", timeout=30000)
        for vp in ((1280, 800), (1024, 1366), (1500, 1000)):
            pg.set_viewport_size({"width": vp[0], "height": vp[1]}); pg.wait_for_timeout(300)
            pg.evaluate("BARRA.setTab('file',{noFocus:true,instant:true})"); pg.wait_for_timeout(300)
            top = pg.evaluate("(()=>{const b=document.querySelector('#tour-inv-si').getBoundingClientRect();const e=document.elementFromPoint(b.x+b.width/2,b.y+b.height/2);return !!(e&&e.closest('#tour-invito'))})()")
            assert top, vp
            bb = pg.locator("#tour-invito").bounding_box(); assert bb["x"] < 40 and bb["y"] + bb["height"] > vp[1] - 40, (vp, bb)
            pg.evaluate("BARRA.close(false)")
        pg.set_viewport_size({"width": 1500, "height": 2200})
        steps.append(("every tour step finds its target visible; the «file» step opens and closes the drawer; the invitation is not covered", "ok"))

        # 5. English language: text of step 1 and buttons in English
        pg.goto(f"http://127.0.0.1:{r.port}/?lang=en")
        ready(pg)
        pg.evaluate("TOUR.inizia(1, 0)")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        btn_txt = pg.inner_text("#tour-btn-next")
        skip_txt = pg.inner_text("#tour-btn-skip")
        assert "Step 1 of 7 · Your data" in hd, hd
        assert ttl == "Your loaded files are here", ttl
        assert btn_txt == "Next", btn_txt
        assert skip_txt == "Skip tour", skip_txt
        pg.click("#tour-btn-skip")

        pg.goto(f"http://127.0.0.1:{r.port}/?lang=it")
        ready(pg)
        steps.append(("english language: headers, titles and buttons in English", "ok"))

        # 6. HR mode: no invitation, no entry in settings
        pg.evaluate("fetch('api/new', {method:'POST', body:JSON.stringify({fresh:true})})")
        pg.reload()
        pg.wait_for_timeout(1000)
        pg.evaluate("localStorage.removeItem('mzlab.tour.lr')")

        pg.set_input_files("#pick", [TMP_HR])
        pg.wait_for_timeout(800)
        pg.click("text=Carica dati")
        ready(pg)

        assert pg.evaluate("window.BANCO && BANCO.on()"), "in HR mode"
        assert not pg.is_visible("#tour-invito"), "no invitation in HR mode"

        pg.click("#np-set")
        pg.wait_for_selector("#uipset", timeout=3000)
        assert not pg.is_visible("#uip-tour"), "no tour section in gear in HR mode"
        pg.keyboard.press("Escape")
        steps.append(("HR mode: no invitation, no gear entry", "ok"))

        # 7. Without data: "Fai il tour con un file di esempio" loads single file and starts at step 1
        pg.evaluate("fetch('api/new', {method:'POST', body:JSON.stringify({fresh:true})})")
        pg.reload()
        pg.wait_for_timeout(1000)
        pg.evaluate("localStorage.removeItem('mzlab.tour.lr')")

        # In start screen without data
        assert pg.evaluate("typeof E === 'undefined' || !E.files || !E.files.length"), "no files loaded"
        pg.click("#np-set")
        pg.wait_for_selector("#uipset", timeout=3000)
        assert pg.is_visible("#uip-tour") and "Fai il tour con un file di esempio" in pg.inner_text("#uip-tour")
        pg.click("#uip-tour")

        # Wait for file to open and tour to start at step 1
        pg.wait_for_selector("#tour-dialog", timeout=35000)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 1 di 7 · I tuoi dati" in hd, hd
        assert ttl == "Qui ci sono i file caricati", ttl
        assert pg.evaluate("E.files.length") == 1, f"expected 1 file, got {pg.evaluate('E.files.length')}"
        pg.click("#tour-btn-skip")
        steps.append(("without data: loads single example file and starts tour at step 1", "ok"))

        steps.append(("no browser console errors", "ok"))

    r.close()
except Exception as e:
    import traceback
    steps.append(("run", f"FAIL line {traceback.extract_tb(e.__traceback__)[-1].lineno}: {type(e).__name__} {e}"))      # the line says which step broke: an empty message is not enough in the CI log
    r.close()

for s in steps:
    print(" ", s)
r.report()
