"""The click of the automatic tool on four overlaid SIM ions (synthetic SIM .wiff of tools/wiff_sim.py, `sim3`, sample Sample_A: one peak per ion at its own time): one click
integrates ONE trace (the one under the pointer) and the peak nearest the click, never one window per channel; labels never overlap; a junk click (a time where the ion has no
peak) is refused with the evidence and a way to keep it (Alt+click, logged in the record); "all channels at this time" leaves the channels without a peak out; the page engine
(QUALITA.click, the .wasm) gives the bytes of `mzlab-cli peak edges --set click=1`, and the CLI exits 0 / 1 for a kept / refused window.
Needs the site and the .wasm (python3 tools/prepara_sito.py) and the CLI (cargo). Run from the repo root: python3 tests_e2e/e2e_integra_clic.py"""
# covers: tools/wiff_sim.py, mzlab/web/integra-clic.js, mzlab/web/area-etichette.js, mzlab/web/picchi.js, mzlab/web/qualita.js, crates/mzlab-core/src/peakq_click.rs
import sys, os, json, subprocess, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *
sys.path.insert(0, str(ROOT / "tests"))
from cargo_util import cli_argv
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_integra_clic.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = Path(tempfile.mkdtemp(prefix="integra_clic_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
(D / "sim3.wiff.log").write_text("msconvert log (synthetic)\n")
TRUE = json.loads((D / "sim3.json").read_text())
P = "E.panels.find(p=>p.type==='mrm')"
APEX = [pk[0] for pk in TRUE["picchi"][TRUE["campioni"].index("Sample_A")][:4]]            # the apex (min) of ions A to D of Sample_A (the isomer pair of ion C is not used)
ION_A, ION_B, ION_C, ION_D = range(4)

r = Run(port=8988, wd="/tmp/wd_integra_clic")
try:
    with sync_playwright() as p:
        pg = r.page(p, device_scale_factor=2); pg.set_viewport_size({"width": 1280, "height": 800})
        stage(pg, [D / "sim3.wiff", D / "sim3.wiff.scan", D / "sim3.wiff.log"]); pg.click("text=Load data"); ready(pg); wait_for_js(pg, "E.files.length >= 4", timeout=120000)
        open_first(pg)
        pg.click("#np-mrm"); wait_for_js(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)
        # one sample (Sample_A), its first four ions, overlaid, the automatic tool: the panel of the report of the owner
        pg.evaluate(f"""(()=>{{E.browse=false;E.files.forEach(f=>{{f.vis=f.label==='Sample_A'}});const p={P};p.tr='';draw(p)}})()""")
        wait_for_js(pg, f"(()=>{{const p={P};return !!(p._a&&p._a.sr&&p._a.sr.length>=4)}})()", timeout=60000)
        pg.evaluate(f"(()=>{{const p={P};p.tr=p._a.sr.slice(0,4).map(s=>s.tk);setActive(p);p.ints=[];p.imode='auto';toOverlaid(p);draw(p);p.el.scrollIntoView({{block:'center'}})}})()"); frames(pg, 3)
        wait_for_js(pg, f"{P}._a.sr.length===4 && !{P}._a.stk")
        def xy(i, x):
            """Screen position of the trace i at time x (min): the point a user clicks."""
            return pg.evaluate(f"(()=>{{const p={P},a=p._a,s=a.sr[{i}],q=p.cv.getBoundingClientRect(),j=nearIdx(s.x,{x});return {{x:q.left+a.X({x}),y:q.top+a.Y(a.U(s,s.ys[j]))}}}})()")
        pg.evaluate("(()=>{const f=QUALITA.click;window.__ck=0;QUALITA.click=async(...a)=>{try{return await f.apply(QUALITA,a)}finally{window.__ck++}}})()")      # counts the answers of the engine
        def click(i, x, alt=False):
            c = xy(i, x); k0 = pg.evaluate("window.__ck")
            if alt: pg.keyboard.down("Alt")
            pg.mouse.click(c["x"], c["y"])
            if alt: pg.keyboard.up("Alt")
            wait_for_js(pg, f"window.__ck>{k0}", timeout=60000); frames(pg, 4)        # the engine (a first call starts its worker) answered, the page drew
        ints = lambda: pg.evaluate(f"{P}.ints.map(i=>({{key:i.key,a:i.a,b:i.b,id:i.id,click:i.click||null}}))")
        key = lambda i: pg.evaluate(f"{P}._a.sr[{i}].key")
        note = lambda: pg.evaluate("(()=>{const n=document.querySelector('#qnote');return n&&!n.hidden?n.textContent:''})()")
        refused_text = pg.evaluate("I18N.t('int.refused',{why:'@'})").split("@")[0]
        def one_click_one_trace():
            click(ION_B, APEX[ION_B] + 0.02)
            w = ints()
            assert len(w) == 1, ("a click made one window per channel", w)
            assert w[0]["key"] == key(ION_B) and w[0]["a"] < APEX[ION_B] < w[0]["b"], w
            c = w[0]["click"]
            assert c and abs(c["apex"] - APEX[ION_B]) < 0.1 and c["points"] >= 5 and c["sn"] >= 3 and c["params"]["min_points"] == 5 and not c["forced"], c      # the evidence and the guard are logged
            assert pg.evaluate(f"{P}.isel") == w[0]["id"], "the new window is the selected one"
            assert pg.evaluate(f"{P}._hl") == pg.evaluate(f"{P}._a.sr[{ION_B}].tk"), "the trace that was integrated is highlighted"
        step("one click on four overlaid ions makes exactly one integration, on the trace under the pointer", one_click_one_trace)
        def second_ion():
            n0 = len(ints()); click(ION_D, APEX[ION_D] + 0.03)
            w = ints(); assert len(w) == n0 + 1 and w[-1]["key"] == key(ION_D) and w[-1]["a"] < APEX[ION_D] < w[-1]["b"], w
        step("a click at the peak of another ion integrates that ion only", second_ion)
        def labels():
            # the report of the owner: a window on every channel at one time. The labels stack, never overlap, and the selected integration is always drawn
            pg.evaluate(f"(()=>{{const p={P};p.ints=[];p.isel=null;p._a.sr.forEach(s=>addInt(p,s,{APEX[ION_B] - 0.2},{APEX[ION_B] + 0.2},false));p.isel=p.ints[0].id;draw(p)}})()"); frames(pg, 3)
            lb = pg.evaluate(f"({P}._a.lbls||[]).filter(l=>l.int).map(l=>[l.x,l.y,l.w,l.h,l.int.id])")
            assert len(lb) >= 2 and pg.evaluate(f"{P}.isel") in [l[4] for l in lb], ("the selected integration is labelled", lb)
            for k, u in enumerate(lb):
                for v in lb[k + 1:]: assert not (u[0] < v[0] + v[2] and v[0] < u[0] + u[2] and u[1] < v[1] + v[3] and v[1] < u[1] + u[3]), ("labels overlap", u, v)
            pg.evaluate(f"(()=>{{const p={P};p.ints=[];p.isel=null;draw(p)}})()")
            click(ION_B, APEX[ION_B] + 0.02); click(ION_D, APEX[ION_D] + 0.03)                  # back to the two single-click windows for the screenshot
        step("four windows at one time: the labels stack and never overlap, the selected one is drawn", labels)
        pg.screenshot(path=str(HERE / "shots" / "integra_clic_one.png"))
        def junk_refused():
            n0 = len(ints()); click(ION_C, APEX[ION_B])                    # ion C has no peak at the time of the peak of ion B
            assert len(ints()) == n0, ("a window was made where no ion has a peak", ints())
            assert refused_text in note(), note()
            assert pg.evaluate("I18N.t('int.why.noPeak',{reach:0.25})") in note(), note()
        step("a click where the trace has no peak is refused with the reason and the way to keep it", junk_refused)
        pg.screenshot(path=str(HERE / "shots" / "integra_clic_refused.png"))
        def force():
            n0 = len(ints()); click(ION_C, APEX[ION_B], alt=True)
            w = ints(); assert len(w) == n0 + 1 and w[-1]["key"] != key(ION_B), ([(i["key"], i["a"], i["b"]) for i in w], note())
            c = w[-1]["click"]; assert c["forced"] is True and c["refused"], c                 # the override and the reasons it overrode are in the record
            assert pg.evaluate("I18N.t('int.forced',{why:'@'})").split("@")[0] in note(), note()
        step("Alt+click keeps the refused window and logs the reasons that were overridden", force)
        def no_window():
            # an engine answer with no window at all (edges null, even "forced"): the page never reads edges[0], makes no row and says why
            n0 = len(ints())
            pg.evaluate("(()=>{window.__qc=QUALITA.click;QUALITA.click=async()=>JSON.stringify({ok:true,edges:null,apex:null,points:0,area:0,height:0,noise:0,sn:0,refused:['no_edges'],forced:true,params:{min_points:5,min_sn:3,reach:0.25,force:true}})})()")
            try:
                pg.evaluate(f"INTCLICK.go({P},{APEX[ION_B]},null,true,false).then(()=>1)"); frames(pg, 2)
                assert len(ints()) == n0, ("a window was made without edges", ints())
                assert pg.evaluate("I18N.t('int.noWin',{why:'@'})").split("@")[0] in note(), note()
            finally: pg.evaluate("(()=>{QUALITA.click=window.__qc})()")
        step("an answer without a window (edges null) makes no row and says why, forced or not", no_window)
        def all_channels():
            pg.evaluate(f"(()=>{{const p={P};p.ints=[];p.isel=null;draw(p)}})()"); frames(pg, 2)
            pg.evaluate(f"INTCLICK.atRt({P}, {APEX[ION_B]})"); wait_for_js(pg, f"{P}.ints.length>0")
            w = ints(); assert [i["key"] for i in w] == [key(ION_B)], ("the channels without a peak got a row", w)
            t = note(); assert t.startswith("1 of 4 channels integrated") and "no peak here" in t, t
            pg.evaluate("HISTORY.undo()"); wait_for_js(pg, f"{P}.ints.length===0")
        step("«all channels at this time» integrates only the channels that have a peak there, names the others, one undo", all_channels)
        def parity():
            for i, x, force_ in ((ION_B, APEX[ION_B] + 0.02, False), (ION_C, APEX[ION_B], False), (ION_C, APEX[ION_B], True), (ION_D, APEX[ION_D] - 0.03, False)):
                pg.evaluate(f"(async()=>{{const s={P}._a.sr[{i}];window.__csv='rt,y\\n'+s.x.map((t,k)=>t+','+s.y[k]).join('\\n');window.__mrm=(s.key&&s.key.startsWith('m|'))||s.tr!=null;window.__c=await QUALITA.click(s.x,s.y,{x},window.__mrm,PK,{str(force_).lower()})}})()")
                wait_for_js(pg, "typeof window.__c==='string'")
                site, csv, mrm = pg.evaluate("window.__c"), pg.evaluate("window.__csv"), pg.evaluate("window.__mrm")
                argv = cli_argv("peak", "edges", "-", "--at", str(x), "--set", "click=1", *(["--set", "force=1"] if force_ else []), *(["--mrm"] if mrm else []))
                out = subprocess.run(argv, input=csv, capture_output=True, text=True, cwd=ROOT)
                ok = json.loads(site)["ok"]
                # kept: the bytes of the page; refused: the contract's error/1 whose `report` carries those same bytes, and the reasons on stderr
                assert out.stdout == site + "\n" if ok else (json.loads(out.stdout)["code"] == 1 and site in out.stdout and "refused:" in out.stderr), (i, x, out.stdout[:300], site[:300])
                assert out.returncode == (0 if ok else 1), (out.returncode, out.stderr[:200])             # 0 kept, 1 refused
                pg.evaluate("window.__c=null")
        step("the page engine and mzlab-cli peak edges --set click=1 give the same bytes and the exit codes 0 / 1", parity)
        errs = [e for e in r.errs if "allow-same-origin" not in e[1]]
        steps.append(("no page errors", "ok" if not errs else "FAIL " + str(errs)[:240]))
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
