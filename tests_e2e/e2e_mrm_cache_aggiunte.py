# e2e_mrm_cache_aggiunte: a list of MRM files opens file by file, and every file appended to the session must keep the answers already fetched for the open ones
# (bootSession0, explore.js: answers keyed by file index k stay valid when files are only appended). Before, each step cleared the memo cache and the shown
# file asked api/mrm again at every new file (N files, N² requests). Synthetic files (tools/dati_sintetici.py). Needs the site and the .wasm: python3 tools/prepara_sito.py
import os, sys, tempfile, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
import dati_sintetici as ds
from pathlib import Path
N = 8
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
# every api/mrm request of the page, by its query (fetch is what J of explore.js calls)
COUNT = """() => { window.__mrm = {}; const f = window.fetch; window.fetch = (u, o) => { const s = String(u); if (s.includes('api/mrm?')) window.__mrm[s] = (window.__mrm[s] || 0) + 1; return f(u, o); }; }"""
d = Path(tempfile.mkdtemp(prefix="mrmcache_"))
files = []
for i in range(N):
    p = d / f"MRM_s{i:02d}.mzML"; ds.mrm(p, 1.0 + 0.1 * i, np.random.default_rng(20 + i), 1 + i); files.append(str(p))
def no_refetch():
    r = Run(port=8856, wd="/tmp/wd_mrm_cache")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            pg.evaluate(COUNT)
            pg.set_input_files("#pick", files)
            # the first file shown as soon as it opens, while the others are still being read and appended
            attendi(pg, "typeof E !== 'undefined' && E.files.filter(f => !f.gone).length >= 1", 120000)
            pg.evaluate("() => { if (!E.files.some(f => f.vis && !f.gone)) soloFile(E.files.find(f => !f.gone)); }")
            attendi(pg, f"E.files.filter(f => !f.gone).length === {N} && UPBUSY === 0 && OPENNOW.busy() === 0", 180000)
            ready(pg)
            n = pg.evaluate("window.__mrm")
            shown = pg.evaluate("E.files.find(f => f.vis && !f.gone).k")
            mine = sum(v for u, v in n.items() if f"k={shown}&" in u + "&" and "ref=" not in u)
            assert mine >= 1, f"the shown file never asked its transitions: {n}"
            assert mine <= 2, f"file {shown} asked api/mrm {mine} times while {N - 1} files were appended: {n}"
            assert sum(n.values()) <= 2 * N, f"{sum(n.values())} api/mrm requests for {N} files: {n}"
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
    finally:
        r.close()
step(f"{N} MRM files appended one by one: the shown file asks its transitions at most twice, all files at most 2N times", no_refetch)
shutil.rmtree(d, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
