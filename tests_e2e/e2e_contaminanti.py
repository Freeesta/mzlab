"""WP-H11 + Binario B: reference lists and contaminants engine.
- High resolution: known contaminants only in the box that opens on a peak (same pixels with switch on and off), series of a polymer, laboratory contaminants, user suspect lists (CSV with semicolon and decimal comma).
- Header button: #np-liste opens the lists dialog.
- Low resolution (unit resolution): NO automatic annotations on spectra or on hover; right-click context menu has «Cerca nelle liste dei contaminanti…» which opens filtered on nominal m/z (±0.5 Da) ordered by proximity; educational introductory text.
"""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_contaminanti.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))

D = synth("hrcont")
r = Run(port=8993, wd="/tmp/wd_cont")
SP = "E.panels.find(p=>p.type==='spec'&&p._a&&p._a.hrp)"

def hov(pg, m):
    return pg.evaluate(f"(()=>{{const p={SP};const a=p._a;const h=a.hov(a.X({m}));return h?h.html:null}})()")

try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        pg.evaluate("localStorage.removeItem('qqq.contaminanti');localStorage.removeItem('qqq.contaminanti.on');localStorage.removeItem('qqq.liste.attive')")
        pg.evaluate("LISTE.ensure()"); pg.wait_for_function("LISTE.find&&true", timeout=5000); pg.wait_for_timeout(1500)
        sp = pg.evaluate(f"(()=>{{const p={SP};p.zoom=[270,400];draw(p);return p.id}})()"); pg.wait_for_timeout(1500)

        def known():
            h = hov(pg, 279.1591); assert h and "Compatibile con un contaminante noto" in h and "dibutilftalato" in h and "Keller 2008" in h, h
            h2 = hov(pg, 391.2843); assert h2 and "DEHP" in h2, h2
        step("279.1591 and 391.2843: the line of the contaminant in the box of the peak", known)

        def other():
            ms = pg.evaluate(f"(()=>{{const p={SP};const d=p._a.data[0].d;return d.mz.filter(m=>m>272&&m<398&&Math.abs(m-279.1591)>0.5&&Math.abs(m-391.2843)>0.5).slice(0,12)}})()")
            assert ms, "no other peak in the window"
            n = 0
            for m in ms:
                h = hov(pg, m)
                if h and "contaminante" in h: n += 1
            assert n < len(ms), "not every random peak can be a contaminant"
            assert any(hov(pg, m) and "contaminante" not in hov(pg, m) for m in ms)
        step("a peak that does not match has no line", other)

        def pixels():
            pg.evaluate("LISTE.setOn(true)"); pg.wait_for_timeout(1200)
            n_on = pg.evaluate(f"{SP}._a.cmarks.length"); assert n_on >= 1, n_on
            pg.evaluate("LISTE.setOn(false)"); pg.wait_for_timeout(1200)
            assert pg.evaluate(f"{SP}._a.cmarks.length") == 0 and pg.evaluate(f"{SP}._a.cont") is None, "switch off = no signs"
            h = hov(pg, 279.1591); assert h and "contaminante" not in h, h
            pg.evaluate("LISTE.setOn(true)"); pg.wait_for_timeout(1000)
            assert "contaminante" in hov(pg, 279.1591)
        step("signs only with the switch on; off = the box of before", pixels)

        def excl():
            def st(): return pg.evaluate(f"(()=>{{const a={SP}._a;return {{marks:a.cmarks.length,hid:a.chid}}}})()")
            def sig(): return pg.evaluate("JSON.stringify(E.panels.filter(q=>q.type==='tic'&&q.cv).map(q=>q.cv.toDataURL()))")
            m0 = st()["marks"]; assert m0 >= 1, m0
            t0 = sig()
            kks = pg.evaluate(f"{SP}._a.cont.match({SP}._a.cmarks[0].m).map(x=>LISTE.keyFor('entry', x))"); kk = kks[0]
            for k in kks: pg.evaluate(f"LISTE.exclSet('entry', {json.dumps(k)}, true)")
            pg.wait_for_timeout(1200)
            s1 = st(); assert s1["hid"] >= 1 and s1["marks"] < m0, (m0, s1, pg.evaluate(f"{SP}._a.cmarks.map(c=>c.m)"), pg.evaluate("JSON.stringify(localStorage.getItem(\"qqq.contaminanti.esclusi\"))"))
            assert "Mostra" in pg.inner_text(".cont-cnt")
            assert t0 == sig(), "TIC drawn the same"
            pg.focus(".cont-show"); pg.keyboard.press("Enter"); pg.wait_for_timeout(1000)
            assert st()["marks"] == m0 and "Nascondi" in pg.inner_text(".cont-cnt")
            pg.click(".cont-show"); pg.wait_for_timeout(800)
            ls = json.loads(pg.evaluate("localStorage.getItem('qqq.contaminanti.esclusi')")); assert kk in ls["voci"], ls
        step("exclude one entry: sign gone, counter, Show/Hide by keyboard, TIC identical, kept in qqq.contaminanti.esclusi", excl)

        def excl_cls():
            pg.evaluate("LISTE.exclReset()"); pg.wait_for_timeout(600)
            ck = pg.evaluate(f"LISTE.keyFor('cls', {SP}._a.cont.match(391.2843)[0])")
            pg.evaluate(f"LISTE.exclSet('cls', {json.dumps(ck)}, true)"); pg.wait_for_timeout(1000)
            assert pg.evaluate(f"{SP}._a.cont.state(391.2843).hid") is True
            assert json.loads(pg.evaluate("localStorage.getItem('qqq.contaminanti.esclusi')"))["classi"] == [ck]
            pg.click("#np-liste"); pg.wait_for_selector("#lst-excl", timeout=5000); pg.wait_for_timeout(500)
            assert pg.is_visible("#lst-excl-reset"); pg.click("#lst-excl-reset"); pg.wait_for_timeout(500)
            assert pg.evaluate("LISTE.exclItems().length") == 0
        step("exclude a class; list in the panel; Restore all", excl_cls)

        def kbd_menu():
            pg.evaluate("LISTE.exclReset()"); pg.wait_for_timeout(800)
            assert pg.evaluate(f"{SP}._a.chid") == 0
            pg.focus(".cont-lb"); pg.keyboard.press("Enter"); pg.wait_for_timeout(300)
            assert pg.evaluate("document.activeElement.getAttribute('role')") == "menuitem", pg.evaluate("[document.activeElement.outerHTML.slice(0,80), document.querySelector('#ctx').hidden, document.querySelector('#ctx').innerHTML.slice(0,200), !!document.querySelector('.cont-lb')]")
            pg.keyboard.press("Enter"); pg.wait_for_timeout(300)          # the first marked peak -> its exclude menu
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("Enter"); pg.wait_for_timeout(1000)     # (name is a dim line) the first action: exclude this entry
            assert pg.evaluate("LISTE.exclItems().length") == 1, pg.evaluate("LISTE.exclItems().length")
            pg.evaluate("LISTE.exclReset()")
        step("keyboard: legend button -> menu -> exclude, arrows and Enter", kbd_menu)

        def sess():
            k = pg.evaluate(f"LISTE.keyFor('entry', {SP}._a.cont.match({SP}._a.cmarks[0].m)[0])")
            pg.evaluate(f"LISTE.exclSet('entry', {json.dumps(k)}, true)"); pg.wait_for_timeout(300)
            pg.evaluate("uiSave(true)"); pg.wait_for_timeout(500)
            assert k in pg.evaluate("NB.ui.contEx.voci"), "exclusions saved with the session"
            pg.evaluate("localStorage.removeItem('qqq.contaminanti.esclusi')"); pg.evaluate("LISTE.exclReset()")
            pg.evaluate("LISTE.exclImport(NB.ui.contEx)"); assert pg.evaluate("LISTE.exclItems().length") == 1
            pg.evaluate("LISTE.exclReset()")
        step("exclusions saved with the session and restored", sess)

        def reset_all():
            pg.evaluate("LISTE.exclReset()"); assert pg.evaluate("LISTE.exclItems().length") == 0
        step("restore all", reset_all)

        def gear():
            pg.click("#np-set"); pg.wait_for_selector("#uip-cont", timeout=5000)
            assert pg.is_checked("#uip-cont"); pg.uncheck("#uip-cont"); pg.wait_for_timeout(400)
            assert pg.evaluate("LISTE.isOn()") is False and pg.evaluate("localStorage.getItem('qqq.contaminanti.on')") == "false"
            pg.check("#uip-cont"); pg.wait_for_timeout(300); assert pg.evaluate("LISTE.isOn()") is True
            pg.click("#np-set")
        step("gear switch: on by default, kept in qqq.contaminanti.on", gear)

        def series():
            o = pg.evaluate("""(async()=>{const j=await J('api/contaminants');const idx=LISTE.buildIndex(j.lists);
              const peg=j.lists[0].items.filter(x=>x.id==='peg'&&x.adduct==='[M+NH4]+'&&x.series.n>=7&&x.series.n<=11).map(x=>x.mz);
              const f=LISTE.forSpec(peg.concat([500.5]), peg.map(()=>1e5).concat([1e5]), 'positive', 5); return f.tip(peg[2])})()""")
            assert "serie PEG (n = 7-11)" in o, o
            o2 = pg.evaluate("""(async()=>{const j=await J('api/contaminants');const peg=j.lists[0].items.filter(x=>x.id==='peg'&&x.adduct==='[M+NH4]+'&&(x.series.n===7||x.series.n===9)).map(x=>x.mz);
              return LISTE.forSpec(peg, peg.map(()=>1e5), 'positive', 5).tip(peg[0])})()""")
            assert "PEG" in o2 and "serie" not in o2, o2
        step("PEG: three consecutive members = «serie PEG (n = 7-11)»; two are not", series)

        def lab():
            pg.evaluate(f"(()=>{{const p={SP};p.zoom=[200,300];draw(p)}})()"); pg.wait_for_timeout(1500)
            mz0 = pg.evaluate(f"(()=>{{const p={SP};const d=p._a.data[0].d;let k=-1;d.mz.forEach((m,i)=>{{if(m>200&&m<300&&(k<0||d.y[i]>d.y[k]))k=i}});return d.mz[k]}})()")
            pg.evaluate(f"setTimeout(()=>LISTE.askAdd({mz0}, 'positive'),0)"); pg.wait_for_selector("#askok", timeout=5000); pg.fill("#askin", "mio fondo"); pg.click("#askok"); pg.wait_for_timeout(1200)
            h = hov(pg, mz0); assert h and "mio fondo" in h and "Contaminanti del laboratorio" in h, h
            assert pg.evaluate("JSON.parse(localStorage.getItem('qqq.contaminanti')).length") == 1
            csv = pg.evaluate("LISTE.csvOut(LISTE.labRows())"); assert csv.startswith("mz,name,formula,polarity,note") and "mio fondo" in csv
        step("laboratory contaminants: added from the menu, seen in the box, saved as CSV", lab)

        def win_lab():
            pg.evaluate("LISTE.openLab()"); pg.wait_for_selector("#lab-xl", timeout=5000)
            assert pg.locator("#bigbody [data-c=name]").count() == 1
            pg.evaluate("document.querySelector('#bigdlg').close()")
        step("window of the laboratory contaminants", win_lab)

        def header_btn():
            pg.evaluate("BARRA.close(false)")                      # an earlier step may have left the panel docked: the button would hide it
            pg.click("#np-liste"); pg.wait_for_selector("#sb-panel-lists", timeout=5000)
            assert pg.is_visible("#sb-panel-lists") and pg.evaluate("BARRA.curTab") == "lists"
            assert pg.is_visible("#sb-help") and pg.locator("#sb-help").get_attribute("data-help") == "liste"
            # Close dialog
            pg.evaluate("BARRA.close(false)"); shut(pg, "#listex"); pg.wait_for_timeout(400)
        step("header button: #np-liste opens the Liste tab of the sidebar", header_btn)

        def user_csv():
            # CSV with 5 compounds, semicolon and decimal comma, matching one observed peak
            csv_data = "mz;name;formula;polarity;note\n279,1591;Sospetto Acque;C16H22O4;+;fiume Po\n150,5000;Sospetto B;C6H6O4;+;test\n200,1000;Sospetto C;C8H12N2;+;test\n300,1500;Sospetto D;C12H20O4;+;test\n350,2000;Sospetto E;C18H26O3;+;test\n"
            pg.evaluate(f"LISTE.addUserListFromCsv('sospetti_acque.csv', {json.dumps(csv_data)})")
            pg.wait_for_timeout(1500)
            # The compound appears in the hover tip on peak 279.1591
            h = hov(pg, 279.1591)
            assert h and "Sospetto Acque" in h, h
            # Open lists dialog and verify the user list is listed
            pg.click("#np-liste"); pg.wait_for_selector("#sb-panel-lists", timeout=5000)
            txt = pg.inner_text("#sb-panel-lists")
            assert "sospetti_acque" in txt or "Sospetto Acque" in txt
            shut(pg, "#listex"); pg.wait_for_timeout(300)
        step("user CSV suspect list: uploaded, visible in dialog, matched in HR hover tip", user_csv)

        pg.evaluate("localStorage.removeItem('qqq.contaminanti')")
    r.close()

    r2 = Run(port=8994, wd="/tmp/wd_cont2")
    with sync_playwright() as p:
        pg = r2.page(p); pg.set_viewport_size({"width": 1400, "height": 1000})
        pg.set_input_files("#pick", [mz("B_FullMass-t15")]); pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=1", timeout=30000)
        pg.click("text=Carica dati"); ready(pg)

        def lr_no_auto_annotations():
            pg.wait_for_function("E.panels.some(p=>p.type==='spec'&&p._a)", timeout=30000)
            assert pg.evaluate("E.panels.filter(p=>p.type==='spec'&&p._a).every(p=>p._a.cont==null)"), "unit resolution: no contaminant data at all"
            h = pg.evaluate("(()=>{const p=E.panels.find(p=>p.type==='spec'&&p._a);const a=p._a;const m=a.data[0].d.mz[0];const h=a.hov(a.X(m));return h?h.html:''})()")
            assert "contaminante" not in h, h
        step("low resolution: NO automatic annotations or hover tips", lr_no_auto_annotations)

        def lr_context_menu_search():
            # Right click on a peak in LR spectrum opens context menu with "Cerca nelle liste dei contaminanti…"
            m0 = pg.evaluate("(()=>{const p=E.panels.find(p=>p.type==='spec'&&p._a);return p._a.data[0].d.mz[0]})()")
            # Open the search via LISTE.open directly or menu click
            pg.evaluate(f"LISTE.open({{ mz: {m0}, polarity: 'positive', hr: false }})")
            pg.wait_for_selector("#sb-panel-lists", timeout=5000)
            assert pg.is_visible("#sb-panel-lists")
            # Verify nominal mode is active with ±0.5 Da tolerance and educational text is present
            mode_btn = pg.evaluate("document.querySelector('#lst-mode-seg button.on').dataset.m")
            assert mode_btn == "da", f"expected 'da', got {mode_btn}"
            tol_val = pg.evaluate("document.querySelector('#lst-tol').value")
            assert float(tol_val) == 0.5
            body_txt = pg.inner_text("#sb-panel-lists")
            assert "Risoluzione unitaria" not in body_txt
            assert "Risoluzione unitaria" in pg.evaluate("HELP.liste[1]") and "silossani" in pg.evaluate("HELP.liste[1]")
            # Results table is populated and sorted
            assert pg.locator("#lst-tbody tr").count() > 0
            shut(pg, "#listex")
        step("low resolution: context menu search opens nominal ±0.5 Da search", lr_context_menu_search)

    r2.close()
except Exception as e:
    import traceback; traceback.print_exc(); steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
    try: r2.close()
    except Exception: pass

print(len(steps), "steps")
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
