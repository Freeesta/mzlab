"""Disegno, «Structure tray» card: an empty canvas adds nothing; draw → «Add to tray» keeps the structure with a name, formula and mass (the canvas
menu row does the same); rename, reorder and remove from the keyboard; «Save suspect list» downloads the CSV of the Rust engine with the ticked
adducts; the tray survives a reload of the page (session); «Open list…» reads the saved list back to the same entries and «Draw» puts one on the
canvas. Needs site/static/wasm-feat/grafo (tools/build_feat_wasm.sh); SKIP without it."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
if not (ROOT / "site" / "static" / "wasm-feat" / "manifest.json").exists():
    print("SKIP site/static/wasm-feat is missing (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))

CAF = "Cn1cnc2c1c(=O)n(C)c(=O)n2C"          # caffeine C8H10N4O2, 194.080376 (AME masses)
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
TMP = HERE / "shots" / "disegno_vassoio_tmp"; TMP.mkdir(parents=True, exist_ok=True)

def items(pg):
    return pg.evaluate("[...document.querySelectorAll('#tray-body .tray-l li')].map(li=>[li.querySelector('.tray-name').value,li.querySelector('.tray-row .muted').textContent])")

def draw(pg, smi, n):
    pg.evaluate(f"{KJ}.setMolecule('{smi}')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === {n}")

def go_draw(pg):
    pg.click("#nav [data-v=draw]"); pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)

saved = {}
r = Run(port=8995, wd="/tmp/wd_disegno_vassoio")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        go_draw(pg)
        def empty():
            pg.click("#tray-add"); pg.wait_for_function("document.getElementById('tray-msg').textContent !== ''", timeout=30000)
            assert "Niente di disegnato" in pg.inner_text("#tray-msg"), pg.inner_text("#tray-msg")
            assert not items(pg)
        step("an empty canvas adds nothing", empty)
        def add():
            draw(pg, CAF, 14)
            pg.click("#tray-add"); pg.wait_for_selector("#tray-body .tray-l li", timeout=30000)
            assert items(pg) == [["Struttura 1", "C8H10N4O2 · 194.0804"]], items(pg)
            draw(pg, "CCO", 3)
            pg.evaluate("""(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:200,clientY:200}))})()""")
            row = pg.locator("#ctx div[data-row=add-to-tray]"); row.wait_for(timeout=10000)
            assert row.inner_text() == "Aggiungi al vassoio", row.inner_text()
            row.click(); pg.wait_for_function("document.querySelectorAll('#tray-body .tray-l li').length === 2", timeout=30000)
            assert items(pg)[1] == ["Struttura 2", "C2H6O · 46.0419"], items(pg)
            assert "Aggiunte al vassoio: 1" in pg.inner_text("#tray-msg")
        step("draw → Add to tray (button and canvas menu row): name, formula and mass", add)
        def keyboard():
            name = pg.locator(".tray-name").first; name.fill("caffeina, \"pura\""); name.press("Tab")
            down = pg.locator("#tray-body button[data-act=down][data-i='0']"); down.focus(); pg.keyboard.press("Enter")
            pg.wait_for_function("document.querySelector('.tray-name').value === 'Struttura 2'", timeout=10000)
            assert [n for n, _ in items(pg)] == ["Struttura 2", "caffeina, \"pura\""], items(pg)
            assert pg.evaluate("document.activeElement.dataset.act + document.activeElement.dataset.i") == "down1" or \
                pg.evaluate("document.activeElement.dataset.i") == "1", pg.evaluate("document.activeElement.outerHTML")
            for b in pg.locator("#tray-body button").all():
                box = b.bounding_box(); assert box["height"] >= 24 and box["width"] >= 24, box
                assert b.get_attribute("aria-label") or b.inner_text().strip(), b.evaluate("e=>e.outerHTML")
        step("rename and move down from the keyboard; the focus follows the entry; targets ≥24 px with names", keyboard)
        def save_csv():
            pg.check("#tray-body input[data-ad='[M+Na]+']")
            with pg.expect_download() as d:
                pg.click("#tray-body button[data-act=save]")
            path = TMP / d.value.suggested_filename; d.value.save_as(path)
            assert path.suffix == ".csv", path
            text = path.read_text(encoding="utf-8")
            lines = text.split("\n")
            assert lines[0] == "name,smiles,formula,mass,charge,adducts", lines
            assert lines[1].startswith("Struttura 2,") and lines[1].endswith(",C2H6O,46.041865,0,[M+H]+;[M+Na]+;[M-H]-"), lines
            assert lines[2].startswith('"caffeina, ""pura""",') and ",C8H10N4O2,194.080376,0," in lines[2], lines
            assert lines[3] == "" and len(lines) == 4, lines
            saved["path"], saved["text"] = path, text
        step("Save suspect list: the CSV of the engine with the ticked adducts", save_csv)
        def session():
            pg.reload(); go_draw(pg)
            pg.wait_for_selector("#tray-body .tray-l li", timeout=30000)
            assert [n for n, _ in items(pg)] == ["Struttura 2", "caffeina, \"pura\""], items(pg)
            assert pg.is_checked("#tray-body input[data-ad='[M+Na]+']")
        step("the tray survives a reload of the page", session)
        def reload_list():
            for _ in range(2):
                pg.click("#tray-body button[data-act=del][data-i='0']")
            pg.wait_for_function("!document.querySelector('#tray-body .tray-l')", timeout=10000)
            pg.set_input_files("#tray-file", str(saved["path"]))
            pg.wait_for_function("document.querySelectorAll('#tray-body .tray-l li').length === 2", timeout=30000)
            assert [n for n, _ in items(pg)] == ["Struttura 2", "caffeina, \"pura\""], items(pg)
            assert "2 strutture aggiunte" in pg.inner_text("#tray-msg"), pg.inner_text("#tray-msg")
            with pg.expect_download() as d:
                pg.click("#tray-body button[data-act=save]")
            again = TMP / "again.csv"; d.value.save_as(again)
            assert again.read_text(encoding="utf-8") == saved["text"]
        step("Open list… reads the saved list back and saves the same bytes", reload_list)
        def draw_back():
            pg.click("#tray-body button[data-act=draw][data-i='1']")
            wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 14")
        step("Draw puts an entry of the tray back on the canvas", draw_back)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
