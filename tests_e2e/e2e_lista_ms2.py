"""File list: the product-ion (MS2) group starts folded and folds/unfolds with the mouse and the keyboard (header, chevron, per-file toggle); precursor rows read «Precursore …»; no «Profilo»/«Centroide» text on file rows. Demo files, light and dark, phone width."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import re
from lib import *
ES = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
r = Run(port=8897, wd="/tmp/wd_lista_ms2")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [str(ES / f"FullScan_t{t}.mzML") for t in (0, 15)] + [str(ES / "MS2_t15.mzML")])
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setTab('ms2')"); ready(pg); drawer(pg)
        N = lambda: pg.locator("#flst input[data-pr]:visible").count()
        H = "#flst .fgh[data-gt=ms2]"
        def folded():
            assert pg.locator(H).count() == 1 and N() == 0, "MS2 group present and folded at start"
            assert pg.get_attribute(H + " .chv", "aria-expanded") == "false"
        step("the MS2 group starts folded", folded)
        def mouse():
            pg.click(H + " .chv"); assert N() >= 1, "chevron opens"
            pg.click(H + " .chv"); assert N() == 0, "chevron folds"
            pg.locator(H).click(position={"x": 200, "y": 8}); assert N() >= 1, "header opens"
            pg.locator(H).click(position={"x": 200, "y": 8}); assert N() == 0, "header folds"
        step("group header and chevron toggle by mouse", mouse)
        def keys():
            pg.focus(H + " .chv"); pg.keyboard.press("Enter"); assert N() >= 1, "Enter opens"
            pg.focus(H + " .chv"); pg.keyboard.press("Space"); assert N() == 0, "Space folds"
        step("the chevron toggles by keyboard", keys)
        def perfile():
            open_ms2(pg)
            t = pg.inner_text("#flst .pr-container") if pg.locator("#flst .pr-container").count() else pg.inner_text("#flst")
            nums = pg.evaluate("[...document.querySelectorAll('#flst .fl.pr .pn')].map(x => x.textContent)")
            assert nums and all(re.fullmatch(r"[0-9.]+", x) for x in nums), nums      # only the precursor number
            assert "Precursore m/z" in pg.get_attribute("#flst .fl.pr input[data-pr]", "aria-label"), "the tooltip words it"
            tog = pg.locator("#flst .pr-tog").first
            if tog.count():
                tog.click(); a = N(); tog.click(); b = N(); assert a != b, (a, b)
            assert "Profilo" not in pg.inner_text("#flst") and "Centroid" not in pg.inner_text("#flst")
        step("precursor rows are named; per-file toggle works; no mode text", perfile)
        def langs():
            for lang, want in (("en", "Precursor m/z"), ("it", "Precursore m/z")):
                pg.evaluate(f"I18N.set('{lang}')"); pg.wait_for_function(f"I18N.lang === '{lang}'"); ready(pg); drawer(pg); open_ms2(pg)
                assert want in pg.get_attribute("#flst .fl.pr input[data-pr]", "aria-label"), lang
        step("English and Italian labels", langs)
        def themes():
            for th in ("light", "dark"):
                pg.evaluate(f"document.documentElement.dataset.theme='{th}'")
                assert pg.evaluate("getComputedStyle(document.querySelector('#flst .fgh[data-gt=ms2]')).display") != "none"
        step("light and dark", themes)
        def labels():
            t = pg.evaluate("[...document.querySelectorAll('#flst .fl:not(.pr)')].map(x => [x.dataset.tip, x.querySelector('small').textContent])")
            ms2 = [x for x in t if x[0].startswith("MS2")]; full = [x for x in t if x[0].startswith("FullScan")]
            assert ms2 and all("min" not in x[1] for x in ms2), t      # no time on a product-ion row
            assert full and all("min" in x[1] for x in full), t        # the time stays on Full Scan rows
            assert not any("." in x[0].split(" ")[0] for x in t), t   # no extension in a name
            r = pg.evaluate("(() => { const fs = [{ label: noExt('CBZ_t10.wiff') }, { label: noExt('CBZ_t10.mzML') }, { label: noExt('CBZ_t15.RAW · MS2') }]; uniqueLabels(fs); return fs.map(f => f.label); })()")
            assert r == ["CBZ_t10", "CBZ_t10 (2)", "CBZ_t15 · MS2"], r
        step("names without extension and unique, time only on Full Scan rows", labels)
        def phone():
            pg.set_viewport_size({"width": 390, "height": 800}); ready(pg)
            assert pg.evaluate("document.documentElement.scrollWidth <= innerWidth + 1"), "no horizontal overflow"
        step("phone width", phone)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
