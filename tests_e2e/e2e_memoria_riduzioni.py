"""C39 phase 2: the memory a tab gives back, proved by STATE (no time is asserted; the only waits are bounded polls of a state).
 1. a closed panel keeps no picture: the canvases it had, still held by a reference, are 1x1; the panel opened again draws (the undo makes a new one);
 2. after the Pipeline of Tabui the workers of its pool are closed once idle (the memory of a .wasm never shrinks) and a second use starts them again;
 3. the cache of scans is limited in bytes as well as in count: the newest scans stay, the oldest leave.
Run from the repo root: python3 tests_e2e/e2e_memoria_riduzioni.py   (needs the built site with static/wasm and static/wasm-feat; SKIP without)"""
import os, sys, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
import dati_sintetici as ds

if not (ROOT / "site" / "static" / "wasm-feat" / "manifest.json").exists():
    print("SKIP site/static/wasm-feat is needed (tools/build_feat_wasm.sh site/static)"); sys.exit(0)

steps = []


def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))
    print(steps[-1][1][:4], name, flush=True)


def series(tmp):
    """Blank, dark and three times: the Pipeline asks for those controls; the roles and times are chosen in its panel."""
    out = []
    for i, (nm, t) in enumerate((("blank", 0), ("dark", 0), ("t0", 0), ("t5", 5), ("t10", 10))):
        f = tmp / f"B_FullMass-{nm}.mzML"; ds.full_scan(f, t, np.random.default_rng(i + 1)); out.append(f)
    return out


def run_pipeline(pg):
    """Open the Pipeline, preview, set roles and times from the names, run on the series; waits for the list of results."""
    pg.evaluate("PIPE.open()"); pg.wait_for_selector("#pipe[open]", timeout=30000)
    pg.click("#pipe-go"); pg.wait_for_function("document.querySelector('#pipe-n').textContent.length > 0", timeout=60000)
    pg.wait_for_selector("#pr-all", timeout=20000)
    rows = [r.split("\t")[0] for r in pg.evaluate("[...document.querySelectorAll('#pr-tab .pr-role')].map(e => e.closest('tr').innerText)")]
    for i, txt in enumerate(rows):
        pg.select_option(f"#pr-r{i}", "blank" if "blank" in txt else "dark" if "dark" in txt else "t0" if "-t0" in txt else "sample")
        tm = {"-t0": "0", "-t5": "5", "-t10": "10"}.get(next((k for k in ("-t10", "-t5", "-t0") if k in txt), ""))
        if tm is not None and not pg.is_disabled(f"#pr-t{i}"): pg.fill(f"#pr-t{i}", tm); pg.press(f"#pr-t{i}", "Enter")
    pg.click("#pr-all"); pg.wait_for_function("!document.querySelector('#pipe-run-go').disabled", timeout=20000)
    pg.click("#pipe-run-go")
    pg.wait_for_function("document.querySelectorAll('#pv-list .pv-row').length > 0 || document.querySelector('#pipe-tab #ft-t')", timeout=120000)


def main():
    r = Run(port=8896, wd="/tmp/wd96")
    tmp = Path(tempfile.mkdtemp(prefix="c39r_"))
    try:
        files = series(tmp)
        with sync_playwright() as p:
            b = p.chromium.launch()
            ctx = b.new_context(viewport={"width": 1400, "height": 900}, reduced_motion="reduce", locale="it-IT")
            ctx.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
            pg = ctx.new_page(); errs = []
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.goto(f"http://127.0.0.1:{r.port}/?perf=1"); attendi(pg, "window.qqReady === true"); ready(pg)
            stage(pg, files); pg.click("#opbtn")
            attendi(pg, "!document.querySelector('#v-data').hidden && PERF.rows.some(x => x.kind === 'disegno' && x.what === 'draw chrom')", 90000); ready(pg)
            pg.evaluate("document.querySelector('#np-map').click()")
            attendi(pg, "PERF.rows.some(x => x.kind === 'disegno' && x.what === 'draw map')", 60000); ready(pg)

            def panels_release():
                n = pg.evaluate("""() => { window.__held = [...document.querySelectorAll('#dpanels canvas')]; return window.__held.map(c => c.width * c.height).filter(a => a > 1).length; }""")
                assert n >= 3, f"the open panels should hold several canvases, {n}"
                pg.evaluate("[...E.panels].forEach(p => closePanel(p))"); ready(pg)
                big = pg.evaluate("window.__held.filter(c => c.width * c.height > 1).map(c => c.id || c.className)")
                assert big == [], f"canvases of closed panels still hold a picture: {big}"
            step("closed panels release their canvases (1x1)", panels_release)

            def panel_reopens():
                pg.evaluate("document.querySelector('#np-map').click()")
                attendi(pg, "document.querySelector('#dpanels canvas') && [...document.querySelectorAll('#dpanels canvas')].some(c => c.width > 100)", 60000); ready(pg)
            step("a panel opened after the close draws again", panel_reopens)

            def scan_cache():
                out = pg.evaluate("""() => {
                  const mk = (i, n) => ({ i, mz: { length: n }, y: { length: n } });          // only the lengths count
                  SC.m.clear(); for (let i = 0; i < 10; i++) SC.m.set('t|' + i, mk(i, 1 << 20));      // 10 x 16 MB
                  scTrim(1); const kept = [...SC.m.keys()], bytes = [...SC.m.values()].reduce((a, s) => a + scBytes(s), 0);
                  SC.m.clear(); for (let i = 0; i < 10; i++) SC.m.set('t|' + i, mk(i, 1 << 20));
                  scTrim(10); const fresh = SC.m.size; SC.m.clear();
                  return { kept, bytes, cap: SC.maxBytes, fresh };
                }""")
                assert out["bytes"] <= out["cap"], out
                assert out["kept"][-1] == "t|9" and "t|0" not in out["kept"], out
                assert out["fresh"] == 10, "the scans just fetched are never evicted by their own fetch: " + str(out)
            step("the scan cache is limited in bytes, the newest stay", scan_cache)

            def pool_closes():
                pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go", timeout=60000)
                pg.wait_for_function("document.querySelectorAll('#tp-files tr').length > 1", timeout=60000)
                run_pipeline(pg)
                assert pg.evaluate("PIPE.state.pool.workers") >= 1, "the pool should hold a worker right after the run"
                # bounded poll of a state: the pool closes its workers once it has been idle for its delay (20 s)
                pg.wait_for_function("PIPE.state.pool.workers === 0", timeout=90000)
            step("the workers of the Pipeline are closed once idle", pool_closes)

            def second_use():
                pg.click("#pipe-go"); pg.wait_for_function("document.querySelector('#pipe-n').textContent.includes('features')", timeout=60000)
                assert pg.evaluate("PIPE.state.pool.workers") >= 1, "the pool starts a worker again on demand"
                assert pg.evaluate("(window.MZLAB_LR_MISS || []).length") == 0
            step("a second use after the close works", second_use)
            assert not errs, errs
            b.close()
    finally:
        r.close()
        import shutil; shutil.rmtree(tmp, ignore_errors=True)
    bad = [s for s in steps if s[1] != "ok"]
    print("\nDONE" if not bad else "\nFAILED: " + "; ".join(f"{n}: {m}" for n, m in bad))
    sys.exit(1 if bad else 0)


main()
