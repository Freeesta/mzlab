"""«Load data» is never held back by the times: the five example Full Scan files go in with the times 0/5/10/15/30 read from the names,
the program starts at once with no message under the button; the t0 is the sample at time 0 (no role of its own) and the dark has a negative time that stays out of the gradient."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
lib.AUTO_TEMPI = False
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
NAMES = ["FullScan_t0", "FullScan_t5", "FullScan_t10", "FullScan_t15", "FullScan_t30"]
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_carica_subito.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8830 + 160, wd="/tmp/wd_carica_subito")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#start:not([hidden])", timeout=60000)
        stage(pg, [str(EX / f"{n}.mzML") for n in NAMES])
        row = lambda n: f"#flist tr:has(td:text-is('{n}.mzML'))"
        def tval(n): return pg.eval_on_selector(f"{row(n)} input[data-k=time]", "e => e.value")
        def times_in_rows():
            pg.wait_for_function("[...document.querySelectorAll('#flist input[data-k=time]')].filter(x => x.value).length >= 4", timeout=30000)
            assert [tval(n) for n in NAMES] == ["0", "5", "10", "15", "30"], [tval(n) for n in NAMES]
            roles = pg.eval_on_selector_all("#flist select[data-k=type]", "els => els.map(e => e.value)")
            assert roles == ["sample"] * 5, roles                                   # the t0 is not a role
            assert not pg.is_disabled(f"{row('FullScan_t0')} input[data-k=time]")
        step("the times 0/5/10/15/30 are in the rows (editable)", times_in_rows)
        def starts_at_once():
            assert pg.eval_on_selector("#flist", "e => e.querySelectorAll('input[aria-invalid=true]').length") == 0, "no cell marked invalid"
            pg.click("#opbtn")
            pg.wait_for_selector("#start", state="hidden", timeout=90000)
            ready(pg)
            assert pg.evaluate("E.files.filter(f => !f.gone).length") == 5
            assert pg.inner_text("#gomsg").strip() == "", "no message under the button"
        step("Load data starts at once, no message", starts_at_once)
        def t0_and_dark():
            got = pg.evaluate("E.files.filter(f => !f.gone).map(f => [f.label, CAMPIONI.kindOf(f), f.time])")
            assert all(g[1] == "sample" for g in got) and sorted(g[2] for g in got) == [0, 5, 10, 15, 30], got
            assert pg.evaluate("SUBREF.autoFile(E.tab).time") == 0                  # the reference chosen by «auto» is the sample at time 0
            pg.evaluate("CAMPIONI.apply([{file: E.files.find(f => f.label.endsWith('t5')).file, role: 'dark', time: 30}])")
            f = pg.evaluate("(() => { const x = E.files.find(f => f.label.endsWith('t5')); return [CAMPIONI.kindOf(x), x.time]; })()")
            assert f == ["dark", -30], f                                            # a positive number written in a dark is read as its negative
            assert pg.evaluate("SUBREF.autoFile(E.tab).time") == 0                  # the dark never becomes the first by time
            leg = pg.evaluate("CAMPIONI.gradientLegend(E.files.filter(f => !f.gone))")
            assert "-30" not in leg and "−30" not in leg, leg                       # the dark is not a point of the gradient
            ent = pg.evaluate("CDOC.session(E.files.filter(f => !f.gone)).map(e => [e.tipo, e.tempo_min, e.tipo_bianco])")
            assert ["dark", -30, None] in ent and not any(e[0] == "t0" for e in ent), ent
        step("t0 = sample at 0, dark time negative and out of the gradient and of the auto reference", t0_and_dark)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else "  -> " + s))
errs = [e for e in r.errs if e[0] in ("pageerror",)]
print("page errors:", errs)
sys.exit(1 if bad or errs else 0)
