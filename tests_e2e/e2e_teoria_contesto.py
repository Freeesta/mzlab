"""The bar of the program in the «Teoria» tab: the commands of the Teoria (TCMD of the iframe), none of the data ones, the gear opens the settings of the
Teoria, the palette searches the Teoria, Next changes chapter inside the iframe, and the data bar and gear come back on the Dati tab."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_contesto.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:220]))
r = Run(port=8972, wd="/tmp/wd_teoria_contesto")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1280, "height": 900})
        fr = lambda: pg.frame_locator("#tframe")
        def data_bar():
            assert pg.is_visible("#stbar [data-cmd=calc]") or pg.locator("#stbar [data-cmd=calc]").count() > 0
            assert pg.locator("#stbar [data-tcmd]").count() == 0
        step("Dati tab: data commands, no Teoria ones", data_bar)
        pg.click("#nav [data-v=theory]"); pg.wait_for_selector("#stbar [data-tcmd=indice]", timeout=15000)
        def theory_bar():
            ids = pg.eval_on_selector_all("#stbar [data-tcmd]", "e=>e.map(x=>x.dataset.tcmd)")
            assert ids == ["indice", "succ", "impostazioni"], ids
            for dead in ("calc", "losses", "adducts", "lists", "more", "search"): assert pg.locator(f"#stbar [data-cmd={dead}]").count() == 0, dead
            kinds = pg.eval_on_selector_all("#stbar > *", "e=>e.map(x=>x.classList.contains('tb-sep')?'|':'b')"); j = "".join(kinds)
            assert "||" not in j and not j.startswith("|") and not j.endswith("|"), j   # no doubled or end separators
            assert not pg.is_visible("#newrun") and not pg.is_visible("#sb-toggle")
            for b in pg.locator("#stbar .tb").all(): bb = b.bounding_box(); assert bb["height"] >= 24 and bb["width"] >= 24, bb
        step("Teoria tab: only the commands of the Teoria; no «Nuova sessione», no side-panel toggle", theory_bar)
        def nxt():
            a = pg.evaluate("document.getElementById('tframe').contentWindow.location.pathname")
            pg.click("#stbar [data-tcmd=succ]"); pg.wait_for_function("(a)=>document.getElementById('tframe').contentWindow.location.pathname!==a", arg=a, timeout=10000)
            pg.wait_for_function("document.querySelectorAll('#stbar [data-tcmd]').length === 4", timeout=10000)      # the bar is rebuilt on the load of the new chapter (which now has «Precedente»)
            assert pg.locator("#stbar [data-tcmd]").count() == 4
        step("Successivo changes chapter in the iframe and the bar stays", nxt)
        def menu():
            f = pg.frame_locator("#tframe")
            f.locator("#sc-strumenti summary").click()
            t = f.locator("#sc-strumenti .sc-sub button").all_inner_texts()
            assert "Frammenta" in t and "Massa e addotti" in t and "Perdite" in t, t
            assert f.locator("#sc-cerca").is_visible() and f.locator("#sc-esporta").is_visible()
        step("the tools and the other commands are in the side panel of the Teoria", menu)
        def palette():
            pg.focus("#stbar [data-tcmd=indice]"); pg.keyboard.press("Control+k"); pg.wait_for_selector("#paletta:not([hidden])")
            pg.fill("#pal-in", "formu"); pg.wait_for_timeout(150)
            t = pg.eval_on_selector_all("#pal-l li", "e=>e.map(x=>x.textContent)")
            assert t and "Formule" in t[0] and not any("Calcolatrice" in x for x in t), t
            pg.fill("#pal-in", "calcol"); pg.wait_for_timeout(150)
            assert "Calcolatrice" not in " ".join(pg.eval_on_selector_all("#pal-l li", "e=>e.map(x=>x.textContent)"))
            pg.keyboard.press("Escape")
        step("palette searches the Teoria commands", palette)
        def gear():
            assert not pg.locator("#np-set").is_visible()      # the Teoria has its own settings on the right: the gear of the program is gone
            pg.click("#stbar [data-tcmd=impostazioni]"); pg.wait_for_timeout(300)
            assert pg.locator("#uipset").count() == 0
            pg.wait_for_function("!document.getElementById('tframe').contentDocument.getElementById('a11y-panel').hidden", timeout=5000)
        step("gear in the Teoria does not open the data settings", gear)
        def back():
            pg.click("#nav [data-v=data]"); pg.wait_for_timeout(400)
            assert pg.locator("#stbar [data-tcmd]").count() == 0 and pg.locator("#stbar [data-cmd=calc]").count() == 1
            pg.click("#np-set"); pg.wait_for_selector("#uipset")
        step("back on Dati: data bar and data settings", back)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
