"""Block 3-4: hover box on m/z labels, panel numbers (+ keys 1-9), right click on files (rename, colour, remove), area label with 3 decimals and the full number in the tooltip."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e21.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8821, wd="/tmp/wd21")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in ["B_FullMass-t0", "B_FullMass-t15", "B_MS2-t15"]]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)      # opened file by file: the view starts on the first file of the list
        def label_hover():
            pg.wait_for_timeout(800)
            sp = pg.evaluate("(()=>{const p=E.panels.find(q=>q.tab===E.tab&&q.type==='spec');p.el.scrollIntoView({block:'center'});return p.id})()"); pg.wait_for_timeout(500)
            pos = pg.evaluate("""id=>{const p=E.panels.find(q=>q.id===id),l=p._a.lbls[0],r=p.cv.getBoundingClientRect();return {n:p._a.lbls.length,px:r.left+l.x+l.w/2,py:r.top+l.y+l.h/2}}""", sp)
            assert pos["n"] > 0, pos
            pg.mouse.move(pos["px"], pos["py"]); pg.wait_for_timeout(300)
            assert pg.evaluate("id=>!E.panels.find(q=>q.id===id).el.querySelector('.lbbox').hidden", sp)
            assert "Tasto destro" in pg.evaluate("id=>E.panels.find(q=>q.id===id).tip.textContent", sp)
            assert pg.evaluate("id=>E.panels.find(q=>q.id===id).cv.style.cursor", sp) == "pointer"
            pg.mouse.move(pos["px"], pos["py"] + 120); pg.wait_for_timeout(200)
            assert pg.evaluate("id=>E.panels.find(q=>q.id===id).el.querySelector('.lbbox').hidden", sp)
        step("hover on an m/z label: box, pointer cursor, tooltip", label_hover)
        def numbering():
            nums = pg.evaluate("E.panels.filter(p=>p.tab===E.tab).sort((a,b)=>a.y-b.y).map(p=>p.el.querySelector('.pnum').textContent)"); print(nums)
            assert nums == ["1", "2"], nums
            pg.evaluate("document.activeElement.blur()"); pg.keyboard.press("2"); pg.wait_for_timeout(300)
            assert pg.evaluate("E.active&&E.active.num") == 2
            assert pg.evaluate("E.panels.filter(p=>p.tab===E.tab).every(p=>!p.el.querySelector('.pnum').hidden)"), "numbers are always shown"
        step("panels are numbered from the top; key 2 activates the second", numbering)
        def files():
            drawer(pg); nm = pg.locator("#flst .fl .nm").first
            nm.click(button="right"); pg.wait_for_timeout(200)
            assert pg.locator("#ctx div", has_text="Rinomina").count() == 1 and pg.locator("#ctx div", has_text="Cambia colore").count() == 1 and pg.locator("#ctx div", has_text="Rimuovi").count() == 1
            pg.locator("#ctx div", has_text="Rinomina").click(); pg.wait_for_timeout(300)
            pg.fill("#askin", "Campione prova"); pg.click("#askok"); pg.wait_for_timeout(500)
            assert "Campione prova" in pg.inner_text("#flst") and pg.evaluate("E.files[0].file").endswith(".mzML") is True
            pg.locator("#flst .fl .nm").first.click(button="right"); pg.locator("#ctx div", has_text="Cambia colore").click(); pg.wait_for_timeout(300)
            pg.locator("#colpop [data-c]").nth(3).click(); pg.wait_for_timeout(400)
            assert pg.evaluate("E.files[0].color") == "#D55E00" and pg.evaluate("E.files[0].colorSet")
            n0 = pg.locator("#flst .fl").count()
            pg.locator("#flst .fl .nm").first.click(button="right"); pg.locator("#ctx div", has_text="Rimuovi").click(); pg.wait_for_timeout(300)
            pg.click("#askok"); pg.wait_for_timeout(800)
            assert pg.locator("#flst .fl").count() == n0 - 1 and pg.evaluate("E.files[0].gone")
        step("right click on a file: rename, colour, remove (with confirmation)", files)
        def areas():
            pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(800)
            info = pg.evaluate("""()=>{const p=E.panels.find(q=>q.tab==='full'&&q.type==='chrom');p.el.scrollIntoView({block:'center'});const s=p._a.sr[0];let m=0;s.ys.forEach((v,i)=>{if(v>s.ys[m])m=i});
              p.ints.push({key:s.key,a:s.x[m]-0.3,b:s.x[m]+0.3});draw(p);return {id:p.id}}""")
            pg.wait_for_timeout(1200)
            lab = pg.evaluate("id=>{const p=E.panels.find(q=>q.id===id);return p._a.lbls.filter(l=>l.tip.includes('Area')).map(l=>l.tip)}", info["id"]); print(lab)
            assert lab and "conteggi" in lab[0]
            assert pg.evaluate("fmtA(2243051)") == "2.243e+6" and pg.evaluate("fmtFull(2243051)") == "2243051"
        step("area label with 3 decimals, the full number (no thousands separator) in the tooltip", areas)
        pg.screenshot(path=SH + "210_labels.png", full_page=True)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
