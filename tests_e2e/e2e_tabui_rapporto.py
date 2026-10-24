"""Tabui printable report with the transformation products (TP_Mine/js/38_feature_export.js `reportHtml`, TP_Mine/js/39_tp_report.js): the section «Transformation products» (products, formed and not
explained), one SVG time course per product with its data table right below, the provenance, one file with no script or external URL, English, no bare «formula», and axe-core without serious
violations. The session is a fake with a `tptab/1` of the shape the engine writes (the numbers of the report are checked against it, 3 rows). MZLAB_PIPE_SITE reuses a built site.
Run from the repo root: python3 tests_e2e/e2e_tabui_rapporto.py"""
import json, os, re, shutil, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)
from lib import axe_path  # noqa: E402

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:900]))

tmp = Path(tempfile.mkdtemp(prefix="tabui_rapporto_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse: subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True, capture_output=True)
else:
    for d in ("files", "js"):
        for f in (ROOT / "TP_Mine" / d).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(50):
    try: __import__("urllib.request").request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)
AXE = axe_path()
TIMES = [0, 5, 15, 30, 60]
TPTAB = {"schema": "tptab/1", "parent": 1, "times": TIMES, "controls": {"t0": 1, "blank": 1}, "rows": [
    {"row": 0, "id": 1, "mz": 273.1234, "rt": 5.12, "status": 4, "kinship": 0, "via": 0, "notes": 0, "n_det": 5, "max_area": 1.5e6, "max_height": 1e5, "tmax": 0, "in_t0": True, "in_blank": False, "series": [1.5e6, 1.2e6, 8e5, 4e5, 1e5]},
    {"row": 1, "id": 2, "mz": 289.1183, "rt": 4.4, "status": 0, "kinship": 3, "via": 1, "notes": 32, "n_det": 4, "max_area": 4.2e5, "max_height": 5e4, "tmax": 30, "in_t0": False, "in_blank": False, "series": [0, 1e5, 3e5, 4.2e5, 2e5]},
    {"row": 2, "id": 3, "mz": 305.113, "rt": 3.9, "status": 0, "kinship": 2, "via": 9, "notes": 0, "n_det": 3, "max_area": 2.1e5, "max_height": 3e4, "tmax": 60, "in_t0": False, "in_blank": False, "series": [0, None, 5e4, 1e5, 2.1e5]},
    {"row": 3, "id": 4, "mz": 311.2, "rt": 6.0, "status": 3, "kinship": 0, "via": 0, "notes": 2, "n_det": 3, "max_area": 9e4, "max_height": 1e4, "tmax": 15, "in_t0": False, "in_blank": False, "series": [0, 0, 9e4, 5e4, 1e4]},
    {"row": 4, "id": 5, "mz": 321.3, "rt": 7.0, "status": 0, "kinship": 4, "via": 2, "notes": 0, "n_det": 5, "max_area": 7e5, "max_height": 8e4, "tmax": 15, "in_t0": False, "in_blank": False, "series": [0, 3e5, 7e5, 5e5, 2e5]}]}
BUILD = """([tp, times]) => {
  const rows = [["id", "mz", "rt"], [1], [2], [3], [4]];                      // the view keeps the ids 1-4: the row 5 is out
  const session = { rows: async () => rows, tpTable: async () => tp, report: async () => ({}),
    interp: async id => ({ change: { neutral_formula: "C10H12N2O" + id, ion_formula: "C10H13N2O" + id } }) };
  return window.TPREPORT.collect({ session, spec: () => "", names: ["a", "b", "c", "d", "e"], tpInfo: id => ({ level: id === 2 ? "3" : null, pred: id === 2 ? 2 : null }) }).then(d => {
    const r = { crate: "9.9.9", tabella_sha256: "a".repeat(64), campioni_sha256: "b".repeat(64), righe: 5, file: 5, celle: { rilevate: 1, mirate: 0, riempite: 0, assenti: 0 }, vista: null,
      qc: { file: [], gruppi: [], standard_cv: null } };
    return window.PIPEX.reportHtml(r, ["a", "b", "c", "d", "e"], { tp: d, provenance: { profile: "small", recipe_sha256: "c".repeat(64), rules: "regole_tp/1 v1", inputs: [{ name: "a.mzML", sha256: "d".repeat(64) }] } });
  });
}"""
html = ""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce"); pg = ctx.new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#nav button[data-v=\"tpmine\"]", state="attached", timeout=120000)
        pg.evaluate("() => { const l = document.getElementById('loading'); if (l && /motore|engine/i.test(l.innerText)) l.remove(); }")
        pg.click('#nav button[data-v="tpmine"]')
        pg.wait_for_function("typeof window.PIPEX === 'object' && typeof window.TPREPORT === 'object'", timeout=120000)
        html = pg.evaluate(BUILD, [TPTAB, TIMES])

        def section():
            assert "<h2>Transformation products</h2>" in html and "<h3>Formed, not explained</h3>" in html and "<h3>Products</h3>" in html
            assert re.search(r"<td>289\.1183</td>", html) and "<td>Prev. 2</td>" in html and "<td>L3</td>" in html, html[html.find("Products"):][:600]
        step("the section Transformation products has the products and the formed-not-explained list", section)

        def numbers():
            for r in TPTAB["rows"][1:4]:
                if r["id"] == 4: assert f"<td>{r['mz']}</td>" in html and f"<td>{r['n_det']}</td>" in html and "<td>9.00e+4</td>" in html
                else: assert f"<td>{r['mz']}</td>" in html and f"<td>{r['rt']}</td>" in html, r
            assert "<td>C10H12N2O2</td>" in html and "<td>C10H13N2O2</td>" in html
            assert "<td>321.3</td>" not in html                                   # the row outside the view is not in the report
        step("the numbers of 3 rows are those of tptab/1 (and a row outside the view is absent)", numbers)

        def charts():
            svgs = len(re.findall(r"<svg\b", html)); assert svgs == 2, svgs
            assert len(re.findall(r"</svg><table>", html)) == svgs, "every svg is followed by its table"
            assert "1.00e+5" in html and "Time (min)" in html
        step("each chart is an SVG followed by its data table", charts)

        def prov():
            for t in ("mzlab-feat", "9.9.9", "regole_tp/1 v1", "Recipe SHA-256", "c" * 64, "a" * 64, "b" * 64, "d" * 64, "Profile"): assert t in html, t
        step("provenance: versions, recipe, table, samples and inputs", prov)

        def one_file():
            assert "<script" not in html.lower() and not re.search(r"(?:src|href)=|https?://|url\(", html), re.findall(r"(?:src|href)=|https?://|url\(", html)
            assert '<html lang="en">' in html and re.findall(r"<h([1-3])>", html)[:2] == ["1", "2"]
            assert not re.search(r"(?<![A-Za-z] )(?<!Neutral )(?<!Ion )(?<!\[M\+H\]\+ )\bformula\b", re.sub(r"<[^>]*>", " ", html).replace("Neutral formula", "").replace("Ion formula", "")), "bare formula"
        step("one file: no script, no external URL, English, no bare formula", one_file)

        def axe():
            if not AXE: return
            page2 = ctx.new_page(); page2.set_content(html); page2.add_script_tag(path=AXE)
            v = page2.evaluate("axe.run(document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length]))")
            assert not v, v
        step("axe-core without serious violations on the report", axe)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else " :: " + s))
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
