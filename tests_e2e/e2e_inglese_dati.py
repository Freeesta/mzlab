"""English mode, part C (Data, low resolution): sidebar, toolbar, panels, menus, XIC window, calculator, tables, settings, method, export names.
At every step all the visible text (text, title, placeholder, aria-label) is read and the test fails on Italian words. Synthetic data."""
import sys, os, re, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from lib import *
import synth
import controlla_i18n as ci
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_inglese_dati.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
WORDS = ci.ITALIAN_WORDS - {"come", "dati", "file-non", "ora", "sia", "tra", "fra", "anche", "solo", "poi", "area", "tempo", "durante", "tipo", "mass", "base", "menu", "la", "In", "lingua", "dell"}
JS = """() => { const out = []; const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT);
    for (let e = walk.currentNode; e; e = walk.nextNode()) { if (e.closest('script,style,#theory-sorry,#phone')) continue; if (!e.checkVisibility()) continue;
      for (const n of e.childNodes) if (n.nodeType === 3) out.push(n.nodeValue);
      for (const a of ['title', 'placeholder', 'aria-label', 'alt']) if (e.getAttribute(a)) out.push(e.getAttribute(a)); }
    return out.join(' | '); }"""
def italian(pg):
    t = pg.evaluate(JS)
    return sorted({w for w in re.findall(r"(?<![\w.\-#&])[A-Za-zÀ-ÿ]+(?!\w|-|\.\w)", t) if w.lower() in WORDS})
r = Run(port=8830 + 120, wd="/tmp/wd_ingl_c")
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        c = b.new_context(locale="en-US", viewport={"width": 1600, "height": 1500}); pg = c.new_page()
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.on("console", lambda m: r.errs.append(("console", m.text)) if "missing key" in m.text else None)
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#start:not([hidden])", timeout=60000)
        shutil.rmtree("/tmp/s_ingl_c", ignore_errors=True)
        stage(pg, synth.make_series("/tmp/s_ingl_c", [0, 15, 60])); pg.click("#opbtn"); ready(pg)
        def first_view():
            it = italian(pg); assert not it, it
            for w in ("Full Scan", "Selected only", "All overlaid", "+ Chromatogram", "+ Spectrum", "+ RT-m/z map", "Method"): assert w in pg.inner_text("#v-data"), w
        step("first view of Data: no Italian", first_view)
        def sidebar():
            for t in ("calc", "losses", "adducts", "lists"):
                pg.click(f"#stbar [data-cmd={t}]"); pg.wait_for_timeout(500)
                it = italian(pg); assert not it, (t, it)
            pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(300); it = italian(pg); assert not it, ("file", it)
            pg.click("#sb-x"); pg.wait_for_timeout(300)
        step("sidebar tabs: calculator, losses, adducts, lists", sidebar)
        def periodic():
            pg.click("#np-pt"); pg.wait_for_timeout(600)
            assert "Hydrogen" in pg.inner_text("#refdlg") or "Carbon" in pg.inner_text("#refdlg"), pg.inner_text("#refdlg")[:200]
            it = italian(pg); assert not it, it
            pg.click("#refx")
        step("periodic table dialog", periodic)
        def xic():
            pg.click("#np-xic"); pg.wait_for_timeout(500); assert pg.is_visible("#xicdlg")
            it = italian(pg); assert not it, it
            assert "Extract ions (XIC)" in pg.inner_text("#xicdlg")
            pg.click("#xic-no")
        step("XIC window", xic)
        def menus():
            pg.click("#np-spec"); pg.wait_for_timeout(500)
            pg.click("#np-map"); pg.wait_for_timeout(800)
            it = italian(pg); assert not it, it
            for i in range(pg.locator(".pnl canvas").count()):
                box = pg.locator(".pnl canvas").nth(i).bounding_box()
                if not box: continue
                for fx in (0.5, 0.3):
                    pg.mouse.click(box["x"] + box["width"] * fx, box["y"] + box["height"] / 2, button="right"); pg.wait_for_timeout(300)
                    it = italian(pg); assert not it, (i, fx, it)
                    pg.keyboard.press("Escape"); pg.mouse.click(5, 5)
        step("panels and right-click menu", menus)
        def gear_method():
            pg.click("#np-set"); pg.wait_for_timeout(200); it = italian(pg); assert not it, it; pg.click("#np-set")
            pg.click("#np-method"); pg.wait_for_timeout(800); it = italian(pg); assert not it, it
        step("gear and method window", gear_method)
        def xic_integrate():
            pg.keyboard.press("Escape"); pg.mouse.click(5, 5)
            pg.click("#np-xic"); pg.wait_for_timeout(300)
            pg.fill("#xic-rows input >> nth=0", "250"); pg.click("#xic-go"); ready(pg); pg.wait_for_timeout(800)
            it = italian(pg); assert not it, it
            n = pg.locator(".pnl").count(); last = pg.locator(".pnl").nth(n - 1)
            box = last.locator("canvas").first.bounding_box()
            pg.mouse.click(box["x"] + box["width"] * 0.5, box["y"] + box["height"] * 0.5, button="right"); pg.wait_for_timeout(300)
            it = italian(pg); assert not it, it
            items = pg.evaluate("[...document.querySelectorAll('#ctx button, #ctx div, #ctx a')].map(e => e.textContent.trim()).filter(Boolean)")
            assert any("Integrate" in x for x in items), items
            pg.locator("#ctx").get_by_text("Integrate the peak").first.click(); pg.wait_for_timeout(800)
            it = italian(pg); assert not it, it
        step("XIC extraction and integration menu", xic_integrate)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
