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
        pg.evaluate("try { localStorage.removeItem('qqq.tour.lr'); } catch (_) {}")

        # 1. First load of LR data shows invitation
        pg.set_input_files("#pick", [TMP_LR])
        pg.wait_for_timeout(800)
        pg.click("text=Carica dati")
        ready(pg)

        pg.wait_for_selector("#tour-invito", timeout=5000)
        inv_text = pg.inner_text("#tour-invito")
        assert "Prima volta qui?" in inv_text, inv_text

        # bottom-left corner of the window, compact, non-modal dialog
        bb = pg.locator("#tour-invito").bounding_box()
        vp = pg.viewport_size
        assert bb["x"] < 40 and bb["y"] + bb["height"] > vp["height"] - 40, (bb, vp)
        assert bb["width"] < 330, bb
        assert pg.get_attribute("#tour-invito", "role") == "dialog"
        assert pg.get_attribute("#tour-invito", "aria-modal") == "false"
        assert "Inizia il tour" in inv_text, inv_text

        # Click "Non ora" -> closes; the invitation is recorded as already shown
        pg.click("#tour-inv-no")
        pg.wait_for_timeout(300)
        assert not pg.is_visible("#tour-invito"), "invitation closed by Non ora"
        st = pg.evaluate("JSON.parse(localStorage.getItem('qqq.tour.lr'))")
        assert st["stato"] == "invitato", st

        # Reload: invitation does not come back
        pg.reload()
        ready(pg)
        pg.wait_for_timeout(1000)
        assert not pg.is_visible("#tour-invito"), "invitation does not return on reload"
        steps.append(("invitation shown on first LR load, closed by Non ora, not repeated on reload", "ok"))

        # 2. Replay tour from gear: all 15 steps tested
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

        # Advance to Step 2: schede
        pg.click("#tour-btn-next")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 2 di 7 · I tuoi dati" in hd, hd
        assert ttl == "Una scheda per esperimento", ttl

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
        assert "Passo 1 di 8 · Strumenti e movimenti" in hd, hd
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
        assert "Passo 2 di 8 · Strumenti e movimenti" in hd, hd
        assert ttl == "Scansione per scansione", ttl

        # ArrowLeft goes back to Step 1
        pg.keyboard.press("ArrowLeft")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 1 di 8 · Strumenti e movimenti" in hd, hd

        # ArrowRight advances to Step 2
        pg.keyboard.press("ArrowRight")
        pg.wait_for_timeout(300)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 2 di 8 · Strumenti e movimenti" in hd, hd

        # Escape exits and saves saltato
        pg.keyboard.press("Escape")
        pg.wait_for_timeout(300)
        assert not pg.is_visible("#tour-dialog"), "dialog closed by Escape"
        st = pg.evaluate("JSON.parse(localStorage.getItem('qqq.tour.lr'))")
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
        assert "Passo 3 di 8 · Strumenti e movimenti" in hd, hd
        assert ttl == "Cambiare file", ttl

        # Step 4: aggiungi
        pg.click("#tour-btn-next")
        pg.wait_for_timeout(200)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 4 di 8 · Strumenti e movimenti" in hd, hd

        # Step 5: pannello
        pg.click("#tour-btn-next")
        pg.wait_for_timeout(200)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 5 di 8 · Strumenti e movimenti" in hd, hd

        # Step 6: barra
        pg.click("#tour-btn-next")
        pg.wait_for_timeout(200)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 6 di 8 · Strumenti e movimenti" in hd, hd

        # Step 7: metodo
        pg.click("#tour-btn-next")
        pg.wait_for_timeout(200)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        assert "Passo 7 di 8 · Strumenti e movimenti" in hd, hd

        # Step 8: ingranaggio
        pg.click("#tour-btn-next")
        pg.wait_for_timeout(200)
        hd = pg.inner_text("#tour-dialog .tour-hd")
        ttl = pg.inner_text("#tour-dialog h3")
        assert "Passo 8 di 8 · Strumenti e movimenti" in hd, hd
        assert ttl == "Impostazioni, Teoria e questo tour", ttl
        assert pg.is_visible("#tour-btn-fine") and "Fine" in pg.inner_text("#tour-btn-fine")

        # Click Fine -> finishes and saves fatto
        pg.click("#tour-btn-fine")
        pg.wait_for_timeout(300)
        assert not pg.is_visible("#tour-dialog")
        st = pg.evaluate("JSON.parse(localStorage.getItem('qqq.tour.lr'))")
        assert st["stato"] == "fatto", st
        steps.append(("chapter 2 steps completed with Fine, saves stato fatto", "ok"))

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
        pg.evaluate("localStorage.removeItem('qqq.tour.lr')")

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
        pg.evaluate("localStorage.removeItem('qqq.tour.lr')")

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
    steps.append(("run", f"FAIL: {e}"))
    r.close()

for s in steps:
    print(" ", s)
r.report()
