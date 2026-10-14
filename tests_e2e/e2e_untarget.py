"""mzFinder, feature tables (untargeted): files of an mzMine table are loaded in the window, every feature with an MS2 is searched in a library imported in the page, the table shows level, best
candidate, entropy and evidence; filter by level, click → mirror and «What is it?», the chemist's decided level (2b only from 2a or 3), the external annotations are evidence and never a level.
Needs the built site (python3 tools/build_site.py). UNT_BENCH=1 also times 5000 features (the result goes to docs/agenti/misure.md).
Run from the repo root: python3 tests_e2e/e2e_untarget.py"""
import json, os, random, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT

PORT = 8851
errs, steps = [], []
def step(name, fn):
    try: fn(); print("  ", (name, "ok")); steps.append(1)
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:500])); steps.append(0)

if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python3 tools/build_site.py")); sys.exit(0)
# the page of the site is a copy: make sure it has the current sources of mzFinder and of the library module
mf = ROOT / "site" / "static" / "mzfinder"
for f in (ROOT / "TP_Mine" / "js").glob("*.js"): shutil.copyfile(f, mf / f.name)
idx = json.loads((mf / "indice.json").read_text()); idx["js"] = sorted(f.name for f in (ROOT / "TP_Mine" / "js").glob("*.js")); (mf / "indice.json").write_text(json.dumps(idx))
for f in ("libreria-worker.js", "libreria.js", "livelli.js", "livelli-1.js"): shutil.copyfile(ROOT / "mzlab" / "web" / f, ROOT / "site" / "static" / f)

A = [(85.0284, 40), (105.0699, 120), (130.0651, 300), (158.0964, 1000), (186.0913, 600), (212.1070, 250), (240.1019, 180), (283.1441, 90)]
ALPHA = "Name: Alpha\nPrecursorMZ: 301.1547\nIon_mode: Positive\nPrecursor_type: [M+H]+\nCollision_energy: 30\nInstrument_type: Orbitrap\nInChIKey: AAAAAAAAAAAAAA-UHFFFAOYSA-N\nNum Peaks: %d\n%s\n" % (len(A), "\n".join(f"{m:.4f}\t{i}" for m, i in A))
def mgf(fid, peaks, n=None):
    return f"BEGIN IONS\nFEATURE_ID={fid}\nPEPMASS=301.1547\nMSLEVEL=2\nIONMODE=Positive\n" + "".join(f"{m} {i}\n" for m, i in peaks) + "END IONS\n"
HEAD = "id,mz,rt,fragment_scans,datafile:S1_pos.mzML:feature_state,datafile:S1_pos.mzML:area,spectral_db_matches:compound_name,spectral_db_matches:similarity_score,spectral_db_matches:n_matching_signals,spectral_db_matches:ion_adduct,spectral_db_matches:mol_formula,spectral_db_matches:precursor_mz\n"
ROWS = ["1,301.1547,5.0,\"1,2,3\",DETECTED,100000,,,,,,", "2,301.1547,6.0,\"1\",DETECTED,90000,,,,,,", "3,450.2,6.5,,DETECTED,5000,,,,,,", "4,299.1754,9.5,,DETECTED,60000,Testagel,0.95,9,M+H,C18H22N2O2,299.1754",
        "5,301.1547,7.0,\"1,2\",DETECTED,40000,,,,,,", "6,301.1547,8.0,\"1,2\",DETECTED,30000,,,,,,"]
OTHER = [(50.0 + 13.7 * k, 100 + 37 * k) for k in range(8)]
MGF = mgf(1, A) + mgf(2, A) + mgf(5, OTHER) + mgf(6, A[:2])
tmp = Path(tempfile.mkdtemp()); (tmp / "tabella_pos.csv").write_text(HEAD + "\n".join(ROWS) + "\n"); (tmp / "fbmn.mgf").write_text(MGF)
ADD = """async ([name, text]) => { const r = await LIB.call("add", { file: new File([text], name, { type: "text/plain" }) }); return r.info; }"""

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); pg = b.new_context(viewport={"width": 1500, "height": 1200}).new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.on("response", lambda r: errs.append(("http%d" % r.status, r.url)) if r.status >= 400 and "logo.svg" not in r.url and "/vendor/" not in r.url else None)
        def boot():
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.LIB !== undefined && LIB.engine !== undefined", timeout=60000)
            for _ in range(5): pg.click("header img.logo")
            pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000); pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-unt", timeout=60000)
        step("mzFinder is on and the feature-table button exists", boot)
        def lib():
            info = pg.evaluate(ADD, ["alpha.msp", ALPHA]); assert info["n"] == 1, info
            pg.click("#tp-unt"); pg.wait_for_selector("#unt[open]", timeout=10000)
        step("a library is imported and the window opens", lib)
        def load():
            pg.set_input_files("#unt-in", [str(tmp / "tabella_pos.csv"), str(tmp / "fbmn.mgf")])
            pg.wait_for_function("document.querySelector('#unt-sum').textContent.includes('6 features')", timeout=20000)
            s = pg.inner_text("#unt-sum"); assert "4 with MS" in s and "positive" in s and "1 with an external annotation" in s, s
            assert pg.evaluate("UNTUI.state.table.file.sha256.length") == 64
        step("the table and its MGF are read; polarity from the file name; SHA-256 of the table", load)
        def match():
            pg.click("#unt-go"); pg.wait_for_selector("#unt-res:not([hidden])", timeout=60000)
            lv = pg.evaluate("Object.fromEntries([...document.querySelectorAll('#unt-t tr.row1')].map(tr => [tr.children[1].textContent, tr.children[0].textContent.trim()]))")
            assert lv == {"1": "2a", "2": "3", "3": "5", "4": "4", "5": "5", "6": "5"}, lv
            r1 = pg.evaluate("[...document.querySelectorAll('#unt-t tr.row1')].find(tr => tr.children[1].textContent === '1').innerText")
            assert "Alpha" in r1 and "1.000" in r1, r1
            assert "Testagel" in pg.evaluate("[...document.querySelectorAll('#unt-t tr.row1')].find(tr => tr.children[1].textContent === '4').innerText")          # evidence of the external source
        step("levels: 2a (3 scans), 3 (one scan), 5 (other spectrum, no MS2), 4 (formula of an external source explains the m/z)", match)
        def filt():
            pg.select_option("#unt-lv", "5"); assert pg.evaluate("document.querySelectorAll('#unt-t tr.row1').length") == 3
            pg.select_option("#unt-lv", "all"); pg.fill("#unt-q", "testagel"); assert pg.evaluate("document.querySelectorAll('#unt-t tr.row1').length") == 1
            pg.fill("#unt-q", ""); pg.check("#unt-ms2"); assert pg.evaluate("document.querySelectorAll('#unt-t tr.row1').length") == 4; pg.uncheck("#unt-ms2")
        step("filter by level, text and MS2", filt)
        def detail():
            pg.click("#unt-t tr.row1[data-k='POS:1']"); pg.wait_for_function("document.querySelector('#unt-mm').textContent.includes('Alpha')", timeout=20000)
            assert pg.evaluate("document.getElementById('unt-cv').width") > 0 and pg.evaluate("!document.getElementById('unt-cv').hidden")
            assert "Level" in pg.inner_text("#unt-det") and "entropy" in pg.inner_text("#unt-det").lower()
            pg.click("#unt-what"); pg.wait_for_selector("#libres[open]", timeout=20000); pg.wait_for_function("document.querySelector('#libres #lr-body').textContent.includes('Alpha')", timeout=20000)
            pg.click("#lr-x")
        step("click → mirror plot and the «What is it?» window", detail)
        def decide():
            pg.select_option("#unt-dec", "2b"); badge = pg.evaluate("document.querySelector(\"#unt-t tr.row1[data-k='POS:1'] .lvb\").textContent")
            assert badge.startswith("2b"), badge
            pg.click("#unt-t tr.row1[data-k='POS:3']")
            pg.select_option("#unt-dec", "2b"); pg.wait_for_function("document.querySelector('#unt-decmsg').textContent.length > 0")
            assert pg.evaluate("document.querySelector(\"#unt-t tr.row1[data-k='POS:3'] .lvb\").textContent.trim()") == "5"          # refused: the programme never gives 2b
        step("the chemist's 2b is accepted from 2a and refused from 5", decide)
        def export():
            def get(sel):
                with pg.expect_download() as d: pg.click(sel)
                path = d.value.path(); return d.value.suggested_filename, Path(path).read_bytes()
            n, b = get("#unt-x-csv"); t = b.decode("utf-8")
            assert n.startswith("untargeted_tabella_pos_") and n.endswith(".csv") and t.startswith("\ufeff"), n
            lines = t.lstrip("\ufeff").split("\r\n")[:-1]; head = lines[0].split(";")
            assert len(lines) == 7 and head[:3] == ["key", "polarity", "feature_id"] and head[-1] == "S1_pos", (len(lines), head[-3:])
            r1 = lines[1].split(";"); assert r1[0] == "POS:1" and r1[head.index("level")] == "2b" and r1[head.index("level_proposed")] == "2a" and r1[head.index("level_decided")] == "2b" and len(r1[head.index("table_sha256")]) == 64
            n, b = get("#unt-x-xlsx"); assert n.endswith(".xlsx") and b[:2] == b"PK" and b"Library records" in b, n
            n, b = get("#unt-x-mgf"); m = b.decode("utf-8")
            assert m.startswith("# mzFinder untargeted export") and m.count("BEGIN IONS") == 4 and "LEVEL=2b" in m and "NAME=Alpha" in m and "not stated" in m
            pg.select_option("#unt-lv", "5"); pg.check("#unt-xf"); n, b = get("#unt-x-csv"); assert len(b.decode("utf-8").lstrip("\ufeff").split("\r\n")[:-1]) == 1 + 3          # only the filtered features
            pg.uncheck("#unt-xf"); pg.select_option("#unt-lv", "all")
        step("export: CSV with «;», xlsx and MGF with levels, decided level, licences; only the filtered features", export)
        def fair():
            def get(sel):
                with pg.expect_download() as d: pg.click(sel)
                return d.value.suggested_filename, Path(d.value.path()).read_bytes()
            n, b = get("#unt-x-gnps"); assert n.startswith("gnps2_") and n.endswith(".zip") and b[:2] == b"PK" and b"quant.csv" in b and b"ms2.mgf" in b and b"row retention time" in b, n
            n, b = get("#unt-x-sirius"); t = b.decode("utf-8"); assert n.endswith(".ms") and t.count(">compound") == 4 and ">parentmass" in t and ">ms2" in t and "NAME=" not in t, n
        step("export for GNPS2 (zip with quant + MGF) and SIRIUS (.ms of the features with MS2)", fair)
        if os.environ.get("UNT_BENCH"):
            def bench():
                rnd = random.Random(7); N = 5000
                lib = []
                for i in range(20000):
                    mz = 150 + rnd.random() * 600; pk = sorted((round(40 + rnd.random() * (mz - 50), 4), 10 + rnd.randint(0, 990)) for _ in range(12))
                    lib.append(f"Name: Syn{i}\nPrecursorMZ: {mz:.4f}\nIon_mode: Positive\nPrecursor_type: [M+H]+\nCollision_energy: 30\nInstrument_type: Orbitrap\nNum Peaks: 12\n" + "\n".join(f"{m}\t{k}" for m, k in pk) + "\n")
                pg.evaluate(ADD, ["syn.msp", "\n".join(lib)])
                rows, blocks = [], []
                for i in range(N):
                    mz = 150 + rnd.random() * 600; pk = sorted((round(40 + rnd.random() * (mz - 50), 4), 10 + rnd.randint(0, 990)) for _ in range(12))
                    rows.append(f"{i},{mz:.4f},{rnd.random() * 20:.2f},\"1,2\",DETECTED,{1000 + i},,,,,,"); blocks.append(f"BEGIN IONS\nFEATURE_ID={i}\nPEPMASS={mz:.4f}\nMSLEVEL=2\nIONMODE=Positive\n" + "".join(f"{m} {k}\n" for m, k in pk) + "END IONS\n")
                (tmp / "big_pos.csv").write_text(HEAD + "\n".join(rows) + "\n"); (tmp / "big.mgf").write_text("".join(blocks))
                pg.evaluate("UNTUI.state.files.length = 0")
                t1 = time.time(); pg.set_input_files("#unt-in", [str(tmp / "big_pos.csv"), str(tmp / "big.mgf")]); pg.wait_for_function("document.querySelector('#unt-sum').textContent.includes('5,000 features')", timeout=60000)
                dl_ = time.time() - t1; t0 = time.time(); pg.click("#unt-go"); pg.wait_for_function("document.querySelector('#unt-msg').textContent.includes('spectra searched')", timeout=600000)
                dt = time.time() - t0; eng = pg.evaluate("LIB.engine.info()")
                print("   BENCH 5000 features x 20000-entry library: read", round(dl_, 1), "s, match", round(dt, 1), "s; wasm engine:", eng.get("wasm"))
                if eng.get("wasm"): assert dt < 15, dt
            step("5000 features in under 15 s (WebAssembly engine)", bench)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
bad = [e for e in errs if not (isinstance(e, tuple) and "wasm-lib" in str(e[1]))]          # the .wasm of the library engine exists only in the CI build: the page then uses the JavaScript path
print("  ", ("browser errors", "ok" if not bad else "FAIL " + str(bad[:3])))
