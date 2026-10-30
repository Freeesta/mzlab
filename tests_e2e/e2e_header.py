"""Block 4 (prompt 3): header without Isotopi, (i) Informazioni + guide as first Teoria chapter, no yellow-cells sentence, example button inside box 1."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_header.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8980, wd="/tmp/wd80")
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
            ps = pg.locator("#helppop p"); assert ps.count() == 3, ps.count()
            assert "{" not in pg.inner_text("#helppop") and ps.first.inner_text() == pg.evaluate("APP_NAME") + " è uno strumento gratuito e open source. Non carica mai i tuoi dati o file personali.", "the name of the program fills {app}"
            assert ps.nth(1).inner_text() == "Questa versione è ancora in lavorazione."
            assert ps.nth(2).inner_text() == "Suggerimenti e correzioni: fedecrista@gmail.com."
            assert pg.locator("#helppop a").count() == 1 and pg.get_attribute("#helppop a", "href") == "mailto:fedecrista@gmail.com"
            pg.keyboard.press("Escape")
        step("(i) opens the box with the mailto link", info)
        def centred():
            from PIL import Image; import io
            for scale in (1, 1.5, 2):
                pg.evaluate(f"document.body.style.zoom={scale}"); frames(pg)
                g = pg.evaluate("""(() => { const b = document.querySelector('button.hq[data-help=header]'), s = b.querySelector('svg'), R = e => { const r = e.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; }; return [R(b), R(s)]; })()""")
                assert abs(g[0][0] - g[1][0]) <= 0.5 and abs(g[0][1] - g[1][1]) <= 0.5, ("the icon box is not in the middle of the button", scale, g)
            pg.evaluate("document.body.style.zoom=1"); frames(pg)
            box = pg.locator("button.hq[data-help=header]").bounding_box()
            im = Image.open(io.BytesIO(pg.screenshot(clip=box))).convert("RGB"); w, h = im.size
            bg = im.getpixel((int(w * .25), int(h * .25)))
            ink = [(x, y) for x in range(int(w * .2), int(w * .8) + 1) for y in range(int(h * .2), int(h * .8) + 1)
                   if sum(abs(a - b) for a, b in zip(im.getpixel((x, y)), bg)) > 150]
            assert ink, "no glyph drawn"
            cx = (min(x for x, _ in ink) + max(x for x, _ in ink) + 1) / 2; cy = (min(y for _, y in ink) + max(y for _, y in ink) + 1) / 2
            assert abs(cx - w / 2) <= 0.75 and abs(cy - h / 2) <= 0.75, ("the drawn i is off centre", cx, cy, w, h)
        step("the i is centred in its round button (box and drawn glyph, three zoom levels)", centred)
        def chapter():
            t = r.teoria(pg)
            assert "MRM" in t and len(t) > 1000, t[:300]
            q = r.b.new_page(); q.goto(f"http://127.0.0.1:{r.port}/static/teoria/index.html"); q.wait_for_timeout(600)
            first = q.evaluate("document.querySelector('#side a').textContent"); assert first.strip() and q.locator("#side a").count() > 10, first
            q.close()
        step("the Teoria opens a real chapter and lists the chapters", chapter)
        def correzione():
            pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t60")]); pg.wait_for_timeout(1000)
            pg.click("text=Carica dati"); ready(pg)
            pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.wait_for_timeout(2000)
            pg.evaluate("document.querySelector('.pnl.xic [data-a=cpar]').click()"); pg.wait_for_timeout(200)
            b = pg.locator('.pnl.xic [data-o="corr"]'); assert b.locator("svg").count() == 1 and "Sottrai" in b.get_attribute("aria-label")      # icon of the choice + the arrow; no words in the header
            assert pg.locator(".pnl.xic select[data-o=corr]").count() == 0, "no longer a plain select"
            b.click(); pg.wait_for_timeout(150)
            pg.keyboard.press("Escape"); pg.click('.pnl.xic [data-o="bkg"]'); pg.wait_for_timeout(150)      # the background has its own menu, independent from the file
            items = pg.locator(".pnl.xic .corrw >> nth=1 >> .corri"); assert items.count() == 4 and all(items.nth(i).locator("svg").count() == 1 for i in range(items.count()))
            t = pg.inner_text(".pnl.xic .corrw >> nth=1 >> .corrm"); assert "Nessuna" in t and "Fondo di un tratto" in t and "Fondo: finestra piatta" in t and "Fondo automatico" in t, t
            pg.click('.pnl.xic .corri[data-c="snip"]'); pg.wait_for_timeout(800)
            assert pg.evaluate("E.panels.find(p=>p.type==='xic').fondo.mode") == "auto" and "Fondo automatico" in pg.get_attribute('.pnl.xic [data-o="bkg"]', "aria-label")
            pg.screenshot(path=SH + "correzione.png")
        step("Correzione: a drop-down with an icon and a short label for every choice", correzione)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
