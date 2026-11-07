"""Flow first screen on a tablet (iPad 1024x768 landscape and 768x1024 portrait, touch) and on a desktop window: the template card leaves the view buttons of
the canvas (zoom, fit, 100 %) free to tap, and the form of the selected step is two columns, every label in the first one (a hidden ⓘ note leaves no gap), its «More (N)» summary at least 24 px tall.
Run from the repo root: python3 tests_e2e/e2e_flow_tablet.py"""
import socket, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, wait_for_js; PORT = free_port()
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

# view buttons whose centre is under another element
HIDDEN = """() => [...document.querySelectorAll('.fl-zoom button')].filter(b => { const r = b.getBoundingClientRect(), h = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
  return !h || !b.contains(h); }).map(b => b.getAttribute('aria-label'))"""
# labels of the form that are not in its first column
OFF = """() => [...document.querySelectorAll('.fl-insp .fl-form')].flatMap(f => { const x = f.getBoundingClientRect().left;
  return [...f.children].filter(e => e.tagName === 'LABEL' && e.offsetWidth).map(e => [e.textContent.trim(), Math.round(e.getBoundingClientRect().left - x)]).filter(([, d]) => Math.abs(d) > 1); })"""

tmp = Path(tempfile.mkdtemp()); site = tmp / "site"
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        for name, w, h, touch in (("iPad landscape", 1024, 768, True), ("iPad portrait", 768, 1024, True), ("desktop", 1400, 1000, False)):
            k = dict(viewport={"width": w, "height": h}, reduced_motion="reduce")
            if touch: k.update(has_touch=True, is_mobile=True)
            pg = b.new_context(**k).new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));sessionStorage.setItem('mzlab.coiReload','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
            pg.click('#nav button[data-v="flow"]'); pg.wait_for_selector(".flx-tpl.show", timeout=30000)
            wait_for_js(pg, "document.querySelectorAll('.fl-insp .fl-form label').length >= 3")
            def free(pg=pg):
                hidden = pg.evaluate(HIDDEN)
                assert pg.locator(".fl-zoom button").count() == 4 and not hidden, f"view buttons under the template card: {hidden}"
            step(f"{name}: the template card leaves the view buttons of the canvas free", free)
            def columns(pg=pg):
                off = pg.evaluate(OFF)
                assert not off, f"form labels out of the label column: {off}"
            step(f"{name}: every label of the step form is in the label column", columns)
            def summary(pg=pg):
                hs = pg.evaluate("[...document.querySelectorAll('.fl-insp details>summary')].filter(s => s.offsetWidth).map(s => [s.textContent.trim(), Math.round(s.getBoundingClientRect().height)])")
                assert hs and all(h >= 24 for _, h in hs), f"summaries of the step form under 24 px: {hs}"
            step(f"{name}: the «More» summary of the step form is at least 24 px tall", summary)
            pg.context.close()
        b.close()
finally:
    srv.terminate()
print("STEPS"); [print(" ", s) for s in steps]; print("page errors:", errs)
sys.exit(1 if errs or any(s[1] != "ok" for s in steps) else 0)
