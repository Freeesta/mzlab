"""mzDraw (the hidden page static/draw2/, the mzdraw .wasm): the page loads its module through the manifest, a benzene template lands with one click,
undo and redo walk back and forth, atoms and bonds are drawn with the pointer, a five-bonded carbon raises the live valence mistake of `check/1`,
charge, erase, select all + Delete and the keyboard shortcuts work, SMILES and molfile are added, molfile / SVG / document are saved by the Rust module,
and the same loop works on a tablet viewport with touch. No page error at any point.
Run from the repo root after tools/build_site.py and tools/build_feat_wasm.sh site/static; SKIP without site/static/wasm-feat or without the mzdraw module."""
# covers: mzlab/web/draw2/draw2.js, mzlab/web/draw2/index.html, mzlab/web/draw2/draw2.css, mzlab/web/lang/en/draw2.js, crates/mzdraw-wasm/src/lib.rs
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
MAN = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "site", "static", "wasm-feat", "manifest.json")
if not os.path.exists(MAN) or "mzdraw" not in json.load(open(MAN, encoding="utf-8")):
    print("SKIP site/static/wasm-feat has no mzdraw module (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8997, wd="/tmp/wd_draw2")
STATE = "JSON.parse(document.documentElement.dataset.d2 || '{}')"


def open_page(pg, w, h):
    pg.set_viewport_size({"width": w, "height": h})
    pg.goto(f"http://127.0.0.1:{r.port}/static/draw2/")
    wait_for_js(pg, "document.documentElement.dataset.d2Ready === '1'")
    r.errs.clear()                                          # what the start page left behind while it was being replaced is not this page's
    b = pg.locator("#d2-canvas").bounding_box()
    return b["x"] + b["width"] / 2, b["y"] + b["height"] / 2


def state(pg): return pg.evaluate(STATE)
def tool(pg, name): pg.click(f'[data-testid="d2-{name}"]')
def want(pg, **kv):
    wait_for_js(pg, f"(() => {{ const s = {STATE}, w = {json.dumps(kv)}; return Object.keys(w).every(k => s[k] === w[k]); }})()", timeout=10000)


def loop(pg, cx, cy, tap):
    """the drawing loop every pointer kind must manage: template, undo/redo, atom, sprouted bonds, the valence mistake, erase"""
    tool(pg, "ring:6a"); tap(cx, cy); want(pg, atoms=6, bonds=6, warnings=0)
    tool(pg, "undo"); want(pg, atoms=0, bonds=0)
    assert pg.is_disabled('[data-testid="d2-undo"]') and pg.is_enabled('[data-testid="d2-redo"]')
    tool(pg, "redo"); want(pg, atoms=6, bonds=6)
    tool(pg, "atom:C"); tap(cx - 220, cy - 150); want(pg, atoms=7, bonds=6)
    tool(pg, "bond:1")
    for n in range(1, 6):
        tap(cx - 220, cy - 150); want(pg, atoms=7 + n, bonds=6 + n)
    wait_for_js(pg, "!!document.querySelector('#d2-warnings li[data-code=valence]')", timeout=10000)       # five bonds on a carbon: the mistake shows without pressing anything
    assert pg.evaluate("I18N.t('draw.check.level.warn')") in pg.inner_text("#d2-warnings li[data-code=valence]")       # the level in words, not only a colour
    tool(pg, "erase"); tap(cx - 220, cy - 150); want(pg, atoms=11, bonds=6)                          # the atom goes with its five bonds
    wait_for_js(pg, "!document.querySelector('#d2-warnings li[data-code=valence]')", timeout=10000)


try:
    with sync_playwright() as p:
        pg = r.page(p)
        cx, cy = open_page(pg, 1280, 800)

        def start():
            assert state(pg) == {"atoms": 0, "bonds": 0, "tool": "bond:1", "warnings": 0, "selected": 0}, state(pg)
            assert pg.evaluate("window.MZDRAW.version()") == "mzdraw/1"
            assert pg.inner_text("#d2-summary") == pg.evaluate("I18N.t('draw.check.empty')")
            assert pg.locator("#d2-tools button").count() >= 26
            bad = pg.evaluate("[...document.querySelectorAll('#d2-tools button')].filter(b => !b.title || b.title.length > 90 || !b.getAttribute('aria-label')).map(b => b.dataset.cmd)")
            assert bad == [], bad                           # every tool says in one short sentence what it does
            small = pg.evaluate("[...document.querySelectorAll('button')].filter(b => b.offsetParent && (b.offsetWidth < 24 || b.offsetHeight < 24)).map(b => b.id || b.dataset.cmd)")
            assert small == [], small
        step("the page loads the module and starts empty on the single-bond tool", start)

        step("mouse: template, undo and redo, atom, bonds, live valence mistake, erase", lambda: loop(pg, cx, cy, pg.mouse.click))

        def drag_and_charge():
            tool(pg, "clear"); want(pg, atoms=0, bonds=0)
            tool(pg, "atom:N"); pg.mouse.click(cx, cy); want(pg, atoms=1)
            tool(pg, "bond:2")
            pg.mouse.move(cx, cy); pg.mouse.down(); pg.mouse.move(cx + 40, cy + 5, steps=4); pg.mouse.move(cx + 90, cy, steps=4); pg.mouse.up()
            want(pg, atoms=2, bonds=1)
            tool(pg, "charge:+"); pg.mouse.click(cx, cy)
            wait_for_js(pg, "window.MZDRAW.save('molfile').includes('M  CHG')")
            tool(pg, "select"); pg.mouse.click(cx, cy); want(pg, selected=1)
        step("a bond is dragged out of an atom; a charge is set with a click; an atom is selected", drag_and_charge)

        def keyboard():
            pg.focus("#d2-canvas")
            pg.keyboard.press("Control+a"); want(pg, selected=3)
            pg.keyboard.press("Delete"); want(pg, atoms=0, bonds=0)
            pg.keyboard.press("Control+z"); want(pg, atoms=2, bonds=1)
            pg.keyboard.press("Control+y"); want(pg, atoms=0)
            pg.keyboard.press("Control+z"); want(pg, atoms=2)
            pg.keyboard.press("Control+Shift+Z"); want(pg, atoms=0)
            pg.keyboard.press("e"); want(pg, tool="erase")
            pg.keyboard.press("2"); want(pg, tool="bond:2")
            pg.keyboard.press("o"); want(pg, tool="atom:O")
            pg.keyboard.press("Escape"); want(pg, tool="select")
            assert pg.get_attribute('[data-testid="d2-select"]', "aria-pressed") == "true"
        step("keyboard: select all, Delete, undo and redo, tool shortcuts", keyboard)

        def texts():
            tool(pg, "clear")
            pg.fill("#d2-text", "CC(=O)O"); pg.click("#d2-add"); want(pg, atoms=4, bonds=3, warnings=0)
            mol = pg.evaluate("window.MZDRAW.save('molfile')")
            tool(pg, "clear"); want(pg, atoms=0)
            pg.fill("#d2-text", mol); pg.click("#d2-add"); want(pg, atoms=4, bonds=3)
            assert pg.inner_text("#d2-note") == ""            # a molfile brings its own coordinates
            pg.fill("#d2-text", "C1CC"); pg.click("#d2-add")
            wait_for_js(pg, "document.querySelector('#d2-note').textContent.length > 0")
            assert "error." not in pg.inner_text("#d2-note") and state(pg)["atoms"] == 4       # a refused text says why in words and changes nothing
            tool(pg, "undo"); want(pg, atoms=0)                # one import is one undo step
            tool(pg, "redo"); want(pg, atoms=4)
        step("SMILES and molfile are added, a broken text is refused in words, an import is one undo step", texts)

        def saves():
            got = {}
            for kind, name in (("molfile", "drawing.mol"), ("svg", "drawing.svg"), ("mzdraw", "drawing.mzdraw.json")):
                with pg.expect_download() as d: pg.click(f"#d2-ex-{kind}")
                assert d.value.suggested_filename == name, d.value.suggested_filename
                got[kind] = open(d.value.path(), encoding="utf-8").read()
                assert got[kind] == pg.evaluate(f"window.MZDRAW.save('{kind}')")
            assert "V2000" in got["molfile"] and got["molfile"].rstrip().endswith("M  END")
            assert got["svg"].startswith("<svg") and "<line" in got["svg"] and ">O<" in got["svg"].replace("OH", "O")
            doc = json.loads(got["mzdraw"]); assert doc["format"] == "mzdraw/1" and len(doc["atoms"]) == 4 and len(doc["bonds"]) == 3
        step("molfile, SVG and the mzdraw/1 document are saved as the module writes them", saves)

        def view():
            before = pg.evaluate("window.MZDRAW.save('mzdraw')")
            for t in ("zoom-in", "zoom-in", "zoom-out", "fit"): tool(pg, t)
            pg.mouse.move(cx, cy); pg.mouse.wheel(0, -300); frames(pg)
            assert pg.evaluate("window.MZDRAW.save('mzdraw')") == before        # the view never changes the document
            ink = pg.evaluate("(() => { const c = document.querySelector('#d2-canvas'), d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data; let n = 0; for (let i = 3; i < d.length; i += 4) if (d[i]) n++; return n; })()")
            assert ink > 200, ink                              # something is painted
        step("zoom, fit and the wheel change the view, not the document; the canvas has ink", view)
        r.b.close()

        pg = r.page(p, touch=True)
        tx, ty = open_page(pg, 768, 1024)

        def tablet():
            assert pg.evaluate("document.documentElement.scrollWidth <= innerWidth"), "the page scrolls sideways at 768 px"
            loop(pg, tx, ty, pg.touchscreen.tap)
        step("tablet 768x1024 with touch: the same loop by tapping, no sideways scroll", tablet)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
bad = [e for e in r.errs if e[0] in ("pageerror", "console.error") or e[0].startswith("http")]
sys.exit(1 if not steps or bad or any(s[1] != "ok" for s in steps) else 0)
