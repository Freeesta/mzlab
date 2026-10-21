"""Flow starts light (docs/agenti/flow-grafica.md, «Avvio»): with the modules of the page and the .wasm held back by the test, the tab of Flow exists and opens on a skeleton with its
controls BEFORE any of those requests finishes; the shell (CORE) comes when its files are let through, the late modules (results, review, runners) only after the first draw, and
once they arrive everything works as before. Order and presence of nodes, never milliseconds. Run from the repo root: python3 tests_e2e/e2e_flow_avvio.py"""
import json, re, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi, axe_path; PORT = free_port()
TAB = '#nav button[data-v="flow"]'
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:1500]))

tmp = Path(tempfile.mkdtemp())
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")], check=True)
site = tmp / "site"
idx = json.loads((site / "static" / "mzfinder" / "indice.json").read_text())
flow_tool = ROOT / "flow" / "js" / "flow-tool.js"; src = flow_tool.read_text()
CORE = re.findall(r'"(flow-[a-z-]+\.js)"', re.search(r"const CORE = \[(.*?)\]", src, re.S).group(1))
LATE = re.findall(r'"(flow-[a-z-]+\.js)"', re.search(r"const LATE = \[(.*?)\]", src, re.S).group(1))
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import socket
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []; held = {"core": [], "late": [], "lists": [], "wasm": []}; gate = {"core": True, "late": True, "lists": True, "wasm": True}; seen = []
def route(r):
    u = r.request.url; name = u.split("?")[0].rsplit("/", 1)[-1]
    kind = "wasm" if u.endswith(".wasm") else "lists" if name.startswith("steps-") else "late" if "/lazy/" in u and name in LATE else "core" if "/lazy/" in u and name in CORE else None
    seen.append((kind, name))
    if kind and gate[kind]: held[kind].append(r)
    else: r.continue_()
def release(kind):
    gate[kind] = False
    for r in held[kind]: r.continue_()
    held[kind].clear()
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        c = b.new_context(viewport={"width": 1400, "height": 1000}); pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
        c.route(re.compile(r"/mzfinder/(lazy/.*\.js|steps-.*\.json)(\?.*)?$|\.wasm(\?.*)?$"), route)
        pg.goto(f"http://127.0.0.1:{PORT}/")
        pg.wait_for_selector(TAB, timeout=120000)
        def early():
            assert idx.get("early") == ["flow-tool.js"], idx.get("early")
            assert "flow-tool.js" not in idx["js"], "the tab script must not be in the ordered list"
        step("the tab script is `early` in the index", early)
        def skeleton():
            pg.click(TAB)
            pg.wait_for_selector("#flx-sk", timeout=30000)
            assert pg.locator("#flx-root").count() == 0
            assert pg.get_attribute("#flx-sk", "aria-busy") == "true"
            labels = pg.locator("#flx-sk button").all_inner_texts()
            for want in ("Templates", "Add analysis", "Simple", "Advanced", "Undo", "Fit", "Checklist", "Open", "Run", "Canvas", "List view"): assert want in labels, (want, labels)
            assert pg.locator("#flx-sk h2").inner_text() == "Flow"
            assert pg.locator("#flx-sk .flx-sk-cv").count() == 1
        step("the skeleton (bar, tabs, canvas) is there while NOTHING of CORE, the lists or the .wasm has arrived", skeleton)
        def nothing_heavy():
            attendi(pg, "!!document.querySelector('#flx-sk')")      # a round trip to the page: the route handler has seen the requests by now
            done = pg.evaluate("[...performance.getEntriesByType('resource')].map(e => e.name).filter(n => /mzfinder\\/(lazy\\/flow-|steps-)/.test(n))")
            assert not done, done
            assert pg.evaluate("typeof FLOW") == "undefined", "no Flow module has run yet"
            assert [k for k, _ in seen if k == "core"], "CORE was asked for (it is held)"
        step("no Flow module or list has finished loading, yet the controls exist", nothing_heavy)
        def shell():
            release("lists"); release("core")
            pg.wait_for_selector("#flx-root", timeout=60000)
            assert pg.locator("#flx-sk").count() == 0
            assert pg.locator("#flx-add").count() == 1 and pg.locator("#flx-run").count() == 1
            assert pg.evaluate("typeof FLOW.resultsCard") == "undefined", "the late modules wait for the first draw and for their files"
        step("CORE and the lists let through: the real shell replaces the skeleton, the late modules are still absent", shell)
        def late_busy():
            pg.click("#flx-run")
            attendi(pg, "document.querySelector('#flx-run').getAttribute('aria-busy') === 'true'")
            assert pg.locator("#flx-add").is_enabled(), "the rest of the bar stays usable"
            pg.click("#flx-add"); pg.wait_for_selector(".flx-pop button", timeout=10000); pg.keyboard.press("Escape")
        step("a click that needs a late module waits on that button only (aria-busy), the rest stays usable", late_busy)
        def late_arrives():
            release("late"); release("wasm")
            attendi(pg, "typeof FLOW.resultsCard === 'function' && typeof FLOW.desktopRunUi === 'function' && typeof FLOW.webRunUi === 'function'", 60000)
            attendi(pg, "document.querySelector('#flx-run').getAttribute('aria-busy') !== 'true' && !!document.querySelector('.fl-cli')", 30000)
        step("once the late modules arrive the waiting Run proceeds (the command box opens)", late_arrives)
        def works():
            pg.click("#flx-add"); pg.click('.flx-pop button:has-text("Confidence level")')
            pg.click('.fln:has-text("Confidence level")')
            attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Confidence level'")
            assert pg.locator('.fl-insp input[type=file][webkitdirectory]').count() == 1
        step("after everything arrived the shell works as before (add an analysis, open its inspector)", works)
        def axe():
            path = axe_path()
            if not path: print("SKIP axe: axe-core not available"); return
            pg.add_script_tag(path=path)
            r = pg.evaluate("axe.run('#v-flow', {runOnly: {type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag22aa']}}).then(x => x.violations.map(v => v.id + ': ' + v.nodes.length))")
            assert not r, r
        step("axe: no violation in the Flow page", axe)
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
