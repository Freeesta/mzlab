"""Disegno: ChemDraw-format files (CDXML text, binary CDX) open through Ketcher's own reader by every route: the Open dialog, a drop on the canvas and pasted text; an unreadable file gets a notice."""
# covers: mzlab/web/draw.js, mzlab/web/disegno-file.js
import sys, os, base64; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
FIXTURE = Path(__file__).resolve().parent.parent / "tests" / "dati" / "benzene_label.cdxml"      # synthetic: phenol drawn by hand
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
DROP = """([b64, name]) => { const d = document.querySelector('#kframe').contentDocument, w = d.defaultView;
  const dt = new w.DataTransfer(); dt.items.add(new w.File([Uint8Array.from(atob(b64), c => c.charCodeAt(0))], name));
  d.body.dispatchEvent(new w.DragEvent('drop', {bubbles: true, cancelable: true, dataTransfer: dt})) }"""
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=8999, wd="/tmp/wd_disegno_cdxml")
try:
    with sync_playwright() as p:
        pg = r.page(p, device_scale_factor=2); pg.set_viewport_size({"width": 1300, "height": 800})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        txt, raw = FIXTURE.read_text(), FIXTURE.read_bytes()
        clear = lambda: pg.evaluate(f"{KJ}.setMolecule('')")
        smi = lambda: pg.evaluate(f"{KJ}.getSmiles()")
        def phenol():
            wait_for_js(pg, f"!{KJ}.editor.struct().isBlank()", timeout=15000)          # sync check: asking Ketcher for SMILES while its reader is busy can stall
            assert smi() == "C1(O)C=CC=CC=1", smi()
        def pasted():
            clear(); pg.evaluate(f"{KJ}.addFragment({txt!r})"); phenol()
        step("pasted CDXML text draws phenol", pasted)
        def dropped():
            clear(); pg.evaluate(DROP, [base64.b64encode(raw).decode(), "phenol.cdxml"]); phenol()
        step("a dropped .cdxml draws phenol", dropped)
        cdx = base64.b64decode(pg.evaluate(f"{KJ}.getCDX()"))
        def dropped_cdx():
            assert cdx[:4] == b"VjCD", cdx[:8]
            clear(); pg.evaluate(DROP, [base64.b64encode(cdx).decode(), "phenol.cdx"]); phenol()
        step("a dropped binary .cdx draws phenol", dropped_cdx)
        def chooser(path):
            clear(); fr = pg.frame_locator("#kframe"); fr.locator('[data-testid="open-file-button"]').last.click()
            fr.locator("input[type=file]").first.set_input_files(path); pg.evaluate("new Promise(r => setTimeout(r, 100))")
            fr.get_by_role("button", name="Add to Canvas").click(); phenol()
        step("Ketcher's Open dialog opens the .cdxml", lambda: chooser(str(FIXTURE)))
        Path("/tmp/wd_disegno_cdxml.cdx").write_bytes(cdx)
        step("Ketcher's Open dialog opens the .cdx", lambda: chooser("/tmp/wd_disegno_cdxml.cdx"))
        def unreadable():
            clear(); pg.evaluate(DROP, [base64.b64encode(b"<CDXML><nothing/></CDXML>").decode(), "empty.cdxml"]); wait_for_js(pg, "!document.getElementById('smi-warn').hidden", timeout=15000)
            t = pg.inner_text("#smi-warn"); assert "empty.cdxml" in t and "Could not read" in t, t
            assert smi() == "", smi()
        step("an unreadable file shows a notice with its name and draws nothing", unreadable)
        pg.evaluate(f"{KJ}.setMolecule('')"); pg.evaluate("document.getElementById('smi-warn').hidden = true")
        pg.evaluate(DROP, [base64.b64encode(raw).decode(), "phenol.cdxml"]); phenol()
        pg.screenshot(path=os.environ.get("CDXML_SHOT", "/tmp/wd_disegno_cdxml.png"))
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
if any(v != "ok" for _, v in steps): sys.exit(1)
