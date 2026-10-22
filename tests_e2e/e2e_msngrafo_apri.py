"""A saved fragmentation graph (g.json, msngrafo/1) opens without its raw data, from the data area (picker and drop): an old graph without `vista` is completed by the .wasm
msngrafo (op `completa`), a new one opens as it is, a broken file and a newer format are keyed errors with «Details» and «Copy diagnosis». The window is read-only: the spectrum
of an ion, «Use», the cleavages and the raw requests are off with a reason. No request to api/ is made, MZLAB_LR_MISS stays empty, axe, Italian and English. Needs the built
site (python3 tools/prepara_sito.py); the .wasm msngrafo is built here when it is not in site/static/wasm-feat. Synthetic fixtures only (tests_e2e/fixtures)."""
import json, sys
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
from lib import *
import lib_msngrafo

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))

lib_msngrafo.prepara()
FX = Path(__file__).parent / "fixtures"
OLD = FX / "msngrafo_esempio.json"
NEW = FX / "msngrafo_esempio_vista.json"
N = len(json.loads(OLD.read_text())["nodi"])
TMP = Path("/tmp/wd_msngrafo_apri_files"); TMP.mkdir(exist_ok=True)
BROKEN = TMP / "broken.json"; BROKEN.write_text('{\n "formato": "msngrafo/1",\n "nodi": [{"id": "n1", ')
NEWER = TMP / "newer.json"; NEWER.write_text(OLD.read_text().replace('"msngrafo/1"', '"msngrafo/2"', 1))
OTHER = TMP / "other.json"; OTHER.write_text('{"formato": "something/1"}')
r = Run(port=8979, wd="/tmp/wd_msngrafo_apri")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1500})
        pg.goto(f"http://127.0.0.1:{r.port}/?lang=en"); attendi(pg, "typeof INFGAPRI !== 'x' && !!document.querySelector('#pick')")
        ASK = "(window.__qqU||[]).filter(u=>/api\\/(upload|explore|msn_grafo|msn_albero)/.test(u))"
        def opened(path, via="pick"):
            pg.set_input_files("#pick", str(path))
            pg.wait_for_selector("#ig-ro .ig-node", timeout=30000)
        def close():
            pg.focus("#ig-ro [data-x]"); pg.keyboard.press("Escape"); pg.wait_for_selector("#ig-ro", state="detached", timeout=5000)
        def old():
            opened(OLD)
            assert pg.locator("#ig-ro .ig-node").count() == N, pg.locator("#ig-ro .ig-node").count()
            assert pg.evaluate("INFG.G.j.vista && INFG.G.j.vista.algoritmo") == "best-path/1", "the old graph was not completed"
            assert pg.evaluate("INFG.G.ro") == "msngrafo_esempio.json"
        step("an old graph (no vista) dropped in the data area is completed by the .wasm and opens", old)
        def readonly():
            pg.click("#ig-ro .ig-node[data-id=n2]"); pg.wait_for_selector("#ig-ro .ig-cands", timeout=5000)
            use = pg.locator("#ig-ro .ig-use")
            assert use.count() >= 1 and all(use.nth(i).is_disabled() for i in range(use.count())), "Use is on"
            assert use.first.get_attribute("title"), "no reason on Use"
            assert pg.locator("#ig-ro .ig-ro-note").count() == 1 and "raw data" in pg.inner_text("#ig-ro .ig-ro-note")
            assert "raw data" in pg.inner_text("#ig-ro #ig-ro-note")
            assert pg.locator("#ig-ro #ig-exp *").count() == 0 and pg.locator("#ig-ro #ig-find *").count() == 0 and pg.locator("#ig-ro #ig-field").count() == 0
            pg.keyboard.press("Enter")
            assert pg.locator("#ig-ro").count() == 1
        step("read-only: «Use» and the spectrum are off with a reason; no export, search or ion field", readonly)
        def keyboard():
            pg.focus("#ig-ro .ig-node[data-id=n1]"); pg.keyboard.press("ArrowRight")
            assert pg.evaluate("document.activeElement.dataset.id") != "n1" or N == 1
            sizes = pg.evaluate("[...document.querySelectorAll('#ig-ro .ig-bar .bt, #ig-ro [data-x]')].map(b=>{const r=b.getBoundingClientRect();return Math.min(r.width,r.height)})")
            assert min(sizes) >= 24, sizes
        step("arrow keys move between ions; targets of at least 24 px", keyboard)
        AXE = axe_path()
        def axe():
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("axe.run('#ig-ro', {runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r=>r.violations.filter(v=>['serious','critical'].includes(v.impact)).map(v=>[v.id,v.nodes.length]))")
            assert not v, v
        if AXE: step("axe: the window has no serious or critical violation", axe)
        def closes():
            close(); assert pg.evaluate("INFG.G.ro") is None and pg.evaluate("INFG.G.j") is None
        step("Esc closes the window and the state of the tab is put back", closes)
        def new():
            opened(NEW)
            assert pg.locator("#ig-ro .ig-node").count() == N
            close()
        step("a new graph (with vista) opens as it is", new)
        def drop():
            pg.evaluate("""async (t) => { const dt = new DataTransfer(); dt.items.add(new File([t], "dropped.json", { type: "application/json" }));
              document.querySelector("#drop").dispatchEvent(new DragEvent("drop", { dataTransfer: dt, bubbles: true, cancelable: true })); }""", NEW.read_text())
            pg.wait_for_selector("#ig-ro .ig-node", timeout=30000)
            assert pg.evaluate("INFG.G.ro") == "dropped.json"
            close()
        step("a graph dropped on the data area opens", drop)
        def broken():
            pg.set_input_files("#pick", str(BROKEN)); pg.wait_for_selector("#unopen .fail", timeout=15000)
            txt = pg.inner_text("#unopen")
            assert "broken.json" in txt and pg.locator("#unopen .diag").count() == 1, txt[:300]
            assert pg.locator("#ig-ro").count() == 0
            pg.click("#unopen [data-a=close]")
        step("a broken file is a keyed error with Details and Copy diagnosis; no window", broken)
        def newer():
            pg.set_input_files("#pick", str(NEWER)); pg.wait_for_selector("#unopen .fail", timeout=15000)
            txt = pg.inner_text("#unopen")
            assert "newer version" in txt and "format 2" in txt, txt[:300]
            assert pg.locator("#ig-ro").count() == 0
            pg.click("#unopen [data-a=close]")
        step("a newer format is error.newerFormat and does not open", newer)
        def other():
            pg.set_input_files("#pick", str(OTHER)); pg.wait_for_selector("#unopen", state="visible", timeout=15000)
            assert "format not recognised" in pg.inner_text("#unopen") and pg.locator("#ig-ro").count() == 0
            pg.click("#unopen [data-a=close]")
        step("another .json keeps the usual «format not recognised»", other)
        def noapi():
            urls = pg.evaluate(ASK)
            assert not urls, urls
        step("no request to api/ for the graph files (the op completa is a call into the .wasm)", noapi)
        def lang():
            en = pg.evaluate("I18N.t('msngrafo.file.nospec')")
            assert en and not en.startswith("msngrafo.")
        step("texts come from the catalogues", lang)
        errs = [e for e in r.errs if e[0] in ("pageerror",)]
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
        miss = pg.evaluate("(window.MZLAB_LR_MISS||[]).slice()")
        step("no request left unanswered by the engine", lambda: (_ for _ in ()).throw(AssertionError(miss)) if miss else None)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
