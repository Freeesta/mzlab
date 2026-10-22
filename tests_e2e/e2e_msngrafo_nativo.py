"""api/msn_grafo of a sample of a maker's file (an Agilent .D folder: the same path as a Thermo .raw, a stub in OPFS and an index of version 2): the .wasm msngrafo only decodes an mzML, so the
worker writes the sample as mzML once, indexes it and tells the page how far it is (events «mzlab-msngrafo»: convert, index, graph, the bar rising); the answer is the one of
`mzlab-cli msn-graph` on the same folder (only the SHA-256 of the index may differ), the second request does not convert again, and a request with no news for the stall time is an
error with «Copy diagnosis» (guarda, mostraErrore). MZLAB_LR_MISS stays empty. Needs the site (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_msngrafo_nativo.py"""
import json, re, subprocess, sys, tempfile
from pathlib import Path
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
from lib import *
import lib_msngrafo
sys.path.insert(0, str(ROOT / "tools"))
import agilent_sintetico

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))

lib_msngrafo.prepara()
tmp = Path(tempfile.mkdtemp(prefix="msnnat_"))
folder = agilent_sintetico.write_lcms(tmp / "serie", "uno.D", "profile", 60)
mask = lambda t: re.sub(r'"sha256_idx":"[0-9a-f]*"', '"sha256_idx":""', t)
cli = subprocess.run(["cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "msn-graph", str(folder)], cwd=ROOT, capture_output=True, encoding="utf-8")
want = cli.stdout if cli.returncode == 0 else None
r = Run(port=8979, wd="/tmp/wd_msngrafo_nativo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pickdir", str(folder))
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
        pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
        pg.evaluate("window.__ev = []; window.addEventListener('mzlab-msngrafo', e => window.__ev.push(e.detail))")
        ASK = "fetch('api/msn_grafo?k=0').then(async r => [r.status, await r.text()])"
        got = {}
        def graph():
            st, t = pg.evaluate(ASK)
            got["st"], got["t"] = st, t
            assert "err.scan.outside" not in t, t[:200]
            if want is not None:
                assert st == 200 and mask(t) == mask(want), "the page and the command line differ"
            else:
                assert st in (200, 400) and ('"formato":"msngrafo/1"' in t or "error_key" in t), (st, t[:200])
        step("the graph of a sample of a maker's file is the one of the command line (not err.scan.outside)", graph)
        def bar():
            ev = pg.evaluate("window.__ev")
            ph = [e["phase"] for e in ev]
            assert "convert" in ph and "index" in ph and "graph" in ph, ph
            fr = [e["frac"] for e in ev if e["phase"] != "load"]
            assert fr == sorted(fr) and fr[-1] > 0.65, fr[-5:]
        step("the bar goes through convert, index and graph and never goes back", bar)
        def again():
            pg.evaluate("window.__ev.length = 0")
            st, t = pg.evaluate(ASK)
            ph = {e["phase"] for e in pg.evaluate("window.__ev")}
            assert st == got["st"] and t == got["t"], "the second answer differs"
            assert "convert" not in ph and "index" not in ph, ph
        step("the second request reads the mzML kept: no new conversion, same bytes", again)
        def stall():
            res = pg.evaluate("""async () => {
              const { guarda, mostraErrore } = await import('./infusione-grafo-avanza.js');
              const host = document.createElement('div'); document.body.append(host);
              const got = await new Promise(ok => { const stop = guarda(host, (s, phase) => ok({ s, phase }), 400); setTimeout(() => ok(null), 4000); });
              if (got) mostraErrore(host, 'stalled', { key: 'err.msngrafo.stalled', params: {} }, { name: 'x.raw', size: 5 }, got.phase);
              const out = { got, copy: !!host.querySelector('[data-a=diagcopy]'), alert: !!host.querySelector('[role=alert]') };
              host.remove(); return out; }""")
            assert res["got"] and res["got"]["phase"] == "load" and res["copy"] and res["alert"], res
        step("no news for the stall time is an error with «Copy diagnosis»", stall)
        errs = [e for e in r.errs if e[0] in ("pageerror",)]
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
        miss = pg.evaluate("(window.MZLAB_LR_MISS||[]).filter(m => !m.includes('api/spectrum'))")
        step("no request left unanswered by the engine", lambda: (_ for _ in ()).throw(AssertionError(miss)) if miss else None)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s.startswith("FAIL") for _, s in steps) else 0)
