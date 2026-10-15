"""Addition B: internal blank (constant mean/median, line between two stretches), band + chip, clip negatives, undo, areas on the subtracted trace, notes in tables/xlsx/PNG metadata, spectrum subtraction, reload."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e23.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8823, wd="/tmp/wd23")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in ["B_FullMass-t0", "B_FullMass-t15"]]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        cid = pg.evaluate("E.panels.find(q=>q.tab==='full'&&q.type==='chrom').id")
        P = f"E.panels.find(q=>q.id=={cid})"
        def raw_min():
            return pg.evaluate("""async()=>{const p=%s;const all=await seriesOf(p);return all.map(s=>({n:s.name,mn:Math.min(...s.y),mx:Math.max(...s.y),lv:s.ibl&&s.ibl.lvl}))}""" % P)
        def mean():
            before = raw_min()
            pg.evaluate("ibkSet(%s,'mean',[0.3,2])" % P); pg.wait_for_timeout(1500)
            after = raw_min(); print(before, after)
            assert all(a["lv"] is not None and a["lv"] > 0 for a in after)
            assert all(abs((b["mx"] - a["mx"]) - a["lv"]) < 1e-6 * max(1, b["mx"]) for a, b in zip(after, before)), "max drops by the level"
            assert pg.evaluate("!%s.clipDraw" % P), "negatives are not clipped: «negativi a 0» is only for the drawing"
            chip = pg.inner_text(f"#dpanels .pnl >> nth=0 >> .ibk"); print(chip)
            assert "fondo del tratto" in chip and "0.3-2 min" in chip
        step("mean of 0.3-2 min subtracted, not clipped, chip visible", mean)
        def noclip():
            pg.evaluate("%s.clipDraw=true;draw(%s)" % (P, P)); pg.wait_for_timeout(1200)
            assert pg.evaluate("Math.min(...%s._a.sr[0].ys)" % P) >= 0, "drawing clipped on request"
            assert min(a["mn"] for a in raw_min()) < 0, "the numbers keep their negatives"
            pg.evaluate("%s.clipDraw=false;draw(%s)" % (P, P)); pg.wait_for_timeout(800)
        step("unclip shows negatives", noclip)
        def median_line():
            pg.evaluate("ibkSet(%s,'median',[0.3,2])" % P); pg.wait_for_timeout(800)
            m = raw_min(); assert all(a["lv"] is not None for a in m)
            pg.evaluate("ibkPick(%s,'line',[0.3,1.5])" % P); assert pg.evaluate("!!%s._ibkA" % P); pg.keyboard.press("Escape")
            pg.evaluate("document.querySelector('#askdlg[open] #askok')&&document.querySelector('#askok').click()")
            pg.evaluate("ibkPick(%s,'line',[18,19.5])" % P); pg.wait_for_timeout(1200)
            assert pg.evaluate("%s.fondo.ib.mode" % P) == "line" and pg.evaluate("%s.fondo.ib.b[0]" % P) == 18
            assert all(a["lv"] is not None for a in raw_min())
        step("median, then line between two stretches", median_line)
        def integ_note():
            pg.evaluate("ibkSet(%s,'mean',[0.3,2])" % P)
            pg.evaluate("""async()=>{const p=%s;await draw(p);const s=p._a.sr[0];let m=0;s.ys.forEach((v,i)=>{if(v>s.ys[m])m=i});p.ints.push({key:s.key,a:s.x[m]-0.3,b:s.x[m]+0.3,ion:s.name,file:'f',time:0});await draw(p)}""" % P); pg.wait_for_timeout(1500)
            it = pg.evaluate("%s.ints[0].corr" % P); print(it); assert it.startswith("sì, 0.3-2 min, media")
            a1 = pg.evaluate("%s.ints[0].area" % P)
            pg.evaluate("%s.fondo={mode:'none',w:1,ib:null};draw(%s)" % (P, P)); pg.wait_for_timeout(1200)
            a0 = pg.evaluate("%s.ints[0].area" % P); print(a0, a1); assert a0 != a1 and pg.evaluate("%s.ints[0].corr" % P) == "no"
            pg.evaluate("ibkSet(%s,'mean',[0.3,2])" % P); pg.wait_for_timeout(1200)
        step("integration on the subtracted trace and declared (table)", integ_note)
        def meta():
            sh = pg.evaluate("plotSheets(%s).map(s=>s.name)" % P); print(sh); assert "Note" in sh
            assert "bianco interno applicato" in pg.evaluate("plotMeta(%s).map(x=>x.join(' ')).join(' ')" % P)
        step("xlsx Note sheet and PNG metadata", meta)
        def menu():
            box = pg.locator("#dpanels .pnl >> nth=0 >> canvas").bounding_box()
            pg.mouse.click(box["x"] + 400, box["y"] + 100, button="right"); pg.wait_for_timeout(300)
            t = pg.inner_text("#ctx"); assert "Sottrai il fondo di questo tratto" in t or "Togli il fondo del tratto" in t, t
            pg.keyboard.press("Escape")
        step("right-click menu offers the internal blank", menu)
        def spec():
            pg.evaluate("document.querySelector('#ctx').hidden=true")
            n = pg.evaluate("E.panels.filter(q=>q.type==='spec'&&(q.link===%d||q.src===%d)).length" % (cid, cid)); assert n >= 1, n
            pg.evaluate("ibkSpectra(%s)" % P); pg.wait_for_timeout(1500)
            assert pg.evaluate("E.panels.filter(q=>q.type==='spec'&&(q.link===%d||q.src===%d)).every(q=>q.bg==='w'&&q.bw0===0.3&&q.bw1===2)" % (cid, cid))
        step("spectrum of the blank stretch subtracted from the linked spectra", spec)
        def reload():
            pg.evaluate("uiSave(true)"); pg.wait_for_timeout(1500); pg.reload(); pg.wait_for_timeout(5000)
            assert pg.evaluate("(E.panels.find(q=>q.tab==='full'&&q.type==='chrom')||{}).fondo.ib.mode") == "mean"
            pg.locator("#dpanels .pnl >> nth=0 >> [data-o=cinfo]").click(); pg.locator("#dpanels .pnl >> nth=0 >> [data-ib=off]").click(); pg.wait_for_timeout(500)
            assert pg.evaluate("E.panels.find(q=>q.tab==='full'&&q.type==='chrom').fondo.mode") == "none"
        step("saved in the notebook, removable with one click", reload)
        pg.screenshot(path=SH + "230_ibk.png", full_page=True)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
