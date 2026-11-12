"""XWC on the built site with the Rust engine: the DAD TWC panel of a synthetic MRM `.wiff` (tools/wiff_dad_mrm.py, no laboratory data) has the «XWC» split
button; one click opens the band the engine proposes (274 ± 2 nm, the strongest peak) under the TWC, the popover takes λ and bandwidth by keyboard
(Shift+Enter adds 230 nm to the same panel as one more trace with its own series key, undoable; Esc closes); the UV spectrum has the button too, with the
band of its time; `api/uvpropose` of the demo FullScan_t10.mzML is the JSON of `mzlab-cli channels --channel uv:auto`; a TWC-only mzML has no button and
no toast. Run from the repo root: python3 tests_e2e/e2e_xwc.py"""
import json, shutil, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="xwc_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_dad_mrm.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "dadmrm.json").read_text())
T0 = D / "SOLOTRACCIA_t0.mzML"; ds.full_scan(T0, 0, np.random.default_rng(5))
DEMO = ROOT / "mzlab" / "web" / "esempi" / "FullScan_t10.mzML"
MRM = "E.panels.find(q => q.type === 'mrm')"
TWC = "E.panels.find(q => q.type === 'chrom' && q.kind === 'pda' && q.mz0 == null)"
XWC = "E.panels.find(q => q.type === 'chrom' && q.kind === 'pda' && q.mz0 != null)"
SPEC = "E.panels.find(q => q.type === 'spec' && q.uv)"
def load(pg, files):
    pg.set_input_files("#pick", [str(f) for f in files])
    wait_for_js(pg, f"document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=90000); pg.click("text=Carica dati"); ready(pg)
    wait_for_js(pg, "E.files.length >= 1", timeout=120000)
def visible(pg, panel, a):
    return pg.evaluate(f"(() => {{ const b = {panel} && {panel}.el.querySelector('[data-a={a}]'); return !!b && !b.hidden && b.offsetParent !== null; }})()")
r = Run(port=8881, wd="/tmp/wd_xwc")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "dadmrm.wiff", D / "dadmrm.wiff.scan"])
        def twc():
            open_kind(pg, "mrm"); wait_for_js(pg, f"!!{MRM} && !!{MRM}.el.querySelector('[data-o=canali]')", timeout=30000)
            pg.evaluate(f"{MRM}.el.querySelector('[data-o=canali]').click()")
            wait_for_js(pg, f"!!{MRM}.el.querySelector('.can-pop:not([hidden]) [data-kind=pda]')")
            pg.evaluate(f"{MRM}.el.querySelector('.can-pop [data-kind=pda]').click()")
            wait_for_js(pg, f"!!{TWC} && !!{TWC}._a && !!{TWC}.el.querySelector('[data-a=xwcl]:not([hidden])')", timeout=60000)
            assert pg.evaluate(f"{TWC}.el.querySelector('[data-a=xic]').textContent") == "XWC"
            assert visible(pg, TWC, "xic") and visible(pg, TWC, "xwcl")
            assert pg.evaluate(f"{TWC}.el.querySelector('[data-a=xwcl]').getAttribute('aria-haspopup')") == "dialog"
        step("the DAD TWC panel has the XWC split button (main part and arrow)", twc)
        def click():
            pg.evaluate(f"{TWC}.el.querySelector('[data-a=xic]').click()")
            wait_for_js(pg, f"!!{XWC} && !!{XWC}._a && {XWC}._a.sr.length === 1", timeout=60000)
            got = pg.evaluate(f"(() => {{ const t = {TWC}, q = {XWC}, st = stackOrder(); return {{ nm0: q.mz0, nm1: q.mz1, after: st.indexOf(q) === st.indexOf(t) + 1, tl: q.tl, at: q._a.sr[0].x[q._a.sr[0].y.indexOf(Math.max(...q._a.sr[0].y))] }}; }})()")
            mid = (got["nm0"] + got["nm1"]) / 2
            assert abs(mid - 274) <= 2 and abs(got["nm1"] - got["nm0"] - 4) < 1e-9, got
            assert got["after"] and got["tl"] and abs(got["at"] - TRUE["a"]["rt"]) < 0.05, got
        step("one click: the band the engine proposes (274 ± 2 nm, 4 nm wide) opens under the TWC, time-linked, its apex at compound A", click)
        def popover():
            pg.evaluate(f"{XWC}.el.querySelector('[data-a=xwcl]').focus()"); pg.keyboard.press("Enter")
            wait_for_js(pg, f"!!{XWC}.el.querySelector('.xb-pop[role=dialog]') && document.activeElement.dataset.f === 'nm'")
            assert pg.evaluate(f"{XWC}.el.querySelector('[data-a=xwcl]').getAttribute('aria-expanded')") == "true"
            pg.keyboard.press("Escape")
            wait_for_js(pg, f"!{XWC}.el.querySelector('.xb-pop') && document.activeElement.dataset.a === 'xwcl'")
            pg.keyboard.press("Enter"); wait_for_js(pg, "document.activeElement.dataset.f === 'nm'")
            pg.keyboard.type("230"); pg.keyboard.press("Tab"); pg.keyboard.press("Control+A"); pg.keyboard.type("4"); pg.keyboard.press("Shift+Enter")
            wait_for_js(pg, f"{XWC}._a && {XWC}._a.sr.length === 2", timeout=60000)
            got = pg.evaluate(f"(() => {{ const q = {XWC}; return {{ xw: q.xw, n: E.panels.filter(x => x.type === 'chrom' && x.kind === 'pda').length, keys: q._a.sr.map(s => s.key), max: Math.max(...q._a.sr[1].y), c: q._a.sr.map(s => s.color) }}; }})()")
            assert got["xw"] == [[228, 232]] and got["n"] == 2, got
            assert len(set(got["keys"])) == 2 and got["keys"][1].endswith("|pda|228-232"), got
            assert abs(got["max"] - TRUE["b"]["max"]) < 0.02 * TRUE["b"]["max"] and got["c"][0] != got["c"][1], (got, TRUE["b"])
        step("popover by keyboard: Enter opens it on λ, Esc closes it back on the arrow; 230 nm with Shift+Enter is one more trace of the XWC panel", popover)
        def undo():
            pg.evaluate("STORIA.annulla()")
            wait_for_js(pg, f"{XWC}._a && {XWC}._a.sr.length === 1 && !({XWC}.xw || []).length", timeout=30000)
        step("the added band is undone like any change of the panel's source", undo)
        def spec():
            pg.evaluate(f"(() => {{ const q = {XWC}; q.sel = null; }})()")
            pg.evaluate(f"uvOpen({TWC}, {TWC}.k ?? E.files[0].k, {TRUE['b']['rt']}, {TRUE['b']['rt']})")
            wait_for_js(pg, f"!!{SPEC} && !!{SPEC}._a && !!{SPEC}.el.querySelector('[data-a=xwc]:not([hidden])')", timeout=60000)
            assert visible(pg, SPEC, "xwc") and visible(pg, SPEC, "xwcl")
            n = pg.evaluate("E.panels.filter(x => x.type === 'chrom' && x.kind === 'pda').length")
            pg.evaluate(f"{SPEC}.el.querySelector('[data-a=xwc]').click()")
            wait_for_js(pg, f"E.panels.filter(x => x.type === 'chrom' && x.kind === 'pda').length === {n + 1}", timeout=60000)
            q = pg.evaluate("(() => { const q = E.panels.filter(x => x.type === 'chrom' && x.kind === 'pda').sort((a, b) => a.id - b.id).pop(); return [q.mz0, q.mz1, q.tab]; })()")
            assert abs((q[0] + q[1]) / 2 - 230) <= 2 and q[2] == 'mrm', q
        step("the UV spectrum at compound B has the XWC button: its band (230 ± 2 nm) opens in the tab of the TWC", spec)
        step("the engine answered every request", lambda: (lambda m: m and (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))

        q = r.page(p); load(q, [DEMO])
        def parity():
            if not shutil.which("cargo"):
                return
            site = q.evaluate("fetch('api/uvpropose?k=0').then(r => r.text())")
            win = q.evaluate("fetch('api/uvpropose?k=0&rt0=1&rt1=2').then(r => r.text())")
            run = lambda *a: subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "channels", str(DEMO), "--channel", "uv:auto", *a], cwd=ROOT, capture_output=True, text=True, check=True).stdout
            assert site + "\n" == run(), (site, run())
            assert win + "\n" == run("--rt", "1,2"), (win, run("--rt", "1,2"))
        step("FullScan_t10.mzML: api/uvpropose (with and without a window) is the JSON of mzlab-cli channels --channel uv:auto, byte for byte", parity)
        q.close()

        q = r.page(p); load(q, [T0])
        def alone():
            pick_kind(q, "pda")
            wait_for_js(q, f"!!{TWC} && !!{TWC}._a", timeout=60000)
            wait_for_js(q, f"E.files[0]._xwc && Object.keys(E.files[0]._xwc).length > 0", timeout=30000)
            wait_for_js(q, f"{TWC}.el.querySelector('[data-a=xic]').hidden && !!{TWC}.el.querySelector('[data-a=xwcl][hidden]')", timeout=30000)
            assert not visible(q, TWC, "xic") and not visible(q, TWC, "xwcl"), "button shown without a matrix"
            assert not q.evaluate("[...document.querySelectorAll('.toast')].some(t => t.textContent.includes(I18N.t('uv.none').slice(0, 20)))")
        step("TWC-only mzML: no XWC button and no «no UV spectra» toast", alone)
        q.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
