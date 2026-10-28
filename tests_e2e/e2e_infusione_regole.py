"""INFG rules: the arcs of the «Fragmentation graph» carry the id of the rule that explains the loss (weak ones tagged), the panel of an ion lists the rules of the losses
that reach it with their reason and their sources as DOI links, the filter «only losses with a rule» works with the keyboard, the data table has the columns rule, source and
priority, the thresholds say «to be calibrated», axe, Italian and English. The engine answer is the one of tests_e2e/fixtures/msngrafo_esempio.json plus the key `regole` of
fixtures/msngrafo_regole.json (synthetic: no real data, no real compounds)."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
sys.path.insert(0, str(ROOT))
from TP_Mine.tests.msn_synth import write_msn, caffeine_nodes
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_infusione_regole.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
FIX = Path(__file__).parent / "fixtures"
FX = json.loads((FIX / "msngrafo_esempio.json").read_text()); RX = json.loads((FIX / "msngrafo_regole.json").read_text())["regole"]
D = synth("infregole")
F = write_msn(D / "MSn_synth_t01.mzML", caffeine_nodes(), n_scans=12)
STAND_IN = """([fx, rx]) => {
  window.__REQ = []; const orig = window.fetch.bind(window);
  window.fetch = (u, init) => {
    const s = String(u), url = new URL(s, location.href), q = url.searchParams, path = url.pathname.replace(/^.*\\/api\\//, "api/");
    if (path === "api/msn_grafo") {
      window.__REQ.push([path, s]);
      const j = JSON.parse(JSON.stringify(fx)); j.avvisi = [];
      if (q.get("regole") === "1") j.regole = JSON.parse(JSON.stringify(rx));
      return Promise.resolve(new Response(JSON.stringify(j), { headers: { "content-type": "application/json" } }));
    }
    return orig(u, init);
  };
}"""
r = Run(port=8979, wd="/tmp/wd_infregole")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1900})
        pg.goto(f"http://127.0.0.1:{r.port}/?lang=en"); attendi(pg, "window.qqReady === true")
        pg.set_input_files("#pick", [str(F)]); attendi(pg, "document.querySelectorAll('#flist input[data-k=use]').length>=1", 180000)
        pg.click("text=Load data"); attendi(pg, "E.files.length>=1", 90000)
        attendi(pg, "E.panels.some(p=>p.type==='spec' && p._a && p._a.data && p.filt && p.filt.startsWith('msn:'))", 60000)
        pg.context.grant_permissions(["clipboard-read", "clipboard-write"])
        pg.evaluate(STAND_IN, [FX, RX]); drawer(pg, "gra"); pg.wait_for_selector(".ig-node", timeout=30000)
        def labels():
            assert "regole=1" in pg.evaluate("window.__REQ[0][1]"), pg.evaluate("window.__REQ[0][1]")
            t = pg.evaluate("[...document.querySelectorAll('.ig-rule')].map(e=>e.textContent)")
            assert "loss_co2" in t and any(x.startswith("loss_nh3_amide") and "weak" in x for x in t), t
            assert any("loss_co2 + loss_nh3_amide" in x for x in t), t
        step("the arcs carry the id of their rule; a weak rule is tagged; a sum shows both", labels)
        def panel():
            pg.click(".ig-node[data-id=n3]"); pg.wait_for_selector(".ig-rl", timeout=10000)
            txt = pg.inner_text(".ig-rl"); assert "loss_co2" in txt and "CO₂" in txt and "carbon dioxide" in txt.lower(), txt
            a = pg.locator(".ig-rl a.ig-src").first
            assert a.get_attribute("href").startswith("https://doi.org/10.1039/c5np00073d") and "noopener" in a.get_attribute("rel") and a.get_attribute("target") == "_blank"
            assert "identified" not in txt.lower()
        step("the panel lists the rule with its reason and its source as a DOI link", panel)
        def weak():
            pg.click(".ig-node[data-id=n2]"); pg.wait_for_selector(".ig-weak", timeout=10000)
            txt = pg.inner_text(".ig-rl"); assert "Weak rule" in txt and "Stepped spectrum" in txt and "Sum of two rules" in txt, txt
            hrefs = pg.evaluate("[...document.querySelectorAll('.ig-rl a.ig-src')].map(a=>a.getAttribute('href'))")
            assert any("%3C351::AID-JMS512%3E" in h for h in hrefs), hrefs
        step("a weak rule keeps its tag and the reason of the engine; the warnings of the global rules come from it; DOIs are encoded", weak)
        def warn():
            pg.click(".ig-node[data-id=n6]"); pg.wait_for_selector(".ig-rl", timeout=10000)
            assert "trap cut-off" in pg.inner_text(".ig-rl").lower(), pg.inner_text(".ig-rl")
        step("below the cut-off of the trap the arc says so", warn)
        def thr():
            pg.click(".ig-node[data-id=n3]"); pg.wait_for_selector(".ig-thr", state="attached", timeout=10000)
            txt = pg.text_content(".ig-thr"); assert pg.locator(".ig-thr .ig-cal").count() == 2 and "Trap cut-off q" in txt and "Choice of mzLab" in txt, txt
        step("the thresholds say where they come from and «to be calibrated»", thr)
        def flt():
            n0 = pg.locator(".ig-node").count(); a0 = pg.locator(".ig-arc").count()
            box = pg.locator(".ig-flt:has(#ig-rf)").bounding_box(); assert box["height"] >= 24 and box["width"] >= 24, box
            pg.focus("#ig-rf"); pg.keyboard.press("Space"); attendi(pg, "INFG.G.rf === true")
            assert pg.locator(".ig-arc").count() == 3 < a0 and pg.locator(".ig-node").count() < n0, (pg.locator(".ig-arc").count(), pg.locator(".ig-node").count())
            assert pg.evaluate("document.activeElement.id") == "ig-rf"
            assert pg.get_attribute("#ig-rf", "aria-label")
        step("the filter «only losses with a rule» works from the keyboard and keeps the focus", flt)
        def data():
            pg.click("#ig-data"); pg.wait_for_selector("dialog#tdati table", timeout=5000)
            head = pg.inner_text("dialog#tdati thead"); assert "Rule" in head and "Source" in head and "Priority" in head, head
            rows = pg.inner_text("dialog#tdati tbody"); assert "loss_co2" in rows and "DEM16 (doi:10.1039/c5np00073d)" in rows, rows
            assert pg.locator("dialog#tdati tbody tr").count() == 3 + 3 and "n0>n1" not in rows, (pg.locator("dialog#tdati tbody tr").count(), rows[:600])
            pg.keyboard.press("Escape")
            pg.click("#ig-copyd"); attendi(pg, "navigator.clipboard.readText().then(t => t.includes('loss_co2'))")
        step("the data table and «Copy data» have the columns rule, source and priority (filtered like the graph)", data)
        AXE = axe_path()
        def axe():
            pg.click(".ig-node[data-id=n2]"); pg.wait_for_selector(".ig-weak", timeout=5000)
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("axe.run('#hrinfo', {runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r=>r.violations.filter(v=>['serious','critical'].includes(v.impact)).map(v=>[v.id,v.nodes.length]))")
            assert not v, v
        if AXE: step("axe: no serious or critical violation with the rules shown", axe)
        pg3 = r.b.new_page(viewport={"width": 1500, "height": 1600}); pg3.goto(f"http://127.0.0.1:{r.port}/?lang=it"); attendi(pg3, "window.qqReady === true")
        load(pg3, [F], 6000, 1); pg3.evaluate(STAND_IN, [FX, RX]); drawer(pg3, "gra"); pg3.wait_for_selector(".ig-node", timeout=30000)
        def it():
            assert "Solo perdite con una regola" in pg3.inner_text(".ig-flt")
            pg3.click(".ig-node[data-id=n2]"); pg3.wait_for_selector(".ig-weak", timeout=5000)
            txt = pg3.inner_text(".ig-rl"); assert "Regole di frammentazione" in txt and "debole" in pg3.inner_text(".ig-weak") and "Fonti:" in txt, txt
        step("Italian: texts from the catalogue", it)
        errs = [e for e in r.errs if e[0] in ("pageerror",)]
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(not s.startswith("ok") for _, s in steps) else 0)
