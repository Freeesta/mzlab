"""The chromatograms of a .wiff sample read by the engine on the site (synthetic pairs of tools/wiff_cromatogrammi.py: Full Scan and MRM experiments in one
sample, two MRM experiments, Full Scan alone; no laboratory data): the index counts one TIC per experiment with data and the SRM chromatograms, as
`mzlab-cli convert` lists them (tests/test_wiff_cromatogrammi.py compares the arrays); an index of an older generation of the readers (`"reader":1` in its
`source`) is refused when the sample is opened again and built once more. Needs the site and the .wasm (python3 tools/prepara_sito.py).
Run from the repo root: python3 tests_e2e/e2e_wiff_cromatogrammi.py"""
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="wiff_crom_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_cromatogrammi.py"), str(D)], check=True, capture_output=True)
WANT = {n: len([x for x in ids if not x.startswith("BPC")]) for n, ids in json.loads((D / "attesi.json").read_text()).items()}   # the BPC is listed once the scans were decoded
PAIRS = [str(D / f"{n}.wiff{x}") for n in WANT for x in ("", ".scan")]
INIT = "try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
DONE = f"ST.files.length === {len(WANT)} && ST.files.every(f => f.kind) && UPBUSY === 0 && OPENNOW.busy() === 0"
COUNTS = "() => Object.fromEntries(E.files.map(f => [String(f.file).split('#')[0].replace(/\\.mzML$/, ''), f.chromatograms]))"      # the items of a mixed sample share its count
# the index of a sample with `"reader":2` written as `"reader":1` (same length: the offsets of the index stay right)
AGE = """async n => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('idx');
  const h = await d.getFileHandle(n + '.mzidx'), b = new Uint8Array(await (await h.getFile()).arrayBuffer()), t = new TextDecoder('latin1').decode(b);
  const i = t.lastIndexOf('"reader":2'); if (i < 0) return false; b[i + 9] = 0x31;
  const w = await h.createWritable(); await w.write(b); await w.close(); return true; }"""
READER = """async n => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('idx');
  const t = new TextDecoder('latin1').decode(await (await (await d.getFileHandle(n + '.mzidx')).getFile()).arrayBuffer()); return /"reader":(\\d+)/.exec(t)?.[1] || null; }"""

r = Run(port=8830 + 152, wd="/tmp/wd_wiff_crom")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); c = b.new_context(viewport={"width": 1400, "height": 900}); c.set_default_timeout(120000); c.add_init_script(INIT)
        def opened(pg):
            pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#drop", state="attached"); wait_for_js(pg, "window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=120000)
            return pg
        A = opened(c.new_page())
        A.set_input_files("#pick", PAIRS); wait_for_js(A, DONE, timeout=120000)
        def counts():
            got = A.evaluate(COUNTS)
            assert got == WANT, (got, WANT)
        step("one TIC per experiment with data and the SRM chromatograms after it (mixed, two MRM, Full Scan alone)", counts)
        name = next(f for f in A.evaluate("ST.files.map(f => f.name)") if f.startswith("fsmrm"))
        step("the index carries the generation of the readers", lambda: (A.evaluate(READER, name) == "2") or (_ for _ in ()).throw(AssertionError(A.evaluate(READER, name))))
        aged = A.evaluate(AGE, name)
        A.close(); B = opened(c.new_page())
        def rebuilt():
            assert aged, "no reader key in the index"
            try: wait_for_js(B, DONE, timeout=60000)
            except Exception: raise AssertionError(B.evaluate("JSON.stringify({n: ST.files.length, f: ST.files.map(f => [f.name, f.kind, f.indexed]), up: UPBUSY, open: OPENNOW.busy(), e: E.files.length})"))
            wait_for_js(B, f"(async () => {{ const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('idx'); return /\"reader\":2/.test(new TextDecoder('latin1').decode(await (await (await d.getFileHandle({json.dumps(name)} + '.mzidx')).getFile()).arrayBuffer())); }})()", timeout=120000)
            got = B.evaluate(COUNTS)
            assert got == WANT, (got, WANT)
        step("an index of an older generation is built again once and gives the same list", rebuilt)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
bad = [n for n, s in steps if s != "ok"]
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report() if bad else None
sys.exit(1 if bad else 0)
