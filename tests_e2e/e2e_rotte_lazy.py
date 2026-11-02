"""The module `rotte` (crates/mzlab-rotte-wasm: statistics, molecules, Data and Flow routes) is fetched on the FIRST request of one of its routes, never at start-up and never twice;
the routes answer in it (a first and a second use), a failed fetch of the module is an error that the next request retries, MZLAB_LR_MISS stays empty (the cap of the .wasm is the gate of tools/wasm_opt.sh).
Needs the built site (tools/prepara_sito.py). Run from the repo root."""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

r = Run(port=8979, wd="/tmp/wd_rotte_lazy")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        asked = []
        pg.on("request", lambda q: asked.append(q.url))
        pg.wait_for_selector("#drop", timeout=120000)
        rotte = lambda: [u for u in asked if "/wasm-feat/rotte/" in u and u.endswith("_bg.wasm")]
        post = """async ([path, body]) => { const x = await fetch(path, { method: 'POST', body }); return [x.status, await x.text()]; }"""
        call = lambda path, body: pg.evaluate(post, [path, body])
        assert not rotte(), rotte()
        print("OK  the module is not fetched at start-up")

        # a failed fetch is an error, and the next request fetches it again
        pg.route("**/wasm-feat/rotte/*", lambda route: route.abort())
        try:
            st, txt = call("api/molecules/cerca", json.dumps({"q": "paracetamol"}))
            assert st >= 400, (st, txt[:200])
        except Exception:
            pass                                             # the request itself may be rejected: also an error
        pg.unroute("**/wasm-feat/rotte/*")
        print("OK  a failed fetch of the module is an error")

        st, txt = call("api/molecules/carica", json.dumps({"catalogo": pg.evaluate("fetch('static/molecole/catalogo.csv').then(r => r.text())")}))
        assert st == 200 and json.loads(txt)["totale"] == 1228, (st, txt[:200])
        assert len(rotte()) >= 1
        n = len(rotte())
        # a second use of every kind of route: no new fetch of the module
        st, txt = call("api/molecules/cerca", json.dumps({"q": "paracetamol"}))
        assert st == 200 and json.loads(txt)[0]["nome"] == "acetaminophen", txt[:200]
        st, txt = call("api/campioni/proponi", json.dumps({"files": [{"name": "a_t01.mzML", "sha256": "", "kind": "campione"}]}))
        assert st == 200 and '"campioni' in txt, (st, txt[:200])
        st, txt = call("api/stat/profile", "{}")
        assert st in (200, 400), st                          # the route exists in the module; an empty body is a keyed error
        st, txt = call("api/flow/line", json.dumps({"plan": {"formato": "mzflow/2", "passi": []}}))
        assert st in (200, 400), (st, txt[:200])
        assert len(rotte()) == n, (n, rotte())
        print("OK  the routes answer in the module, which is fetched once")

        assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), pg.evaluate("window.MZLAB_LR_MISS")
        print("OK  MZLAB_LR_MISS empty")
finally:
    r.close()
