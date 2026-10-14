"""Peak quality (Rust engine, qualita.js) on the built site: the optional columns of the integration table (Gaussian similarity, asymmetry at 10 %,
noise score), the two methods for the limits of the automatic integration (CWT, prominence + local minimum) and the «compare» view.
Needs the site and the .wasm (python tools/build_site.py; the CI builds the .wasm in site/static/wasm)."""
import subprocess, sys, time, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

PORT = 8851
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
site = ROOT / "site"
if not (site / "index.html").exists() or not (site / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "SKIP site/ or site/static/wasm not built")); sys.exit(0)
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
XP = "E.panels.find(p=>p.type==='xic')"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); pg = b.new_context(viewport={"width": 1500, "height": 1600}, locale="it-IT").new_page()
        errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
        pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
        pg.wait_for_function("E.files.length >= 2 && E.pending.length === 0", timeout=120000)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.wait_for_timeout(2500)
        pg.evaluate(f"(()=>{{const p={XP};setActive(p);p.imode=null;p.intf=p._a.sr[0].key}})()")
        def engine(): assert pg.evaluate("window.QUALITA ? QUALITA.available() : false"), "the peak engine does not answer"
        step("the Rust peak engine answers", engine)
        def apex():
            return pg.evaluate(f"(()=>{{const s={XP}._a.sr[0];let k=0;s.y.forEach((v,i)=>{{if(v>s.y[k])k=i}});return s.x[k]}})()")
        def columns():
            x = apex()
            pg.evaluate(f"(()=>{{const p={XP};PK.method='classic';autoInt(p,p._a.sr[0],{x})}})()"); pg.wait_for_timeout(300)
            assert pg.evaluate(f"{XP}.ints.length") == 1
            pg.evaluate("INTOPT.q = true; showInts()"); pg.wait_for_selector("#ig-q", timeout=10000); pg.wait_for_timeout(500)
            heads = pg.locator("#bigbody th[title]").all_inner_texts()
            assert any("gaussiana" in h for h in heads) and any("Asimmetria" in h for h in heads) and any("rumore" in h for h in heads), heads
            cells = pg.evaluate("[...document.querySelectorAll('#bigbody tr:nth-child(2) td.num')].slice(-3).map(c=>c.textContent)")
            g, n = float(cells[0]), float(cells[2])
            assert -1 <= g <= 1 and g > 0.8, cells         # a real chromatographic peak is bell shaped
            assert 0 <= n <= 1, cells
        step("integration table: quality columns with values", columns)
        def excel():
            rows = pg.evaluate("allInts().length"); assert rows == 1
        step("one integration listed", excel)
        def methods():
            pg.evaluate("document.querySelector('#bigdlg').close()")
            x = apex()
            for m in ("cwt", "prom"):
                pg.evaluate(f"(()=>{{const p={XP};p.ints.length=0;PK.method='{m}';autoInt(p,p._a.sr[0],{x})}})()"); pg.wait_for_timeout(1500)
                ed = pg.evaluate(f"{XP}.ints.map(i=>[i.a,i.b])"); assert len(ed) == 1 and ed[0][1] > ed[0][0] and ed[0][0] < x < ed[0][1], (m, ed)
        step("automatic integration with CWT and with prominence + local minimum", methods)
        def compare():
            x = apex()
            pg.evaluate(f"(()=>{{const p={XP};compareMethods(p,{x},0)}})()"); pg.wait_for_selector("#bigbody svg[role=img]", timeout=10000)
            assert pg.locator("#bigbody table tr").count() >= 4
            assert pg.evaluate("document.querySelector('#bigbody svg').getAttribute('aria-label')").startswith("Traccia con i bordi")
        step("compare view: trace, three methods, accessible", compare)
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
        b.close()
finally:
    srv.terminate()
for s in steps: print("  ", s)
