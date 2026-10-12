"""English mode, part B (loading): header, start screen, example choice, tables, upload errors and recipes, loading screen, gear and the
Italian words left on screen. Synthetic data, no real lab files."""
import sys, os, re, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from lib import *
import synth
import controlla_i18n as ci
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_inglese_caricamento.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
# words that are Italian AND English-looking are not in the list; the few that the English texts really use are removed here
WORDS = ci.ITALIAN_WORDS - {"come", "dati", "file-non", "ora", "sia", "tra", "fra", "anche", "solo", "poi"}
def italian_on_screen(pg):
    t = pg.evaluate("""() => { const out = []; const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT);
        for (let e = walk.currentNode; e; e = walk.nextNode()) { if (e.closest('script,style,#theory-sorry')) continue; if (!e.checkVisibility()) continue;
          for (const n of e.childNodes) if (n.nodeType === 3) out.push(n.nodeValue);
          for (const a of ['title', 'placeholder', 'aria-label', 'alt']) if (e.getAttribute(a)) out.push(e.getAttribute(a)); }
        return out.join(' | '); }""")
    return sorted({w for w in re.findall(r"[A-Za-zÀ-ÿ]+", t) if w.lower() in WORDS})
r = Run(port=8830 + 110, wd="/tmp/wd_ingl_b")
try:
    with sync_playwright() as p:
        url = f"http://127.0.0.1:{r.port}/"
        b = p.chromium.launch()
        c = b.new_context(locale="en-US", viewport={"width": 1500, "height": 1400}); pg = c.new_page()
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(url); pg.wait_for_selector("#start:not([hidden])", timeout=60000)
        def header():
            h = pg.inner_text("header")
            for w in ("Calculator", "Losses", "Adducts", "Lists", "Table", "More"): assert w in h, (w, h)
            assert pg.get_attribute("#np-ad", "aria-label") == "ESI adducts" and pg.get_attribute("#np-pt", "aria-label") == "Periodic table"
            pg.click("#tb-more"); pg.wait_for_selector("#stmenu"); assert pg.get_attribute("#np-pc", "title").startswith("Opens PubChem"), pg.get_attribute("#np-pc", "title")
            pg.keyboard.press("Escape")
            assert pg.get_attribute("header .hq", "title") == "About mzLab", pg.get_attribute("header .hq", "title")
        step("header in English", header)
        def start():
            t = pg.inner_text("#start")
            for w in ("1. Load the data files (.mzML)", "Drop the .mzML files here", "Try the example files", "2. Acquisition method", "OPTIONAL", "Load data"): assert w in t.replace("optional", "OPTIONAL"), (w, t)
            assert not italian_on_screen(pg), italian_on_screen(pg)
        step("start screen in English, no Italian words", start)
        def demo():
            pg.click("#demobtn"); pg.wait_for_timeout(200)
            t = pg.inner_text("#demochoice"); assert "5 files · 28 MB" in t and "2 files · 10 MB" in t, t
            assert "5 full scan files" in pg.get_attribute("[data-demo=full]", "title")
            pg.click("#demobtn")
        step("example choice", demo)
        def upload():
            shutil.rmtree("/tmp/s_ingl_b", ignore_errors=True)
            files = synth.make_series("/tmp/s_ingl_b", [0, 15, 60]); stage(pg, files)
            t = pg.inner_text("#flist")
            for w in ("Experiment", "Type", "Time", "Size", "sample", "blank"): assert w in t, (w, t)
            assert re.search(r"\d+ files?", t), t
            assert not italian_on_screen(pg), italian_on_screen(pg)
        step("file table in English", upload)
        def errors():
            open("/tmp/x_ingl_b.txt", "w").write("x")
            pg.set_input_files("#pick", "/tmp/x_ingl_b.txt"); pg.wait_for_timeout(800)
            t = pg.inner_text("#unopen"); assert "format not recognised" in t and "x_ingl_b.txt" in t, t
            open("/tmp/y_ingl_b.raw", "w").write("x")
            pg.set_input_files("#pick", "/tmp/y_ingl_b.raw"); pg.wait_for_timeout(500)
            t = pg.inner_text("#unopen"); assert "This .raw file can't be opened in the browser" in t and "Close" in t and "How to convert" in t, t
            open("/tmp/z_ingl_b.mzML", "w").write("not xml")
            pg.set_input_files("#pick", "/tmp/z_ingl_b.mzML"); pg.wait_for_timeout(800)
            assert not italian_on_screen(pg), italian_on_screen(pg)
        step("upload errors and recipes in English", errors)
        def load():
            pg.click("#opbtn"); ready(pg)
            assert pg.evaluate("E.files.length") >= 3
        step("open the files", load)
        def gear():
            pg.click("#np-set"); pg.wait_for_timeout(200)
            t = pg.inner_text("#uipset")
            for w in ("Settings", "Theme", "Like the system", "Chart colors", "Merge the centroids of the same nominal mass"): assert w in t, (w, t)
            assert "By time (default)" in pg.evaluate("[...document.querySelectorAll('#uip-pal option')].map(o => o.textContent)"), "palette names"
        step("gear in English", gear)
        def phrase():
            pg.evaluate("loading(true)"); pg.wait_for_timeout(300)
            assert pg.inner_text("#ldmsg").strip(". \n") != ""
            assert pg.evaluate("[...Array(17)].every((_, i) => !/[à-ù]/.test(I18N.t('load.phrase.' + (i + 1))))")
            pg.evaluate("loading(false)")
        step("loading phrases in English", phrase)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
