"""The SIM table of the Data view on the built site with the Rust engine (synthetic SIM .wiff of tools/wiff_sim.py, `sim3`: three named samples, seven
ions; no laboratory data): dragging the pair opens three files and one table appears in the work area, seven channel rows x three sample columns named
after the samples, the area and apex RT views, and its CSV export equals `mzlab-cli sim-table` on the same .wiff (after the provenance line). The copy whose
.wiff.scan is cut at 60 % opens the samples before the cut and the table keeps their points. Needs the site and the .wasm (python3 tools/prepara_sito.py)
and cargo for the CLI. Run from the repo root: python3 tests_e2e/e2e_sim_wiff.py"""
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
TRUE = json.loads((D / "sim3.json").read_text())
CLI = ["cargo", "run", "-q", "-p", "mzlab-cli", "--", "sim-table"]
cli = lambda name, *extra: subprocess.run(CLI + [str(D / name), *extra], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout
NAT = {"area": cli("sim3.wiff"), "apex_rt": cli("sim3.wiff", "--set", "output=apex_rt"), "cut": cli("sim3_cut.wiff")}

def table_shape(pg):
    return pg.evaluate("""() => { const t = document.querySelector('#simtab table'); if (!t) return null;
      return { head: [...t.querySelectorAll('thead th')].map(x => x.textContent), rows: t.querySelectorAll('tbody tr').length, shown: !document.getElementById('simtab').hidden }; }""")

def download(pg):
    with pg.expect_download() as d:
        pg.click("#simtab [data-dl]")
    text = Path(d.value.path()).read_text(encoding="utf-8")
    return text.split("\n", 1)[1] if text.startswith("# mzLab ") else text

r = Run(port=8876, wd="/tmp/wd_sim_wiff")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim3.wiff", D / "sim3.wiff.scan"]); pg.click("text=Carica dati"); ready(pg); attendi(pg, "E.files.length >= 3", timeout=120000)
        def appears():
            attendi(pg, "(() => { const t = document.querySelector('#simtab'); return t && !t.hidden && t.querySelector('tbody tr'); })()", timeout=60000)
            s = table_shape(pg)
            assert s["head"] == ["m/z (Q1)", pg.evaluate("I18N.t('canali.simtab.name')")] + TRUE["campioni"], s
            assert s["rows"] == len(TRUE["ioni"]), s
        step("one table: seven channel rows x three sample columns named after the samples", appears)
        def views():
            got = pg.evaluate("SIMTAB.state().csv")
            assert got["area"] == NAT["area"], (got["area"], NAT["area"])
            assert got["apex_rt"] == NAT["apex_rt"], (got["apex_rt"], NAT["apex_rt"])
            pg.click('#simtab [data-o="apex_rt"]')
            assert pg.get_attribute('#simtab [data-o="apex_rt"]', "aria-pressed") == "true"
            first = pg.evaluate("[...document.querySelectorAll('#simtab tbody tr:first-child td.num')].map(x => x.textContent)")
            assert len(first) == 3 and all(x not in ("", "—") for x in first), first
        step("area and apex RT are the bytes of mzlab-cli sim-table", views)
        step("the CSV export (apex RT) is the CLI output", lambda: (lambda t: t == NAT["apex_rt"] or (_ for _ in ()).throw(AssertionError((t, NAT["apex_rt"]))))(download(pg)))
        def area_csv():
            pg.click('#simtab [data-o="area"]')
            t = download(pg)
            assert t == NAT["area"], (t, NAT["area"])
        step("the CSV export (area) is the CLI output", area_csv)
        step("the engine answered every request", lambda: (lambda m: m and (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass

r2 = Run(port=8877, wd="/tmp/wd_sim_wiff_cut")
try:
    with sync_playwright() as p:
        pg = r2.page(p)
        stage(pg, [D / "sim3_cut.wiff", D / "sim3_cut.wiff.scan"]); pg.click("text=Carica dati"); ready(pg); attendi(pg, "E.files.length >= 2", timeout=120000)
        def cut():
            attendi(pg, "(() => { const t = document.querySelector('#simtab'); return t && !t.hidden && t.querySelector('tbody tr'); })()", timeout=60000)
            got = pg.evaluate("SIMTAB.state().csv.area")
            names = got.split("\n", 1)[0].split(",")[2:]
            assert names and set(names) <= set(TRUE["campioni"]) and "Sample_A" in names and "Sample_B" in names, got
            want = NAT["cut"].split("\n")
            cols = [want[0].split(",").index(n) for n in names]
            assert [",".join(l.split(",")[:2] + [l.split(",")[c] for c in cols]) for l in want if l] == [l for l in got.split("\n") if l], (got, NAT["cut"])
        step("a .wiff.scan cut at 60 %: the samples before the cut, their points in the table", cut)
        step("no page error (cut file)", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r2.errs if x[0] == "pageerror"]))
    r2.close()
except Exception as e:
    steps.append(("run cut", "FAIL " + str(e)[:300]))
    try: r2.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report(); r2.report()
