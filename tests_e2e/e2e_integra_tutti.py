"""«Integrate every peak shown» (the ▾ next to the automatic tool, integra-tutti.js) on a jagged 3200 QTRAP-like MRM file (tools/synthetic_data_jagged.py):
every peak of the transition shown (three touching peaks, two valleys) in one undoable step, split at the valleys, the page area equal to the engine's, the JSON of the page
engine (QUALITA.lrAuto, the .wasm) byte for byte the one of `mzlab-cli peak auto-lr` on the same trace, the settings of the dialog reach the engine."""
import sys, os, json, subprocess; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import load
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(ROOT / "tests"))
from synthetic_data_jagged import jagged_file
from cargo_util import cli_argv
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_integra_tutti.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = HERE / "shots" / "integra_tutti_tmp"; D.mkdir(parents=True, exist_ok=True)
F = D / "J_MRM-jagged.mzML"
TRUTH = jagged_file(F)
M = "E.panels.find(p=>p.type==='mrm')"
WINS = f"(()=>{{const p={M};return p.ints.map(i=>[i.ion,+i.a.toFixed(4),+i.b.toFixed(4)]).sort((u,v)=>u[0]<v[0]?-1:u[0]>v[0]?1:u[1]-v[1])}})()"

r = Run(port=8987, wd="/tmp/wd_integra_tutti")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1280, "height": 800})
        load(pg, [F], 4000, 1)
        open_kind(pg, "mrm"); ready(pg)
        wait_for_js(pg, f"(()=>{{const p={M};return !!(p&&p._a&&p._a.sr&&p._a.sr.length>=1)}})()")
        pg.evaluate(f"{M}.ints=[]")
        def every_peak():
            hd_click(pg, ".pnl.mrm [data-a=iall]", keep=True)
            pg.locator("#ctx >> text=" + pg.evaluate("I18N.t('int.menu.all')")).first.click()
            wait_for_js(pg, f"{M}.ints.length>0", timeout=30000)
            assert pg.evaluate(f"{M}._a.sr[0].name").endswith("triple")
            w, tr = pg.evaluate(WINS), TRUTH["triple"]
            assert len(w) == tr["expected"] == 3, w
            for a in tr["apex"]: assert sum(x[1] <= a <= x[2] for x in w) == 1, (a, w)
            for v in tr["valley"]:
                cut = [x[2] for x, y in zip(w, w[1:]) if abs(x[2] - y[1]) < 1e-6 and x[1] < v < y[2]]
                assert len(cut) == 1 and abs(cut[0] - v) <= 0.0625, (v, cut, w)            # the shared border within 2.5 points of the clean valley
            assert pg.evaluate("!document.querySelector('#qnote').hidden")
        step("one click integrates the three touching peaks of the jagged transition, split at the valleys", every_peak)
        def same_area():
            v = pg.evaluate(f"""(async()=>{{const p={M},a=p._a;const out=[];for(const s of a.sr){{const t=await QUALITA.lrAuto(s.x,s.y,a.x0,a.x1,INTTUTTI.engineParams(PK));
              const r=JSON.parse(t);const its=p.ints.filter(i=>i.key===s.key).sort((u,w)=>u.a-w.a);
              its.forEach((it,k)=>{{const q=r.peaks[k];out.push([it.area,(q.area-q.below),it.below,q.below,it.bm,it.snap&&it.snap.a,it.my&&it.my.a,q.b0,it.auto,q])}})}}return out}})()""")
            assert v, v
            for page, net, pb, eb, bm, snap, my, b0, au, q in v:
                assert bm == "line" and snap == "man" and my == b0 and au["half_auto"] is True, v      # the engine's baseline as two manual anchors
                # the page keeps its signed measure of a line (above minus the part under it, shown apart): the engine's area minus its `below`
                assert abs(page - net) <= 1e-6 * abs(net) and abs(pb - eb) <= 1e-6 * max(abs(eb), 1), (page, net, pb, eb)
                # the record keeps every choice: the engine's areas (counts x s), its baseline, why each end stopped and what its clamp did
                assert au["baseline"] == "engine" and abs(au["area"] - q["area"]) <= 1e-9 * q["area"] and au["area"] >= 0 and au["below"] >= 0, au
                assert au["split"] == "drop" and {au["clamp0"], au["clamp1"]} <= {"none", "trace"} and au["end0"] and au["end1"], au
            note = pg.evaluate("document.querySelector('#qnote').textContent")
            assert pg.evaluate("I18N.t('pk.split.0')") in note and pg.evaluate("I18N.t('int.lrAll.done',{n:0,w:0,a:'',v:0,sn:0,noise:0,split:'',c:0,e:0,below:0})").split(" · ")[6] in note, note
        step("each record is the engine's peak: same baseline, same area", same_area)
        pg.screenshot(path=str(HERE / "shots" / "integra_tutti.png"))
        def parity():
            for pk, sets in (("{}", []), ("{lrSmooth:7,lrValley:20,lrSn:4,lrSplit:1}", ["lr_half=3", "lr_valley=20", "lr_sn=4", "lr_split=valley"])):
                pg.evaluate(f"(async()=>{{const s={M}._a.sr[0];window.__csv=s.x.map((t,i)=>t+','+s.y[i]).join('\\n');window.__lr=await QUALITA.lrAuto(s.x,s.y,0,1e9,INTTUTTI.engineParams({{...PK,...{pk}}}))}})()")
                wait_for_js(pg, "typeof window.__lr==='string'")
                site, csv = pg.evaluate("window.__lr"), pg.evaluate("window.__csv")
                out = subprocess.run(cli_argv("peak", "auto-lr", "-", *[a for x in sets for a in ((("--split", x.split("=")[1])) if x.startswith("lr_split=") else ("--set", x))]), input=csv, capture_output=True, text=True, cwd=ROOT, check=True).stdout
                assert out == site + "\n", (out[:300], site[:300])
                assert json.loads(site)["params"]["split"] == ("valley" if sets else "drop") and len(json.loads(site)["peaks"]) == 3
                pg.evaluate("window.__lr=null")
        step("the page engine and mzlab-cli peak auto-lr give the same bytes", parity)
        def undo_again():
            n = pg.evaluate(f"{M}.ints.length")
            pg.evaluate("HISTORY.undo()"); wait_for_js(pg, f"{M}.ints.length===0")
            pg.evaluate(f"INTTUTTI.run({M})"); wait_for_js(pg, f"{M}.ints.length==={n}")
            pg.evaluate(f"INTTUTTI.run({M}).then(()=>1)")                        # run() is async: evaluate waits until it has drawn
            assert pg.evaluate(f"{M}.ints.length") == n                                     # a second run adds nothing over the peaks already there
        step("one undo removes all of them; a second run adds no overlapping window", undo_again)
        def dialog():
            hd_click(pg, ".pnl.mrm [data-a=iall]", keep=True)        # a narrow header moves it into «⋯»
            pg.locator("#ctx >> text=" + pg.evaluate("I18N.t('int.menu.params')")).first.click()
            pg.wait_for_selector("#bigdlg [data-pk=lrSmooth]")
            box = pg.evaluate("(()=>{const r=document.querySelector('#bigdlg').getBoundingClientRect();return [r.bottom,innerHeight]})()")
            assert box[0] <= box[1], box
            pg.fill("#bigdlg [data-pk=lrSmooth]", "8"); pg.fill("#bigdlg [data-pk=lrValley]", "25"); pg.select_option("#bigdlg [data-pk=lrSplit]", "1"); pg.click("#pk-ok")
            assert pg.evaluate("[PK.lrSmooth,PK.lrValley,PK.lrSplit]") == [9, 25, 1]                    # an even window goes to the next odd one
            pg.evaluate(f"(()=>{{{M}.ints=[];const f=QUALITA.lrAuto;window.__p=null;QUALITA.lrAuto=(...a)=>{{window.__p=a[4];QUALITA.lrAuto=f;return f(...a)}}}})()")
            pg.evaluate(f"INTTUTTI.run({M})"); wait_for_js(pg, "window.__p!==null")
            assert pg.evaluate("window.__p") == {"lrHalf": 4, "lrValley": 25, "lrSn": 5, "lrSplit": 1}, pg.evaluate("window.__p")
            wait_for_js(pg, f"{M}.ints.length>0"); assert pg.evaluate(f"{M}.ints.every(i=>i.auto.half===4&&!i.auto.half_auto&&i.auto.split==='valley')")
            pg.evaluate("Object.assign(PK, PK_DEF)")
        step("the settings of the dialog reach the engine and are kept on each record", dialog)
        errs = [e for e in r.errs if "allow-same-origin" not in e[1]]          # the sandboxed iframe of the drawing editor, not this tool
        steps.append(("no page errors", "ok" if not errs else "FAIL " + str(errs)[:240]))
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
