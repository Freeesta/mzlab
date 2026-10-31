"""The loading of many files moves nothing: 24 synthetic .mzML are loaded and, from the first progress row of the side list to the end of the loading, every frame shows the same
landing (drop area, its box, the box of the notices #up: no card, no bar over it; the sample table grows under it) and every file has its row in the side list from the first frame; the
layout-shift entries of the work area in that window add up to ~0 (the rows of the side list go one by one as their files open). The first file read opens by itself (once): the landing frames come first, then only
the frames of the open panels, never back; at the end the other files wait in the list and no loading row is left. Needs the
site and the .wasm (python tools/build_site.py + the CI build). Run: python3 tests_e2e/e2e_carico_stabile.py"""
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
  const snap = () => { if (!(UPBUSY > 0 || OPENNOW.busy() > 0) || !document.querySelector('#fl-up [data-up]')) return null;
    return JSON.stringify({ work: document.getElementById('v-workspace').dataset.work, drop: box(document.getElementById('drop')), start: box(document.getElementById('start')), up: box(document.getElementById('up')),
      table: !document.getElementById('wtable').hidden, card: !!document.querySelector('.convprog'), rows: document.querySelectorAll('#fl-up [data-up]').length }); };
  const loop = () => { const s = snap(); if (s) window.__snap.push([performance.now(), s]); requestAnimationFrame(loop); }; loop();
  const el = n => n && (n.nodeType === 1 ? n : n.parentElement);
  new PerformanceObserver(l => { for (const e of l.getEntries()) window.__ls.push([e.startTime, e.value, e.sources.filter(x => { const n = el(x.node); return !(n && n.closest('#dfiles')); }).length,
    e.sources.map(x => [x.node && (x.node.id || x.node.className || x.node.nodeName), JSON.stringify(x.previousRect), JSON.stringify(x.currentRect)])]); }).observe({ type: 'layout-shift', buffered: true }); }"""

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
        attendi(pg, f"document.querySelectorAll('#flist input[data-k=use]').length >= {N} && !PROGRESSO.busy() && UPBUSY === 0 && OPENNOW.busy() === 0", 240000)
        attendi(pg, "(window.__end = window.__end || performance.now()) && performance.now() - window.__end > 600", 10000)      # some frames after the end
        snaps = pg.evaluate("window.__snap"); ls = pg.evaluate("window.__ls")
        print("   frames of the loading:", len(snaps))
        import json
        def skeleton():
            assert snaps, "the loading never showed its rows"
            first = json.loads(snaps[0][1]); assert first["rows"] == N, ("rows at the first frame", first["rows"])
        step("every file has its row in the side list from the first frame", skeleton)
        def still():
            kinds = {}; seen_panels = False
            for fr in snaps:
                # the sample table appears under the drop area once it has rows: the boxes above it stay
                d = json.loads(fr[1]); d.pop("rows"); d.pop("table")
                if d["work"] != "landing": seen_panels = True
                else: assert not seen_panels, "the landing came back after the first file opened"
                if d["work"] == "landing": kinds.setdefault(json.dumps(d, sort_keys=True), []).append(fr[0])
            assert len(kinds) <= 1, f"{len(kinds)} different layouts of the landing: {list(kinds)[:2]}"
            for k in kinds: assert not json.loads(k)["card"], k
        step("the landing (drop area, its box, the notices) never moves until the first file opens by itself: no card, never back", still)
        def shifts():
            first_open = next((fr[0] for fr in snaps if json.loads(fr[1])["work"] != "landing"), snaps[-1][0])
            start, end = snaps[0][0], first_open
        def shifts():
            print('   window', round(start), round(end))
            tot = sum(e[1] for e in ls if start + 40 <= e[0] <= end and e[2])
            for e in ls:
                if start + 40 <= e[0] <= end and e[2]: print('   shift', round(e[0]), round(e[1], 4), e[3][:3])
            print("   layout-shift of the work area in the loading:", round(tot, 4))
            assert tot < 0.002, tot
        step("no layout-shift of the work area from the first row to the end", shifts)
        def done():
            attendi(pg, "document.querySelectorAll('#fl-up [data-up]').length === 0", 30000)
            assert pg.evaluate("document.getElementById('v-workspace').dataset.work === 'panels' && E.files.filter(f => f.vis && !f.gone).length === 1"), "only the first file opened by itself"
            assert not pg.is_visible("#drop"), "drop area gone with the open panels"
            assert pg.evaluate("document.querySelectorAll('#flist input[data-k=use]').length") >= N
        step("at the end only the first file is open, the others wait in the list and no loading row is left", done)
        assert not errs, errs
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(D, ignore_errors=True)
