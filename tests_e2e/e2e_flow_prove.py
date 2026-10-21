"""Flow, the button «Show proofs» in the tab «Family» of the Feature table (docs/agenti/flow-grafica.md): the pool of the engine is a stub that answers like `family_record`
(famiglia/1) or fails like the .wasm (error.badRecipe, error.featMemory); the tab asks for the file chosen, filters the families that share an ion with the row, shows the arcs with
their proofs and says what went wrong in words; on a phone it only says the function is for computers. The engine itself is proved by e2e_feat_worker.py.
Run from the repo root: python3 tests_e2e/e2e_flow_prove.py"""
import subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi
PORT = free_port(); steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:1500]))

tmp = Path(tempfile.mkdtemp())
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")], check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import socket
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []
OPEN = """async () => {
  window.__calls = []; window.__mode = "ok";
  const rec = { schema: "famiglia/1", file: "a", families: [
    { family: 1, size: 2, members: [{ row: 4, mz: 301.1, rt: 5.0, role: "M", parent: null }, { row: 5, mz: 323.09, rt: 5.0, role: "adduct", parent: 4 }],
      arcs: [{ lo: 4, up: 5, delta_mz: 21.99, delta_rt: 0, r: 0.99, kind: "adduct", admitted_by: "factor", readings: [], proofs: [{ kind: "coelution", value: 0.99, threshold: 0.9, points: 1, sign: 1 }] }] },
    { family: 2, size: 1, members: [{ row: 9, mz: 500.2, rt: 9.0, role: "M", parent: null }], arcs: [] }] };
  ATTRIBUTIO_CORE.pool = async () => ({ family: async (file, recipe) => { window.__calls.push([file, recipe]);
    if (window.__mode === "recipe") throw Object.assign(new Error("x"), { key: "error.badRecipe", params: {} });
    if (window.__mode === "memory") throw Object.assign(new Error("x"), { key: "error.featMemory", params: { fase: "family_record", stima_mb: 900, budget_mb: 192 } });
    return rec; } });
  PIPEQC.open = async () => ({ row: async id => ({ family: { size: 2, members: [{ mz: 301.1, rt: 5.0, role: "M" }, { mz: 323.09, rt: 5.0, role: "adduct" }] } }) });
  PIPET.mount = (box, o) => { window.__fo = o; box.textContent = "mounted"; return { redraw: () => {} }; };
  const meta = new TextEncoder().encode(JSON.stringify({ rows: 3, files: 2, inputs: [{ name: "a.mzidx" }, { name: "b.mzidx" }], polarity: -1, arrays: [] }));
  const head = new Uint8Array(16); head.set(new TextEncoder().encode("MZFEAT01")); new DataView(head.buffer).setBigUint64(8, BigInt(meta.length), true);
  window.__fo = null; await FLOW.openResult({ type: "table", label: "fam.mzfeat", file: new File([head, meta], "fam.mzfeat"), text: async () => "" });
}"""
FILL = "() => { window.__panel && window.__panel.remove(); window.__panel = document.createElement('div'); document.body.append(window.__panel); __fo.extraTabs[0].fill(window.__panel, { id: 3 }); }"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1400, "height": 1000}).new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=60000); pg.click('#nav button[data-v="flow"]'); pg.wait_for_selector("#flx-root", timeout=30000)
        pg.evaluate(OPEN); attendi(pg, "window.__fo && window.__fo.extraTabs")
        def shown():
            pg.evaluate(FILL); attendi(pg, "window.__panel.querySelector('.fl-proofs button')")
            assert pg.locator(".fl-proofs select option").count() == 2
            pg.select_option(".fl-proofs select", "b"); pg.click(".fl-proofs button")
            attendi(pg, "window.__panel.querySelectorAll('.fl-proofs tbody tr').length === 1")
            call = pg.evaluate("window.__calls[0]")
            assert call[0] == {"name": "b", "path": "mzlab/idx/b.mzidx"} and '"polarity":-1' in call[1] and '"mode":"family"' in call[1], call
            t = pg.inner_text(".fl-proofs table")
            assert "301.1000" in t and "323.0900" in t and "coelution 0.99 (threshold 0.9)" in t and "factor" in t, t
            assert pg.locator(".fl-proofs table").count() == 1, "the family without an ion of the row is left out"
        step("«Show proofs» asks the engine for the file chosen and lists the arcs of the family with their proofs", shown)
        def errors():
            for mode, text in (("recipe", "parameters of the pipeline are not valid"), ("memory", "about 900 MB needed of 192 MB")):
                pg.evaluate(f"window.__mode = '{mode}'"); pg.evaluate(FILL); attendi(pg, "window.__panel.querySelector('.fl-proofs button')")
                pg.click(".fl-proofs button"); attendi(pg, "window.__panel.querySelector('.fl-proofs [role=alert]')")
                assert text in pg.inner_text(".fl-proofs [role=alert]"), pg.inner_text(".fl-proofs")
                assert pg.locator(".fl-proofs button").is_enabled()
        step("an error of the engine is told in words and the button works again", errors)
        def phone():
            pg.evaluate("() => { const real = matchMedia.bind(window); window.matchMedia = q => (/coarse/.test(q) ? { matches: true, media: q, addEventListener() {}, removeEventListener() {} } : real(q)); }")
            pg.evaluate(FILL); attendi(pg, "window.__panel.querySelector('.fl-proofs')")
            assert "designed for computers" in pg.inner_text(".fl-proofs") and pg.locator(".fl-proofs button").count() == 0
        step("on a phone the tab says the function is designed for computers", phone)
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
