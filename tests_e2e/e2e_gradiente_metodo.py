"""Gradient of a file whose own method document (metodo/1) carries the pump program (.raw, .wiff, .D, Waters): the Gradient section shows it, as read from the file,
instead of «no gradient»; a document without a pump program keeps the «no gradient» section (the answer of api/metodo is replaced in the page: the engine's readers
have their own parity tests)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
D = HERE / "shots" / "gradiente_metodo_tmp"; D.mkdir(parents=True, exist_ok=True)
F = D / "GRADM_t0.mzML"; ds.full_scan(F, 0, np.random.default_rng(3))
STUB = """window.__gm = null; const f0 = window.fetch; window.fetch = (u, o) => String(u).startsWith('api/metodo') && window.__gm
  ? Promise.resolve(new Response(JSON.stringify({metodo: {lc: window.__gm}}), {headers: {'content-type': 'application/json'}})) : f0(u, o); 0"""
LC = "{gradiente: [{t_min: 0, B_pct: 10, flusso_mL_min: 0.3}, {t_min: 2, B_pct: 10, flusso_mL_min: 0.3}, {t_min: 12, B_pct: 90, flusso_mL_min: 0.3}], durata_min: 15}"
r = Run(port=8866, wd="/tmp/wd_gradiente_metodo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(F)])
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate(STUB)
        def none():
            assert pg.evaluate("(async () => await GRADIENTE.get(E.files[0]))()") is None
            pg.click("#np-method"); pg.wait_for_selector("#grad-sec")
            assert pg.is_visible("#grad-new") and not pg.is_visible("#grad-rm")
        step("a method without pump program: no gradient", none)
        def has():
            pg.evaluate("GRADIENTE.forget(); window.__gm = " + LC + "; 0")
            g = pg.evaluate("(async () => await GRADIENTE.get(E.files[0]))()")
            assert g and g["source"] == "file" and [q["b"] for q in g["points"]] == [10, 10, 90] and g["run_time"] == 15, g
        step("a method with a pump program: gradient read from the file", has)
        def shown():
            pg.keyboard.press("Escape"); pg.click("#np-method"); pg.wait_for_selector("#grad-sec #grad-rm")
            assert pg.is_visible("#grad-sec .lct") or pg.is_visible("#grad-sec table")
        step("Method window shows the table of the gradient", shown)
        errs = pg.evaluate("window.MZLAB_LR_MISS || []"); step("no engine misses", lambda: (_ for _ in ()).throw(AssertionError(errs)) if errs else None)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
