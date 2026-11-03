"""The SIM table of the Data view on the built site with the Rust engine (synthetic SIM .wiff of tools/wiff_sim.py, `sim3`: a blank and three named samples,
seven ions at a 0.8 s cycle, an isomer pair in one channel; no laboratory data): dragging the pair with an msconvert log opens four files (the log is
skipped with a note) and one table appears in the work area, one row per sample in injection order and one column per channel; every view (pivots,
swapped layout, peak table) and its CSV export equal `mzlab-cli sim-table` on the same .wiff (after the provenance line), and the channels of a sample
equal `mzlab-cli channels`. The copy whose .wiff.scan is cut at 60 % opens the samples before the cut and the table keeps their points. Needs the site
and the .wasm (python3 tools/prepara_sito.py) and cargo for the CLI. Run from the repo root: python3 tests_e2e/e2e_sim_wiff.py"""
import json, shutil, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not shutil.which("cargo"):
    print("SKIP cargo is needed for mzlab-cli sim-table"); sys.exit(0)
D = Path(tempfile.mkdtemp(prefix="sim_wiff_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
(D / "sim3.wiff.log").write_text("msconvert log (synthetic)\n")
TRUE = json.loads((D / "sim3.json").read_text())
CLI = ["cargo", "run", "-q", "-p", "mzlab-cli", "--"]
cli = lambda *a: subprocess.run(CLI + [str(x) for x in a], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout
VIEWS = ["area_sum|samples", "area_max|samples", "apex_rt|channels", "n_peaks|samples", "peaks|samples"]
NAT = {v: cli("sim-table", D / "sim3.wiff", "--set", f"output={v.split('|')[0]}", "--set", f"layout={v.split('|')[1]}") for v in VIEWS}
CUT = cli("sim-table", D / "sim3_cut.wiff")
CHANNELS = json.loads(cli("channels", D / "sim3.wiff"))
SHOWN = "(() => { const t = document.querySelector('#simtab'); return t && !t.hidden && t.querySelector('tbody tr'); })()"
throw = lambda m: (_ for _ in ()).throw(AssertionError(m))

def table_shape(pg):
    return pg.evaluate("""() => { const t = document.querySelector('#simtab table');
      return { head: [...t.querySelectorAll('thead th')].map(x => x.textContent), rows: [...t.querySelectorAll('tbody tr')].map(r => r.firstElementChild.textContent) }; }""")

def download(pg):
    with pg.expect_download() as d:
        pg.click("#simtab [data-dl]")
    text = Path(d.value.path()).read_text(encoding="utf-8")
    return text.split("\n", 1)[1] if text.startswith("# mzLab ") else text

r = Run(port=8876, wd="/tmp/wd_sim_wiff")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim3.wiff", D / "sim3.wiff.scan", D / "sim3.wiff.log"]); pg.click("text=Carica dati"); ready(pg); attendi(pg, "E.files.length >= 4", timeout=120000)
        def appears():
            attendi(pg, SHOWN, timeout=60000)
            s = table_shape(pg)
            assert s["head"] == [pg.evaluate("I18N.t('canali.simtab.h.sample')")] + [str(i["q1"]) for i in TRUE["ioni"]], s
            assert s["rows"] == TRUE["campioni"], s
        step("one table: a row per sample in injection order, a column per channel m/z", appears)
        def views():
            for v in VIEWS:
                got = pg.evaluate(f"SIMTAB.ask('{v}')")
                assert got == NAT[v], (v, got, NAT[v])
        step("every view is the bytes of mzlab-cli sim-table", views)
        def ui():
            pg.select_option("#simtab [data-o]", "peaks")
            attendi(pg, "document.querySelector('#simtab caption').textContent === I18N.t('canali.simtab.o.peaks')")
            assert pg.is_disabled("#simtab [data-flip]")
            assert len(table_shape(pg)["rows"]) == NAT["peaks|samples"].count("\n") - 1
            assert download(pg) == NAT["peaks|samples"]
            pg.select_option("#simtab [data-o]", "apex_rt"); pg.click("#simtab [data-flip]")
            attendi(pg, "document.querySelector('#simtab [data-flip]').getAttribute('aria-pressed') === 'true'")
            assert table_shape(pg)["rows"] == [str(i["q1"]) for i in TRUE["ioni"]]
            assert download(pg) == NAT["apex_rt|channels"]
        step("the peak table, the swapped layout and their CSV exports are the CLI output", ui)
        step("the msconvert log is skipped with a note", lambda: (lambda u: "sim3.wiff.log" in u or throw(u))(pg.evaluate("document.getElementById('unopen').textContent")))
        def channels():
            k = pg.evaluate("E.files.find(f => f.label === 'Blank' || /Blank/.test(f.file)).k")
            got = pg.evaluate(f"fetch('api/canali?k={k}').then(r => r.json())")
            assert got["canali"] == CHANNELS["canali"], (got, CHANNELS)
        step("the channels of a sample are those of mzlab-cli channels", channels)
        step("the engine answered every request", lambda: (lambda m: m and throw(m))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        step("no page error", lambda: (lambda e: e and throw(e))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass

r2 = Run(port=8877, wd="/tmp/wd_sim_wiff_cut")
try:
    with sync_playwright() as p:
        pg = r2.page(p)
        stage(pg, [D / "sim3_cut.wiff", D / "sim3_cut.wiff.scan"]); pg.click("text=Carica dati"); ready(pg); attendi(pg, "E.files.length >= 3", timeout=120000)
        def cut():
            attendi(pg, SHOWN, timeout=60000)
            got = [l for l in pg.evaluate("SIMTAB.state().csv['area_sum|samples']").split("\n") if l]
            want = {l.split(",")[0]: l for l in CUT.split("\n") if l}
            assert got[0] == CUT.split("\n")[0] and len(got) >= 4, (got, CUT)
            assert all(want.get(l.split(",")[0]) == l for l in got[1:]), (got, CUT)
        step("a .wiff.scan cut at 60 %: the samples before the cut, their points in the table", cut)
        step("no page error (cut file)", lambda: (lambda e: e and throw(e))([x for x in r2.errs if x[0] == "pageerror"]))
    r2.close()
except Exception as e:
    steps.append(("run cut", "FAIL " + str(e)[:300]))
    try: r2.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report(); r2.report()
