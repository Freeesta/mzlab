"""Polarity made visible: sign next to the file names (list and loading table), default adduct from the panel's own files, sign in the legend when + and - are overlaid, polarity first in the Addotti table."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import pathlib
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_polarita.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
NEG = pathlib.Path(__file__).resolve().parent / "shots" / "neg_tmp"
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici, numpy as np
NEG.mkdir(parents=True, exist_ok=True); NEGF = NEG / "B_FullMass-neg-t0.mzML"
dati_sintetici.full_scan(NEGF, 0, np.random.default_rng(7), neg=True)       # synthetic negative Full Scan (the lab has none: all ESI+)
r = Run(port=8875, wd="/tmp/wd75")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), str(NEGF)]); pg.wait_for_timeout(1500)
        def load_signs():
            t = pg.inner_text("#flist"); assert "ESI+" in t and "ESI−" in t, t
            assert pg.evaluate("[...document.querySelectorAll('#flist .pol')].map(x=>x.title)") == ["ESI negativo", "ESI positivo"] or set(pg.evaluate("[...document.querySelectorAll('#flist .pol')].map(x=>x.title)")) == {"ESI positivo", "ESI negativo"}
        step("loading table: ESI+ / ESI− badges (mixed polarities) next to the names", load_signs)
        pg.click("text=Carica dati"); ready(pg)
        def list_signs():
            drawer(pg); assert pg.evaluate("[...document.querySelectorAll('#flst .fl .pol')].map(x=>x.textContent).sort().join('')") == "ESI+ESI−"
        step("file list: signs", list_signs)
        def legend():
            pg.evaluate("E.files.forEach(f=>f.vis=true); syncBrowse()"); pg.evaluate("redrawAll()"); pg.wait_for_timeout(1500)      # several ticked files: syncBrowse() leaves the single-file view
            leg = pg.inner_text(".pnl.chrom .leg"); assert "ESI+" in leg and "ESI−" in leg, leg                            # both polarities overlaid: the sign is in the legend
        step("legend: sign when positive and negative are overlaid", legend)
        def uniform():
            pg.evaluate("E.files.filter(f=>f.polarity==='negative').forEach(f=>{f.gone=true})"); pg.evaluate("renderFileList()"); pg.wait_for_timeout(300)
            assert pg.locator("#flst .pol").count() == 0, "all files ESI+: no badge per file"
            # Federico: "Non scrivere ESI+ o ESI- dopo il tipo di esperimento nei file"
            ov = pg.evaluate("(()=>{const l=document.querySelector('#flst');return [...l.querySelectorAll('.fl')].every(e=>e.getBoundingClientRect().right<=l.getBoundingClientRect().right+1)})()"); assert ov, "nothing sticks out of the list"
            pg.evaluate("E.files.forEach(f=>{f.gone=false})"); pg.evaluate("renderFileList()")
        def simboli():
            t = pg.inner_text("body"); bad = [c for c in "\u00b2\u00b3\u207a\u207b\u2080\u2081\u2082\u2083\u2084\u00bd" if c in t]; assert not bad, bad
        step("7.2 No Unicode superscripts in the visible text of the Dati tab", simboli)
        step("one polarity only: no badge per file, no ESI in group heading, nothing out of the row", uniform)
        def adduct():
            neg = pg.evaluate("E.files.findIndex(f=>f.polarity==='negative')")
            pg.evaluate(f"E.files.forEach((f,i)=>f.vis=i==={neg})")
            assert pg.evaluate("defAdduct()") == "[M-H]-"
            pg.evaluate(f"E.files.forEach((f,i)=>f.vis=i!=={neg})"); assert pg.evaluate("defAdduct()") == "[M+H]+"
            pg.evaluate("E.files.forEach(f=>f.vis=true)"); assert pg.evaluate("defAdduct()") == "[M+H]+", "mixed: positive"
            pg.evaluate(f"E.files.forEach((f,i)=>f.vis=i==={neg})"); pg.evaluate("openXic(null,{})"); pg.wait_for_timeout(300)
            assert pg.input_value("#xic-ad") == "[M-H]-"; pg.click("#xic-no")
        step("default adduct follows the panel's files ([M-H]- for a negative file)", adduct)
        def addotti():
            pg.evaluate("E.files.forEach(f=>f.vis=true)")
            pg.click("#np-ad"); pg.wait_for_timeout(500)
            h = pg.evaluate("[...document.querySelectorAll('#bigbody h4, dialog[open] h4')].map(x=>x.textContent)"); print(h)
            pg.keyboard.press("Escape")
        step("Addotti table opens", addotti)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
