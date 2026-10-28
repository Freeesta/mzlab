"""The loading of many files moves nothing: 24 synthetic .mzML are loaded and, from the first row of the table to the last file ready, every frame must show the same table (columns, headings,
bars, status cells, blocks over the table) with all the rows already there; the layout-shift entries of the browser in that window add up to ~0. The one move allowed is the final order of the
rows (after the last file is ready). Needs the site and the .wasm (python tools/build_site.py + the CI build). Run: python3 tests_e2e/e2e_carico_stabile.py"""
import os, shutil, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, attendi, free_port
import synth

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:900]))

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py + the .wasm")); sys.exit(0)

N = 24
D = Path(tempfile.mkdtemp(prefix="stabile_e2e_"))
FILES = synth.make_series(str(D), list(range(0, N * 5, 5)), False, False)
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import urllib.request
for _ in range(50):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)      # the server is starting

PROBE = """() => { window.__snap = []; window.__ls = [];
  const box = e => { if (!e) return 0; const b = e.getBoundingClientRect(); return [Math.round(b.left), Math.round(b.top + scrollY), Math.round(b.width), Math.round(b.height)]; };
  const snap = () => { const t = document.querySelector('#flist .ldtab'); if (!t) return null; const rows = [...t.querySelectorAll('tbody tr')]; if (!rows.some(r => r.classList.contains('cvr'))) return null;
    const mid = rows[Math.floor(rows.length / 2)];
    return JSON.stringify({ th: [...t.querySelectorAll('th')].map(box), tab: box(t), tool: box(document.querySelector('#flist .cvtool')), up: box(document.getElementById('up')), n: rows.length,
      rows: rows.map(r => Math.round(r.getBoundingClientRect().top + scrollY)), bars: rows.map(r => box(r.querySelector('.prog'))),
      cells: [rows[0], mid].map(r => ['[data-cvtxt]', '[data-cvnum]', '[data-cvwhen]'].map(s => box(r.querySelector(s)))) }); };
  const loop = () => { const s = snap(); if (s) window.__snap.push([performance.now(), s]); requestAnimationFrame(loop); }; loop();
  new PerformanceObserver(l => { for (const e of l.getEntries()) window.__ls.push([e.startTime, e.value, e.sources.map(x => [x.node && (x.node.id || x.node.className || x.node.nodeName), x.node && x.node.parentElement && (x.node.parentElement.id || x.node.parentElement.className), JSON.stringify(x.previousRect), JSON.stringify(x.currentRect)])]); }).observe({ type: 'layout-shift', buffered: true }); }"""

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1400}); c.set_default_timeout(120000)
        pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
        pg.evaluate(PROBE)
        pg.set_input_files("#pick", FILES)
        attendi(pg, f"document.querySelectorAll('#flist input[data-k=use]').length >= {N} && !PROGRESSO.busy()", 240000)
        pg.wait_for_timeout(600)
        snaps = pg.evaluate("window.__snap"); ls = pg.evaluate("window.__ls")
        print("   frames with the loading table:", len(snaps))
        def skeleton():
            assert snaps, "the loading table never showed"
            import json
            first = json.loads(snaps[0][1]); assert first["n"] == N, ("rows at the first frame", first["n"])
        step("all the rows are there from the first frame", skeleton)
        def still():
            kinds = {}
            for t, s in snaps: kinds.setdefault(s, []).append(t)
            if len(kinds) > 1:
                import json
                a, b2 = [json.loads(k) for k in list(kinds)[:2]]
                diff = [k for k in a if a[k] != b2[k]]
                raise AssertionError(f"{len(kinds)} different layouts, first difference in {diff}: {[a[k] for k in diff][:1]} -> {[b2[k] for k in diff][:1]}")
        step("columns, headings, bars, status cells and blocks never move until the last file is ready", still)
        def shifts():
            start, end = snaps[0][0], snaps[-1][0]; print('   window', round(start), round(end)); tot = sum(e[1] for e in ls if start + 40 <= e[0] <= end)
            for e in ls:
                if start + 40 <= e[0] <= end: print('   shift', round(e[0]), round(e[1], 4), e[2][:3])
            print("   layout-shift in the loading:", round(tot, 4))
            assert tot < 0.002, tot
        step("no layout-shift entry in the loading", shifts)
        def done():
            assert pg.evaluate("document.querySelectorAll('#flist tr.cvr').length") == 0
            assert pg.evaluate("document.querySelectorAll('#flist input[data-k=use]').length") >= N
        step("at the end the rows are the final ones", done)
        assert not errs, errs
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(D, ignore_errors=True)
