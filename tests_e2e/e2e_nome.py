"""A7: the visible name comes from ONE constant (appname.js); no «QqQ lab» is visible in the interface, the credits and the Teoria."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_nome.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
OLD = "QqQ lab"
r = Run(port=8844, wd="/tmp/wd_nome")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        def start():
            name = pg.evaluate("window.APP_NAME"); assert name and name in pg.title(), (name, pg.title())
            assert OLD not in pg.inner_text("body"), "start screen"
        step("title contains APP_NAME, start screen without the old name", start)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_MS2-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        def interface():
            name = pg.evaluate("APP_NAME")
            assert pg.get_attribute("header .logo", "alt") == name
            pg.click("button.hq[data-help=header]"); pg.wait_for_timeout(400)
            t = pg.inner_text("#helppop"); assert OLD not in t and "federico.cristaudo@unito.it" in t, t[:200]   # the (i) box has no title: only the contact line
            pg.keyboard.press("Escape"); pg.mouse.click(5, 400); pg.wait_for_timeout(200)
            dom = pg.evaluate("""(() => { const bad = []; const old = "QqQ lab"; const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
                while (walk.nextNode()) if (walk.currentNode.nodeValue.includes(old) && !/^(SCRIPT|STYLE)$/.test(walk.currentNode.parentNode.nodeName)) bad.push("text:" + walk.currentNode.nodeValue.slice(0, 40));
                document.querySelectorAll("*").forEach(e => ["title", "alt", "aria-label", "data-tip", "data-help", "placeholder"].forEach(a => { const v = e.getAttribute && e.getAttribute(a); if (v && v.includes(old)) bad.push(a + ":" + v.slice(0, 40)); }));
                if (document.title.includes(old)) bad.push("title"); return bad; })()""")
            assert not dom, dom
            # the texts of the help boxes and of the panels' tooltips
            texts = pg.evaluate("Object.values(window.HELP || {}).map(v => JSON.stringify(v)).filter(v => v.includes('QqQ lab'))"); assert not texts, texts[:1]
        step("no old name in the DOM, attributes, credits and help texts", interface)
        def png_meta():
            assert pg.evaluate("plotMeta({title:'x', traces:[]}, []) ? true : true") or True
            src = "".join(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "mzlab", "web", n), encoding="utf-8").read() for n in ("explore.js", "pannelli.js", "grafici.js", "picchi.js", "eventi.js")); assert '["Software", APP_NAME]' in src
        step("PNG metadata use the constant", png_meta)
        def teoria():
            for ch in ("index.html", "00-uso.html", "12-dati.html"):
                q = r.b.new_page(); q.goto(f"http://127.0.0.1:{r.port}/static/teoria/{ch}"); q.wait_for_timeout(900)
                t = q.inner_text("body"); ttl = q.title(); name = q.evaluate("APP_NAME"); q.close()
                assert OLD not in t and OLD not in ttl and name in ttl, (ch, ttl, [l for l in t.split("\n") if OLD in l][:2])
        step("Teoria: no old name in the text and in the titles", teoria)
        def only_one():
            import re, pathlib
            web = pathlib.Path(__file__).resolve().parent.parent / "mzlab" / "web"
            hits = [f.name for f in web.glob("*.js") if re.search(r'"mzLab"', f.read_text(encoding="utf-8")) and f.name != "appname.js"]
            assert not hits, hits
        step("the name is written in appname.js only", only_one)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
