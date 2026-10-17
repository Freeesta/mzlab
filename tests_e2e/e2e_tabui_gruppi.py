"""Tabui, low resolution: several conditions of the same degradation, from the table of the files to the comparison. The 38 synthetic mzML of tests/data/gruppi/ (6 conditions × 3 times × 2
replicates, a blank, a standard of the parent; truth in attesi.json) plus two MS2 files written here (one shared by all the conditions, one of the condition O2 at 10 min) are opened in the Data
tab and dug from Tabui. Checks: nothing from TP_Mine/lazy/ is asked for until the section «Conditions of the files» is opened, and the view only when «Compare conditions» (or a row) is
opened; the conditions come from the names (reference = the one called Reference); the table «Conditions» (one dot per condition, aria-label «present in …; absent in …») and the filter «Only in…»
/ «In all» / «Present in…»; the matrix of presence equals the truth; one name (one row) per TP; a row opened from the matrix goes back to the table with «By condition»; the MS2 file shows its
condition and `Prev.` says «MS2: linked» only where an MS2 of the condition (or a shared one) is matched; the exports (Excel/CSV) have «Present in» and «Only in»; with ONE condition the
Dig has no new tab, no column, no filter and the same columns as before; `MZLAB_LR_MISS` empty. Builds the site and static/wasm-feat (SKIP without cargo, wasm-bindgen or numpy);
MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses one. Run from the repo root."""
import json, os, shutil, subprocess, sys, tempfile, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build the wasm of the engine and of mzFinder"); sys.exit(0)
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the MS2 files"); sys.exit(0)
sys.path.insert(0, str(ROOT / "TP_Mine" / "tests")); import lr_synth  # noqa: E402

tmp = Path(tempfile.mkdtemp(prefix="tabui_gr_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    from lib import build_feat; build_feat(site / "static", dict(os.environ, RUNNER_TEMP=str(tmp / "rt")))
else:                                                      # fresh sources of mzFinder over the site that was built
    mz = site / "static" / "mzfinder"
    for d in ("files", "js"):
        for f in (ROOT / "TP_Mine" / d).glob("*"): shutil.copy2(f, mz / f.name)
    (mz / "lazy").mkdir(exist_ok=True)
    for f in (ROOT / "TP_Mine" / "lazy").glob("*"): shutil.copy2(f, mz / "lazy" / f.name)
    ix = mz / "indice.json"; d = json.loads(ix.read_text()); d["lazy"] = sorted(f.name for f in (ROOT / "TP_Mine" / "lazy").glob("*.js")); ix.write_text(json.dumps(d))

DATA = ROOT / "tests" / "data" / "gruppi"
TRUTH = json.loads((DATA / "attesi.json").read_text())
COND = TRUTH["metadata"]["condizioni"]; REF = "Reference"
GROUPS = [DATA / n for n in sorted(os.listdir(DATA)) if n.endswith(".mzML")]
assert len(GROUPS) == 38, len(GROUPS)
# truth: a compound is present in a condition when it is in a treated file (t5, t10) of it; keyed by the nominal m/z
PRESENT = {}
for c, v in TRUTH["composti"].items():
    if c == "Parent": continue
    PRESENT[v["mz"]] = {k: any(f["composti"][c]["presente"] for n, f in TRUTH["file"].items() if f["condizione"] == k and f["tempo"] != "t0") for k in COND}
MS2_DIR = tmp / "ms2"
SHARED = lr_synth.write_ms2(MS2_DIR, [{"precursor": 273.0, "ce": 10.0, "peaks": [(1.8, 0.07, 6e5)]}], None, name="MS2_shared.mzML")      # TP2 (273): in O2 and Air
ONLY_O2 = lr_synth.write_ms2(MS2_DIR, [{"precursor": 257.0, "ce": 10.0, "peaks": [(2.2, 0.07, 6e5)]}], None, name="MS2_O2.mzML")        # TP1 (257): linked in O2 only
PATHS = [str(p) for p in GROUPS] + [SHARED["path"], ONLY_O2["path"]]
SMILES = "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []; reqs = []
for _ in range(100):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).close(); break
    except Exception: srv.poll()
lazy = lambda: sorted({u.rsplit("/", 1)[-1] for u in reqs if "/mzfinder/lazy/" in u})
HEAD = "[...document.querySelectorAll('#tp-t thead th')].map(t => t.textContent.trim())"
MATRIX = """() => { const heads = [...document.querySelectorAll('#gv-mx thead button[data-sort]')].map(b => b.dataset.sort).filter(k => !['name','mz','rt'].includes(k));
  return Object.fromEntries([...document.querySelectorAll('#gv-mx tbody tr[data-row]')].map(r => { const td = [...r.querySelectorAll('td')]; return [r.dataset.row + '|' + r.querySelector('th button').textContent.trim() + '|' + Math.round(parseFloat(td[0].textContent)),
    Object.fromEntries(heads.map((h, i) => [h, (td[2 + i].querySelector('.sr-only') || {textContent: ''}).textContent.startsWith('present')]))]; })); }"""

def load(pg, paths):
    pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
    pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
    pg.set_input_files("#pick", paths)
    pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(paths)}", timeout=240000)
    pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=240000)
    pg.wait_for_function(f"E.files.length >= {len(paths)} && E.pending.length === 0", timeout=480000)
    pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
    pg.wait_for_function(f"document.querySelectorAll('#tp-files tr').length > 1", timeout=60000)

try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def page():
            ctx = b.new_context(viewport={"width": 1280, "height": 1100}, reduced_motion="reduce", accept_downloads=True)
            pg = ctx.new_page()
            pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
            pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
            pg.on("pageerror", lambda e: errs.append(str(e)))
            ctx.on("request", lambda r: reqs.append(r.url))
            return pg
        pg = page(); load(pg, PATHS)

        def idle():
            assert pg.locator("#tp-cond").is_visible(), "the section of the conditions is offered with two or more full-scan files"
            assert not pg.locator("#tp-cond").evaluate("e => e.open")
            assert lazy() == [], lazy()
        step("opening Tabui and the files asks for nothing from lazy/", idle)

        def conditions():
            pg.click("#tp-cond > summary"); pg.wait_for_selector("#tp-cfiles table .tpg-cond", timeout=120000)
            assert "gruppi-campioni.js" in lazy(), lazy()
            assert "gruppi-vista.js" not in lazy(), lazy()
            got = pg.evaluate("[...new Set([...document.querySelectorAll('#tp-cfiles .tpg-cond')].map(e => e.value).filter(Boolean))].sort()")
            assert got == sorted(COND), got
            assert pg.evaluate("document.querySelector('#tp-cfiles select[aria-label=\"Reference condition\"]').value") == REF
            pg.wait_for_function("document.getElementById('tp-files').hidden", timeout=20000)
            assert pg.evaluate("document.getElementById('tp-files').hidden"), "the plain list of the files gives way to the table of conditions"
        step("the conditions are proposed from the names; the reference is Reference", conditions)

        def ms2_files():
            for name, cond, time in (("MS2_O2", "O2", "10"),):
                row = pg.locator("#tp-cfiles tr", has_text=name)
                row.locator(".tpg-cond").fill(cond); row.locator(".tpg-cond").press("Tab")
                pg.wait_for_function(f"[...document.querySelectorAll('#tp-cfiles tr')].find(r => r.textContent.includes('{name}')).querySelector('.tpg-cond').value === '{cond}'")
                row = pg.locator("#tp-cfiles tr", has_text=name)
                for _ in range(5):                     # the table is drawn again after each change: type until the time stays
                    row = pg.locator("#tp-cfiles tr", has_text=name); row.locator(".tpg-time").fill(time); row.locator(".tpg-time").press("Enter")
                    lib.fermo(pg, "#tp-cfiles table", 400)
                    if pg.evaluate(f"[...document.querySelectorAll('#tp-cfiles tr')].find(r => r.textContent.includes('{name}')).querySelector('.tpg-time').value") == time: break
                else: raise AssertionError("the time of " + name + " does not stay")
            pg.click("#tp-cfiles button:has-text('Confirm conditions')")
            assert pg.evaluate("[...document.querySelectorAll('#tp-cfiles tr')].find(r => r.textContent.includes('MS2_shared')).querySelector('.tpg-cond').value") == ""
        step("the MS2 file of O2 gets its condition and time; the other one stays shared", ms2_files)

        def dig():
            pg.fill("#tp-mol", SMILES)
            pg.click("#tp-go")
            try: pg.wait_for_selector("#tp-t", timeout=480000)
            except Exception: raise AssertionError("no table: " + str(pg.evaluate("document.getElementById('tp-msg').textContent")) + " " + str(errs[-3:]))
            assert pg.evaluate("document.getElementById('tp-msg').textContent") == "Done."
            assert "gruppi-vista.js" not in lazy(), "the view is not loaded by the Dig: " + str(lazy())
            tabs = pg.evaluate("[...document.querySelectorAll('.vtabs .vt')].map(b => b.textContent)")
            assert tabs[-1] == "Compare conditions", tabs
        step("the Dig of the 40 files gives the table and the tab «Compare conditions»", dig)

        def column():
            h = pg.evaluate(HEAD); assert "Conditions" in h, h
            first = pg.locator("#tp-t tbody tr.clk[data-id='0']").first
            lab = pg.locator("#tp-t tbody tr.clk .tpt-dots").first.get_attribute("aria-label")
            assert lab.startswith("present in ") or lab.startswith("absent in "), lab
            labs = pg.evaluate("[...document.querySelectorAll('#tp-t tbody tr.clk .tpt-dots')].map(e => e.getAttribute('aria-label'))")
            assert any("present in" in x and "absent in" in x for x in labs), labs[:5]
            assert all(len(e) == len(COND) for e in pg.evaluate("[...document.querySelectorAll('#tp-t tbody tr.clk .tpt-dots')].map(e => [...e.children])")), "one dot per condition"
        step("the column «Conditions» has one dot per condition and an aria-label", column)

        def filt():
            opts = pg.evaluate("[...document.querySelectorAll('#tp-gflt option')].map(o => o.value)")
            only = {k[5:] for k in opts if k.startswith("only:")}
            truth_only = {c for mzv, d in PRESENT.items() for c in COND if d[c] and sum(d.values()) == 1}
            assert truth_only <= only or not truth_only, (only, truth_only)
            assert "all" in opts and "in:O2" in opts, opts
            ids = lambda: pg.evaluate("[...document.querySelectorAll('#tp-t tbody tr.clk')].map(r => r.dataset.id)")
            every = ids()
            pg.select_option("#tp-gflt", "in:O2"); in_o2 = ids()
            assert 0 < len(in_o2) < len(every), (len(in_o2), len(every))
            pg.select_option("#tp-gflt", "all"); in_all = ids()
            assert len(in_all) <= len(in_o2) and in_all[0] == "0", in_all
            pg.select_option("#tp-gflt", ""); assert ids() == every
        step("the filter: Present in O2 and In all narrow the rows, All rows restores them", filt)

        def vista():
            pg.click('.vt[data-v="cond"]'); pg.wait_for_selector("#gv-mx tbody tr[data-row]", timeout=60000)
            assert "gruppi-vista.js" in lazy(), lazy()
            m = pg.evaluate(MATRIX)
            names = [k.split("|")[1] for k in m]; mzs = [k.split("|")[2] for k in m]
            assert len(set(names)) == len(names) and len(set(mzs)) == len(mzs), ("one name and one row per TP", names, mzs)
            seen = 0
            for key, cells in m.items():
                nominal = float(key.split("|")[2])
                t = PRESENT.get(nominal)
                if t is None: continue
                seen += 1; assert cells == t, (key, cells, t)
            assert seen >= 3, ("TP1, TP2 and TP3 are rows", list(m))
        step("the matrix of presence equals the truth; one name per TP in all the conditions", vista)

        def only_in():
            only = {c for mzv, d in PRESENT.items() for c in COND if d[c] and sum(d.values()) == 1}
            opts = pg.evaluate("[...document.querySelectorAll('#gv-flt option')].map(o => o.value)")
            c = next((c for c in COND if f"only:{c}" in opts), None)
            if c is None: return
            pg.select_option("#gv-flt", f"only:{c}")
            rows = pg.evaluate(MATRIX); assert all(v[c] and sum(v.values()) == 1 for v in rows.values()), (c, rows)
            pg.select_option("#gv-flt", "all")
        step("the filter of the view: Only in … keeps the rows that are in that condition alone", only_in)

        def open_row():
            key = next(k for k in pg.evaluate(MATRIX) if k.split("|")[2] == "257")
            pg.click(f"#gv-mx tr[data-row='{key.split('|')[0]}'] button.gv-nm")      # select + open
            pg.wait_for_selector("#tp-t tr.tpt-exp", timeout=60000)
            assert pg.evaluate("document.querySelector('.vt.on').dataset.v") == "tab"
            pg.click("#tp-t tr.tpt-exp [role=tab]:has-text('Samples')")
            pg.wait_for_selector("#tp-gbc table tbody tr", timeout=60000)
            assert pg.locator("#tp-gbc table tbody tr").count() == len(COND)
            assert pg.locator("#tp-gbc svg.gv-svg").count() >= 1
            assert pg.locator("#tp-c-kin").count() == 0, "the kinetics of all the samples in one curve are not shown with several conditions"
        step("a row opened from the matrix opens in the table, «By condition» with the chart of the TP", open_row)

        def ms2():
            pg.click("#tp-m-ms2 > summary"); txt = pg.inner_text("#tp-m-ms2")
            assert "· O2" in txt and "· shared" in txt, txt[:600]
            pg.click('.vt[data-v="cond"]'); pg.wait_for_selector("#gv-mx tbody tr[data-row]")
            def prev(mzv, cond):
                return pg.evaluate("""([mz, c]) => { const heads = [...document.querySelectorAll('#gv-mx thead button[data-sort]')].map(b => b.dataset.sort).filter(k => !['name','mz','rt'].includes(k));
                  const r = [...document.querySelectorAll('#gv-mx tbody tr[data-row]')].find(r => Math.round(parseFloat(r.querySelectorAll('td')[0].textContent)) === mz);
                  const td = r.querySelectorAll('td')[2 + heads.indexOf(c)]; const p = td.querySelector('.gv-pv'); return p ? p.getAttribute('title') : ''; }""", [mzv, cond])
            assert "MS2: linked" in prev(257, "O2"), ("257/O2", prev(257, "O2"), pg.evaluate("[...document.querySelectorAll('#tp-cfiles tr')].filter(r => r.textContent.includes('MS2_')).map(r => r.textContent.slice(0,40) + '|' + r.querySelector('.tpg-cond').value + '|' + r.querySelector('.tpg-time').value)"), txt[:700])
            assert prev(257, "N2") == "" or "MS2: none" in prev(257, "N2"), prev(257, "N2")
            assert "MS2: linked" in prev(273, "Air") and "MS2: linked" in prev(273, "O2"), ("273", prev(273, "Air"), prev(273, "O2"))
        step("the MS2 files show their condition; «MS2: linked» only where an MS2 of the condition or a shared one is matched", ms2)

        def exports():
            pg.click('.vt[data-v="tab"]'); pg.wait_for_selector("#tp-t")
            with pg.expect_download() as d: pg.click("#tp-x-csv")
            body = Path(d.value.path()).read_text()
            head = body.split("\n")[1]; assert '"Present in";"Only in"' in head, head
            assert any("O2" in l for l in body.split("\n")[2:]), "the column Present in is filled"
        step("the CSV export has «Present in» and «Only in»", exports)

        def miss():
            assert pg.evaluate("(window.MZLAB_LR_MISS || []).length") == 0, pg.evaluate("window.MZLAB_LR_MISS")
            assert not [u for u in reqs if "pyodide" in u.lower()]
        step("MZLAB_LR_MISS is empty and nothing asks for pyodide", miss)

        # ---- ONE condition: nothing new
        pg2 = page(); reqs.clear()
        load(pg2, [str(p) for p in GROUPS if p.name.startswith("Air_") or p.name == "bianco.mzML"])
        def one():
            pg2.click("#tp-cond > summary"); pg2.wait_for_selector("#tp-cfiles table .tpg-cond", timeout=120000)
            conds = pg2.evaluate("[...new Set([...document.querySelectorAll('#tp-cfiles .tpg-cond')].map(e => e.value).filter(Boolean))]")
            assert len(conds) <= 1, conds
            assert not pg2.evaluate("document.getElementById('tp-files').hidden"), "one condition: the plain list stays"
            pg2.fill("#tp-mol", SMILES); pg2.click("#tp-go"); pg2.wait_for_selector("#tp-t", timeout=480000)
            assert pg2.evaluate("[...document.querySelectorAll('.vtabs .vt')].map(b => b.dataset.v)") == ["tab", "map", "kin"]
            h = pg2.evaluate(HEAD); assert "Conditions" not in h, h
            assert pg2.locator("#tp-gflt").count() == 0 and pg2.locator("#tp-t .tpt-dots").count() == 0
            assert "gruppi-vista.js" not in lazy(), lazy()
            pg2.click("#tp-t tr.clk[data-id='0']"); pg2.wait_for_selector("#tp-t tr.tpt-exp")
            pg2.click("#tp-t tr.tpt-exp [role=tab]:has-text('Samples')"); pg2.wait_for_selector("#tp-c-kin", timeout=60000)
            assert pg2.locator("#tp-gbc").count() == 0 and "gruppi-vista.js" not in lazy()
            assert pg2.evaluate("(window.MZLAB_LR_MISS || []).length") == 0
        step("one condition: no tab, no column, no filter, the kinetics as before, nothing loaded from lazy/", one)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
