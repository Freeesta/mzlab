"""The card «Manual integrations» lists the windows drawn on an XIC too (engine route api/mantraces, manint::table_text), and its numbers are those of the table
the toolbar dialog shows: area, height, apex and edges of the same window. Step 1 of one integration table: the dialog stays until the card has everything.
Needs the site and the .wasm (python tools/prepara_sito.py)."""
# covers: mzlab/web/integrazioni-tracce, mzlab/web/integrazioni-manuali, mzlab/web/rust-bridge
import subprocess, sys, time, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

from lib import free_port; PORT = free_port()
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
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); pg = b.new_context(viewport={"width": 1500, "height": 1600}, locale="en-US").new_page()
        errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
        pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
        pg.wait_for_function("E.files.length >= 2 && E.pending.length === 0", timeout=120000)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.wait_for_timeout(2500)
        pg.evaluate(f"(()=>{{const p={XP};setActive(p);p.imode=null;p.intf='all'}})()")
        x = pg.evaluate(f"(()=>{{const s={XP}._a.sr[0];let k=0;s.y.forEach((v,i)=>{{if(v>s.y[k])k=i}});return s.x[k]}})()")
        pg.evaluate(f"(()=>{{const p={XP};intTargets(p,{x},null).forEach(s=>autoInt(p,s,{x}))}})()")
        pg.wait_for_function("document.querySelector('#manint') && !document.querySelector('#manint').hidden && document.querySelector('#manint-x tbody tr')", timeout=60000)
        pg.wait_for_timeout(500)
        def card():
            n = pg.evaluate(f"{XP}.ints.length"); assert n >= 1
            assert pg.locator("#manint-x tbody tr").count() == n, (n, pg.locator("#manint-x tbody tr").count())
            assert f"({n})" in pg.inner_text("#manint-h")
            heads = pg.locator("#manint-x th").all_inner_texts()
            for h in ["Panel", "Channel", "Sample", "Conc.", "Below the line", "Correction"]: assert h in heads, (h, heads)
        step("the card has a row per window drawn on the XIC, with panel, conc., below and correction columns", card)
        def parity():
            dlg = pg.evaluate("allInts().map(r => ({file: r.file, a: r.a, b: r.b, area: r.area, height: r.height, rt: r.rt}))")
            rows = pg.evaluate("(() => { const [h, ...b] = MANINT.rows(MANTR.csv()); const c = n => h.indexOf(n); return b.map(r => ({sample: r[c('sample')], a: +r[c('start_min')], b: +r[c('end_min')], area: +r[c('area')], height: +r[c('height')], rt: +r[c('apex_rt_min')]})); })()")
            assert len(dlg) == len(rows) and dlg, (dlg, rows)
            for d in dlg:
                r = next(r for r in rows if abs(r["a"] - min(d["a"], d["b"])) < 6e-4 and r["sample"] in d["file"] or r["sample"] == d["file"] and abs(r["a"] - d["a"]) < 6e-4)
                for k in ("area", "height"): assert abs(r[k] - d[k]) <= 1e-4 * max(abs(d[k]), 1), (k, d, r)
                assert abs(r["rt"] - d["rt"]) < 6e-4, (d, r)
        step("area, height and apex of the card are those of the dialog table", parity)
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
        b.close()
finally:
    srv.terminate()
for s in steps: print("  ", s)
