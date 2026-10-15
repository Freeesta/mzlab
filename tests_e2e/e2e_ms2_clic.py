"""Click rule on a peak of the Full Scan spectrum (demo files): the double click opens its MS2 if a scan of that precursor exists (tolerance of the Banco), otherwise its XIC; never only one of the two silently."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
ES = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
r = Run(port=8900, wd="/tmp/wd_ms2clic")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [str(ES / f"FullScan_t{t}.mzML") for t in (0, 5)] + [str(ES / f"MS2_t{t}.mzML") for t in (15, 60)])
        pg.click("text=Carica dati"); ready(pg)
        # click rule: MS2 if a scan of that precursor exists, otherwise XIC
        def rule():
            pg.evaluate("setTab('full')"); ready(pg)
            pg.evaluate("E.files.forEach(f => { f.vis = true; })")
            sp = "E.panels.find(p => p.tab === 'full' && p.type === 'spec')"
            ctx = lambda m: f"(() => {{ const p = {sp}; return AZIONI.doppio({{ vista: 'spec', p, m: {m}, has: true, a: p._a, lab: '{m}', px: 0, py: 0 }}); }})()"
            n0 = pg.evaluate("E.panels.filter(p => p.type === 'xic').length"); m0 = pg.evaluate("E.panels.filter(p => p.tab === 'ms2').length")
            assert pg.evaluate(ctx(150.3)) is True; ready(pg)
            assert pg.evaluate("E.panels.filter(p => p.type === 'xic').length") > n0, "no MS2 for 150.3: its XIC opens"
            assert pg.evaluate("E.panels.filter(p => p.tab === 'ms2').length") == m0, "and no MS2 panel"
            n1 = pg.evaluate("E.panels.filter(p => p.type === 'xic').length")
            assert pg.evaluate(ctx(305.2)) is True; ready(pg)
            assert pg.evaluate("E.panels.filter(p => p.type === 'xic').length") == n1, "an MS2 exists for 305: no XIC"
            st = pg.evaluate("E.panels.map(p => [p.tab, p.type, p.prec, p.level])")
            assert pg.evaluate("E.panels.some(p => p.tab === 'ms2' && p.type === 'spec' && String(p.prec) === '305')"), f"its MS2 is open: {st}"
        step("double click on a peak: MS2 if it exists, otherwise XIC", rule)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
