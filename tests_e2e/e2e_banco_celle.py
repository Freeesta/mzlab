"""H3a: the cells of the high-resolution bench: scan filter of a cell (listed with the number of scans of each type), pin, active cell; nothing of this in the
low-resolution world. Synthetic Orbitrap DDA file; the infusion MSn file of the private data repository (skipped without it) for the paths of fragmentation."""
import sys, os, glob; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from dati_percorsi import percorso  # noqa: E402  (tools/ is on sys.path through lib)
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_banco_celle.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrcelle")
r = Run(port=8975, wd="/tmp/wd_hrcelle")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        def world():
            assert pg.evaluate("document.body.classList.contains('hrw')")
            assert pg.locator(".pnl.chrom .bfw select[data-bf=filt]").count() >= 1, "a chromatogram cell has the filter menu"
            assert pg.locator(".pnl.spec .bfw button[data-bf=pin]").count() >= 1, "a spectrum cell has the pin"
        step("high-resolution world: filter menu on the chromatogram, pin on the spectrum", world)
        def listing():
            pg.wait_for_function("(()=>{const s=document.querySelector('.pnl.chrom .bfw select[data-bf=filt]');return s&&s.options.length>1})()", timeout=20000)
            t = pg.evaluate("[...document.querySelector('.pnl.chrom .bfw select[data-bf=filt]').options].map(o=>o.textContent)")
            assert any("Full ms" in x and "×" in x for x in t), t
        step("the menu lists the scan types of the file with their counts", listing)
        def ms2filter():
            pg.evaluate("setTab('ms2')"); pg.wait_for_timeout(2500); pg.evaluate("ms2Switch(null)"); pg.wait_for_timeout(1500)
            pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='ms2').el.setAttribute('data-tt','ms2c')")
            pg.wait_for_function("(()=>{const s=document.querySelector('[data-tt=ms2c] .bfw select[data-bf=filt]');return s&&s.options.length>1})()", timeout=20000)
            opts = pg.evaluate("[...document.querySelector('[data-tt=ms2c] .bfw select[data-bf=filt]').options].map(o=>[o.value,o.textContent])")
            key = [o[0] for o in opts if "ms2" in o[0]][0]
            n = int(opts[[o[0] for o in opts].index(key)][1].split("×")[1].replace(" ", ""))
            pg.select_option("[data-tt=ms2c] .bfw select[data-bf=filt]", key); pg.wait_for_timeout(1500)
            got = pg.evaluate("(()=>{const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='ms2');return {filt:c.filt,flv:c.flv,n:c._a&&c._a.sr?c._a.sr.reduce((a,s)=>a+s.x.length,0):-1}})()")
            assert got["filt"] == key and got["flv"] == 2 and got["n"] == n, (got, key, n)
        step("choosing an MS2 type: the chromatogram has exactly the scans of that type", ms2filter)
        def pin():
            pg.evaluate("setTab('full')"); pg.wait_for_timeout(1500)
            sp = pg.evaluate("(()=>{const s=E.panels.find(p=>p.type==='spec'&&p.link!=null&&p.dda==null&&p.tab==='full');return s?s.id:null})()")
            assert sp is not None, "a linked spectrum"
            pg.click(f".pnl.spec >> nth=0 >> [data-bf=pin]") if False else pg.evaluate(f"E.panels.find(p=>p.id=={sp}).el.querySelector('[data-bf=pin]').click()"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"(()=>{{const s=E.panels.find(p=>p.id=={sp});return [s.pin,s.link]}})()") == [True, None]
            pg.evaluate(f"E.panels.find(p=>p.id=={sp}).el.querySelector('[data-bf=pin]').click()"); pg.wait_for_timeout(500)
            v = pg.evaluate(f"(()=>{{const s=E.panels.find(p=>p.id=={sp});return [s.pin,s.link!=null]}})()"); assert v == [False, True], v
        step("pin: a pinned spectrum leaves the cursor, unpinned it follows again", pin)
        def ranges():
            pg.evaluate("setTab('full')"); pg.wait_for_timeout(1200)
            c = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full')"
            pg.evaluate(f"E.panels.find(p=>p.type==='chrom'&&p.tab==='full').el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            for kind, flt in (("tic", ""), ("bpc", ""), ("tic", "ms2")):
                pg.evaluate(f"BANCO.addRange({c})"); pg.wait_for_selector("#br-ok", timeout=10000)
                pg.select_option("#br-kind", kind)
                if flt:
                    key = pg.evaluate("[...document.querySelector('#br-filt').options].map(o=>o.value).find(v=>v.includes('ms2'))"); pg.select_option("#br-filt", key)
                pg.click("#br-ok"); pg.wait_for_timeout(1500)
            a = pg.evaluate(f"(()=>{{const a={c}._a;return {{stk:a.stk,nrow:a.nrow,rows:[...new Set(a.sr.map(s=>s.row))],nl:a.nl}}}})()")
            assert a["stk"] and a["nrow"] == 3 and a["rows"] == [0, 1, 2] and all(v > 0 for v in a["nl"]), a
            assert a["nl"][0] != a["nl"][2], a      # the MS2 row is not the MS1 one
            pg.evaluate(f"(()=>{{const p={c};p.norm100=true;draw(p)}})()"); pg.wait_for_timeout(800)
            assert pg.evaluate(f"{c}._a.nl") == a["nl"], "the NL stays absolute when the rows are normalised"
            n = pg.locator(".pnl.chrom [data-bf=drop]").count(); assert n == 3, n
            pg.evaluate(f"BANCO.dropRange({c}, {c}.ranges[0].id)"); pg.wait_for_timeout(800)
            assert pg.evaluate(f"{c}.ranges.length") == 2
            pg.evaluate(f"(()=>{{const p={c};p.ranges=null;p.mode='ovl';ctl(p);draw(p)}})()"); pg.wait_for_timeout(500)
        step("stacked graphs: three rows with their NL, normalised 0-100 without losing the NL, one removed", ranges)
        def head():
            pg.wait_for_timeout(500)
            t = pg.evaluate("(()=>{const s=E.panels.find(p=>p.type==='spec'&&p.link!=null&&p.dda==null&&p.tab==='full');return s&&s.el.querySelector('.bnl')?s.el.querySelector('.bnl').textContent:null})()")
            assert t and "NL: " in t, t
        step("the spectrum cell has a short header with the NL", head)
        def active():
            pg.evaluate("E.panels[0].el.scrollIntoView({block:'center'})"); pg.wait_for_timeout(200)
            box = pg.locator(".pnl.chrom canvas").first.bounding_box(); pg.mouse.click(box["x"] + 30, box["y"] + 30); pg.wait_for_timeout(300)
            sh = pg.evaluate("getComputedStyle(document.querySelector('.pnl.act')).boxShadow"); assert sh and sh != "none", sh
        step("the active cell has an outline", active)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
# low resolution: nothing of it
r = Run(port=8976, wd="/tmp/wd_lrcelle")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [D / "B_FullMass-t0.mzML"]); pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        def lr():
            assert not pg.evaluate("document.body.classList.contains('hrw')")
            assert pg.locator(".bfw").count() == 0
        step("low-resolution world: no filter menu, no pin", lr)
    r.close()
except Exception as e:
    steps.append(("lr", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
HRMS = percorso("dati/HRMS")    # None without the private data repository: the block is skipped
F = next(iter(glob.glob(f"{HRMS}/*/*direct-infusion_MSn.mzML")), None) if HRMS else None
if F:
    r = Run(port=8977, wd="/tmp/wd_msncelle")
    try:
        with sync_playwright() as p:
            pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
            load(pg, [F], 8000, 1)
            def paths():
                pg.wait_for_timeout(3000)
                pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='infusion').el.setAttribute('data-tt','ms2c')")
                pg.wait_for_function("(()=>{const s=document.querySelector('[data-tt=ms2c] .bfw select[data-bf=filt]');return s&&s.options.length>10})()", timeout=30000)
                opts = pg.evaluate("[...document.querySelector('[data-tt=ms2c] .bfw select[data-bf=filt]').options].map(o=>[o.value,o.textContent])")
                assert len(opts) >= 18, len(opts)
                key = [o[0] for o in opts if "317 \u2192 261 \u2192 244" in o[1] and "CE 35" in o[1]][0]
                n = int(opts[[o[0] for o in opts].index(key)][1].split("×")[1].replace(" ", ""))
                pg.select_option("[data-tt=ms2c] .bfw select[data-bf=filt]", key); pg.wait_for_timeout(2500)
                got = pg.evaluate("(()=>{const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='infusion');return {flv:c.flv,n:c._a&&c._a.sr?c._a.sr.reduce((a,s)=>a+s.x.length,0):-1}})()")
                assert got["flv"] == 4 and got["n"] == n, (got, n)
            step("MSn infusion: one filter per node of the tree; ms4 317 > 261 > 244 shows its own scans", paths)
        r.close()
    except Exception as e:
        steps.append(("msn", "FAIL " + str(e)[:300]))
        try: r.close()
        except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
