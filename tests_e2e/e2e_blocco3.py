"""Prompt 2, block 3: the XIC window takes several ions (2 rows, + up to 10), m/z or formula in the same box, one panel with a trace per ion,
the file chosen in the window, the calculator shows only the adducts of the polarity of the files."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_blocco3.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8879, wd="/tmp/wd79")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1000})
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        XP = "E.panels.filter(p=>p.type==='xic')"
        rows = lambda: pg.locator("#xic-rows .xrw")
        def two():
            pg.click("#np-xic"); assert rows().count() == 2
            t = pg.inner_text("#xic-rows"); assert "Ione 1" in t and "Ione 2 (facoltativo)" in t, t
            pg.locator("#xic-rows input").nth(0).fill("194.2"); pg.locator("#xic-rows input").nth(1).fill("152"); pg.wait_for_timeout(300)
            s = pg.locator("#xic-rows .xr-sum").all_inner_texts(); print(s); assert "193.8 - 194.8" in s[0] and "151.8 - 152.8" in s[1], s
            n0 = pg.evaluate(f"{XP}.length"); pg.click("#xic-go"); ready(pg)
            assert pg.evaluate(f"{XP}.length") == n0 + 1 and pg.evaluate(f"{XP}.pop().traces.length") == 2
        step("3.1: two ions -> one panel with two traces", two)
        def one():
            pg.click("#np-xic"); pg.locator("#xic-rows input").nth(0).fill("364"); pg.locator("#xic-rows input").nth(1).fill("")
            n0 = pg.evaluate(f"{XP}.length"); pg.click("#xic-go"); ready(pg)
            assert pg.evaluate(f"{XP}.pop().traces.length") == 1 and pg.evaluate(f"{XP}.length") == n0 + 1
        step("3.1: row 2 empty -> only ion 1", one)
        def limit():
            pg.click("#np-xic")
            for i in range(12): pg.click("#xic-add") if pg.locator("#xic-add").is_enabled() else None
            assert rows().count() == 10 and pg.locator("#xic-add").is_disabled()
            pg.locator("#xic-rows .xr-x").nth(3).click(); assert rows().count() == 9 and pg.locator("#xic-add").is_enabled()
            pg.click("#xic-no")
        step("3.1: up to 10 rows, × removes one", limit)
        def mixed():
            pg.click("#np-xic"); pg.locator("#xic-rows input").nth(0).fill("c14h13f4n3o2s"); pg.locator("#xic-rows input").nth(1).fill("300"); hold(pg, 1300)
            s = pg.locator("#xic-rows .xr-sum").all_inner_texts(); print(s); assert "363.8 - 364.8" in s[0] and "interpretata" in s[0] and "299.8 - 300.8" in s[1], s
            assert pg.locator("#xic-rows select").nth(0).is_visible() and not pg.locator("#xic-rows select").nth(1).is_visible(), "the adduct only for a formula"
            pg.click("#xic-no")
        step("3.1: formula and m/z in the same kind of box", mixed)
        def many_file():
            pg.click("#np-xic")
            for v in ["100", "110", "120", "130"]:
                if pg.locator("#xic-rows .xrw").count() < 4: pg.click("#xic-add")
            ins = pg.locator("#xic-rows input")
            for i, v in enumerate(["100", "110", "120", "130"]): ins.nth(i).fill(v)
            pg.select_option("#xic-file", index=2); n = pg.evaluate("E.files[+document.querySelector('#xic-file').value].label")
            pg.click("#xic-go"); ready(pg)
            hid = pg.evaluate(f"(()=>{{const p={XP}.pop();return Object.keys(p.hid).filter(k=>p.hid[k]).map(k=>+k.split('|')[1])}})()")
            keep = pg.evaluate("+document.querySelector('#xic-file').value") if False else None
            ks = pg.evaluate("tabFiles('full').map(f=>f.k)"); print(n, hid, ks); assert len(set(hid)) == len(ks) - 1, (hid, ks)
        step("3.2: the file chosen in the window (the others are hidden in the panel)", many_file)
        def calc():
            pg.click("#np-calc2"); pg.fill("#calcin", "C14H13F4N3O2S"); pg.wait_for_timeout(1500)
            t = pg.inner_text("#calcout").replace("\u2212", "-"); print(t[:200].replace("\n", " | "))
            assert "[M+H]+" in t and "[M-H]-" not in t and "mostra anche ESI" in t, t
            pg.click("#calcmore"); pg.wait_for_timeout(300); t = pg.inner_text("#calcout").replace("\u2212", "-"); assert "[M-H]-" in t and "ESI+" in t, t
            pg.evaluate("BARRA.close(false)"); shut(pg, "#calcx")
        step("3.4: the calculator shows only ESI+ adducts for positive files", calc)
        def neg():
            pg.evaluate("CALC_MORE=false;E.files.forEach(f=>{f.polarity='negative'})"); pg.click("#np-calc2"); pg.fill("#calcin", ""); pg.fill("#calcin", "C14H13F4N3O2S"); pg.wait_for_timeout(1500)
            t = pg.inner_text("#calcout").replace("\u2212", "-"); assert "[M-H]-" in t and "[M+H]+" not in t and "[M+Na]+" not in t, t
            pg.evaluate("E.files.forEach(f=>{f.polarity='mixed'})"); pg.fill("#calcin", ""); pg.fill("#calcin", "C14H13F4N3O2S"); pg.wait_for_timeout(1500)
            t = pg.inner_text("#calcout").replace("\u2212", "-"); assert "ESI+" in t and "[M-H]-" in t and "[M+H]+" in t, t
            pg.evaluate("BARRA.close(false)"); shut(pg, "#calcx")
        step("3.4: negative files -> only negative adducts; mixed -> both under ESI+ / ESI-", neg)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s_ in steps: print(" ", s_)
r.report()
