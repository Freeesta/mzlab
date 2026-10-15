"""INF_B: a direct-infusion MSn file is ONE item «MSn infusion» with the tree of its precursors: the nodes are the filters of the cells, the spectrum of a node is its
mean (or sum), «Fragment this ion» goes to the child node. Synthetic file of TP_Mine/tests/msn_synth.py (no real data); a DDA file stays «Full Scan + DDA»."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
sys.path.insert(0, str(ROOT))
from TP_Mine.tests.msn_synth import write_msn, caffeine_nodes
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_infusione.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("infmsn")
F = write_msn(D / "MSn_synth_t01.mzML", caffeine_nodes(), n_scans=12)
NODES = len(caffeine_nodes()) - 1                  # every path but the survey is a node
r = Run(port=8977, wd="/tmp/wd_infmsn")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1600})
        load(pg, [F], 6000, 1)
        attendi(pg, "E.panels.some(p=>p.type==='spec' && p._a && p._a.data && p.filt && p.filt.startsWith('msn:'))", 60000)
        def one():
            assert pg.evaluate("E.files.filter(f=>!f.gone).length") == 1, "one item, not MS1 + MS2"
            assert pg.evaluate("E.files[0].kind") == "infusion", pg.evaluate("E.files[0].kind")
            assert pg.evaluate("E.tab") == "infusion"
            name = pg.evaluate("I18N.t('inf.name')"); assert name.lower() in pg.evaluate("document.body.textContent").lower(), name
            assert pg.evaluate("ESP.etichetta('infusion')") == name
        step("the file is one item of kind «MSn infusion»", one)
        def tree():
            j = pg.evaluate("fetch('api/msn_albero?k=0').then(r=>r.json())")
            assert j["formato"] == "msn_albero/1" and len(j["nodi"]) == NODES, (j["formato"], len(j["nodi"]))
            assert sorted(set(n["livello"] for n in j["nodi"])) == [2, 3, 4, 5], j["livelli"]
        step("the tree has the nodes of the synthetic file", tree)
        def start():
            st = pg.evaluate("(()=>{const c=E.panels.find(p=>p.type==='chrom'),s=E.panels.find(p=>p.type==='spec');return {c:c.filt,s:s.filt,n:s._a&&s._a.data&&s._a.data[0]?s._a.data[0].d.y.length:0,tic:c._a&&c._a.sr?c._a.sr.reduce((a,x)=>a+x.x.length,0):0}})()")
            assert st["c"] and st["c"] == st["s"] and st["n"] > 0 and st["tic"] > 0, st
        step("the starting cells show a node: its TIC and its mean spectrum", start)
        def tab():
            drawer(pg, "alb"); pg.wait_for_selector("#inf-tree tbody tr", timeout=30000)
            assert pg.locator("#inf-tree tbody tr").count() == NODES + 1
            assert pg.locator("#inf-tree tbody tr[aria-selected=true]").count() == 1
        step("the tree tab lists the survey and the nodes", tab)
        def click():
            ids = pg.evaluate("[...document.querySelectorAll('#inf-tree tbody tr[data-id]')].map(t=>t.dataset.id).filter(Boolean)")
            now = pg.evaluate("E.panels.find(p=>p.type==='spec').filt")
            other = [i for i in ids if "msn:" + i != now][-1]
            pg.click(f"#inf-tree tbody tr[data-id='{other}']"); attendi(pg, f"E.panels.some(p=>p.type==='spec' && p.filt==='msn:{other}' && p._a && p._a.data && p._a.data[0] && p._a.data[0].d.y.length>0)", 30000)
            st = pg.evaluate("(()=>{const c=E.panels.find(p=>p.type==='chrom'),s=E.panels.find(p=>p.type==='spec');return {c:c.filt,s:s.filt,n:s._a&&s._a.data&&s._a.data[0]?s._a.data[0].d.y.length:0}})()")
            assert st["c"] == st["s"] == "msn:" + other and st["n"] > 0, st
        step("a click on a node puts it on the chromatogram and the spectrum", click)
        def keys():
            pg.focus("#inf-tree tbody tr[tabindex='0']"); pg.keyboard.press("ArrowDown")
            assert pg.evaluate("document.activeElement.closest('#inf-tree') !== null")
        step("the arrows walk the tree", keys)
        def frag():
            r0 = pg.evaluate("""(async()=>{const till=async f=>{for(let i=0;i<600&&!f();i++) await new Promise(r=>requestAnimationFrame(r))};
              const spec=()=>E.panels.find(p=>p.type==='spec'),shown=(id)=>()=>{const s=spec();return s.filt==='msn:'+id&&s._a&&s._a.data&&s._a.data[0]&&s._a.data[0].d.y.length>0};
              const j=await fetch('api/msn_albero?k=0').then(r=>r.json());const root=j.nodi.find(n=>n.livello===2);
              await INF.pick(E.panels.find(p=>p.type==='chrom'), root.id); await till(shown(root.id));
              const kid=j.nodi.find(n=>n.genitore===root.id);
              const it=INF.menuPeak(spec(), kid.prec).find(x=>x.label===I18N.t('inf.frag')); await it.fn(); await till(shown(kid.id));
              const after=spec().filt; const back=INF.menuPeak(spec(), 1).find(x=>x.label===I18N.t('inf.up')); await back.fn(); await till(shown(root.id));
              return {root:root.id, kid:j.nodi.filter(n=>n.genitore===root.id).map(n=>n.id), after, again:spec().filt}})()""")
            assert r0["after"][4:] in r0["kid"], r0
            assert r0["again"] == "msn:" + r0["root"], r0
        step("«Fragment this ion» goes to the child node, «Back to the precursor» returns", frag)
        def total():
            q = pg.evaluate("""(async()=>{const j=await fetch('api/msn_albero?k=0').then(r=>r.json());const n=j.nodi.find(x=>x.livello===2);
              const u=`api/spectrum?k=0&rt0=${n.rt0-1e-4}&rt1=${n.rt1+1e-4}&level=${n.livello}&precursor=&filt=${encodeURIComponent('msn:'+n.id)}`;
              const a=await fetch(u).then(r=>r.json()),b=await fetch(u+'&modo=somma').then(r=>r.json());
              return {n:a.scans,ma:Math.max(...a.y),mb:Math.max(...b.y),nb:b.scans}})()""")
            assert q["n"] >= 10 and abs(q["mb"] - q["ma"] * q["n"]) <= 1e-6 * q["mb"], q
        step("the sum is the mean times the number of scans", total)
        def dda():
            assert pg.evaluate("I18N.t('inf.name')") != "Full Scan + DDA"
        step("texts exist", dda)
        errs = [e for e in r.errs if e[0] in ("pageerror",)]
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
        step("no request left unanswered by the engine", lambda: (_ for _ in ()).throw(AssertionError(pg.evaluate("window.MZLAB_LR_MISS"))) if pg.evaluate("(window.MZLAB_LR_MISS||[]).length") else None)
        # a DDA file next to it keeps its name
        pg2 = r.b.new_page(viewport={"width": 1500, "height": 1600}); pg2.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg2, "window.qqReady === true")
        load(pg2, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        def keeps():
            t = pg2.evaluate("E.files.map(f=>f.kind).sort().join()"); assert "infusion" not in t, t
        step("a DDA file stays «Full Scan + DDA» (no infusion)", keeps)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
