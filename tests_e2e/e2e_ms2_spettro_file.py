"""The product-ion spectrum never asks a Full Scan file for MS2 scans (synthetic pairs of tools/wiff_misto.py and tools/wiff_modi.py, EPI of 55.5; no laboratory data):
when the file of the spectrum lacks its precursor (the pair is switching, or the student picked another precursor) the spectrum falls back to a loaded file
whose precursors include it, not to a file without a precursor list (a Full Scan sample, answered «route unavailable»), also when that sample is the target. Also after quick file switches.
Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_ms2_spettro_file.py"""
import json, random, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="ms2_spf_"))
for tool in ("wiff_modi.py", "wiff_misto.py"):
    subprocess.run([sys.executable, str(ROOT / "tools" / tool), str(D)], check=True, capture_output=True)
FILES = [D / "misto.wiff", D / "misto.wiff.scan", D / "modi" / "EPI.wiff", D / "modi" / "EPI.wiff.scan"]
# every MS2 spectrum request sent to a file that is not an MS2 experiment
HOOK = """() => { window.__bad = []; const f0 = window.fetch; window.fetch = function (u) { const m = /api\\/spectrum\\?k=(\\d+).*level=2/.exec(String(u));
  if (m && E.files[+m[1]] && E.files[+m[1]].kind !== 'ms2') window.__bad.push([String(u), E.files[+m[1]].file, E.files[+m[1]].kind]); return f0.apply(this, arguments); }; }"""
SPEC = "E.panels.find(p => p.tab === 'ms2' && p.type === 'spec' && p.el && p.el.offsetParent)"

def clean(pg):
    bad = pg.evaluate("window.__bad"); miss = pg.evaluate("window.MZLAB_LR_MISS || []")
    assert not bad and not miss, (bad, miss)

r = Run(port=8876, wd="/tmp/wd_ms2_spf")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, FILES); open_first(pg); ready(pg); pg.evaluate(HOOK)
        k_epi = pg.evaluate("E.files.find(f => f.file === 'EPI_s1.mzML' && !f.gone).k")
        def other_precursor():
            pg.evaluate("k => soloFile(E.files[k])", k_epi); ready(pg); frames(pg, 3)
            pg.wait_for_function(f"(() => {{ const s = {SPEC}; return s && s._a && s._a.data && s._a.data.length; }})()", timeout=30000)
            # the spectrum of the EPI file (55.5 only) set to 305, a precursor of the product-ion samples of misto: it falls back to one of them
            pg.evaluate(f"(() => {{ const s = {SPEC}; s.prec = 305; return draw(s); }})()"); ready(pg); frames(pg, 3)
            got = pg.evaluate(f"(() => {{ const s = {SPEC}; return (s._a && s._a.data || []).map(x => [x.f.file, x.f.kind, x.d.mz.length]); }})()")
            assert got and all(g[1] == "ms2" and g[2] > 0 for g in got), got
            clean(pg)
        step("a precursor the file of the spectrum lacks: the spectrum falls back to an MS2 file that has it", other_precursor)
        def full_scan_target():
            # the spectrum pointed at a Full Scan sample (no precursor list, no MS2 scans) with a precursor set: it falls back, never asks that file
            k_full = pg.evaluate("E.files.find(f => !f.gone && f.kind === 'full' && !(f.precursors || []).length && !f.ms2).k")
            pg.evaluate(f"""(async () => {{ const s = {SPEC}, b = E.browse; E.browse = false; s.k = {k_full}; s.kSet = true; s.prec = 305;
              try {{ await draw(s); }} finally {{ E.browse = b; }} }})()"""); ready(pg); frames(pg, 3)
            got = pg.evaluate(f"(() => {{ const s = {SPEC}; return (s._a && s._a.data || []).map(x => [x.f.file, x.f.kind, x.d.mz.length]); }})()")
            assert got and all(g[1] == "ms2" and g[2] > 0 for g in got), got
            clean(pg)
        step("a Full Scan file as the target of a precursor spectrum: the spectrum falls back to an MS2 file that has it", full_scan_target)
        def switching():
            ks = pg.evaluate("E.files.filter(f => !f.gone).map(f => f.k)"); rnd = random.Random(1)
            for _ in range(3):
                rnd.shuffle(ks)
                for k in ks: pg.evaluate("k => soloFile(E.files[k])", k)
                ready(pg)
                for k in ks: pg.evaluate("k => soloFile(E.files[k])", k); ready(pg)
            frames(pg, 3); clean(pg)
        step("quick file switches: no MS2 request to a Full Scan file, the engine answered every request", switching)
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
