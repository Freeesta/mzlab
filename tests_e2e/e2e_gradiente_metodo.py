"""Gradient of a file whose own method document (method/1) carries the pump program (.raw, .wiff, .D, Waters): the Gradient section shows it, as read from the file,
and the section is read-only (no insert / edit / import / course buttons); a document without a pump program shows no gradient section (the answer of api/metodo is replaced in the page: the engine's readers
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
# api/metodo feeds GRADIENTE.get (the overlay); api/filemethod feeds the Method window (its `lc` view is made in Rust: the real chain is tested in e2e_metodo_onesto)
STUB = """window.__gm = null; window.__gf = null; const f0 = window.fetch; window.fetch = (u, o) => {
  const j = x => Promise.resolve(new Response(JSON.stringify(x), {headers: {'content-type': 'application/json'}}));
  if (String(u).startsWith('api/metodo') && window.__gm) return j({method: {lc: window.__gm}});
  if (String(u).startsWith('api/filemethod') && window.__gf) return j(window.__gf);
  return f0(u, o); }; 0"""
GF = """({schema: 'file_method/1', segments: [], lc: {run_time_min: 15, run_time_shown: '15.00', flow_mL_min: 0.3, flow_shown: '0.30', oven_C: null, oven_shown: null, eluents: [], hold: {from_min: 12, to_min: 15, B_pct: 90, flow_mL_min: 0.3, shown: {from: '12.00', to: '15.00', a: '10.00', b: '90.00', flow: '0.30'}},
  gradient: [[0, 10], [2, 10], [12, 90]].map(([t, b]) => ({t_min: t, B_pct: b, flow_mL_min: 0.3, shown: {t: t.toFixed(2), a: (100 - b).toFixed(2), b: b.toFixed(2), flow: '0.30'}})),
  sources: {gradient: {kind: 'read', at: 'test stream'}, run_time: {kind: 'read', at: 'test stream'}, flow: {kind: 'read', at: 'test stream'}, a_pct: {kind: 'inferred', at: '100 - %B'}, hold: {kind: 'inferred', at: 'held'}, eluents: null, oven: null}}, dad: null})"""
LC = "{gradient: [{t_min: 0, B_pct: 10, flow_mL_min: 0.3}, {t_min: 2, B_pct: 10, flow_mL_min: 0.3}, {t_min: 12, B_pct: 90, flow_mL_min: 0.3}], duration_min: 15}"
r = Run(port=8866, wd="/tmp/wd_gradiente_metodo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(F)])
        pg.click("text=Load data"); ready(pg)
        pg.evaluate(STUB)
        def none():
            assert pg.evaluate("(async () => await GRADIENTE.get(E.files[0]))()") is None
            pg.click("#np-method"); pg.wait_for_selector("#bigdlg[open]")
            assert not pg.query_selector("#grad-sec")
        step("a method without pump program: no gradient and no gradient section", none)
        def has():
            pg.evaluate("GRADIENTE.forget(); window.__gm = " + LC + "; window.__gf = " + GF + "; 0")
            g = pg.evaluate("(async () => await GRADIENTE.get(E.files[0]))()")
            assert g and g["source"] == "file" and [q["b"] for q in g["points"]] == [10, 10, 90] and g["run_time"] == 15, g
        step("a method with a pump program: gradient read from the file", has)
        def shown():
            pg.keyboard.press("Escape"); pg.click("#np-method"); pg.wait_for_selector("#grad-sec")
            assert pg.is_visible("#grad-sec .lct") or pg.is_visible("#grad-sec table")
        step("Method window shows the table of the gradient read from the file", shown)
        def readonly():
            for i in ("grad-new", "grad-edit", "grad-rm", "grad-raw", "grad-csv", "grad-course", "grad-last", "grad-ed"): assert not pg.query_selector("#" + i), i
            assert not pg.query_selector("#grad-sec button")
            txt = pg.inner_text("#bigdlg").lower(); assert "corso" not in txt and "course" not in txt
        step("the gradient section has no buttons and no mention of the course", readonly)
        errs = pg.evaluate("window.MZLAB_LR_MISS || []"); step("no engine misses", lambda: (_ for _ in ()).throw(AssertionError(errs)) if errs else None)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
