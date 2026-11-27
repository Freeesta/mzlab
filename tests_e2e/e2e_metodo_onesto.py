"""The Method window shows only what the file says, and says where each value comes from (docs/agenti/method.md, "Honest view"), at 1400 and 390 px on the built site.
Synthetic files only (tools/wiff_riferimento.py): `dadmetodo.wiff` (an LC method with a pump program that ends before the run does, a PDA and a DAD header) and
`dispersione.wiff` (tandem segments whose fill time and Q1 resolution are not read). Checks: the pump line is drawn to the end of the run (the held part dashed, named as a hold),
the run time sits under the table it belongs to, no eluent composition appears when the file names none, every LC/DAD value carries its source on hover and in a collapsed list,
the tandem table uses n/a and n/r with a legend and the long text as tooltip and aria-label, the DAD is a labelled section (no raw stream path in its text), every number has at most
two decimals. Screenshots (deviceScaleFactor 2) go to MZLAB_SHOTS (default: the temporary folder). Needs the site and the .wasm (python tools/build_site.py)."""
# covers: mzlab/web/metodo-file.js, mzlab/web/metodo-doc.js, mzlab/web/eventi.js, crates/mzlab-core/src/methodview.rs
import os, re, shutil, subprocess, sys, tempfile
from pathlib import Path
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("FAIL site/ (with static/wasm) not built: python tools/build_site.py"); sys.exit(1)

D = Path(tempfile.mkdtemp(prefix="metodo_onesto_e2e_"))
SHOTS = Path(os.environ.get("MZLAB_SHOTS", tempfile.gettempdir()))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(D)], check=True, capture_output=True)
R = Run()
errs = []
BOX = """s => { const e = document.querySelector(s); if (!e) return null; const r = e.getBoundingClientRect(); return { l: r.left, r: r.right, t: r.top, b: r.bottom }; }"""

def open_method(pg, stem):
    pg.goto(f"http://127.0.0.1:{R.port}/")
    pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=60000)
    pg.set_input_files("#pick", [str(D / f"{stem}.wiff"), str(D / f"{stem}.wiff.scan")])
    pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
    pg.evaluate("Array.from(document.querySelectorAll('#flist input[data-k=use]')).forEach(i => { if (!i.checked) i.click(); })")
    pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
    pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000)
    pg.click("#np-method")
    pg.wait_for_selector("#bigdlg[open]", timeout=60000)

def lc_checks(pg, w):
    pg.wait_for_selector("#bigdlg [data-mlc=lc]", timeout=60000)
    def hold():
        line = pg.query_selector("#bigdlg [data-mlc=hold]")
        assert line, "no held part drawn"
        assert (line.get_attribute("stroke-dasharray") or "").strip(), "the held part is not dashed"
        tip = line.evaluate("e => e.querySelector('title').textContent")
        assert "Held" in tip and "not points of the file" in tip, tip
        x2 = float(line.get_attribute("x2"))
        svg_w = float(pg.evaluate("() => document.querySelector('#bigdlg [data-mlc=lc] svg').viewBox.baseVal.width"))
        assert x2 == svg_w - 10, ("the line does not reach the end of the axis", x2, svg_w)
        assert pg.query_selector("#bigdlg [data-mlc=holdrow]"), "the held row is missing from the table"
    step(f"{w}: the pump line is drawn to the end of the run, the held part dashed and named", hold)
    def runtime():
        foot, tbl = pg.evaluate(BOX, "#bigdlg [data-mlc=foot]"), pg.evaluate(BOX, "#bigdlg [data-mlc=table]")
        assert foot and tbl, (foot, tbl)
        assert foot["t"] >= tbl["b"] - 1 and foot["t"] - tbl["b"] < 60, ("the run time is not directly under the table", foot, tbl)
        assert abs(foot["r"] - tbl["r"]) < 40 or foot["r"] <= tbl["r"] + 1 and foot["r"] > tbl["l"], ("not on the right of the table", foot, tbl)
        t = pg.inner_text("#bigdlg [data-mlc=run]")
        assert re.search(r"Run time\s+2\.00 min", t), t
    step(f"{w}: the run time sits under the right side of the table, two decimals", runtime)
    def provenance():
        txt = pg.inner_text("#bigdlg")
        for bad in ("0.1% FA", "0.1 % FA", "formic", "H2O +", "ACN +", "Acetonitrile"):
            assert bad.lower() not in txt.lower(), (bad, "an eluent composition the file does not name")
        assert not pg.query_selector("#bigdlg [data-mlc=eluents]")
        titles = pg.evaluate("() => Array.from(document.querySelectorAll('#bigdlg [data-mlc=table] [title]')).map(e => e.getAttribute('title'))")
        assert any("Time Program" in t and t.startswith("Read from the file") for t in titles), titles
        assert any(t.startswith("Inferred") and "100 - %B" in t for t in titles), titles
        chip = pg.get_attribute("#bigdlg [data-mlc=run]", "title"); assert chip and "Stop event" in chip, chip
        pg.click("#bigdlg [data-mlc=lc] details[data-mfile=sources] summary")
        lst = pg.inner_text("#bigdlg [data-mlc=lc] details[data-mfile=sources]")
        assert "Inferred" in lst and "Read from the file" in lst and "VendorAppMethod" in lst, lst
    step(f"{w}: no eluent composition is printed; each value carries its source (read or inferred) on hover and in the list", provenance)
    def dad():
        sec = "#bigdlg [data-mdad=dad]"
        assert pg.query_selector(sec), "no DAD section"
        t = pg.inner_text(sec)
        assert "Diode array (DAD)" in t, t
        for lab in ("Wavelength range", "Step"):
            assert lab in t, (lab, t)
        assert "210.00" in t and "nm" in t, t
        assert "SampleSubtree" not in t and "DADSampleHeader" not in t and "wavelength_min" not in t and "VendorAppMethod" not in t, t
        tips = pg.evaluate("(s) => Array.from(document.querySelectorAll(s + ' [title]')).map(e => e.getAttribute('title'))", sec)
        assert any("PDA Detector" in x and x.startswith("Read from the file") for x in tips), tips
    step(f"{w}: the DAD is a labelled section; the stream path is only a tooltip", dad)
    def numbers():
        txt = pg.inner_text("#bigdlg")
        assert not re.search(r"\d+\.\d{3,}", txt), re.findall(r"\S*\d+\.\d{3,}\S*", txt)
    step(f"{w}: every number of the window has at most two decimals", numbers)

def tandem_checks(pg, w):
    pg.wait_for_selector("#bigdlg [data-mfile=segments]", timeout=60000)
    def codes():
        t = pg.inner_text("#bigdlg [data-mfile=segments]")
        assert "n/r" in t and "not read for this format" not in t and "not in the method" not in t, t
        leg = pg.inner_text("#bigdlg [data-mfile=legend]")
        assert "n/r = not read by mzLab for this format" in leg, leg
        cell = pg.query_selector("#bigdlg [data-mfile=segments] td[aria-label]:has-text('n/r')")
        assert cell, "no coded cell with an aria-label"
        assert len(cell.get_attribute("aria-label")) > 20 and cell.get_attribute("title") == cell.get_attribute("aria-label")
        for want in ("253.10", "35.00 eV", "15.00 eV", "60.00 V"):
            assert want in t, (want, t)
    step(f"{w}: the tandem table uses short codes with a one-line legend; the long text is tooltip and aria-label", codes)

try:
    with sync_playwright() as p:
        for w, h in ((1400, 1200), (390, 1400)):
            pg = R.page(p, viewport={"width": w, "height": h}, device_scale_factor=2)
            pg.set_default_timeout(120000); pg.on("pageerror", lambda e: errs.append(str(e)))
            open_method(pg, "dadmetodo")
            lc_checks(pg, w)
            def noscroll():
                sw = pg.evaluate("() => document.documentElement.scrollWidth"); assert sw <= w + 1, ("horizontal page scroll", sw, w)
            step(f"{w}: no horizontal page scroll", noscroll)
            pg.screenshot(path=str(SHOTS / f"metodo-onesto-lc-{w}.png"), full_page=False)
            pg.close()
            pg = R.page(p, viewport={"width": w, "height": h}, device_scale_factor=2)
            pg.set_default_timeout(120000); pg.on("pageerror", lambda e: errs.append(str(e)))
            open_method(pg, "dispersione")
            tandem_checks(pg, w)
            pg.screenshot(path=str(SHOTS / f"metodo-onesto-tandem-{w}.png"), full_page=False)
            pg.close()
        step("no page error", lambda: (_ for _ in ()).throw(AssertionError(errs)) if errs else None)
finally:
    R.close()
    shutil.rmtree(D, ignore_errors=True)
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
