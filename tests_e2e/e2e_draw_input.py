"""Draw, text and files that reach the editor: a paste inside one of Ketcher's own fields (text tool, Open dialog) is left alone, a compound name dropped on
the canvas draws nothing (as a pasted one), an empty file gets a notice, a UTF-16 molfile is read, one «Draw» runs at a time.
Needs the built site (tools/build_site.py). Run from the repo root: python3 tests_e2e/e2e_draw_input.py"""
# covers: mzlab/web/draw.js, mzlab/web/disegno-cerca.js, mzlab/web/disegno-file.js
import sys, os, base64; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
DROP_FILE = """([b64, name]) => { const d = document.querySelector('#kframe').contentDocument, w = d.defaultView;
  const dt = new w.DataTransfer(); dt.items.add(new w.File([Uint8Array.from(atob(b64), c => c.charCodeAt(0))], name));
  d.body.dispatchEvent(new w.DragEvent('drop', {bubbles: true, cancelable: true, dataTransfer: dt})) }"""
DROP_TEXT = """(text) => { const d = document.querySelector('#kframe').contentDocument, w = d.defaultView;
  const dt = new w.DataTransfer(); dt.setData('text/plain', text);
  d.body.dispatchEvent(new w.DragEvent('drop', {bubbles: true, cancelable: true, dataTransfer: dt})) }"""
# a paste event with `text` on the element found by `sel` in the editor document: [reached the element, default prevented]
PASTE = """([sel, text]) => { const d = document.querySelector('#kframe').contentDocument, w = d.defaultView, el = d.querySelector(sel);
  if (!el) return null;
  const dt = new w.DataTransfer(); dt.setData('text/plain', text);
  const ev = new w.ClipboardEvent('paste', {clipboardData: dt, bubbles: true, cancelable: true});
  let reached = false; el.addEventListener('paste', e => { reached = true; e.preventDefault(); e.stopPropagation(); }, {once: true});
  el.dispatchEvent(ev); return [reached, ev.defaultPrevented && !reached]; }"""
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8941, wd="/tmp/wd_draw_input")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1300, "height": 800})
        pg.click("#nav [data-v=draw]")
        wait_for_js(pg, "!!(window.TPDraw && TPDraw.ready()) && !document.getElementById('kframe').classList.contains('kwait')", timeout=90000)
        fr = pg.frame_locator("#kframe"); box = pg.locator("#kframe").bounding_box()
        smi = lambda: pg.evaluate(f"{KJ}.getSmiles()")
        note = lambda: pg.evaluate("(() => { const w = document.getElementById('smi-warn'); return w.hidden ? '' : w.textContent; })()")
        def clear():
            pg.evaluate(f"{KJ}.setMolecule('')"); pg.evaluate("(() => { const w = document.getElementById('smi-warn'); w.hidden = true; w.textContent = ''; })()")
            wait_for_js(pg, f"{KJ}.editor.struct().isBlank()", timeout=15000)
        b64 = lambda raw: base64.b64encode(raw).decode()

        def text_tool():
            fr.locator('[data-testid="text"]:visible').first.click()
            pg.mouse.click(box["x"] + 500, box["y"] + 300)
            fr.locator("dialog [contenteditable=true]").first.wait_for(timeout=15000)
            got = pg.evaluate(PASTE, ["dialog [contenteditable=true]", "hello world"])
            fr.locator('dialog input[value="Cancel"]').click()
            assert got == [True, False], got
            assert note() == "", note()
        step("a paste of plain words inside the text tool reaches Ketcher's editor", text_tool)

        def open_dialog():
            fr.locator('[data-testid="open-file-button"]:visible').first.click()
            fr.locator('[data-testid="paste-from-clipboard-button"]').click()
            fr.locator("textarea:not(.cliparea)").wait_for(timeout=15000)
            got = pg.evaluate(PASTE, ["textarea:not(.cliparea)", "my notes"])
            assert got == [True, False], got
            assert note() == "", note()
            pg.keyboard.press("Escape")
        step("a paste inside the Open dialog's text box reaches it", open_dialog)

        def canvas_name():
            clear()
            got = pg.evaluate(PASTE, [".cliparea", "aspirin"])
            assert got == [False, True], got
            assert "not found" in note(), note()
        step("a name pasted on the canvas is still stopped with the note", canvas_name)

        def one_line_xml():
            clear()
            got = pg.evaluate(PASTE, [".cliparea", '<?xml version="1.0"?><CDXML><page><fragment/></page></CDXML>'])
            assert got == [True, False], got
        step("a one-line CDXML pasted on the canvas is handed to Ketcher's reader", one_line_xml)

        def dropped_name():
            clear(); pg.evaluate(DROP_TEXT, "aspirin")
            wait_for_js(pg, "!document.getElementById('smi-warn').hidden", timeout=15000)
            assert "not found" in note(), note()
            assert smi() == "", smi()
        step("a compound name dropped on the canvas draws nothing and shows the note", dropped_name)

        def dropped_name_file():
            clear(); pg.evaluate(DROP_FILE, [b64(b"aspirin\n"), "name.txt"])
            wait_for_js(pg, "!document.getElementById('smi-warn').hidden", timeout=15000)
            assert "name.txt" in note() and "Could not read" in note(), note()
            assert smi() == "", smi()
        step("a dropped file holding only a name draws nothing and is named in the notice", dropped_name_file)

        def dropped_smiles():
            clear(); pg.evaluate(DROP_TEXT, "CCO")
            wait_for_js(pg, f"!{KJ}.editor.struct().isBlank()", timeout=15000)
            assert smi() == "CCO", smi()
        step("a SMILES dropped on the canvas is still drawn", dropped_smiles)

        def dropped_lines():
            for how, text, atoms in (("text", "CC=O.OCC>>CC(=O)OCC", 12), ("file", "CC=O.OCC>>CC(=O)OCC\n", 12), ("text", "CCO ethanol", 3), ("file", "CCO ethanol\n", 3), ("text", "C[C@H](N)C(=O)O |a:1|", 6), ("file", "C[C@H](N)C(=O)O |a:1|\n", 6)):
                clear()
                if how == "text": pg.evaluate(DROP_TEXT, text)
                else: pg.evaluate(DROP_FILE, [b64(text.encode()), "line.smi"])
                wait_for_js(pg, f"!{KJ}.editor.struct().isBlank() || !document.getElementById('smi-warn').hidden", timeout=15000)
                assert note() == "", (how, text, note())
                assert pg.evaluate(f"{KJ}.editor.struct().atoms.size") == atoms, (how, text, pg.evaluate(f"{KJ}.editor.struct().atoms.size"))
        step("a dropped one-line reaction SMILES, titled .smi line or CXSMILES is drawn, as text and as a file", dropped_lines)

        def empty_file():
            for raw in (b"", b" \r\n\t\n"):
                clear(); pg.evaluate(DROP_FILE, [b64(raw), "empty.mol"])
                wait_for_js(pg, "!document.getElementById('smi-warn').hidden", timeout=15000)
                assert "empty.mol" in note() and "Could not read" in note(), note()
        step("an empty dropped file gets a notice with its name", empty_file)

        def utf16():
            clear(); pg.evaluate(f"{KJ}.setMolecule('CCO')"); wait_for_js(pg, f"!{KJ}.editor.struct().isBlank()", timeout=15000)
            mol = pg.evaluate(f"{KJ}.getMolfile()")
            for name, raw in (("le.mol", b"\xff\xfe" + mol.encode("utf-16-le")), ("be.mol", b"\xfe\xff" + mol.encode("utf-16-be")), ("bom8.mol", b"\xef\xbb\xbf" + mol.encode())):
                clear(); pg.evaluate(DROP_FILE, [b64(raw), name])
                wait_for_js(pg, f"!{KJ}.editor.struct().isBlank() || !document.getElementById('smi-warn').hidden", timeout=15000)
                assert smi() == "CCO", (name, smi(), note())
        step("a molfile saved as UTF-16 (either byte order) or UTF-8 with a byte order mark is read", utf16)

        SAVED_O = """/"label":\\s*"O"/.test(NB.ket || '')"""                 # the autosave runs once the changes have stopped
        def double_draw():
            clear(); wait_for_js(pg, "!" + SAVED_O, timeout=15000)
            pg.evaluate(f"(() => {{ const k = {KJ}, add = k.addFragment.bind(k); window.__adds = 0; window.__add0 = k.addFragment; k.addFragment = x => {{ window.__adds++; return add(x); }}; }})()")
            pg.fill("#ex-smi", "CCO")
            pg.evaluate("(() => { const b = document.getElementById('ex-load'); b.click(); b.click(); })()")
            wait_for_js(pg, SAVED_O, timeout=15000)                         # saved after the last change: a second drawing would be in by now
            assert pg.evaluate("window.__adds") == 1, pg.evaluate("window.__adds")
            assert smi() == "CCO" and pg.input_value("#ex-smi") == "", (smi(), pg.input_value("#ex-smi"))
            pg.evaluate(f"void ({KJ}.addFragment = window.__add0)")
        step("a double click on «Draw» draws the structure once", double_draw)

        def pick_while_drawing():
            clear()
            pg.evaluate(f"""(() => {{ const k = {KJ}, add = k.addFragment.bind(k); k.addFragment = async x => {{ if (x === 'CCO') {{ k.addFragment = add; const f = document.getElementById('ex-smi'); f.value = 'CCN'; document.getElementById('ex-load').click(); }} return add(x); }}; }})()""")
            pg.fill("#ex-smi", "CCO"); pg.click("#ex-load")
            wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 6 && document.getElementById('ex-smi').value === ''", timeout=15000)
        step("a text entered while a draw runs is drawn next, not lost", pick_while_drawing)

        def silent_editor():
            clear()
            pg.evaluate(f"""(() => {{ const k = {KJ}; window.__add = k.addFragment; k.addFragment = () => new Promise(() => {{}});
              const st = window.setTimeout; window.__st = st; window.setTimeout = (f, ms, ...a) => st(f, ms === 20000 ? 300 : ms, ...a); }})()""")
            pg.fill("#ex-smi", "CCO"); pg.click("#ex-load")
            wait_for_js(pg, "!document.getElementById('smi-warn').hidden", timeout=15000)
            assert "did not answer" in note(), note()
            pg.evaluate(f"(() => {{ {KJ}.addFragment = window.__add; window.setTimeout = window.__st; }})()")
            pg.click("#ex-load")                                             # the button is not left dead
            wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 3", timeout=15000)
        step("an editor that never answers gets a notice and «Draw» works again", silent_editor)

        def late_editor():
            clear()
            pg.evaluate(f"""(() => {{ const k = {KJ}, get = k.getKet.bind(k), add = k.addFragment.bind(k); window.__adds = 0; window.__late = false;
              k.addFragment = x => {{ window.__adds++; return add(x); }};
              k.getKet = () => new Promise(ok => {{ window.__answer = () => {{ k.getKet = get; get().then(ok).then(() => {{ window.__late = true; }}); }}; }});
              const st = window.setTimeout; window.__st = st; window.setTimeout = (f, ms, ...a) => st(f, ms === 20000 ? 300 : ms, ...a); }})()""")
            pg.fill("#ex-smi", "CCO"); pg.click("#ex-load")
            wait_for_js(pg, "!document.getElementById('smi-warn').hidden", timeout=15000)
            assert "did not answer" in note(), note()
            pg.evaluate("(() => { window.setTimeout = window.__st; window.__answer(); })()")          # the editor answers after the limit
            wait_for_js(pg, "window.__late", timeout=15000)
            pg.evaluate("new Promise(ok => requestAnimationFrame(() => requestAnimationFrame(ok)))")
            assert pg.evaluate("window.__adds") == 0 and smi() == "", (pg.evaluate("window.__adds"), smi())
            pg.click("#ex-load")
            wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 3 && document.getElementById('ex-smi').value === ''", timeout=15000)
            assert pg.evaluate("window.__adds") == 1, pg.evaluate("window.__adds")
        step("an editor that answers after the limit adds nothing late, and the next «Draw» draws once", late_editor)

        def element_words():
            clear(); pg.evaluate(DROP_TEXT, "Co nitrate")
            wait_for_js(pg, "!document.getElementById('smi-warn').hidden", timeout=15000)
            assert "not found" in note() and smi() == "", (note(), smi())
        step("a name whose first word is spelled like atoms draws nothing", element_words)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
if any(v != "ok" for _, v in steps): sys.exit(1)
