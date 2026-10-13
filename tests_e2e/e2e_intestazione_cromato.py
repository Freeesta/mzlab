"""Chromatogram header: Smoothing and Log buttons on the right, «Range» popover (m/z, RT) with a clear ×, «Correz.» menu with 4 entries and a file chooser."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import ds, load
import numpy as np
from pathlib import Path
D = HERE / "shots" / "intestaz_cromato_tmp"; D.mkdir(parents=True, exist_ok=True)
for t in (0, 60): ds.full_scan(D / f"B_FullMass-t{t}.mzML", t, np.random.default_rng(2026 + t))
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_intestazione_cromato.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8896, wd="/tmp/wd96")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / "B_FullMass-t0.mzML"), str(D / "B_FullMass-t60.mzML")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.wait_for_timeout(2000)
        X, C = ".pnl.xic", ".pnl.chrom"
        SH = f"(q=>{{const c=q.cv;return c.toDataURL().length+':'+c.toDataURL().slice(-200)}})(E.panels.find(p=>p.type==='xic'))"
        def smooth():
            b = pg.locator(f'{X} .tbs [data-a=smooth]'); assert b.count() == 1 and b.locator("svg").count() == 1
            assert pg.locator(f'{X} .ctl [data-o=smooth]').count() == 0, "not in the left controls"
            assert b.get_attribute("aria-label") == "Smoothing" and b.get_attribute("aria-pressed") == "true" and "on" in b.get_attribute("class").split()
            a = pg.evaluate(SH); b.click(); pg.wait_for_timeout(500)
            assert b.get_attribute("aria-pressed") == "false" and "on" not in b.get_attribute("class").split() and pg.evaluate("E.panels.find(p=>p.type==='xic').smooth") is False
            assert pg.evaluate(SH) != a, "the trace changes"
            assert pg.get_attribute(f'{C} [data-a=smooth]', "aria-pressed") == "true", "state per panel"
            b.click(); pg.wait_for_timeout(300)
        step("Smoothing: right-hand button, aria-pressed, effect on the trace, per panel", smooth)
        def log():
            b = pg.locator(f'{X} .tbs [data-a=log]'); assert b.count() == 1 and b.get_attribute("aria-label") == "Scala logaritmica" and b.get_attribute("aria-pressed") == "false"
            assert pg.locator(f'{X} [data-o=log]').count() == 0, "no checkbox any more"
            b.click(); pg.wait_for_timeout(400)
            assert b.get_attribute("aria-pressed") == "true" and pg.evaluate("E.panels.find(p=>p.type==='xic').log") is True
            assert pg.get_attribute(f'{C} [data-a=log]', "aria-pressed") == "false"
            b.click(); pg.wait_for_timeout(300); assert pg.evaluate("E.panels.find(p=>p.type==='xic').log") is False
            pg.click(f'{X} [data-a=cpar]'); assert pg.locator(f'{X} .cpop [data-o=log]').count() == 0, "log is not in the popover"; pg.click(f'{X} [data-a=cpar]')
        step("Log: right-hand button, out of the popover", log)
        def order():
            n = pg.evaluate("[...document.querySelectorAll('.pnl.xic .tbs [data-a]')].map(b=>b.dataset.a).join()")
            assert "smooth,log" in n and n.index("iauto") < n.index("smooth") and n.index("log") < n.index("dl"), n
        step("order …, integration, Smoothing, Log, …, download", order)
        def rng():
            b = pg.locator(f'{X} [data-a=cpar]'); assert b.inner_text().strip() == "Intervallo" and "on" not in b.get_attribute("class").split()
            assert pg.locator(f'{X} [data-a=rngx]').count() == 0
            b.click(); pg.wait_for_timeout(200)
            pg.fill(f'{X} .cpop [data-o=rt0]', "1"); pg.fill(f'{X} .cpop [data-o=rt1]', "5"); pg.press(f'{X} .cpop [data-o=rt1]', "Enter"); pg.wait_for_timeout(600)
            assert "on" in pg.get_attribute(f'{X} [data-a=cpar]', "class").split() and pg.locator(f'{X} [data-a=rngx]').count() == 1
            assert pg.evaluate("E.panels.find(p=>p.type==='xic').zoom") is not None
            pg.click(f'{X} [data-a=rngx]'); pg.wait_for_timeout(400)
            assert pg.evaluate("E.panels.find(p=>p.type==='xic').zoom") is None and pg.locator(f'{X} [data-a=rngx]').count() == 0
            pg.click(f'{C} [data-a=cpar]'); pg.wait_for_timeout(200)
            pg.fill(f'{C} .cpop [data-o=mz0]', "300"); pg.fill(f'{C} .cpop [data-o=mz1]', "400"); pg.press(f'{C} .cpop [data-o=mz1]', "Enter"); pg.wait_for_timeout(600)
            assert "on" in pg.get_attribute(f'{C} [data-a=cpar]', "class").split()
            pg.click(f'{C} [data-a=rngx]'); pg.wait_for_timeout(400)
            assert pg.evaluate("(p=>[p.mz0,p.mz1])(E.panels.find(p=>p.type==='chrom'))") == [None, None]
        step("Range: m/z and RT set and cleared, active state and ×", rng)
        def corr():
            pg.click(f'{X} [data-o=corr]'); pg.wait_for_timeout(200)
            it = pg.locator(f'{X} .corrw >> nth=0 >> .corri'); assert it.count() == 2, it.count()          # «Sottrai»: none + the file; «Fondo» has its own menu
            assert pg.locator(f'{X} .corrm select').count() == 1, "one file chooser"
            assert pg.locator(f'{X} .corri', has_text="Sottrai file").count() == 1
            assert all(len(t) < 60 for t in pg.locator(f'{X} .corri .sm').all_inner_texts()), "one-line hints"
            pg.keyboard.press("Escape"); pg.wait_for_timeout(100); assert pg.evaluate("document.querySelector('.pnl.xic .corrm').hidden")
        step("Correz. menus: Sottrai (2 entries, one file chooser), Esc closes", corr)
        def pick():
            exp = pg.evaluate("(()=>{const r=SUBREF.autoFile('full');return r.label})()")
            pg.click(f'{X} [data-o=corr]'); pg.wait_for_timeout(150)
            assert exp in pg.locator(f'{X} .corri', has_text="Sottrai file").inner_text(), "default = reference of the session (t0)"
            assert pg.eval_on_selector(f'{X} .corrsel', "s=>s.selectedOptions[0].textContent") == exp
            other = pg.eval_on_selector(f'{X} .corrsel', "s=>[...s.options].find(o=>!o.selected).value")
            pg.select_option(f'{X} .corrsel', other); pg.wait_for_timeout(1200)
            assert pg.evaluate("E.panels.find(p=>p.type==='xic').bk") == int(other)
            assert pg.locator(f'{X} .subchip').count() == 1 and "on" in pg.get_attribute(f'{X} [data-o=corr]', "class").split()
            assert pg.eval_on_selector(f'{X} .subchip', "c=>c.textContent").find(pg.evaluate(f"E.files[{other}].label")) >= 0, "the legend chip names the file"
            pg.click(f'{X} [data-o=corr]'); pg.locator(f'{X} .corrw >> nth=0 >> .corri', has_text="Nessuna").click(); pg.wait_for_timeout(500)
            assert pg.evaluate("E.panels.find(p=>p.type==='xic').bk") == ""
        step("Subtract file: default = session reference, choosing updates the chip", pick)
        def tlink():
            b = pg.locator(f'{C} [data-a=tlink]'); assert b.get_attribute("aria-pressed") == "true" and "Collega zoom e cursore" in b.get_attribute("aria-label")
            b.click(); assert b.get_attribute("aria-pressed") == "false" and "on" not in b.get_attribute("class").split(); b.click()
        step("chromatogram link button: own icon, label and aria-pressed", tlink)
        def follow():
            pg.evaluate("(()=>{const c=E.panels.find(p=>p.type==='chrom');newSpec(c,5,5.2,E.cur)})()"); pg.wait_for_timeout(1500)
            S = ".pnl.spec"; fol = pg.locator(f'{S} .fol.on').first
            assert "Segue: pann." in fol.inner_text() and "on" in fol.get_attribute("class").split() and fol.get_attribute("aria-label")
            fol.click(); pg.wait_for_timeout(200)
            t = pg.inner_text("#ctx"); assert "Nessuno (spettro fisso)" in t and "Pannello" in t, t
            pg.locator("#ctx div", has_text="Nessuno").click(); pg.wait_for_timeout(500)
            assert pg.evaluate("E.panels.filter(p=>p.type==='spec').every(p=>!p.link)") and pg.locator(f'{S} .fol', has_text="Fisso").count() >= 1
            pg.locator(f'{S} .fol').first.click(); pg.locator("#ctx div", has_text="Pannello").first.click(); pg.wait_for_timeout(500)
            assert pg.evaluate("E.panels.some(p=>p.type==='spec'&&p.link)")
        step("spectrum chip: follows panel N, menu picks the chromatogram or «none» (fixed)", follow)
        def spcorr():
            S = ".pnl.spec"; pg.locator(f'{S} [data-o=corr]').first.click(); pg.wait_for_timeout(150)
            n = (pg.locator(f'{S} .corrm').first.locator('.corri').count(), pg.locator(f'{S} .corrm').first.locator('select').count())
            assert n == (3, 1), n
            pg.locator(f'{S} .corrm').first.locator('.corri', has_text="Fondo di un tratto").click(); pg.wait_for_timeout(500)
            assert pg.evaluate("E.panels.filter(p=>p.type==='spec').some(p=>p.bg==='w')") and pg.locator(f'{S} [data-o=bw0]').count() == 1
            pg.locator(f'{S} [data-o=corr]').first.click(); pg.locator(f'{S} .corrm').first.locator('.corri', has_text="Nessuna").click(); pg.wait_for_timeout(300)
            assert pg.evaluate("E.panels.filter(p=>p.type==='spec').every(p=>p.bg===''||p.bg==='x'||p.bg==null)")
        step("spectrum Correzione: same menu as the chromatogram, without the baseline", spcorr)
        def single():
            pg.evaluate("document.querySelector('.pnl.xic [data-o=corr]').click()")
            pg.evaluate("(()=>{const p=E.panels.find(p=>p.type==='xic');const keep=E.files;E.files=[keep[0]];window.__k=keep;ctl(p)})()"); pg.wait_for_timeout(200)
            try:
                b = pg.locator('.pnl.xic .corri', has_text="Sottrai file"); assert b.is_disabled() and b.get_attribute("title"), "disabled with a reason"
            finally:
                pg.evaluate("(()=>{E.files=window.__k;ctl(E.panels.find(p=>p.type==='xic'))})()")
        step("with one file the Subtract entry is disabled with a reason", single)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
