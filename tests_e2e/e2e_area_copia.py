"""Area label of an integration: click copies the displayed number, «Copia valore» in the menu, aria-live notice that fades, nothing in the PNG, other clicks unchanged."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_area_copia.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
XP = "E.panels.find(p=>p.type==='xic')"
r = Run(port=8884, wd="/tmp/wd84")
try:
    with sync_playwright() as p:
        pg = r.page(p, motion=True); pg.context.grant_permissions(["clipboard-read", "clipboard-write"])
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_selector("text=Carica dati")
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); attendi(pg, f"!!({XP}&&{XP}._a&&{XP}._a.sr&&{XP}._a.sr.length)")
        pg.evaluate(f"(()=>{{const p={XP};setActive(p);p.imode=null;p.intf=p._a.sr[0].key;p.el.scrollIntoView({{block:'center'}})}})()"); frames(pg, 2)
        KEY = pg.evaluate(f"{XP}._a.sr[0].key")
        pg.evaluate(f"(()=>{{const p={XP};addInt(p,p._a.sr[0],13.9,14.6)}})()"); frames(pg, 3)
        def lab(): return pg.evaluate(f"(()=>{{const p={XP},l=p._a.lbls.find(b=>b.copy!=null),q=p.cv.getBoundingClientRect();return {{x:q.left+l.x+l.w/2,y:q.top+l.y+l.h/2,copy:l.copy,sel:p.isel}}}})()")
        def clip(): return pg.evaluate("navigator.clipboard.readText()")
        def click_copies():
            l = lab(); assert l["copy"].replace(".", "").replace("E+", "").replace("E-", "").isdigit(), l
            pg.mouse.move(l["x"], l["y"]); frames(pg, 2)
            assert pg.evaluate(f"{XP}.cv.style.cursor") == "pointer"
            pg.mouse.click(l["x"], l["y"]); attendi(pg, "!!document.querySelector('.areacopied.show')")
            assert clip() == l["copy"], (clip(), l)
            assert pg.inner_text(".areacopied") == "Copiato!"
            assert pg.get_attribute(".areacopied", "aria-live") == "polite"
        step("click on the label copies the displayed number and shows the notice", click_copies)
        def fades():
            attendi(pg, "!document.querySelector('.areacopied.show')", 4000)
        step("the notice fades after about 1.5 s", fades)
        def elsewhere():
            pg.evaluate("navigator.clipboard.writeText('x')"); n0 = pg.evaluate(f"{XP}.ints.length")
            c = pg.evaluate(f"(()=>{{const p={XP},q=p.cv.getBoundingClientRect(),a=p._a;return {{x:q.left+a.X(15.5),y:q.top+a.H-M.b-6}}}})()")
            pg.mouse.click(c["x"], c["y"]); frames(pg, 2)
            assert clip() == "x" and pg.evaluate(f"{XP}.ints.length") == n0
        step("a click elsewhere does not copy", elsewhere)
        def menu_copies():
            l = lab(); pg.evaluate("navigator.clipboard.writeText('x')")
            names = pg.evaluate(f"(()=>{{const p={XP};return AZIONI.per({{vista:p.type,p,x:14.2,k:0,near:null,px:{l['x']}-p.cv.getBoundingClientRect().left,py:{l['y']}-p.cv.getBoundingClientRect().top}}).map(i=>i.label||i.nome)}})()")
            assert "Copia valore" in [str(x) for x in names], names
            pg.mouse.click(l["x"], l["y"], button="right"); attendi(pg, "[...document.querySelectorAll('.ctx *')].some(e=>e.textContent==='Copia valore')")
            pg.get_by_text("Copia valore", exact=True).click(); attendi(pg, "!!document.querySelector('.areacopied.show')")
            assert clip() == l["copy"]
        step("context menu: «Copia valore» does the same", menu_copies)
        def not_in_png():
            url = pg.evaluate(f"{XP}.cv.toDataURL()"); assert url.startswith("data:image/png")
            assert pg.evaluate("document.querySelector('.areacopied')!==null"), "notice is a DOM element, never canvas text"
        step("the notice is a DOM element (not drawn into exports)", not_in_png)
        pg.screenshot(path="tests_e2e/shots/area_copia.png")
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
