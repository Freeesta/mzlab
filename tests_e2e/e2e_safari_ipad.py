"""Safari / iPad (docs/agenti/compat-safari.md): the site on an emulated iPad (touch only, no hover) is the full site, not the phone card; the file
picker of «Load» opens with every file (compat.js frees the accept list that iPadOS would grey out) and an example file opens with its charts; the page
keeps its width at Split View 1/3 and 1/2 and after a rotation; with AbortSignal.any and createWritable taken away (what Safari before 17.4 and 26
lacks) compat.js stands in and an OPFS file written through createWritable reads back the same. No browser errors.
The real engine of Safari is WebKit: with --browser webkit (the nightly round on macOS) every step runs in WebKit; the cloud has only Chromium, which
emulates the iPad (touch, coarse pointer, mobile viewport) and the output says so."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_safari_ipad.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8853, wd="/tmp/wd_safari_ipad")
SKIP_TOUR = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');sessionStorage.setItem('mzlab.coiReload','1');}catch(_){}"
# what Safari before 17.4 / 26 lacks, taken away before any script of the page runs
TAKE_AWAY = "try{delete AbortSignal.any;delete FileSystemFileHandle.prototype.createWritable;}catch(_){}"
WIDE = "document.documentElement.scrollWidth - document.documentElement.clientWidth"

def ipad(b, w=1180, h=820, extra=""):
    k = dict(viewport={"width": w, "height": h}, has_touch=True, device_scale_factor=2, reduced_motion="reduce")
    if BROWSER != "firefox": k["is_mobile"] = True              # Firefox has no mobile emulation
    c = b.new_context(**k); c.add_init_script(SKIP_TOUR)
    if extra: c.add_init_script(extra)
    pg = c.new_page()
    pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
    pg.on("console", lambda m: r.errs.append(("console." + m.type, m.text)) if m.type == "error" else None)
    pg.goto(f"http://127.0.0.1:{r.port}/"); ready(pg); return pg

if BROWSER == "firefox":          # Playwright's Firefox cannot be a touch-only device (no isMobile, the pointer stays fine): nothing here would be an iPad
    print("  skipped in Firefox: it cannot emulate a touch-only device; the iPad steps run in WebKit and Chromium"); r.srv.terminate(); sys.exit(0)
try:
    from playwright.sync_api import sync_playwright
    print(f"  browser: {BROWSER}" + ("" if BROWSER == "webkit" else " (iPad emulated; the WebKit round is the nightly --browser webkit)"))
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        S = {}
        def full_site():
            pg = S["pg"] = ipad(b)
            assert pg.evaluate("matchMedia('(hover: none) and (pointer: coarse)').matches"), "the emulated iPad has a hover or a fine pointer"
            assert pg.evaluate("window.MZLAB_PHONE") is False and pg.locator("#drop").is_visible(), "an iPad got the phone card"
            assert "accept" in pg.evaluate("window.__compat || []"), pg.evaluate("window.__compat")
        step("iPad: the full site, compat.js installed", full_site)
        def picker():
            pg = S["pg"]
            with pg.expect_file_chooser() as fc: pg.evaluate("document.getElementById('pick').click()")
            assert pg.get_attribute("#pick", "accept") is None, "the accept list is still on the input"
            assert ".wiff" in (pg.get_attribute("#pick", "data-accept") or ""), "the list is not kept in data-accept"
            fc.value.set_files([str(EX / "FullScan_t0.mzML")])
            attendi(pg, "document.querySelectorAll('#flist input[data-k=use]').length>=1", 60000)
            pg.click("#opbtn"); ready(pg)
            attendi(pg, "E.panels.length > 0 && E.panels.some(q => q.el.querySelector('canvas') && q.el.querySelector('canvas').width > 0)", 60000)
        step("iPad: «Load» shows every file and an example file opens with its charts", picker)
        def widths():
            pg = S["pg"]; out = []
            for w, h in ((1180, 820), (820, 1180), (375, 820), (590, 820), (1180, 820)):      # landscape, portrait, Split View 1/3 and 1/2, back
                pg.set_viewport_size({"width": w, "height": h}); attendi(pg, f"innerWidth === {w}"); frames(pg, 3); ready(pg)
                out.append((w, pg.evaluate(WIDE), pg.evaluate("E.panels[0].el.getBoundingClientRect().width")))
            print("  width, horizontal overflow, first panel width:", out)
            assert all(o <= 1 for _, o, _ in out), f"the page scrolls sideways: {out}"
            assert all(0 < pw <= w for w, _, pw in out), f"a panel is wider than the window: {out}"
            assert pg.evaluate("window.MZLAB_PHONE") is False
        step("iPad: rotation and Split View 1/3, 1/2 keep the page in the window", widths)
        def fallbacks():
            pg = ipad(b, extra=TAKE_AWAY)
            got = pg.evaluate("window.__compat || []")
            assert "AbortSignal.any" in got and "createWritable" in got, got
            back = pg.evaluate("""async () => {
              const a = new AbortController(), s = AbortSignal.any([a.signal, new AbortController().signal]); a.abort('r');
              const d = await (await navigator.storage.getDirectory()).getDirectoryHandle('compat-prova', { create: true });
              const fh = await d.getFileHandle('t.txt', { create: true }), w = await fh.createWritable();
              await w.write('m/z '); await w.write(new Blob(['301.1'])); await w.close();
              const text = await (await fh.getFile()).text();
              await (await navigator.storage.getDirectory()).removeEntry('compat-prova', { recursive: true });
              return [s.aborted && s.reason === 'r', text]; }""")
            assert back == [True, "m/z 301.1"], back
        step("without AbortSignal.any and createWritable: compat.js stands in (OPFS write reads back)", fallbacks)
        b.close()
finally:
    r.srv.terminate()
for s in steps: print("  ", s)
bad = [s for s in steps if s[1] != "ok"]
if r.errs: print("browser errors:", r.errs[:5])
sys.exit(1 if bad or r.errs else 0)
