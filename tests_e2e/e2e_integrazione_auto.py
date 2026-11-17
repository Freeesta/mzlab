"""«Integrate peaks in all selected files» (the ▾ of the automatic tool): the engine finds every peak of every ion of every file shown (synthetic SIM .wiff of
tools/wiff_sim.py, `sim3`: a blank and three samples whose peaks drift 0.04 min from one to the next, an isomer pair of one ion in one sample only; no laboratory
data), groups the peaks at the same time, and the page adds them in one undoable step: one row of the card «Manual integrations» per peak, grouped by ion then by
peak (Origin «Automatic», or «No counterpart» for the peak no other file has), the area labels the usual ones, a live status with every number the engine used, the
per-file log of the peaks found under the card, no layout shift. The card is the bytes of `mzlab-cli manual-table` for the same windows and `api/manauto` is the
bytes of `mzlab-cli manual-table --set auto=true`. Needs the site and the .wasm (python3 tools/prepara_sito.py) and cargo for the CLI.
Run from the repo root: python3 tests_e2e/e2e_integrazione_auto.py"""
# covers: mzlab/web/integra-tutti.js, mzlab/web/integrazioni-manuali.js, mzlab/web/integra-segue.js, crates/mzlab-core/src/manint_auto.rs, tools/wiff_sim.py
import json, shutil, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not shutil.which("cargo"):
    print("SKIP cargo is needed for mzlab-cli manual-table"); sys.exit(0)
D = Path(tempfile.mkdtemp(prefix="auto_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "sim3.json").read_text())
CLI = ["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--"]      # the release binary tools/verifica.py builds once before the tests
cli = lambda *a: subprocess.run(CLI + [str(x) for x in a], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout
throw = lambda m: (_ for _ in ()).throw(AssertionError(m))
need = lambda c, m: c or throw(m)
SAMPLES = TRUE["campioni"]                           # Blank, Sample_A, Sample_B, Sample_C: the order of the files of the .wiff
PK = TRUE["picchi"]                                  # PK[sample][ion] = apexes (min)
ROWS = "[...document.querySelectorAll('#manint tbody tr')].map(r => [...r.children].map(c => c.querySelector('input') ? c.querySelector('input').value : c.textContent))"

def all_ions(pg):
    pg.evaluate("(() => { E.browse = false; E.files.forEach(f => { f.vis = true; }); const p = E.panels.find(q => q.type === 'mrm'); p.tr = ''; draw(p); })()")
    wait_for_js(pg, "(() => { const p = E.panels.find(q => q.type === 'mrm' && q._a); return !!p && new Set(p._a.sr.map(s => s.k)).size === E.files.length; })()", timeout=60000)

def cli_table(st):
    """manual-table on the .wiff for the windows the page sent (files numbered as the .wiff numbers its samples)."""
    order = [n[:-5] for n in st["names"]]
    ws = [{**x, "file": SAMPLES.index(order[x["file"]])} for x in json.loads(st["windows"])]
    w = D / "windows.json"; w.write_text(json.dumps(ws))
    t = D / "times.json"; t.write_text(json.dumps(st["times"]))      # the time points the page shows (the helper fills the ones the names lack)
    return cli("manual-table", D / "sim3.wiff", "--set", f"windows={w}", "--set", f"times={t}")

r = Run(port=8879, wd="/tmp/wd_integrazione_auto")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim3.wiff", D / "sim3.wiff.scan"]); pg.click("text=Load data"); ready(pg); wait_for_js(pg, "E.files.length >= 4", timeout=120000)
        open_first(pg)
        pg.click("#np-mrm"); wait_for_js(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)
        all_ions(pg)
        pg.evaluate("window.__cls = 0; new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__cls += e.value; }).observe({ type: 'layout-shift', buffered: true })")
        def run_it():
            pg.evaluate("carica('m:integra-tutti.js').then(() => INTTUTTI.runFiles(E.panels.find(q => q.type === 'mrm' && q._a)))")
            wait_for_js(pg, "document.querySelectorAll('#manint tbody tr').length >= 4 && !!document.querySelector('.manint-log')", timeout=90000)
        step("the action adds the peaks of every file", run_it)
        def groups():
            rows = pg.evaluate(ROWS)
            heads = pg.evaluate("[...document.querySelectorAll('#manint thead th')].map(c => c.textContent)")
            want = [pg.evaluate("k => I18N.t('canali.manint.h.' + k)", k) for k in ("channel", "sample", "time", "label", "area", "start", "end", "apex", "height", "snr", "base", "origin")]
            need(heads[:12] == want, ("columns in the order of the owner", heads))
            T = lambda k: pg.evaluate("k => I18N.t('canali.manint.org.' + k)", k)
            ion = lambda x: x[0]
            # every peak the writer put in a sample has one row at its apex; its origin says whether another sample has the same peak (the same channel)
            need(len(rows) == sum(len(v) for s in PK for v in s), ("one row per peak", len(rows)))
            for s, name in enumerate(SAMPLES):
                for j, pk in enumerate(PK[s]):
                    for t in pk:
                        hit = [x for x in rows if x[1] == name and abs(float(x[7]) - t) < 0.06]
                        need(len(hit) == 1, ("a row at the peak the writer put", name, j, t))
                        twins = [x for s2 in range(len(SAMPLES)) if s2 != s for t2 in PK[s2][j] if abs(t2 - t) < 0.10
                                 for x in rows if x[1] == SAMPLES[s2] and abs(float(x[7]) - t2) < 0.06]
                        need(hit[0][-2] == (T("auto") if twins else T("unmatched")), ("origin", name, j, t, hit[0][-2], len(twins)))
                        need(all(x[0] == hit[0][0] for x in twins), ("the same peak is in the same channel", name, j, t))
        step("one row per peak, matched across the files, with its origin", groups)
        def status():
            live = pg.evaluate("document.getElementById('manint-live').textContent")
            need("4 files" in live and "S/N" in live and "valley" in live and "0.10 min" in live and "0 with no counterpart" in live, live)
            need(pg.evaluate("document.getElementById('manint-live').getAttribute('aria-live')") in ("polite", "assertive") or pg.evaluate("!!document.getElementById('manint-live').closest('[aria-live]')"), "a live region")
            log = pg.evaluate("[...document.querySelectorAll('.manint-log li')].map(l => l.textContent)")
            need(len(log) >= 12 and any("min" in l for l in log) and any(s in l for s in SAMPLES for l in log), log)
        step("the live status says every number the engine used; the log lists the peaks per file", status)
        def labels():
            lb = pg.evaluate("E.panels.find(q => q.type === 'mrm' && q._a)._a.lbls.filter(l => l.int).map(l => l.text)")
            need(len(lb) == len(pg.evaluate(ROWS)) and all(l.startswith("A = ") or "=" in l for l in lb), lb)
        step("every peak has its area label (the usual ones)", labels)
        def parity():
            st = pg.evaluate("MANINT.state()")
            need(st["csv"] == cli_table(st), "the card is the bytes of mzlab-cli manual-table")
            ks = pg.evaluate("n => n.map(x => E.files.find(f => f.label === x || f.file.startsWith(x + '.')).k)", SAMPLES)
            q = json.dumps({"half": 0, "valley": 10, "sn": 5, "split": "drop"})
            site = pg.evaluate("([ks, q]) => fetch(`api/manauto?k=${ks.join(',')}&p=${encodeURIComponent(q)}`).then(r => r.text())", [ks, q])
            pf = D / "params.json"; pf.write_text(q)
            need(site == cli("manual-table", D / "sim3.wiff", "--set", "auto=true", "--set", f"params={pf}").strip(), "api/manauto is the bytes of the CLI")
        step("the card is the CLI's table and api/manauto is the CLI's auto=true", parity)
        def once():
            n = len(pg.evaluate(ROWS))
            pg.evaluate("INTTUTTI.runFiles(E.panels.find(q => q.type === 'mrm' && q._a))"); frames(pg, 5)
            need(len(pg.evaluate(ROWS)) == n, "a second run adds nothing: every window overlaps one already there")
        step("a second run adds nothing", once)
        def undo():
            pg.evaluate("STORIA.annulla()"); frames(pg, 3)
            wait_for_js(pg, "E.panels.find(q => q.type === 'mrm').ints.length === 0", timeout=30000)
            wait_for_js(pg, "document.querySelector('#manint').hidden || document.querySelectorAll('#manint tbody tr').length === 0", timeout=30000)
            pg.evaluate("STORIA.rifai && STORIA.rifai()"); frames(pg, 3)
        step("one undo removes them all", undo)
        def unmatched():
            pg.evaluate("carica('m:integra-tutti.js').then(() => INTTUTTI.runFiles(E.panels.find(q => q.type === 'mrm' && q._a), { rt_tol: 0.01 }))")
            wait_for_js(pg, "document.querySelectorAll('#manint tbody tr').length >= 4", timeout=90000)
            rows = pg.evaluate(ROWS); T = pg.evaluate("I18N.t('canali.manint.org.unmatched')")
            need(len(rows) >= 12 and all(x[-2] == T for x in rows), ("a tolerance of 0.01 min leaves every drifted peak alone", [x[-2] for x in rows][:4]))
            need(pg.evaluate("document.querySelectorAll('#manint tbody tr[data-org=unmatched] td[title]').length") == len(rows), "each is marked")
            tip = pg.evaluate("I18N.t('canali.manint.unmatched.tip')"); need(0 < len(tip) <= 60, ("tooltip up to 60 characters", tip))
            need("0.01 min" in pg.evaluate("document.getElementById('manint-live').textContent"), "the tolerance used is said")
            st = pg.evaluate("MANINT.state()"); need(st["csv"] == cli_table(st), "the card is the bytes of the CLI")
        step("a peak with no counterpart is marked, with a short tooltip", unmatched)
        step("no layout shift", lambda: need(pg.evaluate("window.__cls") < 0.01, pg.evaluate("window.__cls")))
        step("the engine answered every request", lambda: (lambda m: m and throw(m))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        step("no page error", lambda: (lambda e: e and throw(e))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
sys.exit(1 if any(s.startswith("FAIL") for _, s in steps) else 0)
