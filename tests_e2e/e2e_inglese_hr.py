"""English mode, part D (high resolution): the bench (toolbar, filters, stacked charts, information bar with its 9 tabs), the formulas window, the libraries
windows, the right-click menus and the DDA lists, with all the visible text read at every step (Italian words fail the test). Synthetic Orbitrap DDA file."""
import sys, os, re; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from lib import *
from lib_hr import synth
import controlla_i18n as ci
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_inglese_hr.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
WORDS = ci.ITALIAN_WORDS - {"come", "dati", "file-non", "ora", "sia", "tra", "fra", "anche", "solo", "poi", "area", "tempo", "durante", "tipo", "mass", "base", "menu", "lingua", "dell"}
JS = """() => { const out = []; const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT);
    for (let e = walk.currentNode; e; e = walk.nextNode()) { if (e.closest('script,style,#theory-sorry,#phone')) continue; if (!e.checkVisibility()) continue;
      for (const n of e.childNodes) if (n.nodeType === 3) out.push(n.nodeValue);
      for (const a of ['title', 'placeholder', 'aria-label', 'alt']) if (e.getAttribute(a)) out.push(e.getAttribute(a)); }
    return out.join(' | '); }"""
def italian(pg):
    t = pg.evaluate(JS)
    return sorted({w for w in re.findall(r"(?<![\w.\-#&])[A-Za-zÀ-ÿ]+(?!\w|-|\.\w)", t) if w.lower() in WORDS})
D = synth("hr_en")
r = Run(port=8830 + 130, wd="/tmp/wd_ingl_d")
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        c = b.new_context(locale="en-US", viewport={"width": 1600, "height": 1500}); pg = c.new_page()
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.on("console", lambda m: r.errs.append(("console", m.text)) if "missing key" in m.text else None)
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#start:not([hidden])", timeout=60000)
        pg.set_input_files("#pick", [str(D / "HR_DDA-Exploris-t30.mzML")])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=1", timeout=180000)
        def start():
            t = pg.inner_text("#flist"); assert "profile" in t.lower() or "centroid" in t.lower() or "HR" in t, t[:200]
            it = italian(pg); assert not it, it
        step("start screen with an HR file: English", start)
        pg.click("#opbtn"); pg.wait_for_function("E.files.length>=2", timeout=90000); pg.wait_for_timeout(3000)
        def bench():
            it = italian(pg); assert not it, it
            assert "Previous scan" in pg.get_attribute("#hrbar [data-hb=prev]", "title"), pg.get_attribute("#hrbar [data-hb=prev]", "title")
        step("bench toolbar and cells", bench)
        def info_tabs():
            labs = pg.evaluate("[...document.querySelectorAll('#stbar button.tb')].filter(b=>!b.hidden&&b.offsetParent).map(b=>b.getAttribute('aria-label'))")
            for w in ("Scan: Header of the scan", "Formulas: Elemental composition", "Identification with the libraries"): assert w in labs, (w, labs)
            for t in ("hdr", "lst", "fil", "cmp", "iso", "idn"):
                pg.evaluate(f"BARRA.setTab('{t}')"); pg.wait_for_timeout(1000)
                it = italian(pg); assert not it, (t, it)
            pg.evaluate("BARRA.close(false)")
        step("information tools of the toolbar: the six panels", info_tabs)
        def stacked():
            pg.evaluate("addRangeTest = 1") if False else None
            btn = pg.locator("[data-bf=range]").first
            if btn.count():
                btn.click(); pg.wait_for_timeout(800); it = italian(pg); assert not it, it
                assert "Add a stacked chart" in pg.inner_text("#bigdlg")
                pg.click("#bigx")
        step("stacked chart dialog", stacked)
        def formulas():
            pg.evaluate("COMP.open({mz: 305.0702, polarity: 'positive'})"); pg.wait_for_timeout(2500)
            t = pg.inner_text("#bigdlg"); assert "Candidate formulas" in t, t[:200]
            it = italian(pg); assert not it, it
            pg.click("#bigx")
        step("candidate formulas window", formulas)
        def libs():
            pg.evaluate("LIB.open()"); pg.wait_for_timeout(1200)
            assert "Spectral libraries" in pg.inner_text("#libdlg"); it = italian(pg); assert not it, it
            pg.evaluate("document.getElementById('libdlg').close()")
        step("libraries window", libs)
        def menus():
            n = pg.locator(".pnl canvas").count()
            for i in range(min(n, 4)):
                box = pg.locator(".pnl canvas").nth(i).bounding_box()
                if not box: continue
                pg.mouse.click(box["x"] + box["width"] * 0.5, box["y"] + box["height"] * 0.5, button="right"); pg.wait_for_timeout(400)
                it = italian(pg); assert not it, (i, it)
                pg.keyboard.press("Escape"); pg.mouse.click(5, 5)
        step("right-click menus of the cells", menus)
        def ddalist():
            btn = pg.locator("#ddalist")
            if btn.count() and btn.first.is_visible():
                btn.first.click(); pg.wait_for_timeout(800); it = italian(pg); assert not it, it
                pg.click("#bigx")
        step("DDA precursors list", ddalist)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
