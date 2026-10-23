"""The stand-alone HTML of `mzlab-cli msn-graph figure` (80 ions of tests_e2e/fixtures/msngrafo_grande.json), opened from file://: size, no network request, no console error,
the two views, the keyboard, «Copy data», the SVG and PNG downloads and axe-core clean. No server and no engine are involved."""
import sys, os, subprocess, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *

ROOT = Path(__file__).resolve().parent.parent
CLI = os.environ.get("MZLAB_CLI") or str(ROOT / "target" / "debug" / "mzlab-cli")
AXE = axe_path()
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))

tmp = Path(tempfile.mkdtemp(prefix="msnfig"))
subprocess.run([CLI, "msn-graph", "figure", str(ROOT / "tests_e2e" / "fixtures" / "msngrafo_grande.json"), "--figura", "svg,html", "--out", str(tmp / "g")], check=True, capture_output=True)
html = tmp / "g.html"
step("one file of at most 2 MB", lambda: (html.stat().st_size <= 2_000_000) or (_ for _ in ()).throw(AssertionError(html.stat().st_size)))
try:
    with sync_playwright() as p:
        try: b = p.chromium.launch()
        except Exception: b = p.chromium.launch(executable_path="/opt/pw-browsers/chromium", args=["--no-sandbox"])
        ctx = b.new_context(accept_downloads=True, viewport={"width": 1300, "height": 900}); pg = ctx.new_page()
        reqs, errs = [], []
        pg.on("request", lambda q: reqs.append(q.url) if not q.url.startswith(("file:", "data:", "blob:")) else None)
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" else None)
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(html.as_uri())
        step("ions drawn in the tree view", lambda: pg.locator("#graph svg").count() >= 1 or (_ for _ in ()).throw(AssertionError("no svg")))
        def views():
            t = pg.inner_text("#graph"); pg.click("#v-radial"); assert pg.get_attribute("#v-radial", "aria-pressed") == "true"
            assert pg.locator("#graph svg").count() >= 1; assert pg.inner_text("#graph") != t
            pg.click("#v-tree"); assert pg.get_attribute("#v-tree", "aria-pressed") == "true"
        step("tree and radial views", views)
        def keys():
            pg.focus("#graph"); pg.keyboard.press("Tab"); first = pg.evaluate("document.activeElement && document.activeElement.getAttribute('data-id')")
            assert first, "no ion takes the focus"
            pg.keyboard.press("Enter"); assert pg.inner_text("#side").strip(), "no detail"
        step("keyboard: Tab to an ion, Enter shows the detail", keys)
        def table():
            pg.evaluate("document.getElementById('data').open = true"); assert pg.locator("#table tr").count() > 10
            pg.evaluate("window.__c=null; navigator.clipboard.writeText = t => { window.__c = t; return Promise.resolve(); }")
            pg.click("#copy"); pg.wait_for_function("window.__c"); assert "\t" in pg.evaluate("window.__c")
        step("data table and Copy data", table)
        def svg_dl():
            with pg.expect_download() as d: pg.click("#dl-svg")
            txt = Path(d.value.path()).read_text(); assert txt.lstrip().startswith("<svg") or "<svg" in txt[:200]; assert "<style" not in txt or "url(http" not in txt
        step("SVG download", svg_dl)
        def png_dl():
            with pg.expect_download() as d: pg.click("#dl-png")
            assert Path(d.value.path()).read_bytes()[:4] == b"\x89PNG"
        step("PNG download from the canvas", png_dl)
        def axe():
            if not AXE: return
            for mode in ("#v-tree", "#v-radial"):
                pg.click(mode); pg.add_script_tag(path=AXE) if not pg.evaluate("typeof axe") == "object" else None
                v = pg.evaluate("axe.run(document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length]))")
                assert not v, v
        step("axe-core finds no violation", axe)
        step("no network request, no console error", lambda: (not reqs and not errs) or (_ for _ in ()).throw(AssertionError((reqs[:3], errs[:3]))))
        b.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
