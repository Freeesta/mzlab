"""Tabui, the «Elemental composition» block of the Identity tab (TP_Mine/js/46_tp_composizione.js, TPCOMP) on a fixture of the engine (`formula_id/1`, caffeine [M+H]+, written by
`untargeted::formula_id` from a pattern computed with the natural abundances): the block waits for the click on «Suggest compositions», shows the candidates with «Neutral formula» and
«Ion formula» (never the word formula alone), the choice of the program and the reasons, lets the chemist pick another candidate (it says so and why), reports the error of the engine,
and its buttons are at least 24 px. The block alone on a blank page (no site, no wasm). Run from the repo root."""
import os
import json, re, sys
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
FIX = json.loads((ROOT / "tests/data/untargeted/formula_id/caffeine_out.json").read_text(encoding="utf-8"))
SRC = ROOT / "TP_Mine/js/46_tp_composizione.js"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:600]))

with sync_playwright() as p:
    b = p.chromium.launch(executable_path=os.environ.get("PW_CHROMIUM") or None)
    pg = b.new_page(viewport={"width": 900, "height": 900}, reduced_motion="reduce")
    errs = []
    pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.on("console", lambda m: errs.append(m.text) if m.type == "error" else None)
    pg.set_content('<div id="p"></div>')
    pg.add_style_tag(content=".tpr-more{box-sizing:border-box;min-height:24px;min-width:24px;padding:0 8px;font-size:12px}")  # the rule of 44_tp_riga.js
    pg.add_script_tag(path=str(SRC))
    pg.evaluate("""fix => { window.calls = 0; window.fail = false;
      TPCOMP.mount(document.getElementById('p'), { run: () => { window.calls++; return window.fail ? Promise.reject(new Error('engine said no')) : Promise.resolve(fix); } }); }""", FIX)

    def waits():
        assert pg.locator("h4").inner_text() == "Elemental composition"
        assert pg.locator("button", has_text="Suggest compositions").count() == 1
        assert pg.evaluate("window.calls") == 0, "the engine is asked only on the click"
    step("waits for the click", waits)

    def shows():
        pg.locator("button", has_text="Suggest compositions").click()
        pg.wait_for_selector("table.tpr-cand")
        rows = pg.locator("table.tpr-cand tbody tr")
        assert rows.count() == len(FIX["candidates"]), rows.count()
        heads = pg.locator("table.tpr-cand thead th").all_inner_texts()
        assert "Neutral formula" in heads and "Ion formula" in heads, heads
        text = pg.locator("#p").inner_text()
        assert not re.findall(r"(?<!Neutral )(?<!Ion )\bformula\b", text, re.I), "the word formula alone"
        assert "Choice of the program" in text and FIX["candidates"][FIX["chosen"]]["formula"] in text
        assert "Mass error" in text and "Isotope pattern match" in text and "chemical rules met" in text, text
        assert "not an identification" in text
        first = rows.nth(0).locator("button")
        assert first.get_attribute("aria-pressed") == "true" and first.inner_text() == "Using"
    step("shows the candidates and the choice of the program", shows)

    def changes():
        pg.locator("table.tpr-cand tbody tr").nth(1).locator("button").click()
        t = pg.locator("#p").inner_text()
        assert "Your choice" in t and "the program chose" in t, t
        assert pg.locator("table.tpr-cand tbody tr").nth(1).locator("button").get_attribute("aria-pressed") == "true"
        assert pg.locator("table.tpr-cand tbody tr").nth(0).locator("button").get_attribute("aria-pressed") == "false"
        assert pg.evaluate("document.activeElement.dataset.i") == "1", "the focus stays on the button"
    step("lets the chemist pick another one", changes)

    def sizes():
        for bt in pg.locator("#p button").all():
            r = bt.bounding_box()
            assert r["width"] >= 24 and r["height"] >= 24, r
    step("buttons of at least 24 px", sizes)

    def error():
        pg.evaluate("window.fail = true")
        pg.locator("button", has_text="Suggest again").click()
        pg.wait_for_selector("p.err")
        assert "engine said no" in pg.locator("p.err").inner_text()
    step("reports the error of the engine", error)
    b.close()
steps.append(("no errors in the page", "ok" if not errs else "FAIL " + str(errs)[:300]))
bad = [s for s in steps if s[1] != "ok"]
for n, r in steps: print(("PASS " if r == "ok" else "FAIL ") + n + ("" if r == "ok" else ": " + r[5:]))
sys.exit(1 if bad else 0)
