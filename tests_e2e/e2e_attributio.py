"""Voice «Attributio» of the administrator mode (TP_Mine/js/50_attributio.js, 51_attributio_table.js, feat-worker.js, the .wasm `attributio`). Without `qqq.admin` there is no entry; with it the
synthetic campaign folder (tests/data/attributio/campagna_sintetica) is loaded with set_input_files, the table has 15 features with the levels of livelli_attesi.csv, the filters and the sort
work, CSV and XLSX are downloaded, the result and its fingerprint reach the notebook, the controls are at least 24 x 24 px and MZLAB_LR_MISS stays empty. Builds the site and the .wasm
(SKIP without cargo and wasm-bindgen); MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses a site. Run from the repo root: python3 tests_e2e/e2e_attributio.py"""
import csv, io, json, os, re, shutil, subprocess, sys, tempfile, threading, urllib.request, zipfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-attributio-wasm"); sys.exit(0)

FOLDER = ROOT / "tests" / "data" / "attributio" / "campagna_sintetica"
tmp = Path(tempfile.mkdtemp(prefix="attr_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    subprocess.run(["tools/build_feat_wasm.sh", str(site / "static")], cwd=ROOT, check=True, env=env, capture_output=True)
else:
    for sub in ("js", "files"):
        for f in (ROOT / "TP_Mine" / sub).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
    ix = site / "static" / "mzfinder" / "indice.json"; d = json.loads(ix.read_text())
    d["js"] = sorted(f.name for f in (ROOT / "TP_Mine" / "js").glob("*.js")); ix.write_text(json.dumps(d))
man = json.loads((site / "static" / "wasm-feat" / "manifest.json").read_text())
size = (site / "static" / "wasm-feat" / man["attributio"]["wasm"]).stat().st_size
assert size < 512000, size
print("  attributio .wasm:", size, "bytes")

want = {}
with open(FOLDER / "livelli_attesi.csv", newline="", encoding="utf-8") as f:
    for r in csv.DictReader(f, delimiter=";"): want[(r["_pol"], r["_id"])] = (r["level"] + r["level_sub"], r["level_basis"])

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
URL = f"http://127.0.0.1:{PORT}/"
for _ in range(100):
    try: urllib.request.urlopen(URL, timeout=1).read(); break
    except Exception: threading.Event().wait(0.1)
errs = []
def new_page(b, admin):
    pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce", accept_downloads=True).new_page()
    pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
    if admin: pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")
    pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
    pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
    return pg
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        pg0 = new_page(b, False)
        pg0.wait_for_function("typeof uipAdminOn === 'function' && typeof QTOOLS !== 'undefined'", timeout=120000)
        step("without the administrator mode there is no entry", lambda: (_ for _ in ()).throw(AssertionError("entry found")) if pg0.locator("[data-v=attributio], #v-attributio").count() else None)
        pg0.close()

        pg = new_page(b, True)
        pg.wait_for_selector("[data-v=attributio]", timeout=120000)
        pg.click("[data-v=attributio]"); pg.wait_for_selector("#at-pick")

        def small():
            bad = pg.evaluate("""[...document.querySelectorAll('#v-attributio button, #v-attributio select')].filter(e => e.offsetParent)
                .map(e => [e, e.getBoundingClientRect()]).filter(([e, r]) => r.width < 24 || r.height < 24).map(([e, r]) => (e.id || e.textContent) + ' ' + Math.round(r.width) + 'x' + Math.round(r.height))""")
            assert not bad, bad
        step("the controls are at least 24 x 24 px and have a name", small)

        def load():
            pg.set_input_files("#at-dir", str(FOLDER))
            try: pg.wait_for_function("document.querySelectorAll('#at-table tbody tr').length > 0", timeout=120000)
            except Exception: raise AssertionError("no rows: " + pg.inner_text("#at-msg") + " " + str(errs[:3]))
            rows = pg.evaluate("[...document.querySelectorAll('#at-table tbody tr')].map(tr => [...tr.cells].map(c => c.textContent))")
            assert len(rows) == 15, (len(rows), pg.inner_text("#at-msg"))
            got = {(r[0], r[1]): (r[4][1:], r[6]) for r in rows}
            assert got == want, {k: (got.get(k), want.get(k)) for k in want if got.get(k) != want.get(k)}
            txt = pg.inner_text("#at-counts"); assert txt.startswith("15 features") and "L1 1" in txt, txt
        step("the synthetic folder gives 15 features with the levels of livelli_attesi.csv", load)

        def filters():
            pg.select_option("#at-flv", "3a")
            n = pg.evaluate("document.querySelectorAll('#at-table tbody tr').length")
            assert n == sum(1 for v in want.values() if v[0] == "3a"), n
            pg.select_option("#at-flv", ""); pg.select_option("#at-fpol", "NEG")
            assert pg.evaluate("document.querySelectorAll('#at-table tbody tr').length") == 2
            pg.select_option("#at-fpol", "")
            pg.click("#at-table th:nth-child(5) button")
            assert pg.get_attribute("#at-table th:nth-child(5)", "aria-sort") == "ascending"
            first = pg.evaluate("document.querySelector('#at-table tbody tr').cells[4].textContent")
            assert first == "L0", first
        step("the filters of level and polarity and the sort work", filters)

        def exports():
            with pg.expect_download() as d: pg.click("#at-csv")
            text = Path(d.value.path()).read_text(encoding="utf-8")
            rows = list(csv.reader(io.StringIO(text), delimiter=";"))
            assert rows[0][:5] == ["pol", "id", "mz", "rt", "level"] and len(rows) == 16, (rows[0], len(rows))
            with pg.expect_download() as d: pg.click("#at-xlsx")
            assert d.value.suggested_filename.endswith(".xlsx") and Path(d.value.path()).read_bytes()[:2] == b"PK"
        step("Export CSV and Export XLSX download the table", exports)

        def notebook():
            op = pg.evaluate("(NB.ops || []).filter(o => o.op === 'attributio').pop() || null")
            assert op and len(op["attributio"]["fingerprint"]) == 24 and op["attributio"]["counts"]["1"] == 1, op
        step("the fingerprint and the counts reach the notebook", notebook)

        def export_menu():
            # the exports need the calibration, the curation and the Level 1-3 decisions of the export campaign laid over the synthetic one
            merged = tmp / "merged"; merged.mkdir()
            for part in ("campagna_sintetica", "campagna_esportazione"): shutil.copytree(ROOT / "tests" / "data" / "attributio" / part, merged, dirs_exist_ok=True)
            pg.set_input_files("#at-dir", str(merged))
            pg.wait_for_function("!document.querySelector('#at-export').disabled || document.querySelector('#at-msg').className === 'err'", timeout=120000)
            assert pg.get_attribute("#at-msg", "class") != "err", pg.inner_text("#at-msg")
            pg.click("#at-export"); assert pg.get_attribute("#at-export", "aria-expanded") == "true"
            items = pg.evaluate("[...document.querySelectorAll('#at-export-menu button')].map(b => [b.dataset.kind, b.getBoundingClientRect().height])")
            assert [k for k, _ in items] == ["evidence", "schymanski", "publication", "quantify", "gradient"] and all(h >= 24 for _, h in items), items
            pg.keyboard.press("Escape"); assert pg.get_attribute("#at-export", "aria-expanded") == "false"
            got = {}
            for kind in ("evidence", "schymanski", "publication", "quantify", "gradient"):
                pg.click("#at-export")
                with pg.expect_download(timeout=120000) as d: pg.click(f"#at-export-menu [data-kind={kind}]")
                got[kind] = (d.value.suggested_filename, Path(d.value.path()).read_bytes())
            assert len(got) == 5 and all(v[1][:2] == b"PK" for v in got.values()), {k: (v[0], v[1][:4]) for k, v in got.items()}
            def first_row(blob, name="xl/worksheets/sheet1.xml"):
                with zipfile.ZipFile(io.BytesIO(blob)) as z: x = z.read(name).decode("utf-8")
                return re.findall(r"<t[^>]*>([^<]*)</t>", x.split("</row>")[0])
            def inner(blob):
                with zipfile.ZipFile(io.BytesIO(blob)) as z: return {n.split("/", 1)[1]: z.read(n) for n in z.namelist()}
            assert got["evidence"][0].startswith("evidence_") and got["evidence"][0].endswith(".zip")
            ev = inner(got["evidence"][1]); assert any(n.startswith("evidence_") and n.endswith(".json") for n in ev) and any(n.endswith(".csv") for n in ev), list(ev)
            assert first_row(got["schymanski"][1])[:3] == ["schymanski_level", "identity", "category"], first_row(got["schymanski"][1])
            pub = inner(got["publication"][1]); xl = [n for n in pub if n.endswith(".xlsx")]; assert xl and any(n.endswith(".csv") for n in pub), list(pub)
            assert first_row(pub[xl[0]])[:3] == ["ID", "Compound", "Confidence level"], first_row(pub[xl[0]])
            assert first_row(got["quantify"][1])[:3] == ["_pol", "_id", "std_name"], first_row(got["quantify"][1])
            gr = first_row(got["gradient"][1]); assert gr[:2] == ["canonical_name", "status"], gr
        step("Export gives the five files and the spreadsheets open with their headers", export_menu)

        step("no request is left unserved and the page has no error", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if (errs or pg.evaluate("(window.MZLAB_LR_MISS || []).length")) else None)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
