"""The SIM table with two SIM lists and the sidebar groups (synthetic SIM .wiff of tools/wiff_sim.py: `sim_senzaq1`, seven channels without Q1, and
`sim_listab_senzaq1`, three other channels without Q1, a water blank and samples named at T10, T20, T40; no laboratory data): no explanatory paragraph;
without groups «ione 1» is one column; grouped as List A / List B the lists get their own columns, the rows follow the groups under folding header rows
and the CSV is `mzlab-cli sim-table --set group.SAMPLE=GROUP` byte for byte; areas read 7.841E+04; `H2O` has the role Bianco; a picked column draws the
time plot (one line, the blank dashed: a timeless blank is a level across the axis) with its data table, and a column of files without times gives the hint. Needs the site and the .wasm
(python3 tools/prepara_sito.py) and cargo for the CLI. Run from the repo root: python3 tests_e2e/e2e_sim_gruppi.py"""
import re, shutil, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not shutil.which("cargo"):
    print("SKIP cargo is needed for mzlab-cli sim-table"); sys.exit(0)
D = Path(tempfile.mkdtemp(prefix="sim_grp_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
A, B = ["Blank", "Sample_A", "Sample_B", "Sample_C"], ["H2O", "B_T10", "B_T20", "B_T40"]
CLI = ["cargo", "run", "-q", "-p", "mzlab-cli", "--"]
cli = lambda *a: subprocess.run(CLI + [str(x) for x in a], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout
FILES = [D / "sim_senzaq1.wiff", D / "sim_listab_senzaq1.wiff"]
FLAT = cli("sim-table", *FILES)
GROUPS = sum((["--set", f"group.{s}=List A"] for s in A), []) + sum((["--set", f"group.{s}=List B"] for s in B), [])
GROUPED = cli("sim-table", *FILES, *GROUPS)
SHOWN = "(() => { const t = document.querySelector('#simtab'); return t && !t.hidden && t.querySelector('tbody tr'); })()"
def need(c, m): c or (_ for _ in ()).throw(AssertionError(m))

r = Run(port=8878, wd="/tmp/wd_sim_grp")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [FILES[0], D / "sim_senzaq1.wiff.scan", FILES[1], D / "sim_listab_senzaq1.wiff.scan"]); pg.click("text=Carica dati"); ready(pg)
        attendi(pg, "E.files.length >= 8 && E.files.every(f => f.sim > 0)", timeout=180000)
        attendi(pg, SHOWN, timeout=60000)
        head = lambda: pg.evaluate("[...document.querySelectorAll('#simtab thead th')].map(x => x.textContent)")
        def flat():
            need(pg.evaluate("SIMTAB.ask('area_sum|samples')") == FLAT, (pg.evaluate("SIMTAB.state().csv['area_sum|samples']"), FLAT))
            h = head(); need(len(h) == 8 and h[1] == pg.evaluate("I18N.t('canali.sim.ion', {n: 1})"), h)
            need(not pg.evaluate("/integrat/i.test(document.querySelector('#simtab').textContent)"), "the explanatory paragraph is gone")
        step("no groups: one «ione N» column per place, the CLI bytes, no explanatory paragraph", flat)
        def number():
            cells = pg.evaluate("[...document.querySelectorAll('#simtab tbody td.num')].map(x => x.textContent).filter(x => x !== '—')")
            need(cells and all(re.fullmatch(r"\d\.\d{3}E[+-]\d{2}", c) for c in cells), cells[:10])
            need(pg.evaluate("I18N.sciE(78407.7)") == "7.841E+04" and pg.evaluate("I18N.sciE(78407.7, 5)") == "7.84077E+04", "sciE")
        step("areas read 7.841E+04 (point, no thousands separator)", number)
        step("the water sample has the role Bianco", lambda: need(pg.evaluate("QAQC.roleOf(E.files.find(f => /H2O/.test(f.file + ' ' + (f.label || ''))))") == "blank",
                                                                pg.evaluate("E.files.map(f => [f.file, f.label, f.role, f.type])")))
        def group():
            pg.evaluate("""() => { const of = names => E.files.filter(f => names.some(n => (f.label || f.file).includes(n))).map(f => fileBase(f.file));
              GRUPPI.set(of(%r), 'List A'); GRUPPI.set(of(%r), 'List B'); }""" % (A, B))
            attendi(pg, "SIMTAB.state().groups.filter(Boolean).length === 8 && !!SIMTAB.state().csv['area_sum|samples']", timeout=60000)
            got = pg.evaluate("SIMTAB.state().csv['area_sum|samples']")
            need(got == GROUPED, (got, GROUPED))
            need(GROUPED.split("\n")[0].startswith("sample,group,ion 1 [List A]") and "ion 1 [List B]" in GROUPED.split("\n")[0], GROUPED.split("\n")[0])
            attendi(pg, "document.querySelectorAll('#simtab .simtab-g [data-g]').length === 2")
            h = head(); need(len(h) == 11 and any("List B" in x for x in h), h)
        step("grouped: List A and List B apart, rows by group, the CLI bytes with group.", group)
        def fold():
            pg.click("#simtab [data-g='List A']")
            attendi(pg, "document.querySelector(\"#simtab [data-g='List A']\").getAttribute('aria-expanded') === 'false'")
            need(pg.evaluate("[...document.querySelectorAll('#simtab tbody tr:not(.simtab-g)')].filter(r => !r.hidden).length") == 4, "four rows left")
            pg.click("#simtab [data-g='List A']")
            attendi(pg, "document.querySelector(\"#simtab [data-g='List A']\").getAttribute('aria-expanded') === 'true'")
        step("a group header folds its rows", fold)
        def kinetic():
            need(pg.evaluate("!!document.querySelector('#simtab [data-kin-hint]')"), "hint before a pick")
            pg.click("#simtab [data-c='ion 2 [List B]']")
            attendi(pg, "!!document.querySelector('#simtab .simtab-kin svg polyline')", timeout=30000)
            lines = pg.evaluate("[...document.querySelectorAll('#simtab .simtab-kin svg polyline')].map(l => l.getAttribute('stroke-dasharray'))")
            need(len(lines) == 2 and sum(1 for d in lines if d) == 1, lines)
            pg.click("#simtab [data-kdata]"); pg.wait_for_selector("#tdati[open]")
            n = pg.evaluate("document.querySelectorAll('#tdati tbody tr').length"); need(n == 8, n)
            pg.keyboard.press("Escape")
            pg.click("#simtab [data-c='ion 2 [List B]']"); pg.click("#simtab [data-c='ion 1 [List A]']")
            attendi(pg, "document.querySelectorAll('#simtab .simtab-kin svg polyline').length === 2", timeout=30000)
            pg.evaluate("E.files.filter(f => f.ugrp === 'List A').forEach(f => { f.time = null; }); SIMTAB.sync()")
            attendi(pg, "document.querySelector('#simtab [data-kin-hint]')?.textContent === I18N.t('canali.simtab.kin.notime')", timeout=30000)
        step("a picked column draws the time plot with the blank dashed and its data table; no times = the hint", kinetic)
        step("no page error", lambda: need(not [x for x in r.errs if x[0] == "pageerror"], r.errs))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
