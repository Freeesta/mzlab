"""INFG: the «Fragmentation graph» tab of an MSn infusion. The tab shows only with ONE file loaded that is an infusion; an ion formula (or a name) gives the graph with the
candidates and their ppm, «Use» chooses one, the isolation window and the unexplained peaks are written, a drawing box keeps the user's fragment, the cleaned spectra
export as CSV, MGF and MSP, «Add to library» warns about a duplicate, «Copy data», the keys, axe, Italian and English. The engine routes (/api/msn_grafo, the export,
/api/msnlib/aggiungi, /api/molecules/*) are answered here by a stand-in with the JSON of tests_e2e/fixtures/msngrafo_esempio.json until the Rust ones are in main; the
file is the synthetic one of TP_Mine/tests/msn_synth.py (no real data, no real compounds)."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
sys.path.insert(0, str(ROOT))
from TP_Mine.tests.msn_synth import write_msn, caffeine_nodes
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_infusione_grafo.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
FX = json.loads((Path(__file__).parent / "fixtures" / "msngrafo_esempio.json").read_text())
D = synth("infgrafo")
F = write_msn(D / "MSn_synth_t01.mzML", caffeine_nodes(), n_scans=12)
F2 = write_msn(D / "MSn_synth_t02.mzML", caffeine_nodes(), n_scans=12)
STAND_IN = """(fx) => {
  window.__REQ = []; const orig = window.fetch.bind(window);
  const jr = o => new Response(JSON.stringify(o), { headers: { "content-type": "application/json" } });
  window.fetch = (u, init) => {
    const s = String(u), url = new URL(s, location.href), q = url.searchParams, path = url.pathname.replace(/^.*\\/api\\//, "api/");
    if (path.startsWith("api/msn_grafo") || path.startsWith("api/msnlib") || path.startsWith("api/molecules")) window.__REQ.push([path, s, init && init.body ? String(init.body) : ""]);
    if (path === "api/msn_grafo/export") return Promise.resolve(new Response(`export ${q.get("formato")} clean=${q.get("pulito")} node=${q.get("nodo")}\\nmz,rel,formula,ppm\\n244.06035,100,C13H10NO4,-0.35\\n`));
    if (path === "api/msn_grafo") {
      const j = JSON.parse(JSON.stringify(fx));
      if (!q.get("formula")) { j.ione = null; j.nodi.forEach(n => { n.formula = null; }); }
      (JSON.parse(q.get("scelte") || "[]")).forEach(c => { const n = j.nodi.find(x => Math.abs(x.mz - c.mz) < 1e-3); if (n && n.formula) { n.formula.scelta = c.formula; n.formula.fonte = "utente"; } });
      j.avvisi = [{ chiave: "warn.msngrafo.isolation", albero: "x", nodo: "n1" }];
      return Promise.resolve(jr(j));
    }
    if (path === "api/molecules/cerca") return Promise.resolve(jr([{ id: "ik:TESTONLY", nome: "Exampline", formula: "C14H15N3O5", massa: 305.1, smiles: "CCOC(=O)c1ccccc1N", addotti: ["[M+H]+"], punteggio: 1 }]));
    if (path === "api/molecules/get") return Promise.resolve(jr({ id: "ik:TESTONLY", nome: "Exampline", formula: "C14H15N3O5", massa: 305.1, smiles: "CCOC(=O)c1ccccc1N", addotti: ["[M+H]+"] }));
    if (path === "api/msnlib/aggiungi") { const b = JSON.parse(init.body), tag = "entry:" + b.molecola_id + "\\n"; const dup = (b.libreria || "").includes(tag); return Promise.resolve(jr({ libreria: dup ? b.libreria : (b.libreria || "") + tag, doppione: dup })); }
    return orig(u, init);
  };
}"""
r = Run(port=8978, wd="/tmp/wd_infgrafo")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1900})
        load(pg, [F], 6000, 1)
        attendi(pg, "E.panels.some(p=>p.type==='spec' && p._a && p._a.data && p.filt && p.filt.startsWith('msn:'))", 60000)
        tree = pg.evaluate("fetch('api/msn_albero?k=0').then(r=>r.json())")
        ids = [n["id"] for n in tree["nodi"]]
        fx = json.loads(json.dumps(FX))
        for n, tid in zip([n for n in fx["nodi"] if n["isolato"]], ids): n["albero"] = [tid]
        pg.context.grant_permissions(["clipboard-read", "clipboard-write"])
        pg.evaluate(STAND_IN, fx)
        def appears():
            assert pg.evaluate("INF.single()") is True
            drawer(pg, "gra"); pg.wait_for_selector(".ig-node", timeout=30000)
            assert pg.locator("#ig-ion").count() == 1
        step("one infusion file: the tab shows and the graph comes from the file", appears)
        def masses():
            assert pg.locator(".ig-node").count() == len(fx["nodi"]), pg.locator(".ig-node").count()
            assert pg.evaluate("window.__REQ.filter(r=>r[0]==='api/msn_grafo')[0][1]").find("&formula=") < 0
            assert "Ion formula" in pg.inner_text("#ig-field") or pg.evaluate("I18N.lang") != "en"
        step("without an ion formula: masses only", masses)
        def formula():
            pg.fill("#ig-ion", "C14H15N3O5+"); pg.click("#ig-go"); pg.wait_for_function("document.querySelector('.ig-cand')", timeout=15000)
            last = pg.evaluate("window.__REQ.filter(r=>r[0]==='api/msn_grafo').slice(-1)[0][1]")
            assert "formula=C14H15N3O5%2B" in last and "ppm_formula=10" in last and "mz_basso=150" in last and "ppm_basso=20" in last and "prove_min=1" in last, last
            txt = pg.inner_text("#ig-wrap")
            assert "ppm" in txt and "C12H15N3O" in txt, txt[:300]
            assert "ppm" in pg.inner_text("#ig-sum") or "ppm" in pg.inner_text("#ig-msg")
            assert pg.locator("#ig-msg .ig-warn").count() == 1
        step("a formula gives the graph with the candidates and their ppm; one line per warning", formula)
        def use():
            pg.click(".ig-node[data-id=n2]"); pg.wait_for_selector(".ig-use", timeout=10000)
            assert pg.locator(".ig-cands tbody tr").count() == 2
            assert "identified" not in pg.inner_text("#ig-node").lower()
            pg.locator(".ig-use").first.click(); pg.wait_for_function("window.__REQ.some(r=>r[1].includes('scelte='))", timeout=10000)
            pg.wait_for_selector(".ig-cands tr.on", timeout=10000)
            assert pg.locator(".ig-cands tr.on").count() == 1
        step("«Use» asks again with the choice; several candidates are never proposed alone", use)
        def notes():
            pg.click(".ig-node[data-id=n0]"); pg.wait_for_selector(".ig-unexp", timeout=10000)
            assert "2" in pg.inner_text(".ig-unexp") and "261.2000" in pg.inner_text(".ig-unexp")
            pg.click(".ig-node[data-id=n2]"); pg.wait_for_selector(".ig-cands", timeout=10000)
            assert "Isotopes are not expected" in pg.inner_text("#ig-node") or pg.evaluate("I18N.lang") != "en"
        step("isolation window and unexplained peaks are written", notes)
        def draw():
            pg.click(".ig-node[data-id=n2]"); pg.fill("#ig-smi", "CCOC(=O)c1ccccc1N"); pg.dispatch_event("#ig-smi", "change"); pg.wait_for_timeout(400)
            pg.click(".ig-node[data-id=n0]"); pg.wait_for_selector("[data-a=draw]", timeout=10000)
            assert pg.locator("[data-a=draw]").count() == 1 and pg.locator("[data-a=save]").count() == 1
            assert pg.locator(".ig-draw .muted").count() >= 1
            pg.click("[data-a=draw]"); pg.wait_for_function("S.view==='draw'", timeout=15000)
            pg.wait_for_function("window.TPDraw && TPDraw.ready()", timeout=90000)
            pg.evaluate("setView('data')"); pg.wait_for_selector("[data-a=save]", timeout=15000)
            drawer(pg, "gra"); pg.click(".ig-node[data-id=n0]"); pg.click("[data-a=save]")
            pg.wait_for_selector(".ig-pic svg", timeout=20000)
        step("the drawing box opens Draw from the SMILES and keeps the user's fragment", draw)
        def exports():
            pg.click(".ig-node[data-id=n2]")
            got = []
            for fmt in ("csv", "mgf", "msp"):
                pg.select_option("#ig-fmt", fmt); pg.select_option("#ig-scope", "tutto")
                with pg.expect_download(timeout=15000) as d: pg.click("#ig-exp [data-a=dl]")
                body = open(d.value.path(), encoding="utf-8").read(); got.append((fmt, d.value.suggested_filename, body))
            for fmt, name, body in got:
                assert name.endswith("." + fmt) and f"export {fmt} clean=1 node=tutto" in body, (fmt, name, body[:80])
            pg.select_option("#ig-scope", index=1); pg.select_option("#ig-clean", "0"); pg.select_option("#ig-fmt", "mgf")
            pg.click("#ig-exp [data-a=copy]"); pg.wait_for_timeout(300)
            clip = pg.evaluate("navigator.clipboard.readText()")
            assert "export mgf clean=0 node=n2" in clip, clip[:80]
        step("clean spectra: CSV, MGF, MSP for the tree, copy for a node (all, discarded marked)", exports)
        def library():
            pg.click("#ig-lib"); pg.wait_for_timeout(500)
            n1 = pg.evaluate("window.__REQ.filter(r=>r[0]==='api/msnlib/aggiungi').length"); assert n1 == 1
            pg.click("#ig-lib"); pg.wait_for_timeout(500)
            body = json.loads(pg.evaluate("window.__REQ.filter(r=>r[0]==='api/msnlib/aggiungi').slice(-1)[0][2]"))
            assert body["libreria"] and body["formula"].startswith("C14H15N3O5"), body
            assert pg.evaluate("document.body.textContent").count("already in the library") > 0 or pg.evaluate("I18N.lang") != "en"
        step("Add to library: the second time says the entry is already there", library)
        def data():
            pg.click("#ig-copyd"); pg.wait_for_timeout(300)
            clip = pg.evaluate("navigator.clipboard.readText()"); assert "n2" in clip and "C13H10NO4" in clip, clip[:200]
            pg.click("#ig-data"); pg.wait_for_selector("dialog#tdati table", timeout=5000)
            assert pg.locator("dialog#tdati tbody tr").count() == len(fx["nodi"]) + len(fx["archi"])
            pg.keyboard.press("Escape")
        step("Copy data and the table of the chart data", data)
        def keys():
            pg.click(".ig-node[data-id=n0]"); pg.focus(".ig-node[data-id=n0]")
            pg.keyboard.press("ArrowRight"); pg.wait_for_function("INFG.G.sel!=='n0' && INFG.G.sel!==null", timeout=5000)
            sel = pg.evaluate("INFG.G.sel"); assert sel in ("n1", "n2", "n3"), sel
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("ArrowLeft")
            assert pg.evaluate("INFG.G.sel") == "n0", pg.evaluate("INFG.G.sel")
            pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(200)
            cur = pg.evaluate("INFG.G.sel"); node = [n for n in fx["nodi"] if n["id"] == cur][0]
            pg.keyboard.press("Enter")
            if node["albero"]: attendi(pg, f"E.panels.some(p=>p.type==='spec' && p.filt==='msn:{node['albero'][0]}')", 15000)
            pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
            assert pg.evaluate("INFG.G.sel") is None
        step("arrows move between ions, Enter puts the spectrum in the active cell, Esc clears", keys)
        def zoom():
            w0 = pg.evaluate("+document.querySelector('.ig-svg').getAttribute('width')")
            pg.click("[data-z=in]"); w1 = pg.evaluate("+document.querySelector('.ig-svg').getAttribute('width')"); assert w1 > w0
            pg.click("[data-z=fit]")
            sizes = pg.evaluate("[...document.querySelectorAll('.ig-bar .bt')].map(b=>{const r=b.getBoundingClientRect();return Math.min(r.width,r.height)})")
            assert min(sizes) >= 24, sizes
        step("zoom and move with buttons; targets of at least 24 px", zoom)
        AXE = axe_path()
        def axe():
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("axe.run('#hrinfo', {runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r=>r.violations.filter(v=>['serious','critical'].includes(v.impact)).map(v=>[v.id,v.nodes.length]))")
            assert not v, v
        if AXE: step("axe: the tab has no serious or critical violation", axe)
        def pol():
            assert pg.evaluate("[...document.querySelectorAll('#ig-add option')].map(o=>o.value).join()") == "[M+H]+,[M+Na]+,[M+NH4]+,[M+K]+"
        step("positive file: the positive adducts", pol)
        def names():
            pg.fill("#ig-ion", "Exam"); pg.wait_for_selector(".ig-sugb", timeout=5000)
            pg.click(".ig-sugb"); pg.wait_for_function("INFG.G.mol && INFG.G.mol.id==='ik:TESTONLY'", timeout=5000)
            pg.wait_for_timeout(500)
            last = pg.evaluate("window.__REQ.filter(r=>r[0]==='api/msn_grafo').slice(-1)[0][1]")
            assert "formula=C14H15N3O5+%5BM%2BH%5D%2B" in last.replace("%20", "+") and "smiles=" in last, last
        step("a name fills the ion formula with the adduct and the SMILES; the id is kept", names)
        errs = [e for e in r.errs if e[0] in ("pageerror",)]
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
        # two files loaded: no tab
        pg2 = r.b.new_page(viewport={"width": 1500, "height": 1600}); pg2.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg2, "window.qqReady === true")
        load(pg2, [F, F2], 6000, 2)
        step("two files loaded: the tab is not there", lambda: (_ for _ in ()).throw(AssertionError("single")) if pg2.evaluate("INF.single()") else None)
        # Italian
        pg3 = r.b.new_page(viewport={"width": 1500, "height": 1600}); pg3.goto(f"http://127.0.0.1:{r.port}/?lang=it"); attendi(pg3, "window.qqReady === true")
        load(pg3, [F], 6000, 1); pg3.evaluate(STAND_IN, fx); drawer(pg3, "gra"); pg3.wait_for_selector(".ig-node", timeout=30000)
        step("Italian: texts from the catalogue", lambda: None if "Formula dello ione" in pg3.inner_text("#ig-field") else (_ for _ in ()).throw(AssertionError(pg3.inner_text("#ig-field"))))
        step("no request left unanswered by the engine", lambda: (_ for _ in ()).throw(AssertionError(pg.evaluate("window.MZLAB_LR_MISS"))) if pg.evaluate("(window.MZLAB_LR_MISS||[]).length") else None)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(not s.startswith("ok") for _, s in steps) else 0)
