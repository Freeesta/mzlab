"""Lab variant of the site (MZLAB_LAB=1): only Data and Draw, expiry page, no banner, noindex; the normal build is left alone.
Builds both sites and serves them. Run from the repo root: python3 tests_e2e/e2e_lab.py  """
import os, socket, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)
from lib import free_port; from lib_sito import add_wasm  # noqa: E402
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL line %d: %s" % (e.__traceback__.tb_next.tb_lineno if e.__traceback__.tb_next else 0, str(e)[:500])))
tmp = Path(tempfile.mkdtemp()); procs = []
def build(name, lab):
    env = {k: v for k, v in os.environ.items() if not k.startswith("MZLAB_LAB")}
    if lab: env.update(MZLAB_LAB="1", MZLAB_LAB_EXPIRES="2026-12-29")
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / name)], check=True, env=env)
    add_wasm(tmp / name)
    port = free_port()
    procs.append(subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(port), str(tmp / name)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
    return f"http://127.0.0.1:{port}/"
LAB = build("lab", True); NORMAL = build("normal", False)
for u in (LAB, NORMAL):          # wait for the servers to listen (state, not a pause)
    port = int(u.rsplit(":", 1)[1].strip("/"))
    while True:
        try: socket.create_connection(("127.0.0.1", port), timeout=1).close(); break
        except OSError: pass
ADMIN = "try{localStorage.setItem('mzlab.admin','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}"      # the administrator mode stays off in the lab (Teoria editor and the like); Tabui and mzFlow no longer depend on it
CLOCK = "(()=>{const R=Date,t=R.parse('%s');class D extends R{constructor(...a){a.length?super(...a):super(t)}static now(){return t}};window.Date=D})()"      # fake clock
NAV = "[...document.querySelectorAll('#nav button')].map(b => b.dataset.v)"
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def page(url, init, hash=""):
            ctx = b.new_context(viewport={"width": 1500, "height": 1000}); pg = ctx.new_page(); pg.add_init_script(init)
            reqs = []; pg.on("request", lambda r: reqs.append(r.url)); pg.goto(url + hash, wait_until="commit"); return pg, reqs
        def hidden():
            pg, reqs = page(LAB, ADMIN + CLOCK % "2026-10-29T12:00:00Z")      # admin flag already set: the lab must not honour it
            pg.wait_for_selector("#drop", timeout=120000)
            assert pg.evaluate("window.QTOOLS ? QTOOLS.list().length : 0") == 0 and pg.evaluate(NAV) == ["data", "draw"], pg.evaluate(NAV)
            assert not [u for u in reqs if "mzfinder/" in u]      # the small loader script is part of the page, inert in the lab (window.MZLAB_LAB); the Tabui and mzFlow package is never fetched, reqs
            assert not pg.evaluate("document.querySelector('#v-tpmine, #v-flow, [data-v=tpmine], [data-v=flow]')")
            assert pg.evaluate("localStorage.getItem('mzlab.admin')") is None
            for _ in range(5): pg.click(".logo")                      # the five clicks of the administrator mode
            pg.wait_for_function("document.querySelector('.logo').style.opacity !== '0'", timeout=15000)
            assert pg.evaluate("window.QTOOLS ? QTOOLS.list().length : 0") == 0 and pg.evaluate(NAV) == ["data", "draw"]
            assert pg.evaluate("document.querySelector('meta[name=robots]').content") == "noindex,nofollow"
            assert pg.evaluate("window.MZLAB_LAB.expires") == "2026-12-29"
        step("lab: Tabui and mzFlow are hidden, even with the administrator flag and the five clicks", hidden)
        def routes():
            for h in ("#/flow", "#/tpmine", "#/attributio", "#/theory", "#/practice"):
                pg, _ = page(LAB, CLOCK % "2026-10-29T12:00:00Z", h)
                pg.wait_for_selector("#drop", timeout=120000)
                assert pg.evaluate("location.hash") == "#/data" and pg.evaluate("location.pathname.endsWith('/')"), (h, pg.url)
            pg, _ = page(LAB, CLOCK % "2026-10-29T12:00:00Z", "#/draw"); pg.wait_for_selector("#v-draw:not([hidden])", timeout=120000)
            assert pg.evaluate("location.hash") == "#/draw"
        step("lab: other addresses go back to #/data, #/draw stays", routes)
        def banner():
            for lang in ("en", "it"):
                pg, _ = page(LAB + "?lang=" + lang, CLOCK % "2026-10-29T12:00:00Z"); pg.wait_for_selector("#drop", timeout=120000)
                assert pg.evaluate("document.querySelector('#lab-banner')") is None and pg.evaluate("document.body.innerText.includes('29 Dec') || document.body.innerText.includes('29 dic')") is False
        step("lab: no banner and no date shown", banner)
        def expiry():
            pg, reqs = page(LAB, CLOCK % "2026-12-30T00:00:00Z")
            pg.wait_for_selector("main[role=alert]", timeout=30000)
            t = pg.inner_text("main"); assert "expired" in t and "scaduta" in t, t
            assert pg.evaluate("document.querySelector('#drop')") is None and pg.evaluate("typeof E") == "undefined"
            assert not [u for u in reqs if "carica.js" in u or "/wasm/" in u], reqs
            pg, _ = page(LAB, CLOCK % "2026-12-29T23:59:59Z"); pg.wait_for_selector("#drop", timeout=120000)      # the last day is still valid
        step("lab: expiry page after the date, no app loaded", expiry)
        def noexamples():
            assert not (tmp / "lab" / "static" / "esempi").exists() and not list((tmp / "lab").rglob("*.mzML")) and not list((tmp / "lab").rglob("esempio-*.png"))
            pg, reqs = page(LAB, CLOCK % "2026-10-29T12:00:00Z"); pg.wait_for_selector("#drop", timeout=120000)
            assert pg.evaluate("(() => { const e = document.querySelector('#demo'); return !e || e.offsetParent === null })()"), "example chooser visible"
            assert not pg.is_visible("#demobtn") and not pg.is_visible("#demochoice")
            pg.wait_for_function("typeof setView === 'function'", timeout=120000); pg.evaluate("setView('draw')"); pg.wait_for_selector("#v-draw:not([hidden])", timeout=120000)      # setView comes from a script loaded after #drop exists
            assert not pg.is_visible("#ex-link") and pg.evaluate("document.body.innerText.includes('example files') || document.body.innerText.includes('file di esempio')") is False
            assert pg.evaluate("TOUR.caricaEsempio.toString().includes('esempi')") is False
            assert not [u for u in reqs if "/esempi/" in u or "esempio-" in u], reqs
            assert pg.evaluate("fetch('static/esempi/FullScan_t15.mzML').then(r => r.status)") == 404
        step("lab: no example files shipped and no example entry points", noexamples)
        def normal():
            pg, reqs = page(NORMAL, ""); pg.wait_for_selector("#drop", timeout=120000)
            assert pg.evaluate("document.querySelector('#lab-banner')") is None and pg.evaluate("typeof MZLAB_LAB") == "undefined"
            assert not [u for u in reqs if u.split("?")[0].endswith("/lab.js")], reqs
            pg.wait_for_selector('#nav button[data-v="flow"]', timeout=60000)      # no flag, no click: Tabui and mzFlow are in the header of the normal build
            pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
            assert pg.evaluate(NAV)[:2] == ["data", "draw"] and pg.evaluate("localStorage.getItem('mzlab.admin')") is None
        step("normal build: no lab, Tabui and mzFlow visible at first load", normal)
        b.close()
finally:
    for pr in procs: pr.terminate()
fails = [s for s in steps if s[1] != "ok"]
for s in steps: print("  ", s)
sys.exit(1 if fails else 0)
