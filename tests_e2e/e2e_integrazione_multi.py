"""An integration drawn on one sample is applied to the same peak of the other samples shown (synthetic SIM .wiff of tools/wiff_sim.py, `sim3`: a blank and
three samples whose peaks drift 0.04 min from one to the next; no laboratory data): the area label «A = …» above the peak of each sample, one row of the card
«Manual integrations» per sample (columns Channel, Sample, Name, Area, Start, End, Apex, Height, S/N, Baseline, Origin), the engine's decision per sample
(applied inside the window, shifted to the apex that drifted out of it, no peak) in the Origin column, a name typed on one row for the whole group (Enter) or
for that sample alone (Alt+Enter), a live status line «Integration applied to N files», no notice about a cap of files; the card is the bytes of `mzlab-cli
manual-table` for the same windows (`follow`, `names`), and the shift of a window is `mzlab-cli manual-table --set shifts=true`. Needs the site and the .wasm
(python3 tools/prepara_sito.py) and cargo for the CLI. Run from the repo root: python3 tests_e2e/e2e_integrazione_multi.py"""
# covers: mzlab/web/integrazioni-manuali.js, mzlab/web/integra-segue.js, crates/mzlab-core/src/manint.rs, tools/wiff_sim.py
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
D = Path(tempfile.mkdtemp(prefix="multi_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "sim3.json").read_text())
CLI = ["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--"]      # the release binary tools/verifica.py builds once before the tests
cli = lambda *a: subprocess.run(CLI + [str(x) for x in a], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout
throw = lambda m: (_ for _ in ()).throw(AssertionError(m))
need = lambda c, m: c or throw(m)
SAMPLES = TRUE["campioni"]                           # Blank, Sample_A, Sample_B, Sample_C: the order of the files of the .wiff
PK = TRUE["picchi"]                                  # PK[sample][ion] = apexes (min)
ROWS = "[...document.querySelectorAll('#manint tbody tr')].map(r => [...r.children].map(c => c.querySelector('input') ? c.querySelector('input').value : c.textContent))"
COUNT = "document.querySelectorAll('#manint tbody tr').length === %d"

INTEG = """([name, ion, a, b]) => { const p = E.panels.find(q => q.type === 'mrm' && q._a);
  const f = E.files.find(x => x.label === name || x.file.startsWith(name + '.')); const s = p._a.sr.filter(x => x.k === f.k)[ion];
  addInt(p, s, a, b, false); return s.key; }"""
LB = "E.panels.find(q => q.type === 'mrm' && q._a)._a.lbls.filter(l => l.int).map(l => ({ text: l.text, tip: l.tip, fol: !!l.int.fol, key: l.int.key, a: l.int.a, b: l.int.b, nm: l.int.nm || '' }))"

def all_ions(pg):
    pg.evaluate("(() => { E.browse = false; E.files.forEach(f => { f.vis = true; }); const p = E.panels.find(q => q.type === 'mrm'); p.tr = ''; draw(p); })()")
    wait_for_js(pg, "(() => { const p = E.panels.find(q => q.type === 'mrm' && q._a); return !!p && new Set(p._a.sr.map(s => s.k)).size === E.files.length; })()", timeout=60000)

def cli_table(pg, st):
    """manual-table on the .wiff for the windows the page sent (files numbered as the .wiff numbers its samples)."""
    order = [n[:-5] for n in st["names"]]
    ws = []
    for x in json.loads(st["windows"]):
        y = {**x, "file": SAMPLES.index(order[x["file"]])}
        if "names" in y: y["names"] = {str(SAMPLES.index(order[int(i)])): n for i, n in y["names"].items()}
        ws.append(y)
    w = D / "windows.json"; w.write_text(json.dumps(ws))
    t = D / "times.json"; t.write_text(json.dumps(st["times"]))      # the time points the page shows (the helper fills the ones the names lack)
    return cli("manual-table", D / "sim3.wiff", "--set", f"windows={w}", "--set", f"times={t}")

r = Run(port=8878, wd="/tmp/wd_integrazione_multi")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim3.wiff", D / "sim3.wiff.scan"]); pg.click("text=Load data"); ready(pg); wait_for_js(pg, "E.files.length >= 4", timeout=120000)
        open_first(pg)
        pg.click("#np-mrm"); wait_for_js(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)
        all_ions(pg)
        a0 = PK[1][0][0]                                   # apex of ion 0 in Sample_A
        def applied():
            pg.evaluate(INTEG, ["Sample_A", 0, a0 - 0.15, a0 + 0.15])
            wait_for_js(pg, COUNT % 4, timeout=60000)         # Blank, A, B, C
            rows = pg.evaluate(ROWS)
            need([x[:2] for x in rows] == [["253.1", s] for s in SAMPLES], rows)         # grouped by channel, then sample, in the order of the files
            heads = pg.evaluate("[...document.querySelectorAll('#manint thead th')].map(c => c.textContent)")
            want = [pg.evaluate("k => I18N.t('canali.manint.h.' + k)", k) for k in ("channel", "sample", "time", "label", "apex", "area", "start", "end", "height", "snr", "base", "origin")]
            need(heads[:12] == want, ("columns in the order of the owner, no «name in the method»", heads))
            org = {x[1]: x[-2] for x in rows}                 # the last cell is the action button
            T = lambda k: pg.evaluate("k => I18N.t('canali.manint.org.' + k)", k)
            need(org == {"Blank": T("nopeak"), "Sample_A": T("manual"), "Sample_B": T("applied"), "Sample_C": T("applied")}, org)
            lb = pg.evaluate(LB)
            need(len(lb) == 4 and sum(1 for l in lb if l["fol"]) == 3 and all(l["text"].startswith("A = ") for l in lb) and not any("(" in l["text"] for l in lb), lb)
            need(pg.evaluate("document.querySelectorAll('#manint tbody tr [data-del]').length") == 1, "only the window drawn has Delete")
            need(pg.evaluate("document.getElementById('manint-live').textContent") == pg.evaluate("I18N.t('canali.manint.applied', { n: 3 })"), "announced")
        step("a window drawn on Sample_A is applied to the other samples: a label, a row each, the engine's origin", applied)
        def parity():
            st = pg.evaluate("MANINT.state()")
            need(st["csv"] == cli_table(pg, st), "the table is the bytes of mzlab-cli manual-table")
            w = json.loads(st["windows"])
            need(len(w) == 1 and w[0].get("follow") is True, w)
        step("the card is the bytes of mzlab-cli manual-table", parity)
        def shifted():
            pg.evaluate(INTEG, ["Sample_A", 1, PK[1][1][0] - 0.02, PK[1][1][0] + 0.02])     # a narrow window: the next samples drift 0.04 min out of it
            wait_for_js(pg, COUNT % 8, timeout=60000)
            rows = [x for x in pg.evaluate(ROWS) if x[0] == "267.1"]
            T = lambda k: pg.evaluate("k => I18N.t('canali.manint.org.' + k)", k)
            org = {x[1]: x[-2] for x in rows}
            need(org["Sample_A"] == T("manual") and org["Sample_B"] == T("shifted") and org["Blank"] == T("nopeak"), org)
            # a row without a peak keeps the engine's numbers (the chart integrates the same window): area and height are copy buttons, the origin says "No peak"
            np_ = pg.evaluate("[...document.querySelectorAll('#manint tbody tr[data-org=no_peak]')].map(r => [r.querySelectorAll('.manint-np').length, r.querySelectorAll('[data-cp]').length])")
            need(np_ and all(x[0] == 0 and x[1] >= 2 for x in np_), ("no peak: area and height are numbers like any other row", np_))
            # the card shows the engine's numbers: the copy value of each area/height cell is the CSV's, and each area label of the chart is the card's area
            cmp = pg.evaluate("""(() => { const [h, ...b] = MANINT.rows(MANINT.state().csv); const k = n => h.indexOf(n), tr = [...document.querySelectorAll('#manint tbody tr')];
              const rows = b.filter(r => r[k('sample')] !== 'sum' || r[k('start_min')] !== '');
              const cells = tr.map((r, i) => [...r.querySelectorAll('[data-cp]')].map(c => c.dataset.cp));
              const ok = rows.every((r, i) => cells[i] && cells[i].includes(r[k('area')]) && cells[i].includes(r[k('height')]));
              const txt = new Set(rows.map(r => 'A = ' + I18N.sciE(+r[k('area')])));
              const lab = E.panels.find(q => q.type === 'mrm' && q._a)._a.lbls.filter(l => l.int && l.int.fol).map(l => l.text);
              return { ok, n: rows.length, lab, missing: lab.filter(t => !txt.has(t)) }; })()""")
            need(cmp["ok"] and cmp["n"] >= 4 and not cmp["missing"], ("card = engine numbers = chart labels", cmp))
            w1 = D / "w1.json"; w1.write_text(json.dumps([{"file": 1, "tr": 1, "a": PK[1][1][0] - 0.02, "b": PK[1][1][0] + 0.02, "baseline": "zero"}]))
            sh = json.loads(cli("manual-table", D / "sim3.wiff", "--set", "shifts=true", "--set", f"windows={w1}"))
            ks = pg.evaluate("n => n.map(x => E.files.find(f => f.label === x || f.file.startsWith(x + '.')).k)", SAMPLES)
            site = pg.evaluate("([ks, w]) => fetch(`api/manfollow?k=${ks.join(',')}&w=${encodeURIComponent(w)}`).then(r => r.text())", [ks, w1.read_text()[1:-1]])
            need(site == cli("manual-table", D / "sim3.wiff", "--set", "shifts=true", "--set", f"windows={w1}").strip(), ("api/manfollow is the bytes of the CLI", site))
            by = {x["file"]: x for x in sh}
            need(abs(by[2]["shift"] - 0.04) < 0.01 and by[2]["origin"] == "shifted", by)
            need(by[3]["origin"] in ("shifted", "no_peak") and by[0]["origin"] == "no_peak", by)
        step("a peak that drifted out of the window moves it; none leaves it, and the table says which", shifted)
        def names():
            n0 = "#manint tbody tr:nth-child(%d) input[data-nm]"
            pg.click(n0 % 2); pg.keyboard.type("Caffeine"); pg.keyboard.press("Enter")            # the row of Sample_A, channel 253.1: the whole group
            wait_for_js(pg, "MANINT.state().csv.split('\\n').filter(l => l.includes(',Caffeine,')).length === 4", timeout=60000)
            need(all(pg.evaluate(f"document.querySelector('{n0 % i}').value") == "Caffeine" for i in (1, 2, 3, 4)), "the name goes to the same peak of every sample")
            need(pg.evaluate("[...document.querySelectorAll('#manint input[data-nm]')].filter(i => i.value === 'Caffeine').length") >= 4, "the name is on every row of the card")
            pg.click(n0 % 4); pg.keyboard.press("Control+a"); pg.keyboard.type("Caffeine C"); pg.keyboard.press("Alt+Enter")     # Sample_C alone
            wait_for_js(pg, "MANINT.state().csv.includes(',Caffeine C,')", timeout=60000)
            got = [pg.evaluate(f"document.querySelector('{n0 % i}').value") for i in (1, 2, 3, 4)]
            need(got == ["Caffeine", "Caffeine", "Caffeine", "Caffeine C"], got)
            pg.click(n0 % 3); pg.keyboard.press("Control+a"); pg.keyboard.type("Caffeine 2"); pg.keyboard.press("Enter")     # the group again: C keeps its own
            wait_for_js(pg, "MANINT.state().csv.includes(',Caffeine 2,')", timeout=60000)
            got = [pg.evaluate(f"document.querySelector('{n0 % i}').value") for i in (1, 2, 3, 4)]
            need(got == ["Caffeine 2", "Caffeine 2", "Caffeine 2", "Caffeine C"], got)
            st = pg.evaluate("MANINT.state()")
            need(st["csv"] == cli_table(pg, st), "the names are the bytes of mzlab-cli manual-table (`names`)")
            tip = pg.evaluate("document.querySelector('#manint thead th:nth-child(4)').title")
            need(0 < len(tip) <= 60, tip)
            pg.evaluate("STORIA.annulla()"); frames(pg, 3)
            wait_for_js(pg, "!MANINT.state().csv.includes(',Caffeine 2,')", timeout=60000)
        step("a name goes to the same peak of every sample (Enter), or to one sample alone (Alt+Enter); an own name stays; undo", names)
        def no_notice():
            need(pg.evaluate("!document.querySelector('.mrm-all')") and not pg.evaluate("[...document.querySelectorAll('#dpanels *')].some(e => /primi 12|first 12/.test(e.textContent) && e.children.length === 0)"), "no «first 12 of N files» notice")
        step("no notice about a cap of files", no_notice)
        def delete_group():
            # what an undo leaves behind until the next sync: p.ints rebuilt from JSON (new objects, same ids) while the card still holds the records of the old ones
            pg.evaluate("(() => { const p = E.panels.find(q => q.type === 'mrm'); p.ints = JSON.parse(JSON.stringify(p.ints)); })()")
            pg.click("#manint tbody tr:nth-child(2) [data-del]")
            wait_for_js(pg, COUNT % 4, timeout=60000)
            need(pg.evaluate("E.panels.find(q => q.type === 'mrm').ints.length") == 1, "deleting the window drawn takes its applied rows with it")
        step("deleting the window drawn removes the rows applied from it, also right after an undo", delete_group)
        def only_selected():
            pg.evaluate("(() => { E.files.forEach(f => { f.vis = f.file.startsWith('Sample_A') || f.file.startsWith('Sample_B'); }); draw(E.panels.find(q => q.type === 'mrm')); })()")
            wait_for_js(pg, "(() => { const p = E.panels.find(q => q.type === 'mrm' && q._a); return !!p && new Set(p._a.sr.map(s => s.k)).size === 2; })()", timeout=60000)
            wait_for_js(pg, COUNT % 2, timeout=60000)
            need({x[1] for x in pg.evaluate(ROWS)} == {"Sample_A", "Sample_B"}, "rows only for the samples selected")
        step("only the samples shown have a row", only_selected)
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
