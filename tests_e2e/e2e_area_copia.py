"""Area label of an integration: click copies the displayed number, «Copy value» in the menu, aria-live notice that fades, nothing in the PNG, other clicks unchanged."""
# covers: mzlab/web/area-copia.js, mzlab/web/integra-zero.js
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
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_selector("text=Load data")
        pg.click("text=Load data"); ready(pg)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); wait_for_js(pg, f"!!({XP}&&{XP}._a&&{XP}._a.sr&&{XP}._a.sr.length)")
        pg.evaluate(f"(()=>{{const p={XP};setActive(p);p.imode=null;p.intf=p._a.sr[0].key;p.el.scrollIntoView({{block:'center'}})}})()"); frames(pg, 2)
        KEY = pg.evaluate(f"{XP}._a.sr[0].key")
        pg.evaluate(f"(()=>{{const p={XP};addInt(p,p._a.sr[0],13.9,14.6)}})()"); frames(pg, 3)
        def lab(): return pg.evaluate(f"(()=>{{const p={XP},l=p._a.lbls.find(b=>b.copy!=null),q=p.cv.getBoundingClientRect();return {{x:q.left+l.x+l.w/2,y:q.top+l.y+l.h/2,copy:l.copy,sel:p.isel}}}})()")
        def clip(): return pg.evaluate("navigator.clipboard.readText()")
        def click_copies():
            l = lab(); assert l["copy"].replace(".", "").replace("E+", "").replace("E-", "").isdigit(), l
            pg.mouse.move(l["x"], l["y"]); frames(pg, 2)
            assert pg.evaluate(f"{XP}.cv.style.cursor") == "pointer"
            pg.mouse.click(l["x"], l["y"]); wait_for_js(pg, "!!document.querySelector('.areacopied.show')")
            assert clip() == l["copy"], (clip(), l)
            assert pg.inner_text(".areacopied") == "Copied!"
            assert pg.get_attribute(".areacopied", "aria-live") == "polite"
        step("click on the label copies the displayed number and shows the notice", click_copies)
        def tip_is_hint():
            t = pg.evaluate(f"{XP}._a.lbls.find(b=>b.copy!=null).tip")
            assert not t, t      # no hover text at all: the label only copies on click
            pg.mouse.move(*pg.evaluate(f"(()=>{{const b={XP}._a.lbls.find(b=>b.copy!=null),r={XP}.cv.getBoundingClientRect();return [r.left+b.x+b.w/2,r.top+b.y+b.h/2]}})()"))
            assert pg.evaluate(f"{XP}.tip.hidden"), "no hover box over the area label"
        step("the area label has no hover text", tip_is_hint)
        def fades():
            # the click of click_copies was just made: still visible at ~1 s (a 0.5 s timer fails here), gone by ~2 s
            assert pg.evaluate("document.querySelector('.areacopied').getAttribute('role')") == "status" and pg.evaluate("document.querySelector('.areacopied').getAttribute('aria-live')") == "polite"
            pg.evaluate("new Promise(r => setTimeout(r, 700))")
            assert pg.evaluate("document.querySelector('.areacopied').classList.contains('show')"), "still visible about 1 s after the click"
            wait_for_js(pg, "!document.querySelector('.areacopied.show')", 1300)      # 1.2 s visible, then a 150 ms fade
        step("the notice (role=status) is visible at ~1 s and gone by itself by ~2 s", fades)
        def restarts():
            l = lab(); pg.mouse.click(l["x"], l["y"]); wait_for_js(pg, "!!document.querySelector('.areacopied.show')"); pg.evaluate("new Promise(r => setTimeout(r, 900))")
            pg.mouse.click(l["x"], l["y"]); pg.evaluate("new Promise(r => setTimeout(r, 700))")                      # 1.6 s after the first click: still there, the second one restarted the timer
            assert pg.evaluate("document.querySelectorAll('.areacopied').length") == 1 and pg.evaluate("document.querySelector('.areacopied').classList.contains('show')")
            wait_for_js(pg, "!document.querySelector('.areacopied.show')", 2000)
        step("a second click restarts the timer, one notice only", restarts)
        def tremor():
            l = lab(); pg.evaluate("navigator.clipboard.writeText('x')")
            pg.mouse.move(l["x"], l["y"]); pg.mouse.down(); pg.mouse.move(l["x"] + 3, l["y"] + 2); pg.mouse.up()
            wait_for_js(pg, "!!document.querySelector('.areacopied.show')"); assert clip() == l["copy"], "a tap with a small movement copies"
            wait_for_js(pg, "!document.querySelector('.areacopied.show')", 2000)
            pg.evaluate("navigator.clipboard.writeText('x')")
            pg.mouse.move(l["x"], l["y"]); pg.mouse.down(); pg.mouse.move(l["x"] + 40, l["y"] + 30); pg.mouse.up(); frames(pg, 2)
            assert clip() == "x", "a real drag copies nothing"
        step("a tap with a tremor copies, a drag does not", tremor)
        def redrawn():
            l = lab(); pg.evaluate("navigator.clipboard.writeText('x')")
            pg.mouse.move(l["x"], l["y"]); pg.mouse.down(); pg.evaluate(f"draw({XP})"); frames(pg, 2); pg.mouse.up()      # the label objects are new when the button comes up
            wait_for_js(pg, "!!document.querySelector('.areacopied.show')"); assert clip() == l["copy"]
            wait_for_js(pg, "!document.querySelector('.areacopied.show')", 2000)
        step("a redraw between press and release still copies and says so", redrawn)
        def fallback():
            l = lab(); pg.evaluate("window._ex = []; Object.defineProperty(navigator, 'clipboard', { value: { writeText: () => Promise.reject(new Error('no focus')) }, configurable: true }); document.execCommand = c => (window._ex.push([c, document.activeElement.tagName]), true)")
            pg.mouse.click(l["x"], l["y"]); wait_for_js(pg, "!!document.querySelector('.areacopied.show')")
            assert ["copy", "TEXTAREA"] in pg.evaluate("window._ex") and pg.inner_text(".areacopied") == pg.evaluate("I18N.t('int.copy.done')") and not pg.evaluate("document.querySelector('.areacopied').classList.contains('bad')")
            assert pg.evaluate("document.querySelectorAll('textarea[aria-hidden=true][readonly]').length") == 0, "the hidden textarea is removed"
            wait_for_js(pg, "!document.querySelector('.areacopied.show')", 2000)
            back = pg.evaluate("""(async () => { const i = document.createElement('input'); document.body.appendChild(i); i.focus(); await AREACOPY.put('t', 100, 100);
                const r = [document.activeElement === i, document.querySelectorAll('textarea[aria-hidden=true][readonly]').length]; i.remove(); return r; })()""")
            assert back == [True, 0], ("the focus goes back to the previous element and the textarea is gone", back)
            wait_for_js(pg, "!document.querySelector('.areacopied.show')", 2000)
            pg.evaluate("document.execCommand = () => false"); pg.mouse.click(l["x"], l["y"]); wait_for_js(pg, "document.querySelector('.areacopied.show.bad')")
            assert pg.inner_text(".areacopied") != pg.evaluate("I18N.t('int.copy.done')"), "a short error, not the success text"
            assert pg.evaluate("document.querySelectorAll('textarea[aria-hidden=true][readonly]').length") == 0
            wait_for_js(pg, "!document.querySelector('.areacopied.show')", 2000)
            pg.evaluate("delete navigator.clipboard; delete document.execCommand")      # the real clipboard and execCommand again
        step("async API refused: execCommand fallback says Copied; both refused: a short error", fallback)
        def above_dialog():
            pg.evaluate("Object.defineProperty(navigator, 'clipboard', { value: { writeText: () => Promise.reject(new Error('refused')) }, configurable: true }); document.execCommand = () => true")
            pg.evaluate("(()=>{const d=document.createElement('dialog');d.id='tdlg';document.body.appendChild(d);d.showModal()})()")
            pg.evaluate("AREACOPY.say(200, 200, true)"); wait_for_js(pg, "document.querySelector('.areacopied.show')")
            assert pg.evaluate("document.querySelector('.areacopied').parentElement.id") == "tdlg", "inside the modal dialog, so above it"
            pg.evaluate("(async () => { await AREACOPY.put('y', 200, 200); })()"); wait_for_js(pg, "document.querySelector('.areacopied.show') && !document.querySelector('.areacopied.bad')")      # refused API, fallback ok, inside the dialog too
            assert pg.evaluate("document.querySelector('.areacopied').parentElement.id") == "tdlg"
            pg.evaluate("document.querySelector('#tdlg').close(); document.querySelector('#tdlg').remove(); delete navigator.clipboard; delete document.execCommand"); wait_for_js(pg, "!document.querySelector('.areacopied.show')", 2000)
        step("the notice moves into an open modal dialog (visible over it)", above_dialog)
        def border():
            # a press within 6 px of an integration border under the label: the click copies (no movement) and leaves the borders alone; a real drag moves the border and copies nothing
            g = pg.evaluate(f"(()=>{{const p={XP},q=p.cv.getBoundingClientRect(),l=p._a.lbls.find(b=>b.copy!=null),i=l.int,bx=p._a.X(i.a);return {{inside:bx>l.x+2&&bx<l.x+l.w-2,x:q.left+bx+2,y:q.top+l.y+l.h/2,a:i.a,b:i.b,copy:l.copy}}}})()")
            assert g["inside"], ("the integration border is under the label box", g)
            pg.evaluate("navigator.clipboard.writeText('x')")
            pg.mouse.click(g["x"], g["y"]); wait_for_js(pg, "!!document.querySelector('.areacopied.show')"); assert clip() == g["copy"], "a click on a border under the label copies"
            assert pg.evaluate(f"[{XP}.ints[0].a, {XP}.ints[0].b]") == [g["a"], g["b"]], "and does not move the border"
            wait_for_js(pg, "!document.querySelector('.areacopied.show')", 2000)
            pg.evaluate("navigator.clipboard.writeText('x')")
            pg.mouse.move(g["x"], g["y"]); pg.mouse.down(); pg.mouse.move(g["x"] - 30, g["y"], steps=5); pg.mouse.up(); frames(pg, 2)
            assert clip() == "x" and pg.evaluate(f"{XP}.ints[0].a") != g["a"], "a drag from the border moves it and copies nothing"
            pg.evaluate(f"(()=>{{const p={XP};p.ints[0].a={g['a']};p.ints[0].b={g['b']};draw(p)}})()"); frames(pg, 2)
        step("a press within 6 px of a border: click copies, drag moves the border and does not copy", border)
        def elsewhere():
            pg.evaluate("navigator.clipboard.writeText('x')"); n0 = pg.evaluate(f"{XP}.ints.length")
            c = pg.evaluate(f"(()=>{{const p={XP},q=p.cv.getBoundingClientRect(),a=p._a;return {{x:q.left+a.X(15.5),y:q.top+a.H-M.b-6}}}})()")
            pg.mouse.click(c["x"], c["y"]); frames(pg, 2)
            assert clip() == "x" and pg.evaluate(f"{XP}.ints.length") == n0
        step("a click elsewhere does not copy", elsewhere)
        def menu_copies():
            l = lab(); pg.evaluate("navigator.clipboard.writeText('x')")
            names = pg.evaluate(f"(()=>{{const p={XP};return AZIONI.per({{vista:p.type,p,x:14.2,k:0,near:null,px:{l['x']}-p.cv.getBoundingClientRect().left,py:{l['y']}-p.cv.getBoundingClientRect().top}}).map(i=>i.label||i.nome)}})()")
            assert "Copy value" in [str(x) for x in names], names
            pg.mouse.click(l["x"], l["y"], button="right"); wait_for_js(pg, "[...document.querySelectorAll('#ctx *')].some(e=>e.textContent==='Copy value')")
            pg.get_by_text("Copy value", exact=True).click(); wait_for_js(pg, "!!document.querySelector('.areacopied.show')")
            assert clip() == l["copy"]
        step("context menu: «Copy value» does the same", menu_copies)
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
