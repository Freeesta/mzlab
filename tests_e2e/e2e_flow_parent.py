"""Flow, the node «Parent structure» (flow/lazy/flow-parent.js): the box is the search box of the molecules database (a name, a synonym, a formula, an InChIKey or an m/z, with the ranked
list and the keyboard, one slip of a letter forgiven); a formula or an m/z is asked what it is, the neutral molecule M (the default) or an ion of the adduct table, and what the plan keeps is
the NEUTRAL molecule with the choice written in it (S.parents, S.samplesJson, SHA-256 of the input). The catalogue of the site is public; no lab data. Builds the site and the engine .wasm.
Run from the repo root: python3 tests_e2e/e2e_flow_parent.py"""
import json, shutil, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi
if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-wasm"); sys.exit(0)
PORT = free_port(); steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:1500]))

tmp = Path(tempfile.mkdtemp(prefix="flow_parent_")); site = tmp / "site"
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import socket
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []
SAVED = "() => JSON.parse(FLOW.parentText(FLOW.current, FLOW.current.plan.inputs.find(i => i.type === 'structure').name))"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1400, "height": 1000}, reduced_motion="reduce").new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
        pg.evaluate("() => { const S = FLOW.current; FLOW.addInput(S.plan, 'structure'); S.redraw(); }")
        BOX = ".flx-parent-box input[role=combobox]"
        def open_node():
            pg.click('.fln:has-text("Parent structure")'); attendi(pg, "document.querySelector('.flx-parent-box input[role=combobox]')")
        def options(): return pg.locator(".flx-parent-box [role=option]").all_inner_texts()

        def empty():
            open_node(); assert "No parent yet." in pg.inner_text(".flx-parent-box [role=status].fl-note"), pg.inner_text(".flx-parent-box")
            assert pg.locator(".flx-parent-kind").is_hidden(), "no formula typed: no question yet"
        step("the node opens with the search box and «No parent yet.»", empty)

        def by_name():
            pg.fill(BOX, "bentazon"); attendi(pg, "document.querySelectorAll('.flx-parent-box [role=option]').length > 0")
            assert "bentazon" in options()[0].lower(), options()
            pg.press(BOX, "ArrowDown"); pg.press(BOX, "Enter")
            attendi(pg, "/M = 240\\.0569/.test(document.querySelector('.flx-parent-box p.fl-note').textContent)")
            j = pg.evaluate(SAVED)
            assert j["formula"] == "C10H12N2O3S" and j["name"] == "bentazon" and j["typed"]["adduct"] == "neutral" and j["smiles"] and j["inchikey"].startswith("ZOMSMJKLGFBRBS"), j
            assert pg.locator(".flx-parent-kind").is_hidden()
            assert pg.evaluate("() => /^[0-9a-f]{64}$/.test(FLOW.current.plan.inputs.find(i => i.type === 'structure').sha256)")
        step("a name is picked from the list with the keyboard and the node holds the molecule", by_name)

        def slip():
            pg.fill(BOX, "bentazan"); attendi(pg, "document.querySelectorAll('.flx-parent-box [role=option]').length > 0")
            assert "bentazon" in options()[0].lower(), options()
            pg.fill(BOX, "ZOMSMJKLGFBRBS"); attendi(pg, "document.querySelectorAll('.flx-parent-box [role=option]').length > 0")
            assert "bentazon" in options()[0].lower(), options()
        step("a slip of one letter and an InChIKey find the same molecule", slip)

        def formula():
            pg.fill(BOX, "C10H13N2O3S"); attendi(pg, "!document.querySelector('.flx-parent-kind').hidden")
            assert pg.input_value(".flx-parent-adduct") == "neutral", "the default is the neutral molecule (M)"
            assert pg.locator(".flx-parent-adduct option").all_inner_texts()[:2] == ["the neutral molecule (M)", "an ion [M+H]+"]
            pg.press(BOX, "Escape"); pg.press(BOX, "Enter")
            attendi(pg, "FLOW.parentText(FLOW.current, FLOW.current.plan.inputs.find(i => i.type === 'structure').name).includes('C10H13N2O3S')")
            assert pg.evaluate(SAVED)["typed"] == {"text": "C10H13N2O3S", "adduct": "neutral"}
            pg.select_option(".flx-parent-adduct", "[M+H]+")
            attendi(pg, "/read from C10H13N2O3S as \\[M\\+H\\]\\+/.test(document.querySelector('.flx-parent-box p.fl-note').textContent)")
            j = pg.evaluate(SAVED)
            assert j["formula"] == "C10H12N2O3S" and abs(j["mass"] - 240.056863) < 1e-6 and j["typed"] == {"text": "C10H13N2O3S", "adduct": "[M+H]+"}, j
            pg.select_option(".flx-parent-adduct", "[M-H]-")   # C10H13N2O3S has no atom to give back as [M-H]-... it gains H: M = C10H14N2O3S
            attendi(pg, "FLOW.parentText(FLOW.current, FLOW.current.plan.inputs.find(i => i.type === 'structure').name).includes('C10H14N2O3S')")
            pg.select_option(".flx-parent-adduct", "[M+Na]+")
            attendi(pg, "/no \\[M\\+Na\\]\\+ atoms/.test(document.querySelector('.flx-parent-box p.fl-note').textContent)")
        step("a formula asks what it is (default M), and the plan keeps the neutral formula of the chosen adduct", formula)

        def mz():
            pg.select_option(".flx-parent-adduct", "[M+H]+")
            pg.fill(BOX, "241.0641"); attendi(pg, "document.querySelectorAll('.flx-parent-box [role=option]').length > 0")
            assert "bentazon" in options()[0].lower(), ("the list is searched with the neutral mass", options())
            pg.press(BOX, "Escape"); pg.press(BOX, "Enter")
            attendi(pg, "!('formula' in JSON.parse(FLOW.parentText(FLOW.current, FLOW.current.plan.inputs.find(i => i.type === 'structure').name)))")
            j = pg.evaluate(SAVED)
            assert abs(j["mass"] - 240.056824) < 1e-5 and j["typed"] == {"text": "241.0641", "adduct": "[M+H]+"}, j
            assert "no formula yet" in pg.inner_text(".flx-parent-box p.fl-note")
        step("an m/z is read under the chosen adduct: the neutral mass is kept and the list is searched with it", mz)

        def kept():
            pg.evaluate("() => FLOW.current.redraw()"); open_node()
            assert pg.input_value(BOX) == "241.0641" and pg.input_value(".flx-parent-adduct") == "[M+H]+" and pg.locator(".flx-parent-kind").is_visible()
        step("the choice survives a redraw of the node", kept)

        def smiles():
            pg.select_option(".flx-parent-adduct", "neutral"); pg.fill(BOX, "CC(C)N1C(=O)c2ccccc2NS1(=O)=O"); pg.press(BOX, "Enter")
            attendi(pg, "FLOW.parentText(FLOW.current, FLOW.current.plan.inputs.find(i => i.type === 'structure').name).startsWith('CC(C)N1')")
            pg.fill(BOX, "nosuchthing"); pg.press(BOX, "Enter")
            attendi(pg, "/No molecule called nosuchthing/.test(document.querySelector('.flx-parent-box p.fl-note').textContent)")
        step("a SMILES is kept as text and an unknown word says so", smiles)
        pg.fill(BOX, "bentazon"); attendi(pg, "document.querySelectorAll('.flx-parent-box [role=option]').length > 0")
        pg.screenshot(path=str(tmp / "parent.png"))
        print("screenshot", tmp / "parent.png")
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
