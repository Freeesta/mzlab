"""Undo of the actions of the Dati view, one step per action of the registry (comandi.js): the ruler of the spectrum (clicks, button, Esc), the keys
with the focus on a button, a checkbox or a select, redo, the state of the buttons. Steps in TODO are known gaps of the undo not ported yet: they must
FAIL today and the test fails as soon as one passes (then it moves out of TODO), so the list only shrinks."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
TODO: set[str] = set()
def step(name, fn):
    todo = name in TODO
    try:
        fn()
        steps.append((name, "FAIL: passes but is listed in TODO" if todo else "ok"))
    except Exception as e:
        if todo: steps.append((name, "ok (known gap)")); return
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_annulla_azioni.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
r = Run(port=8885, wd="/tmp/wd85")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        tick = lambda: pg.evaluate("new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>setTimeout(r,0))))")
        stage(pg, [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        pg.evaluate(f"{SPEC}.el.scrollIntoView({{block:'center'}})"); tick()
        mod = "Meta" if pg.evaluate("/Mac/.test(navigator.platform)") else "Control"
        def peak_x(m):
            return pg.evaluate(f"(()=>{{const p={SPEC},q=p.cv.getBoundingClientRect(),a=p._a,d=a.data[0].d;let b=0;d.mz.forEach((v,i)=>{{if(Math.abs(v-{m})<0.6&&d.y[i]>d.y[b]||Math.abs(d.mz[b]-{m})>=0.6)b=i}});return {{x:q.left+a.X(d.mz[b]),y:q.top+a.Y(d.y[b])}}}})()")
        def click_peak(m):
            a = peak_x(m); pg.mouse.click(a["x"], a["y"] + 3); tick()
        meas = lambda: pg.evaluate(f"(()=>{{const m={SPEC}.meas;return m?{{ref:m.ref,n:m.list.length}}:null}})()")
        rul = lambda: pg.evaluate(f"!!{SPEC}.rul")
        def undo(via="key"):
            if via == "key": pg.keyboard.press(f"{mod}+z")
            else: pg.click("#np-undo")
            tick()
        def redo(via="key"):
            if via == "key": pg.keyboard.press(f"{mod}+Shift+z")
            else: pg.click("#np-redo")
            tick()
        def reset():
            pg.evaluate(f"(()=>{{const p={SPEC};p.meas=null;p.rul=false;p.el.querySelector('[data-a=rul]').classList.remove('on');STORIA.svuota();draw(p)}})()")
            pg.evaluate("document.activeElement&&document.activeElement.blur()")
        def ruler_keys():
            reset()
            pg.click(".pnl.spec [data-a=rul]"); click_peak(364.4); click_peak(152.2)
            assert rul() and meas()["ref"] is not None and meas()["n"] == 1
            undo(); assert meas()["n"] == 0 and meas()["ref"] is not None, "Ctrl+Z takes back the measure, keeps the reference"
            undo(); assert meas() is None, "Ctrl+Z takes back the reference"
            undo(); assert not rul() and not pg.evaluate(f"{SPEC}.el.querySelector('[data-a=rul]').classList.contains('on')"), "Ctrl+Z switches the ruler off"
            redo(); assert rul(), "redo: the ruler is on again"
            redo(); assert meas()["ref"] is not None and meas()["n"] == 0, "redo: the reference"
            redo(); assert meas()["n"] == 1, "redo: the measure"
        step("ruler: clicks and button toggle, undo and redo with the keys", ruler_keys)
        def ruler_button():
            reset()
            assert pg.evaluate("document.getElementById('np-undo').disabled") is True
            pg.click(".pnl.spec [data-a=rul]"); click_peak(364.4); click_peak(152.2)
            assert pg.evaluate("document.getElementById('np-undo').disabled") is False, "the button is enabled after a ruler action"
            undo("button"); assert meas()["n"] == 0
            assert pg.evaluate("document.getElementById('np-redo').disabled") is False, "the redo button is enabled after an undo"
            redo("button"); assert meas()["n"] == 1
            for _ in range(3): undo("button")
            assert not rul() and pg.evaluate("document.getElementById('np-undo').disabled") is True, "stack empty: the undo button is disabled"
        step("ruler: undo and redo buttons, enabled state", ruler_button)
        def ruler_esc():
            reset()
            pg.click(".pnl.spec [data-a=rul]"); click_peak(364.4); click_peak(152.2)
            pg.keyboard.press("Escape"); tick()
            assert meas() is None and not rul(), "Esc clears the ruler"
            undo(); assert meas() and meas()["n"] == 1 and rul(), "Ctrl+Z puts the whole ruler back"
            redo(); assert meas() is None
        step("ruler: Esc is undoable in one step", ruler_esc)
        def ruler_zoom():
            reset()
            pg.click(".pnl.spec [data-a=rul]"); click_peak(364.4); click_peak(152.2)
            pg.evaluate(f"(()=>{{const p={SPEC};pushZh(p);p.zoom=[100,400];draw(p)}})()"); tick()
            assert meas() is None, "a new zoom cancels the measure"
            undo(); assert pg.evaluate(f"{SPEC}.zoom") is None and meas() and meas()["n"] == 1, "undoing the zoom brings the measure of that view back"
        step("ruler: undo of a zoom brings back the measure it cancelled", ruler_zoom)
        def focus_controls():
            reset()
            pg.click(".pnl.spec [data-a=rul]"); click_peak(364.4)
            for html in ["<input type=checkbox id=tmpc>", "<input type=range id=tmpc>", "<select id=tmpc><option>a</select>", "<button id=tmpc>b</button>"]:
                pg.evaluate("h=>{document.getElementById('tmpc')?.remove();document.body.insertAdjacentHTML('beforeend',h);document.getElementById('tmpc').focus()}", html)
                pg.keyboard.press(f"{mod}+z"); tick()
                assert meas() is None, f"Ctrl+Z with the focus on {html} undoes the reference"
                redo()
            pg.evaluate("document.getElementById('tmpc').remove()")
        step("keys: Ctrl+Z works with the focus on a checkbox, a range, a select, a button", focus_controls)
        def text_field():
            reset()
            pg.click(".pnl.spec [data-a=rul]"); click_peak(364.4)
            pg.evaluate("(()=>{const i=document.createElement('input');i.id='tmpt';document.body.appendChild(i);i.focus()})()")
            pg.keyboard.press(f"{mod}+z"); tick()
            assert meas() is not None, "inside a text field the browser keeps Ctrl+Z"
            pg.evaluate("document.getElementById('tmpt').remove()")
        step("keys: Ctrl+Z in a text field stays the browser's", text_field)
        def registro_e_raffica():
            reset()
            assert pg.evaluate("COMANDI.senzaGestore()") == [], "every command of the table has its handler"
            pg.evaluate(f"(()=>{{const p={SPEC};COMANDI.fai('spec.ruler',p,()=>{{p.rul=true}},{{unisci:'g'}});COMANDI.fai('spec.ruler',p,()=>{{p.rul=false;p.meas={{ref:1,list:[],key:measKey(p)}}}},{{unisci:'g'}})}})()")
            assert pg.evaluate("STORIA.pila.v.length") == 1, "steps of one gesture (same unisci, less than 0.8 s) are ONE entry"
            undo(); assert meas() is None and not rul(), "one Ctrl+Z takes the whole gesture back"
            assert pg.evaluate("(()=>{try{COMANDI.fai('nope.x',null,()=>1);return 'no'}catch(e){return 'throws'}})()") == "throws", "an action outside the registry is refused"
        step("registry: handlers present, a gesture is one step, unknown id refused", registro_e_raffica)
        def pan_drag():
            reset()
            pg.evaluate(f"(()=>{{const p={SPEC};pushZh(p);p.zoom=[100,400];draw(p)}})()"); tick()
            n0 = pg.evaluate("STORIA.pila.v.length"); box = pg.evaluate(f"(()=>{{const q={SPEC}.cv.getBoundingClientRect();return {{x:q.left+q.width/2,y:q.top+q.height/2}}}})()")
            pg.keyboard.down("Shift"); pg.mouse.move(box["x"], box["y"]); pg.mouse.down(); pg.mouse.move(box["x"] + 60, box["y"], steps=5); pg.mouse.move(box["x"] + 120, box["y"], steps=5); pg.mouse.up(); pg.keyboard.up("Shift"); tick()
            assert pg.evaluate(f"{SPEC}.zoom") != [100, 400], "the drag moved the view"
            assert pg.evaluate("STORIA.pila.v.length") == n0 + 1, "a whole pan drag is ONE step"
            undo(); assert pg.evaluate(f"{SPEC}.zoom") == [100, 400], "Ctrl+Z puts the view before the pan back"
        step("view: a pan drag is one undoable step", pan_drag)
        def annotazioni():
            reset()
            pg.evaluate(f"(()=>{{const p={SPEC};p.anns=p.anns||[];COMANDI.fai('spec.annotation',p,()=>p.anns.push({{x:364.4,text:'a'}}));draw(p)}})()")
            assert pg.evaluate(f"{SPEC}.anns.length") == 1
            undo(); assert pg.evaluate(f"{SPEC}.anns.length") == 0, "Ctrl+Z takes the note back"
            redo(); assert pg.evaluate(f"{SPEC}.anns.length") == 1, "redo puts it back"
        step("annotation: add, undo, redo", annotazioni)
        def opzioni():
            reset()
            old = pg.evaluate(f"{SPEC}.thr")
            pg.evaluate(f"(()=>{{const p={SPEC};COMANDI.fai('spec.display',p,()=>{{p.thr=({{}}.x??{old}) + 7}});draw(p)}})()")
            undo(); assert pg.evaluate(f"{SPEC}.thr") == old, "Ctrl+Z restores the display option"
            redo(); assert pg.evaluate(f"{SPEC}.thr") == old + 7
        step("display options: change, undo, redo", opzioni)
        def sorgente():
            reset()
            pg.evaluate(f"(()=>{{const p={SPEC};COMANDI.fai('panel.source',p,()=>{{p.log=!p.log}});draw(p)}})()")
            was = pg.evaluate(f"!!{SPEC}.log")
            undo(); assert pg.evaluate(f"!!{SPEC}.log") != was, "Ctrl+Z restores the panel choice"
        step("panel source: choice, undo", sorgente)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1].startswith("FAIL") for s in steps): sys.exit(1)
