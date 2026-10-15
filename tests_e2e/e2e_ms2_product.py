"""Product ion panels (demo files): the spectrum offers only the files with scans of its precursor and the precursors of the active file with their scan count; no «Follows», no overlay; the pair badge on one line; the second pair (_bis) obeys the same rules."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
ES = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
# what the controls of a product-ion spectrum offer
RULES = """(p => { const c = p.el.querySelector('.ctl'), o = n => { const s = c.querySelector(`select[data-o="${n}"]`); return s ? [...s.options].map(x => x.text) : null; };
  return { files: o('k'), prec: o('prec'), fol: !!c.querySelector('.fol'), all: !!c.querySelector('[data-o="all"]'), file: E.files[p.k].label }; })"""
def specs(pg): return "E.panels.filter(p => p.tab === 'ms2' && p.type === 'spec')"
r = Run(port=8899, wd="/tmp/wd_ms2prod")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [str(ES / f"MS2_t{t}.mzML") for t in (15, 60)])
        pg.click("text=Carica dati"); ready(pg)
        pg.locator('#flst [data-pg="229.1"]').dblclick(); attendi(pg, f"{specs(pg)}.length >= 1"); ready(pg)
        def menu():
            q = pg.evaluate(f"({RULES})({specs(pg)}[0])")
            assert q["files"] == ["MS2_t15"], q["files"]                 # MS2_t60 has no scan of 229.1
            assert q["prec"] == ["tutti i prec.", "prec. 229.1 (821)", "prec. 305 (184)"], q["prec"]
            assert not q["fol"] and not q["all"], q
        step("product ion: files and precursors of the active file with counts, no Follows, no overlay", menu)
        def badge():
            ws = pg.evaluate("[...document.querySelectorAll('.ppair')].map(e => getComputedStyle(e).whiteSpace)"); assert ws and set(ws) == {"nowrap"}, ws
        step("pair badge on one line", badge)
        def bis():
            pg.locator('#flst input[data-pr="237"]').click(); attendi(pg, "E.panels.filter(p => p.tab === 'ms2' && p.type === 'chrom').length >= 2"); ready(pg)
            pg.locator('#flst [data-pg="237"]').dblclick(); attendi(pg, f"{specs(pg)}.length >= 2"); ready(pg)
            q = pg.evaluate(f"{specs(pg)}.map(p => ({RULES})(p))")
            assert [x["file"] for x in q] == ["MS2_t15", "MS2_t60"] or sorted(x["file"] for x in q) == ["MS2_t15", "MS2_t60"], q
            b = [x for x in q if x["file"] == "MS2_t60"][0]
            assert b["files"] == ["MS2_t60"] and b["prec"] == ["tutti i prec.", "prec. 237 (597)", "prec. 333 (529)"] and not b["fol"] and not b["all"], b
            n = pg.evaluate(f"{specs(pg)}.every(p => p._a && p._a.data && p._a.data.length >= 1)"); assert n, "both spectra have data"
        step("second pair (_bis): same rules, own file, with data", bis)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
