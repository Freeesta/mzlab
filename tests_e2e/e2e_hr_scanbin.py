"""H2: scan-by-scan navigation on a high-resolution file reads the scans from the binary block (api/scanbin), never from the JSON of api/spectra;
the spectrum follows the arrows; a low-resolution file keeps the JSON."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_scanbin.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrsb")
r = Run(port=8966, wd="/tmp/wd_hrsb")
def walk(pg, n=8):
    c = pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='chrom'&&q.tab===E.tab),r=p.cv.getBoundingClientRect();return {x:r.left+p._a.X((p._a.x0+p._a.x1)/2),y:r.top+r.height/2}})()")
    pg.mouse.click(c["x"], c["y"]); pg.wait_for_timeout(1200)
    s0 = pg.evaluate("(()=>{const s=E.panels.find(q=>q.type==='spec'&&q.link!=null);return s?s.si:null})()")
    for _ in range(n): pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(120)
    attendi(pg, "(()=>{const s=E.panels.find(q=>q.type==='spec'&&q.link!=null);return !!s&&s.si!=null&&s.si>=5})()")
    return s0, pg.evaluate("(()=>{const s=E.panels.find(q=>q.type==='spec'&&q.link!=null);return s?s.si:null})()")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        reqs = lambda: [u for u in pg.evaluate("window.__qqU || []") if "api/spectra?" in u or "api/scanbin?" in u]    # the engine answers in the page: no network request to watch
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        def hr():
            s0, s1 = walk(pg)
            assert s1 is not None and s1 >= 5, (s0, s1)          # the arrows set the scan index (it starts empty after a click)
            assert any("api/scanbin?" in u for u in reqs()), reqs()
            assert not any("api/spectra?" in u for u in reqs()), reqs()
            assert pg.evaluate("(()=>{const s=E.panels.find(q=>q.type==='spec'&&q.link!=null);return !!(s&&s._a&&s._a.data&&s._a.data[0]&&s._a.data[0].d.mz.length>0)})()")
        step("HR file: arrows move the spectrum; the data come from api/scanbin only", hr)
        def block():
            j = pg.evaluate("(async()=>{const k=E.files.find(f=>f.kind==='full').k;const b=await scBin(`api/scanbin?k=${k}&i0=0&i1=5&level=1`);const a=await J(`api/spectra?k=${k}&i0=0&i1=5&level=1`);return [b.scans.length,a.scans.length,b.n,a.n,b.scans[0].mz.length,a.scans[0].mz.length,Math.abs(b.scans[2].mz[3]-a.scans[2].mz[3])]})()")
            assert j[0] == j[1] == 6 and j[2] == j[3] and j[4] == j[5] and j[6] < 2e-3, j
        step("scBin gives the same scans as api/spectra", block)
    r.close()
    r = Run(port=8967, wd="/tmp/wd_hrsb2")
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        reqs = lambda: [u for u in pg.evaluate("window.__qqU || []") if "api/spectra?" in u or "api/scanbin?" in u]    # the engine answers in the page: no network request to watch
        load(pg, [D / "B_FullMass-t0.mzML"], 4000, 1)
        def lr():
            s0, s1 = walk(pg)
            assert s1 is not None and s1 >= 5, (s0, s1)
            assert any("api/spectra?" in u for u in reqs()) and not any("api/scanbin?" in u for u in reqs()), reqs()
        step("LR file: the JSON of always (nothing changes in the low-resolution world)", lr)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
