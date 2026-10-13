"""Disegno in English: the static texts, the shortcut table, the export file names and the context texts are English, nothing Italian is left
in the visible text of the view; the Italian Disegno keeps the same texts. Synthetic data (nothing is loaded)."""
import sys, os, re; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_lingua_disegno.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
ITALIAN = re.compile(r"\b(il|della|dei|delle|non|per|sotto|ogni|struttura|esporta|tela|sfondo|trasparente|scorciatoie|strumenti|disegna|incolla)\b", re.I)
r = Run(port=8840 + 100, wd="/tmp/wd_lingua_dis")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); url = f"http://127.0.0.1:{r.port}/"
        def page(locale):
            c = b.new_context(locale=locale, viewport={"width": 1500, "height": 1400}); pg = c.new_page()
            pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(url); pg.wait_for_timeout(800); pg.click("#nav [data-v=draw]")
            pg.wait_for_function("window.TPDraw && TPDraw.ready()", timeout=90000); pg.wait_for_timeout(500); return pg
        def visible_text(pg):
            return pg.evaluate("""() => { const v = document.querySelector('#v-draw'); const out = [v.innerText];
              v.querySelectorAll('[title],[placeholder],[aria-label]').forEach(e => out.push(e.title || '', e.placeholder || '', e.getAttribute('aria-label') || '')); return out.join('\\n'); }""")
        def english():
            pg = page("en-US"); t = visible_text(pg)
            for w in ("Under each structure", "Quick draw", "Export the canvas", "Transparent background", "File name", "Shortcuts", "Editor tools", "Hide panels"):
                assert w in t, (w, t[:300])
            bad = ITALIAN.findall(t); assert not bad, bad
            assert pg.input_value("#ex-name").startswith("drawing_"), pg.input_value("#ex-name")
            assert pg.get_attribute("#ex-smi", "placeholder") == "Paste a SMILES"
            pg.click("#keys-card summary"); k = pg.inner_text("#keys-body"); assert "Copy, paste, cut the selection" in k and "Right-click" in k, k
            pg.click("#tools-card summary"); assert "Quick rings (benzene, cyclohexane...)" in pg.inner_text("#tools-body")
        step("English: static texts, shortcuts, tools card, default file name drawing_...", english)
        def toggle():
            pg = page("en-US"); pg.click("#side-toggle"); assert pg.inner_text("#side-toggle").strip() == "Show panels"
            pg.click("#side-toggle"); assert pg.inner_text("#side-toggle").strip() == "Hide panels"
        step("English: show / hide panels button", toggle)
        def invalid():
            pg = page("en-US"); pg.fill("#ex-smi", "C1CC"); pg.click("#ex-load"); pg.wait_for_timeout(1500)
            w = pg.inner_text("#smi-warn"); assert w.startswith("Invalid SMILES"), w
        step("English: invalid SMILES message", invalid)
        def italian():
            pg = page("it-IT"); t = visible_text(pg)
            for w in ("Sotto ogni struttura", "Disegna veloce", "Esporta la tela", "Sfondo trasparente", "Nome file", "Scorciatoie"):
                assert w in t, w
            assert pg.input_value("#ex-name").startswith("disegno_")
        step("Italian: unchanged", italian)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
