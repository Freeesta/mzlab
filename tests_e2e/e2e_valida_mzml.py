# e2e_valida_mzml: the menu of an mzML file in Dati has «Validate mzML»; it opens a dialog with the findings of `mzlab-cli validate` (severity, message, location), loads only the
# `valida` module (never the readers), misses no engine route and has no accessibility violation. Needs the site and the module: tools/prepara_sito.py, tools/build_lettori_wasm.sh site/static
import os, re, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
EXAMPLE = ROOT / "mzlab" / "web" / "esempi" / "FullScan_t10.mzML"
r = Run(port=8846, wd="/tmp/wd_valida")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        asked = []
        pg.on("request", lambda q: asked.append(q.url) if "wasm-lettori/" in q.url else None)
        stage(pg, [EXAMPLE]); pg.click("text=Carica dati"); ready(pg)
        def menu_entry():
            if not pg.is_visible("#flst"): pg.click("#stbar [data-cmd=file]")
            pg.wait_for_selector("#flst .fl", state="visible"); pg.click("#flst .fl .nm", button="right")
            pg.wait_for_selector("#ctx div:text-matches('Valida mzML|Validate mzML')", state="visible")
        step("the menu of the mzML file has «Validate mzML»", menu_entry)
        def report():
            pg.click("#ctx div:text-matches('Valida mzML|Validate mzML')")
            pg.wait_for_selector("#bigdlg[open] #bigbody table, #bigdlg[open] #bigbody p:text-matches('Nessun problema|No problem')", timeout=60000)
            t = pg.inner_text("#bigbody")
            assert re.search(r"(Spettri|Spectra) 1110", t), t[:300]
            assert re.search(r"(Errore|Error)", t) and pg.locator("#bigbody th[scope=col]").count() == 3, t[:300]
            rows = pg.locator("#bigbody tbody tr").count()
            n = pg.evaluate("fetch('api/mzml_valida?name=FullScan_t10.mzML').then(r=>r.json()).then(j=>j[0].problems.length)")
            assert rows == n and rows > 0, (rows, n)
        step("the dialog lists the findings of the report", report)
        def only_valida():
            fams = {u.split("wasm-lettori/")[1].split("/")[0] for u in asked}
            assert fams == {"manifest.json", "valida"} or fams == {"valida"} or fams <= {"manifest.json", "valida"}, fams
            assert "valida" in fams
            assert pg.evaluate("(window.MZLAB_LR_MISS||[]).length") == 0
        step("only the module valida was loaded and no route missed", only_valida)
        def a11y():
            assert pg.locator("#bigbody [role=region][tabindex='0']").count() == 1
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('#bigdlg[open]')")
        step("the findings are reachable by keyboard and Esc closes the dialog", a11y)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
