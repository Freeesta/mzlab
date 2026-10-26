"""Tabui, the box «Plausibility of the values» of the quality control (TP_Mine/js/36_feature_qc.js, PIPEQC.plausibleHtml): a text of the engine (`plausible/1`) with suspect values gives a status
line, one Warning/Note item per flagged check with its row ids, and the «Not checked» note; a clean text gives the all-clear line and no list. Targets, axe in the light and dark theme, 390 px.
The box is mounted by hand with synthetic texts in #tp-right of the built site; the real box over a real run is in e2e_pipeline.py. MZLAB_PIPE_SITE reuses a built site.
Run from the repo root: python3 tests_e2e/e2e_tabui_plausible.py"""
import json, os, shutil, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)
from lib import axe_path  # noqa: E402

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tabui_plausible.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:900]))

def check(cid, flag="ok", count=0, limit=(), rows=(), missing=None): return {"id": cid, "flag": flag, "count": count, "limit": list(limit), "rows": list(rows), "missing": missing}
# every check of the engine, in its order; the suspect text flags five of them and cannot read the ppm error
SUSPECT = {"schema": "plausible/1", "rows": 1200, "summary": {"ok": 1, "note": 1, "warn": 4}, "checks": [
    check("non_finite", "warn", 2, rows=[4, 9]), check("mz_range", "warn", 1, [20, 5000], [17]), check("rt_range", "warn", 30, [0, 600], list(range(21, 41))),
    check("negative", "ok"), check("peak_shape", "warn", 3, [0, 30], [5, 6, 7]), check("empty_rows", "note", 1, rows=[88]), check("ppm_error", "na", 0, [50], [], "accurate_mass")]}
CLEAN = {"schema": "plausible/1", "rows": 50, "summary": {"ok": 7, "note": 0, "warn": 0}, "checks": [
    check(i, "ok", 0, l) for i, l in [("non_finite", []), ("mz_range", [20, 5000]), ("rt_range", [0, 600]), ("negative", [0]), ("peak_shape", [0, 30]), ("empty_rows", []), ("ppm_error", [50])]]}

tmp = Path(tempfile.mkdtemp(prefix="tabui_plausible_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse: subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True, capture_output=True)
else:                                                      # fresh sources of mzFinder over the site that was built
    for d in ("files", "js"):
        for f in (ROOT / "TP_Mine" / d).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
for _ in range(50):
    try: __import__("urllib.request").request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)
AXE = axe_path()
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0,3).map(n => n.target.join(' ') + ' :: ' + (n.failureSummary||'').split('\\n').slice(0,3).join(' | '))]))"
SHOTS = Path(os.environ.get("TMPDIR", "/tmp")) / "tabui_plausible_shots"; SHOTS.mkdir(exist_ok=True)
MOUNT = "t => { document.querySelector('#tp-right').innerHTML = '<div id=\"pipe\"><style>' + PIPEQC.css + '</style><div class=\"qc\" id=\"pl-host\">' + PIPEQC.plausibleHtml(t) + '</div></div>'; }"

try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce"); pg = ctx.new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#nav button[data-v=\"tpmine\"]", state="attached", timeout=120000)
        pg.evaluate("() => { const l = document.getElementById('loading'); if (l && /motore|engine/i.test(l.innerText)) l.remove(); }")     # the box needs no engine
        pg.wait_for_function("typeof window.PIPEQC === 'object'", timeout=120000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-right", timeout=60000)
        mount = lambda t: pg.evaluate(MOUNT, t)
        txt = lambda: pg.inner_text("#pl-host")

        def suspect():
            mount(SUSPECT)
            assert pg.inner_text("#pl-host h4") == "Plausibility of the values"
            assert pg.get_attribute("#pl-host section", "aria-labelledby") == "qc-pl-h" and pg.locator("#qc-pl-h").count() == 1
            assert pg.get_attribute("#pl-host [role=status]", "class") == "qc-pl-bad" and pg.inner_text("#pl-host [role=status]") == "5 of 6 checks of the values flag rows.", pg.inner_text("#pl-host [role=status]")
            li = pg.locator("#pl-host li").all_inner_texts(); assert len(li) == 5, li
            assert li[0] == "Warning: 2 rows with a value that is not a number (NaN or infinite) in m/z, RT, area or height. First row ids: 4, 9.", li[0]
            assert li[1] == "Warning: 1 row with m/z outside 20–5000, where no LC-MS run reads. First row ids: 17.", li[1]
            assert li[2].startswith("Warning: 30 rows with RT outside 0–600 min: check the unit of the times. First row ids: 21, 22,") and li[2].endswith(", …."), li[2]      # 20 ids shown of 30
            assert li[3].startswith("Warning: 3 rows with a detected peak") and "above 30 min" in li[3], li[3]
            assert li[4] == "Note: 1 row with no peak in any file. First row ids: 88.", li[4]
            assert "Not checked: ppm error (needs the identity columns and accurate mass)." in txt() and "negative" not in txt().lower().replace("a negative", ""), txt()      # the clean check has no item
        step("suspect values: status line, one Warning/Note item per flagged check with its row ids, the unread check named", suspect)

        def clean():
            mount(CLEAN)
            assert pg.inner_text("#pl-host [role=status]") == "The values look plausible: 7 checks, no row flagged." and pg.get_attribute("#pl-host [role=status]", "class") == "mut"
            assert pg.locator("#pl-host li").count() == 0 and "Not checked" not in txt() and "Warning" not in txt(), txt()
        step("values that are all right: the all-clear line, no list, nothing unread", clean)

        def tolerance():
            ok = dict(CLEAN, checks=[dict(c, missing="identity") if c["id"] == "ppm_error" else c for c in CLEAN["checks"]])
            mount(ok); assert "6 checks, no row flagged" in txt() and "Not checked: ppm error" in txt(), txt()      # unit resolution or no identity: six checks are made, the ppm error is named as not read
        step("the ppm error that cannot be read is not counted among the checks", tolerance)

        def targets():
            mount(SUSPECT)
            bad = pg.evaluate("[...document.querySelectorAll('#pl-host a, #pl-host button, #pl-host input')].filter(e => { const r = e.getBoundingClientRect(); return r.width && (r.width < 24 || r.height < 24); }).map(e => e.outerHTML.slice(0, 80))")
            assert not bad, bad
        step("no control in the box is smaller than 24 px (the box has none)", targets)

        def axe(theme):
            def f():
                pg.evaluate("t => { document.documentElement.dataset.theme = t; }", theme)
                for t in (SUSPECT, CLEAN):
                    mount(t)
                    if not AXE: print("SKIP axe-core not available")
                    if AXE:
                        if not pg.evaluate("typeof axe") == "object": pg.add_script_tag(path=AXE)
                        v = pg.evaluate(RUN % "'#pl-host'"); assert not v, v
                mount(SUSPECT); pg.locator("#pl-host").screenshot(path=str(SHOTS / f"plausible-{theme}-1280.png"))
                pg.set_viewport_size({"width": 390, "height": 800})
                try:
                    w = pg.evaluate("[document.querySelector('#pl-host').scrollWidth, document.querySelector('#pl-host').clientWidth]"); assert w[0] <= w[1] + 2, ("no sideways scroll at 390 px", w)
                    pg.locator("#pl-host").screenshot(path=str(SHOTS / f"plausible-{theme}-390.png"))
                finally: pg.set_viewport_size({"width": 1280, "height": 900})
            return f
        step("axe: no serious violation, both texts, light theme (+ 390 px)", axe("light"))
        step("axe: no serious violation, both texts, dark theme (+ 390 px)", axe("dark"))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else " :: " + s))
if errs: print("PAGE ERRORS", errs[:5])
sys.exit(1 if any(s != "ok" for _, s in steps) or errs else 0)
