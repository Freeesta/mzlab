"""Garbage in the typed numbers of the tools (campi.js gate, lib_campi.sweep): the peak parameters dialog, the lab contaminant rows, the ± tolerance of the
reference lists, the target screening view, the «What is it?» window (precursor, tolerances, thresholds) and the fragmentation graph of an infusion; each field
refuses letters, 1e999, empty, out-of-bound numbers and decimals where a whole number is due with one note and the old value; Enter in a refused field runs
nothing and a comma decimal is taken."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
from lib_campi import sweep, typed
sys.path.insert(0, str(ROOT))
from TP_Mine.tests.msn_synth import write_msn, caffeine_nodes
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_strumenti_garbage.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = synth("strgarb")
F = write_msn(D / "MSn_synth_t01.mzML", caffeine_nodes(), n_scans=12)
TRIO = "(()=>{const s1=E.panels.find(p=>p.type==='spec'&&p.duo!=null),s2=E.panels.find(p=>p.dda!=null);return {s1,s2}})()"
urls = []
r = Run(port=8954, wd="/tmp/wd_strgarb")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1500})
        pg.on("request", lambda q: urls.append(q.url))
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2); wait_for_js(pg, "E.files.some(f=>f.kind==='ms2')")
        def peaks():
            pg.evaluate("Object.assign(PK, PK_DEF); peakParams()"); pg.wait_for_selector("#pk-ok", timeout=5000)
            assert sweep(pg, "#bigbody", "JSON.stringify(PK)") >= 5
            typed(pg, "[data-pk=edgeFrac]", "2,5"); pg.click("#pk-ok"); wait_for_js(pg, "PK.edgeFrac === 2.5", 5000)
        step("peak parameters: every number refuses garbage; «2,5» is taken", peaks)
        def lab():
            pg.evaluate("localStorage.setItem(NOMI.KEY('contaminanti'), JSON.stringify([{mz: 100, name: 'x'}])); document.querySelectorAll('dialog[open]').forEach(d => d.close()); LISTE.openLab()")
            pg.wait_for_selector("#bigdlg[open] [data-c=mz]", timeout=5000)
            sweep(pg, "#bigbody", "localStorage.getItem(NOMI.KEY('contaminanti'))")
            typed(pg, "#bigbody [data-c=mz]", "150,25"); wait_for_js(pg, "JSON.parse(localStorage.getItem(NOMI.KEY('contaminanti')))[0].mz === 150.25", 5000)
            pg.evaluate("document.querySelector('#bigdlg').close()")
        step("lab contaminants: the m/z of a row refuses garbage; «150,25» is taken", lab)
        def lists():
            pg.evaluate("BARRA.close(false)"); pg.click("#np-liste"); pg.wait_for_selector("#sb-panel-lists #lst-tol", timeout=5000)
            sweep(pg, "#sb-panel-lists .liste-bar")
            typed(pg, "#lst-tol", "2,5"); assert pg.get_attribute("#lst-tol", "aria-invalid") is None and pg.input_value("#lst-tol") == "2,5"
        step("reference lists: the ± tolerance refuses garbage; «2,5» is taken", lists)
        def target():
            pg.evaluate("document.querySelectorAll('dialog[open]').forEach(d=>d.close()); LISTE.openTargetScreening({})"); pg.wait_for_selector("#ts-start", timeout=20000)
            assert sweep(pg, "#lst-target-box") == 2
        step("target screening: ppm and S/N refuse garbage", target)
        def what_is():
            info = pg.evaluate("""(async()=>{const k=E.files.find(f=>f.kind==='ms2').k, d=await J('api/dda?k='+k), cnt={};d.tgt.forEach(t=>{if(t)cnt[t]=(cnt[t]||0)+1});
              const t=+Object.keys(cnt).sort((a,b)=>cnt[b]-cnt[a])[0], i=d.tgt.indexOf(t), s=await J('api/scan?k='+k+'&sid='+d.sid[i]);
              return {t, sid:d.sid[i], mz:s.mz, y:s.y}})()""")
            pk = [(m, y) for m, y in zip(info["mz"], info["y"]) if m < info["t"] - 1.5 and y > 0]
            text = (f"Name: Voce di prova\nPrecursorMZ: {info['t']}\nPrecursor_type: [M+H]+\nIon_mode: Positive\nFormula: C10H10\nInChIKey: AAAAAAAAAAAAAA-AAAAAAAAAA-N\nNum Peaks: {len(pk)}\n"
                    + "\n".join(f"{m:.5f} {y:.1f}" for m, y in pk) + "\n\n")
            pg.evaluate("(async t=>{await LIB.call('add',{file:new File([t],'prova.msp')})})(%s)" % json.dumps(text))
            pg.evaluate("document.querySelectorAll('dialog[open]').forEach(d=>d.close()); BARRA.close(false)")
            pg.evaluate("(async()=>{const t=" + TRIO + ";await DDA.selectMs2(t.s2," + str(info["sid"]) + ")})()"); wait_for_js(pg, "(()=>{const t=" + TRIO + ";return !!(t.s2&&t.s2._a)})()"); frames(pg, 3)
            pos = pg.evaluate("(()=>{const t=" + TRIO + ";t.s2.el.scrollIntoView({block:'center'});const r=t.s2.cv.getBoundingClientRect();return {x:r.left+r.width*0.5,y:r.top+r.height*0.3}})()")
            pg.mouse.click(pos["x"], pos["y"], button="right"); wait_for_js(pg, "!document.querySelector('#ctx').hidden && document.querySelector('#ctx').innerText.includes(\"Cos'è?\")")
            pg.click("#ctx >> text=Cos'è?"); pg.wait_for_selector("#libwhat[open] #lw-list table", timeout=60000)
            pg.click("#lw-opt summary"); pg.wait_for_selector("#lw-tol", state="visible")
            assert sweep(pg, "#libwhat", "localStorage.getItem('mzlab.lib.soglie')") >= 10
        step("«What is it?»: precursor, tolerances and thresholds refuse garbage", what_is)
        def what_enter():
            n0 = len(urls); x = pg.locator("#lw-prec"); old = x.input_value()
            x.focus(); x.press("ControlOrMeta+a"); x.press_sequentially("abc"); x.press("Enter")
            wait_for_js(pg, "document.querySelector('#lw-prec').getAttribute('aria-invalid') === 'true'", 5000)
            assert pg.input_value("#lw-prec") == old, pg.input_value("#lw-prec")
            typed(pg, "#lw-frag", "0,25", "Enter"); assert pg.get_attribute("#lw-frag", "aria-invalid") is None and pg.input_value("#lw-frag") == "0,25"
            pg.fill("#libwhat [data-thr=entropia_min]", "0,5"); pg.dispatch_event("#libwhat [data-thr=entropia_min]", "change")
            wait_for_js(pg, "(() => { try { return JSON.parse(localStorage.getItem('mzlab.lib.soglie')).mzlab.entropia_min === 0.5; } catch (e) { return false; } })()", 5000)
            assert all("abc" not in u for u in urls[n0:])
        step("«What is it?»: Enter in a refused field runs nothing; «0,25» and «0,5» are taken", what_enter)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
STAND_IN = (Path(__file__).parent / "e2e_infusione_grafo.py").read_text().split('STAND_IN = """', 1)[1].split('"""', 1)[0]
FX = json.loads((Path(__file__).parent / "fixtures" / "msngrafo_esempio.json").read_text())
r2 = Run(port=8955, wd="/tmp/wd_strgarb2")
try:
    with sync_playwright() as p:
        pg = r2.page(p); pg.set_viewport_size({"width": 1500, "height": 1900})
        pg.on("request", lambda q: urls.append(q.url))
        load(pg, [F], 6000, 1)
        wait_for_js(pg, "E.panels.some(p=>p.type==='spec' && p._a && p._a.data && p.filt && p.filt.startsWith('msn:'))", 60000)
        tree = pg.evaluate("fetch('api/msn_albero?k=0').then(r=>r.json())")
        for n, tid in zip([n for n in FX["nodi"] if n["isolato"]], [n["id"] for n in tree["nodi"]]): n["albero"] = [tid]
        pg.evaluate(STAND_IN.replace('\\\\', '\\'), FX)
        def graph():
            drawer(pg, "gra"); pg.wait_for_selector(".ig-node", timeout=30000)
            pg.evaluate("document.querySelectorAll('.ig-set').forEach(d => d.open = true)")
            req = "window.__REQ.filter(r => r[0] === 'api/msn_grafo').length"
            assert sweep(pg, "body", req) >= 6
            typed(pg, "#ig-ppmf", "2,5"); wait_for_js(pg, "window.__REQ.filter(r => r[0] === 'api/msn_grafo').slice(-1)[0][1].includes('2.5')", 15000)
        step("fragmentation graph: delta, min. rel. and settings refuse garbage; «2,5» ppm is taken", graph)
    r2.close()
except Exception as e:
    steps.append(("run2", "FAIL " + str(e)[:300]))
    try: r2.close()
    except Exception: pass
def clean():
    bad = [u for u in urls if any(s in u for s in ("NaN", "Infinity", "undefined"))]
    assert not bad, bad[:3]
step("no request carried NaN, Infinity or undefined", clean)
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.errs.extend(e for e in r2.errs if "can escape its sandboxing" not in str(e))
r.errs[:] = [e for e in r.errs if "can escape its sandboxing" not in str(e)]
r.report()
