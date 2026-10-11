"""Block 4 (prompt 3): header without Isotopi, (i) Informazioni + guide as first Teoria chapter, no yellow-cells sentence, example button inside box 1."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_header.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8887, wd="/tmp/wd87")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        def start():
            assert pg.locator("#np-iso").count() == 0 and "Isotopi" not in pg.inner_text("header")
            assert pg.locator("#guessnote").count() == 0
        step("no Isotopi in the header, no yellow-cells sentence", start)
        def info():
            b = pg.locator("button.hq[data-help=header]"); assert b.count() == 1 and b.get_attribute("title") == "Informazioni su mzLab" and b.locator("svg").count() == 1 and b.inner_text().strip() == ""
            b.click(); pg.wait_for_timeout(300)
            assert pg.is_visible("#helppop") and pg.locator("#helppop .hp-t b").inner_text().strip() == "", "no title"
            ps = pg.locator("#helppop p"); assert ps.count() == 1, ps.count()
            assert ps.nth(0).inner_text().startswith("Suggerimenti e correzioni: federico.cristaudo@unito.it (Federico Cristaudo, Università di Torino).")
            assert pg.get_attribute("#helppop a", "href") == "mailto:federico.cristaudo@unito.it"
            pg.keyboard.press("Escape")
        step("(i) opens the box with the mailto link", info)
        def chapter():
            t = r.teoria(pg)
            assert "Come si usa mzLab" in t and "Aprire i dati" in t and "Backspace" in t and "Ctrl/Cmd + Z" in t and "Crediti e licenze" in t, t[:300]
            q = r.b.new_page(); q.goto(f"http://127.0.0.1:{r.port}/static/teoria/index.html"); q.wait_for_timeout(600)
            first = q.evaluate("document.querySelector('#side a, nav a').textContent"); assert "Come si usa" in first, first
            q.close()
        step("the guide is the first chapter of the Teoria", chapter)
        def correzione():
            pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t60")]); pg.wait_for_timeout(1000)
            pg.click("text=Carica dati"); ready(pg)
            pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.wait_for_timeout(2000)
            pg.evaluate("document.querySelector('.pnl.xic [data-a=cpar]').click()"); pg.wait_for_timeout(200)
            b = pg.locator('.pnl.xic [data-o="corr"]'); assert b.locator("svg").count() == 1 and "Correzione" in b.get_attribute("aria-label")      # icon of the choice + the arrow; no words in the header
            assert pg.locator(".pnl.xic select[data-o=corr]").count() == 0, "no longer a plain select"
            b.click(); pg.wait_for_timeout(150)
            items = pg.locator(".pnl.xic .corri"); assert items.count() >= 3 and all(items.nth(i).locator("svg").count() == 1 for i in range(items.count()))
            t = pg.inner_text(".pnl.xic .corrm"); assert "Nessuna" in t and "Fondo di un tratto" in t and "Fondo automatico" in t, t
            pg.click('.pnl.xic .corri[data-c="snip"]'); pg.wait_for_timeout(800)
            assert pg.evaluate("E.panels.find(p=>p.type==='xic').snip") is True and "Fondo automatico" in pg.get_attribute('.pnl.xic [data-o="corr"]', "aria-label")
            pg.screenshot(path=SH + "correzione.png")
        step("Correzione: a drop-down with an icon and a short label for every choice", correzione)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
