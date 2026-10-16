"""Banco 2b of Attributio (TP_Mine/js/52_attributio_banco.js, feat-worker.js, the .wasm `attributio_rev`). With `qqq.admin` the synthetic campaign plus tests/data/attributio/campagna_banco (one more
feature, a Level 3 paracetamol with an MS2 and a library of three isomers) is loaded; the Banco opens from the row of POS 15 (class 2b-check, six boxes none ticked, charts with their table of data),
Save stays disabled while a decision that changes something has no note, three boxes + a note + Save download feature_decisions.csv with the expected row and evidence_checklist, the table shows
the feature as 2b, the arrows go to the next feature, the controls are at least 24 x 24 px, MZLAB_LR_MISS stays empty and both .wasm are under their cap. Builds the site and the .wasm (SKIP without
cargo and wasm-bindgen); MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses a site. Run from the repo root: python3 tests_e2e/e2e_attributio_banco.py"""
import csv, io, json, os, shutil, subprocess, sys, tempfile, threading, urllib.request
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

DATA = ROOT / "tests" / "data" / "attributio"
tmp = Path(tempfile.mkdtemp(prefix="attrb_"))
FOLDER = tmp / "camp"
shutil.copytree(DATA / "campagna_sintetica", FOLDER)
shutil.copytree(DATA / "campagna_banco", FOLDER, dirs_exist_ok=True)
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
for key, cap in (("attributio", 512000), ("attributio_rev", 1000000)):
    size = (site / "static" / "wasm-feat" / man[key]["wasm"]).stat().st_size
    assert size < cap, (key, size)
    print(f"  {key} .wasm:", size, "bytes")

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
URL = f"http://127.0.0.1:{PORT}/"
for _ in range(100):
    try: urllib.request.urlopen(URL, timeout=1).read(); break
    except Exception: threading.Event().wait(0.1)
errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        pg = b.new_context(viewport={"width": 1280, "height": 1000}, reduced_motion="reduce", accept_downloads=True).new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector("[data-v=attributio]", timeout=120000)
        pg.click("[data-v=attributio]"); pg.wait_for_selector("#at-pick")
        pg.set_input_files("#at-dir", str(FOLDER))
        pg.wait_for_function("document.querySelectorAll('#at-table tbody tr').length > 0", timeout=120000)

        def table():
            assert pg.evaluate("document.querySelectorAll('#at-table tbody tr').length") == 16
            assert pg.locator('.at-rev[data-pol="POS"][data-id="15"]').count() == 1
        step("the table has the 16 features and a Review button per row", table)

        def opens():
            pg.click('.at-rev[data-pol="POS"][data-id="15"]'); pg.wait_for_selector("#bn-save", timeout=120000)
            txt = pg.inner_text("#at-banco")
            assert "2b-check" in txt and "Metacetamol" in txt and "Banco 2b" in txt, txt[:600]
            boxes = pg.evaluate("[...document.querySelectorAll('#at-banco [data-chk]')].map(c => c.checked)")
            assert boxes == [False] * 6, boxes
            assert "3 of 3 peaks explained" in txt
            assert pg.locator("#bn-mirror").count() == 1 and pg.locator("#bn-iso").count() == 1
        step("the Banco opens on POS 15: class 2b-check, reasons, proofs, six boxes none ticked", opens)

        def data_table():
            pg.focus("#bn-mirror"); pg.keyboard.press("t"); pg.wait_for_selector("dialog#tdati[open]")
            rows = pg.evaluate("[...document.querySelectorAll('#tdati tbody tr')].map(r => [...r.cells].map(c => c.textContent))")
            assert len(rows) == 3 and rows[0][2] == "C6H8NO", rows
            pg.keyboard.press("Escape"); pg.wait_for_selector("dialog#tdati", state="detached")
        step("the mirror spectrum has its table of data (T on the chart)", data_table)

        def zoom():
            w0 = pg.evaluate("[...document.querySelectorAll('#bn-mirror text')].length")
            pg.click('#at-banco [data-zoom="in"]'); w1 = pg.evaluate("[...document.querySelectorAll('#bn-mirror text')].length")
            pg.click('#at-banco [data-zoom="reset"]')
            assert pg.evaluate("document.querySelectorAll('#bn-mirror').length") == 1 and w1 <= w0, (w0, w1)
        step("zoom works with buttons", zoom)

        def small():
            bad = pg.evaluate("""[...document.querySelectorAll('#at-banco button, #at-banco select, #at-banco input')].filter(e => e.offsetParent)
                .map(e => [e, e.getBoundingClientRect()]).filter(([e, r]) => r.width < 24 || r.height < 24).map(([e, r]) => (e.id || e.getAttribute('aria-label')) + ' ' + Math.round(r.width) + 'x' + Math.round(r.height))""")
            assert not bad, bad
            nameless = pg.evaluate("[...document.querySelectorAll('#at-banco button, #at-banco select, #at-banco input, #at-banco textarea')].filter(e => !e.getAttribute('aria-label') && !e.textContent.trim()).length")
            assert nameless == 0, nameless
        step("the controls are at least 24 x 24 px and have a name", small)

        def needs_note():
            pg.fill("#bn-by", "rev"); pg.select_option("#bn-lvl", "2b")
            assert pg.is_disabled("#bn-save"), "Save must wait for the note"
            assert "motivation is required" in pg.inner_text("#bn-msg")
        step("a decision against the proposal's level has no Save without a note", needs_note)

        def saves():
            for k in ("diagnostic_fragment", "neutral_loss", "alternatives_excluded"): pg.check(f'[data-chk="{k}"]')
            pg.select_option("#bn-ev", "fragments"); pg.fill("#bn-note", "diagnostic ions explained")
            assert not pg.is_disabled("#bn-save")
            with pg.expect_download() as d: pg.click("#bn-save")
            assert d.value.suggested_filename == "feature_decisions.csv"
            rows = list(csv.reader(io.StringIO(Path(d.value.path()).read_text(encoding="utf-8")), delimiter=";"))
            head = rows[0]; assert head[-1] == "evidence_checklist", head
            mine = [dict(zip(head, r)) for r in rows[1:] if r[head.index("feature_id")] == "15"]
            assert len(mine) == 1, rows
            r = mine[0]
            assert (r["pol"], r["livello"], r["decided_by"], r["motivazione"]) == ("POS", "2b", "rev", "diagnostic ions explained"), r
            assert r["evidence_checklist"] == "diagnostic_fragment|neutral_loss|alternatives_excluded" and r["evidence_type"] == "fragments", r
            assert len(rows) == 6, len(rows)
        step("three boxes + a note + Save download feature_decisions.csv with the row and evidence_checklist", saves)

        def reruns():
            pg.wait_for_function("document.querySelector('#at-table tr:has(.at-rev[data-id=\"15\"]) .lvb')?.textContent === 'L2b'", timeout=60000)
            assert pg.evaluate("localStorage.getItem('qqq.attributio.reviewer')") == "rev"
            op = pg.evaluate("(NB.ops || []).filter(o => o.op === 'attributio' && o.attributio.feature_decisions).pop() || null")
            assert op and "diagnostic_fragment|neutral_loss|alternatives_excluded" in op["attributio"]["feature_decisions"], op
        step("the cascade runs again (POS 15 is L2b), the reviewer is remembered and the decisions reach the notebook", reruns)

        def arrows():
            h0 = pg.inner_text("#bn-h"); pg.focus("#bn-h"); pg.keyboard.press("ArrowRight")
            pg.wait_for_function("(h) => document.querySelector('#bn-h') && document.querySelector('#bn-h').textContent !== h", arg=h0, timeout=60000)
            h1 = pg.inner_text("#bn-h"); assert "2 of 16" in h1 or "(" in h1, h1
            pg.keyboard.press("ArrowLeft")
            pg.wait_for_function("(h) => document.querySelector('#bn-h') && document.querySelector('#bn-h').textContent === h", arg=h0, timeout=60000)
        step("the arrows go to the next and the previous feature", arrows)

        step("no request is left unserved and the page has no error", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if (errs or pg.evaluate("(window.MZLAB_LR_MISS || []).length")) else None)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
