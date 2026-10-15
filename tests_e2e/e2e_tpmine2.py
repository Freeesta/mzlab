"""mzFinder (internal name TP Mine) (b)+(c): the engine in Pyodide and the mzFinder tab. Synthetic LR demo + the real B_ series if the mzML are there.
Run from the repo root: python3 tests_e2e/e2e_tpmine2.py   (env: TPMINE_SRC, QQQ_MZML, PYODIDE_DIR)"""
import os, shutil, subprocess, sys, time, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
SRC = Path(os.environ.get("TPMINE_SRC", ROOT / "TP_Mine"))
MZ = Path(os.environ.get("QQQ_MZML", ROOT.parent / "Data" / "mzML"))
PORT = 8842
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:400]))
tmp = Path(tempfile.mkdtemp())
cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")]
if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
subprocess.run(cmd, check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")          # the engine of the page is Rust: the .wasm goes in the copy of the site
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1500, "height": 1400}).new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.on("response", lambda r: errs.append(("http%d" % r.status, r.url)) if r.status >= 400 and "logo.svg" not in r.url and "/vendor/" not in r.url and "wasm-feat/manifest.json" not in r.url else None)   # logo, vendor and wasm-feat (tools/build_feat_wasm.sh) are not in the cloud copy
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_timeout(3200)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        def demo():
            sys.path[:0] = [str(SRC / "py"), str(ROOT)]
            from tpmine.demo import make_demo
            dd = Path(tempfile.mkdtemp()); fl = make_demo(dd)
            pg.set_input_files("#tp-in", [f["path"] for f in fl])
            pg.wait_for_function("document.querySelector('#tp-files table') && document.querySelectorAll('#tp-files tr').length >= 9", timeout=240000)
            assert pg.evaluate("document.querySelector('.pm-bar') === null") and pg.evaluate("document.querySelectorAll('#tp-minecard button').length") == 1 and pg.evaluate("document.querySelector('#tp-go svg') !== null")
            assert pg.evaluate("[...document.querySelectorAll('#tp-files tr')].slice(1).filter(t => !t.hidden).length") == 6 and pg.get_attribute("#tp-more", "aria-expanded") == "false"
            assert pg.inner_text("#tp-fsum").strip() != "" and pg.evaluate("document.getElementById('tp-disc').offsetParent !== null") and pg.get_attribute("#tp-disc-h", "title")
            pg.click("#tp-more"); assert pg.evaluate("[...document.querySelectorAll('#tp-files tr')].slice(1).every(t => !t.hidden)") and pg.get_attribute("#tp-more", "aria-expanded") == "true"
            assert pg.input_value("#tp-tol") == "0.35" and pg.input_value("#tp-rtt") == "0.25"
            pg.fill("#tp-smi", "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O")
            pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('241.0641')", timeout=60000)
            assert "C10H12N2O3S" in pg.inner_text("#tp-prev")
            assert "hydroxylation" in pg.input_value("#tp-tr")
            pg.click("#tp-go"); pg.wait_for_selector("#tp-view .gem", timeout=300000)
            assert pg.evaluate("document.querySelectorAll('.gem.forte').length") == 3
            pg.wait_for_timeout(1300)
            assert pg.inner_text(".stat.g b").strip() == "3" and "Vein found" in pg.inner_text(".hero")
            assert pg.evaluate("document.querySelector('#tp-struct svg') !== null")          # parent structure drawn from the SMILES (needs vendor/openchemlib.js)
            pg.click('.vt[data-v="tab"]'); pg.wait_for_selector("#tp-t tr.clk")
            t = pg.inner_text("#tp-t")
            for need in ("hydroxylation", "loss of propene", "dehydrogenation"): assert need in t.lower(), need      # names are shown capitalised
            pg.click('.vt[data-v="map"]'); pg.wait_for_selector(".mnode"); assert pg.evaluate("document.querySelectorAll('.mnode').length") >= 4
            pg.click('.vt[data-v="kin"]'); pg.wait_for_selector("#tp-c-all"); assert pg.evaluate("document.getElementById('tp-c-all').width") > 0
            pg.click('.vt[data-v="film"]'); pg.wait_for_selector(".frow"); pg.click("#tp-play")
            pg.wait_for_function("/t = (5|10) min/.test(document.getElementById('tp-tl').textContent)", timeout=8000)
            pg.click('.vt[data-v="gems"]'); pg.click(".gem:has-text('hydroxylation')"); pg.wait_for_selector("#tp-c-xic", timeout=60000); pg.wait_for_timeout(500)
            d = pg.inner_text("#tp-det")
            assert "In words" in d and "Level 3" in d and "257.1" in d, d[:700]
            pg.screenshot(path=str(HERE / "shots" / "tpmine2_demo.png"), full_page=True)
        step("LR demo (synthetic): files, formula from SMILES, search, ranking, detail with MS2 and level 3", demo)
        def xl():
            with pg.expect_download(timeout=120000) as dl: pg.click("#tp-x-xlsx")
            path = dl.value.path(); import zipfile; z = zipfile.ZipFile(path); assert z.testzip() is None; names = z.namelist(); assert any("sheet" in n for n in names), names
        step("xlsx export", xl)
        def hr():
            sys.path[:0] = [str(SRC / "tests"), str(SRC / "py"), str(ROOT)]
            import lc_synth, msn_synth                                          # synthetic HR series (TP_Mine/tests): the HR mode end to end
            d = Path(tempfile.mkdtemp()); fs = lc_synth.write_series(d, hidden_isf=True); mf = msn_synth.write_msn(d / "cafe_msn.mzML", msn_synth.caffeine_nodes())
            pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.set_input_files("#tp-in", [f["path"] for f in fs] + [str(mf)])
            pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 8", timeout=300000)
            t = pg.evaluate("document.getElementById('tp-files').textContent"); assert "LC-HRMS" in t and "MSn (infusion)" in t, t
            assert pg.evaluate("document.getElementById('tp-disc').offsetParent === null")          # HR: the unexpected-ions box is hidden
            pg.fill("#tp-smi", "Cn1cnc2c1c(=O)n(C)c(=O)n2C"); pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('195.0877')", timeout=60000)
            pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
            tab = pg.inner_text("#hr-t")
            assert "C8H11N4O3" in tab and "C7H9N4O2" in tab, tab[:600]
            assert "hypothesis, not an identification" in pg.inner_text("#tp-right")
            pg.click("#hr-t tr.clk:has-text('C8H11N4O3')"); pg.wait_for_selector("#hr-ms2", timeout=60000); pg.wait_for_timeout(500)
            assert pg.evaluate("document.getElementById('hr-kin').width") > 0 and pg.evaluate("document.getElementById('hr-ms2').width") > 0
            assert "Criteria" in pg.inner_text("#tp-det")
            neg = pg.evaluate("[...document.querySelectorAll('#hr-t tr.clk')].map(t => +t.dataset.id).filter(i => i < 0).length")
            assert neg == 1, neg                                                  # the row hidden under an in-source fragment
            pg.click("#hr-t tr.clk[data-id^='-']"); pg.wait_for_function("document.querySelector('#tp-det').innerText.includes('C6H8N3O')", timeout=60000)
            assert "co-eluting" in pg.inner_text("#hr-t")
            pg.select_option("#hr-ord", "level"); pg.wait_for_selector("#hr-t tr.clk")
            with pg.expect_download(timeout=60000) as dlc: pg.click("#hr-x-csv")
            assert "C6H8N3O" in Path(dlc.value.path()).read_text(encoding="utf-8")
            with pg.expect_download(timeout=120000) as dl: pg.click("#hr-x-xlsx")
            import zipfile; z = zipfile.ZipFile(dl.value.path()); assert z.testzip() is None and any("sheet" in n for n in z.namelist())
            with pg.expect_download(timeout=60000) as dl2: pg.click("#hr-x-incl")
            assert dl2.value.path()
            pg.screenshot(path=str(HERE / "shots" / "tpmine2_hr.png"), full_page=True)
        step("HR mode (synthetic series + MSn): ranking, detail, mirror MS2, Excel, inclusion list", hr)
        def parent_list():
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-name")
            reqs = []
            pg.on("request", lambda r: reqs.append(r.url))
            pg.fill("#tp-name", ""); pg.fill("#tp-smi", ""); pg.fill("#tp-mol", "")
            pg.type("#tp-name", "caff", delay=40)
            pg.wait_for_selector('#tp-sug li[role=option]', timeout=30000)
            assert "caffeine" in pg.inner_text("#tp-sug").lower() and "C8H10N4O2" in pg.inner_text("#tp-sug")
            assert pg.get_attribute("#tp-name", "aria-expanded") == "true"
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("Enter")
            pg.wait_for_function("document.querySelector('#tp-mol').value === 'C8H10N4O2' && document.querySelector('#tp-smi').value.length > 5", timeout=30000)
            pg.wait_for_function("document.querySelector('#tp-struct svg')", timeout=60000)
            assert "195.0877" in pg.inner_text("#tp-prev")
            # unknown name -> only the PubChem search entry, with the exact word
            pg.fill("#tp-name", "")
            pg.type("#tp-name", "zzqxwv", delay=30)
            pg.wait_for_selector('#tp-sug li', timeout=15000)
            assert pg.evaluate("document.querySelectorAll('#tp-sug li').length") == 1 and 'Search PubChem for "zzqxwv"' in pg.inner_text("#tp-sug")
            with pg.expect_popup() as pop:
                pg.keyboard.press("ArrowDown"); pg.keyboard.press("Enter")
            assert pop.value.url.startswith("https://pubchem.ncbi.nlm.nih.gov/#query=zzqxwv") or "pubchem.ncbi.nlm.nih.gov" in pop.value.url, pop.value.url
            pop.value.close()
            # formula missing: computed from the SMILES; formula and SMILES that disagree -> warning
            pg.fill("#tp-name", ""); pg.fill("#tp-mol", ""); pg.fill("#tp-smi", "Cn1cnc2c1c(=O)n(C)c(=O)n2C")
            pg.wait_for_function("document.querySelector('#tp-mol').value === 'C8H10N4O2'", timeout=30000)
            pg.fill("#tp-mol", "C6H6")
            pg.wait_for_function("document.querySelector('#tp-warn').textContent.includes('does not match')", timeout=30000)
            assert pg.evaluate("document.getElementById('tp-draw').offsetParent !== null")
            assert not any("composti" in u for u in reqs), reqs
        step("parent: name list, SMILES field, PubChem link, structure in the title", parent_list)
        if "dati_sintetici" in str(MZ):
            steps.append(("real B_ series: offset +0.3 Da, decay, strong candidates", "SKIP the numbers of the real series need the private data (MZLAB_DATI); the synthetic look-alikes are not enough"))
        elif MZ.exists() and (MZ / "B_FullMass-t0.mzML").exists():
            def real():
                pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
                pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
                pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
                names = ["B_FullMass-t0", "B_FullMass-t5", "B_FullMass-t10", "B_FullMass-t15", "B_FullMass-t30 (2)", "B_FullMass-t45", "B_FullMass-t60", "B_MS2-t15", "B_MS2-t45", "B_MS2-t60", "B_MRM-t0", "B_MRM-t5", "B_MRM-t10", "B_MRM-t15", "B_MRM-t30", "B_MRM-t45", "B_MRM-t60", "B_MRM-STD_0_06ppm", "B_MRM-STD_0_6ppm", "B_MRM-STD_2_4ppm", "B_MRM-STD_7_2ppm", "B_MRM-STD_12ppm", "B_MRM-STD_18ppm"]
                pg.set_input_files("#tp-in", [str(MZ / (n + ".mzML")) for n in names])
                pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 24", timeout=300000)
                pg.fill("#tp-smi", "CC(C)N(C(=O)COc1nnc(s1)C(F)(F)F)c1ccc(F)cc1"); pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('364.0737')", timeout=60000)
                pg.click("#tp-go"); pg.wait_for_selector("#tp-view .gem", timeout=600000); pg.wait_for_timeout(1300)
                head = pg.inner_text(".hero")
                assert "t½" in head and "offset" in head
                vals = pg.evaluate("[...document.querySelectorAll('.stat b')].map(b=>parseFloat(b.textContent))")
                assert 0.25 <= vals[4] <= 0.35 and 2.0 < vals[3] < 4.0 and vals[0] >= 3, vals
                m = pg.inner_text("#tp-right")
                assert "MRM: automatic integration" in m and "Line: area" in m and "364.1>194.1" in m, m[:400]      # the mzFinder text is English only
                assert pg.evaluate("document.getElementById('tp-c-cal').width") > 0
                pg.screenshot(path=str(HERE / "shots" / "tpmine2_real.png"), full_page=True)
            step("real B_ series: offset +0.3 Da, decay, strong candidates", real)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s_ in steps: print(s_)
print("page errors:", errs)
sys.exit(1 if any(s_[1] != "ok" and not s_[1].startswith("SKIP") for s_ in steps) or errs else 0)
