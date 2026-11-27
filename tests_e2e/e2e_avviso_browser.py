"""Closable browser notice (avviso-browser.js, started by capacita.js): a browser that lacks a feature gets a notice, never a block. Chromium simulates the missing
feature with ?cap-off=; a normal Chromium (it has showDirectoryPicker and the engine capacities) shows nothing. The folder notice suggests only Chrome and Edge (Firefox
has no showDirectoryPicker either); the engine notice suggests Chrome, Firefox and Edge. Closing is remembered (localStorage, also when it throws); Escape and the
close button work from the keyboard; 390 px wide there is no sideways scroll; the site keeps working behind it; no notice in the desktop app or on a touch device (folder case).
MZLAB_SHOTS_DIR: where the screenshots (device scale factor 2) are written."""
# covers: mzlab/web/avviso-browser, mzlab/web/capacita
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
SHOTS = os.environ.get("MZLAB_SHOTS_DIR", "")
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_avviso_browser.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8852, wd="/tmp/wd_avviso_browser")
N = "#browser-notice"
LINKS = f"[...document.querySelectorAll('{N} a')].map(a => a.href)"
CHROME, FIREFOX, EDGE = "https://www.google.com/chrome/", "https://www.mozilla.org/firefox/new/", "https://www.microsoft.com/edge"

def load(b, q="", w=1440, h=950, init=None, **ctx):
    c = b.new_context(viewport={"width": w, "height": h}, **ctx)
    if init: c.add_init_script(init)
    pg = c.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
    pg.goto(f"http://127.0.0.1:{r.port}/{q}"); ready(pg); return pg

def settled(pg):
    """The page knows its capacities (the engine worker told opfsSync) and has drawn: the point where a notice, if due, would be there."""
    wait_for_js(pg, "window.CAPACITA && CAPACITA.statoCap('opfsSync') !== 'verifica'"); frames(pg, 3)

def shot(pg, name):
    if SHOTS: os.makedirs(SHOTS, exist_ok=True); pg.screenshot(path=os.path.join(SHOTS, name + ".png"))

try:
    from playwright.sync_api import sync_playwright
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        def chrome_none():
            pg = load(b); settled(pg)
            assert pg.locator(N).count() == 0, "notice in a normal Chromium"
            assert not pg.evaluate("document.getElementById('bn-css')"), "module loaded without a notice due"
        step("Chromium with every capacity: no notice", chrome_none)
        def folder():
            pg = load(b, "?cap-off=cartellaLocale"); wait_for_js(pg, f"document.querySelector('{N}')")
            assert pg.get_attribute(N, "data-kind") == "folder"
            assert pg.evaluate(LINKS) == [CHROME, EDGE], "Firefox must not be suggested for the folder: " + str(pg.evaluate(LINKS))
            assert all(a == "_blank" and "noopener" in (rel or "") for a, rel in pg.evaluate(f"[...document.querySelectorAll('{N} a')].map(a => [a.target, a.rel])")), "links must open in a new tab with rel=noopener"
            assert "Firefox" not in pg.inner_text(N).replace("Everything else", ""), pg.inner_text(N)
            shot(pg, "folder-desktop")
            # the site works behind it
            pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML")]); wait_for_js(pg, "document.querySelectorAll('#flist input[data-k=use]').length>=1")
            pg.click("#opbtn"); ready(pg); assert pg.evaluate("E.panels.length") > 0, "Data opened no panel behind the notice"
            assert pg.locator(N).count() == 1, "the notice must stay until closed"
        step("folder missing: notice with Chrome and Edge only, links in a new tab, the site works behind it", folder)
        def close_click():
            pg = load(b, "?cap-off=cartellaLocale"); wait_for_js(pg, f"document.querySelector('{N}')")
            pg.click(f"{N} .bn-x"); assert pg.locator(N).count() == 0
            assert pg.evaluate("localStorage.getItem('mzlab.browserNotice')") == "1"
            pg.reload(); ready(pg); settled(pg); assert pg.locator(N).count() == 0, "shown again after closing"
        step("closing is remembered across loads", close_click)
        def keyboard():
            pg = load(b, "?cap-off=cartellaLocale"); wait_for_js(pg, f"document.querySelector('{N}')")
            pg.keyboard.press("Tab"); assert pg.evaluate(f"document.querySelector('{N}').contains(document.activeElement)"), "Tab does not reach the notice first"
            assert pg.evaluate("document.activeElement.tagName") == "A"
            pg.keyboard.press("Tab"); pg.keyboard.press("Tab"); assert pg.evaluate("document.activeElement.className") == "bn-x", "close button not reachable"
            assert pg.get_attribute(f"{N} .bn-x", "aria-label"), "close button has no name"
            pg.keyboard.press("Escape"); assert pg.locator(N).count() == 0, "Escape does not close"
            assert pg.evaluate("localStorage.getItem('mzlab.browserNotice')") == "1"
        step("keyboard: Tab reaches the notice first, Escape closes", keyboard)
        def no_storage():
            js = "Storage.prototype.getItem = function(k){ if (k === 'mzlab.browserNotice') throw new Error('blocked'); return null; }; Storage.prototype.setItem = function(k, v){ if (k === 'mzlab.browserNotice') throw new Error('blocked'); };"
            pg = load(b, "?cap-off=cartellaLocale", init=js); wait_for_js(pg, f"document.querySelector('{N}')")
            pg.click(f"{N} .bn-x"); assert pg.locator(N).count() == 0
            assert not [e for e in r.errs if "blocked" in str(e)], r.errs
        step("localStorage throwing: still shown and closable, no error", no_storage)
        def phone():
            pg = load(b, "?cap-off=simd128", w=390, h=780, device_scale_factor=2); wait_for_js(pg, f"document.querySelector('{N}')")
            assert pg.get_attribute(N, "data-kind") == "engine"
            assert pg.evaluate(LINKS) == [CHROME, FIREFOX, EDGE]
            bx = pg.evaluate(f"(() => {{ const r = document.querySelector('{N}').getBoundingClientRect(); return [r.left, r.right, innerWidth, document.documentElement.scrollWidth]; }})()")
            assert bx[0] >= 0 and bx[1] <= bx[2] and bx[3] <= bx[2], f"notice or page wider than 390 px: {bx}"
            assert pg.evaluate(f"[...document.querySelectorAll('{N} a, {N} button')].every(e => {{ const q = e.getBoundingClientRect(); return q.width >= 24 && q.height >= 24; }})"), "target under 24 px"
            top = pg.evaluate(f"(() => {{ const n = document.querySelector('{N}').getBoundingClientRect(), l = document.getElementById('cap-live'); return [n.top, l ? l.getBoundingClientRect().bottom : null]; }})()")
            assert top[1] is None or top[1] <= top[0], f"the capacity line is hidden under the notice: {top}"
            shot(pg, "engine-390")
            pg.click(f"{N} .bn-x"); assert pg.locator(N).count() == 0
        step("engine missing at 390 px: notice with Chrome, Firefox, Edge, fits, targets >= 24 px", phone)
        def phone_folder():
            pg = load(b, "?cap-off=cartellaLocale", w=390, h=780, device_scale_factor=2); wait_for_js(pg, f"document.querySelector('{N}')")
            bx = pg.evaluate(f"(() => {{ const r = document.querySelector('{N}').getBoundingClientRect(); return [r.left, r.right, innerWidth, document.documentElement.scrollWidth]; }})()")
            assert bx[0] >= 0 and bx[1] <= bx[2] and bx[3] <= bx[2], f"notice or page wider than 390 px: {bx}"
            shot(pg, "folder-390")
        step("folder missing at 390 px (mouse): fits, no sideways scroll", phone_folder)
        def touch_device():
            pg = load(b, "?cap-off=cartellaLocale", w=1024, h=768, has_touch=True); settled(pg)
            assert pg.locator(N).count() == 0, "folder notice on a touch device, where another browser does not help"
            pg = load(b, "?cap-off=simd128", w=1024, h=768, has_touch=True); wait_for_js(pg, f"document.querySelector('{N}')")
            assert pg.evaluate(LINKS) == [], "download links on a touch device"
        step("touch tablet: no folder notice; engine notice without download links", touch_device)
        def desktop():
            pg = load(b, "?cap-off=simd128", init="window.MZLAB_DESKTOP = true;"); settled(pg)
            assert pg.locator(N).count() == 0, "notice in the desktop app"
        step("desktop app: no notice", desktop)
        def late_engine():
            pg = load(b, "?cap-off=cartellaLocale"); wait_for_js(pg, f"document.querySelector('{N}')")
            pg.evaluate("window.MZLAB_CAP_OFF = ['simd128']; CAPACITA.nota({opfsSync: true})"); wait_for_js(pg, f"document.querySelector('{N}').dataset.kind === 'engine'")
            assert pg.locator(N).count() == 1
        step("engine problem found later replaces the folder notice", late_engine)
        b.close()
finally:
    r.srv.terminate()
for s in steps: print("  ", s)
bad = [s for s in steps if s[1] != "ok"]
if r.errs: print("browser errors:", r.errs[:3])
sys.exit(1 if bad else 0)
