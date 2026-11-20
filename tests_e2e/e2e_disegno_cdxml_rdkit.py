"""Disegno: CDXML written by RDKit (tests/dati/cdxml/, made by tools/cdxml_fixtures.py) opens in Ketcher by the drop, the Open dialog and the paste, and Ketcher's InChI, atom, bond, fragment, charge, isotope and radical counts equal what RDKit itself reads from the same file; files that need a change before Indigo can read them (empty attribute, UTF-16, 8-bit text) get a notice saying what was changed. The text Ketcher reads is made by the engine (`open/1`, the molkit .wasm): run from the repo root after tools/build_site.py and
tools/build_feat_wasm.sh site/static (molkit)."""
# covers: mzlab/web/draw.js, mzlab/web/disegno-file.js, mzlab/web/disegno-cerca.js, mzlab/web/disegno-verifica.js, tests/dati/cdxml/, tools/cdxml_fixtures.py
import sys, os, json, base64, re, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
DATA = Path(__file__).resolve().parent.parent / "tests" / "dati" / "cdxml"
EXPECTED = json.loads((DATA / "expected.json").read_text(encoding="utf-8"))["files"]
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
DROP = """([b64, name]) => { const d = document.querySelector('#kframe').contentDocument, w = d.defaultView;
  const dt = new w.DataTransfer(); dt.items.add(new w.File([Uint8Array.from(atob(b64), c => c.charCodeAt(0))], name));
  d.body.dispatchEvent(new w.DragEvent('drop', {bubbles: true, cancelable: true, dataTransfer: dt})) }"""
PASTE = """(text) => { const d = document.querySelector('#kframe').contentDocument, w = d.defaultView;
  const dt = new w.DataTransfer(); dt.setData('text/plain', text);
  d.body.dispatchEvent(new w.ClipboardEvent('paste', {bubbles: true, cancelable: true, clipboardData: dt})) }"""
READ = """async () => { const k = document.querySelector('#kframe').contentWindow.ketcher, s = k.editor.struct(), RAD = {0: 0, 1: 2, 2: 1, 3: 2};
  let charge = 0, iso = 0, rad = 0; s.atoms.forEach(a => { charge += a.charge || 0; if (a.isotope || /^[DT]$/.test(a.label)) iso++; rad += RAD[a.radical || 0]; });
  const ket = JSON.parse(await k.getKet());
  return { atoms: s.atoms.size, bonds: s.bonds.size, charge, isotopes: iso, radicals: rad, fragments: Object.keys(ket).filter(n => /^mol\\d+$/.test(n)).length, smiles: await k.getSmiles(), inchi: await k.getInchi().catch(() => null) } }"""
KEEPS_LABEL = {"generic_label.cdxml"}
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300])); pg.keyboard.press("Escape")       # a dialog left open would fail the next step too
tmp = Path(tempfile.mkdtemp(prefix="mzlab_cdxml_"))
r = Run(port=8998, wd="/tmp/wd_disegno_cdxml_rdkit")
try:
    with sync_playwright() as p:
        pg = r.page(p, device_scale_factor=2); pg.set_viewport_size({"width": 1300, "height": 800})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        def clear():
            pg.evaluate(f"(() => {{ {KJ}.editor.tool('select'); }})()")                                # the Open dialog leaves Ketcher's paste tool holding the structure on the cursor; a later tool change would roll it back and wipe what a following paste added
            pg.evaluate(f"(async () => {{ await {KJ}.setMolecule(''); }})()")      # nothing returned: the page cannot serialize what Ketcher resolves with
            wait_for_js(pg, f"{KJ}.editor.struct().isBlank()", timeout=10000)      # Ketcher's promise can resolve before a canvas that held a reaction arrow is empty: a paste made at once would be wiped by the late clear
            pg.evaluate("document.getElementById('smi-warn').hidden = true")
        def settle():
            wait_for_js(pg, f"!{KJ}.editor.struct().isBlank() || !document.getElementById('smi-warn').hidden", timeout=60000)
            wait_for_js(pg, "!document.querySelector('#kframe').contentWindow.ketcher.editor.struct().isBlank()", timeout=5000)   # an unreadable file never gets here: it is a failure with its notice
            return pg.evaluate(READ), pg.inner_text("#smi-warn")
        def drop(data, name):
            clear(); pg.evaluate(DROP, [base64.b64encode(data).decode(), name]); return settle()
        def dialog(path):
            clear(); fr = pg.frame_locator("#kframe"); fr.locator('[data-testid="open-file-button"]').last.click()
            fr.locator("input[type=file]").first.set_input_files(path)
            fr.get_by_role("button", name="Add to Canvas").click(); return settle()
        def paste(text):
            clear(); pg.evaluate(PASTE, text); return settle()
        def same(got, name):
            want = EXPECTED[name]["reads"]
            for key in ("atoms", "bonds", "charge", "isotopes", "radicals", "fragments"):
                assert got[key] == want[key], f"{key}: Ketcher {got[key]}, RDKit {want[key]}"
            rx = EXPECTED[name].get("reaction")
            if rx:                                              # InChI has no reactions: the molecules on each side of the arrow are counted in Ketcher's reaction SMILES
                left, right = got["smiles"].split(" |")[0].split(">>")
                assert (len(left.split(".")), len(right.split("."))) == (rx["reactants"], rx["products"]), got["smiles"]
            elif name in KEEPS_LABEL:                           # RDKit reads a label-only node as a carbon; Ketcher keeps the word as a pseudo atom
                assert "Boc" in got["smiles"], got["smiles"]
            else:
                assert got["inchi"] == want["inchi"], f"InChI: Ketcher {got['inchi']}, RDKit {want['inchi']}"
        for name in sorted(EXPECTED):
            raw = (DATA / name).read_bytes(); empty = b'BondLength=""' in raw
            for route, fn in (("drop", lambda: drop(raw, name)), ("Open dialog", lambda: dialog(str(DATA / name))), ("paste", lambda: paste(raw.decode("utf-8")))):
                def one(fn=fn, name=name):
                    got, note = fn(); same(got, name)
                    assert ("BondLength" in note) == empty, note       # the notice names what was dropped, and only when something was
                step(f"{name}: {route}", one)
        # a file that is already clean says nothing
        def clean():
            got, note = drop((DATA.parent / "benzene_label.cdxml").read_bytes(), "benzene_label.cdxml"); assert note == "" and got["atoms"] == 7, (got, note)
        step("a clean file gives no notice", clean)
        # feeding variants of one RDKit file
        base = (DATA / "aromatics.cdxml").read_text(encoding="utf-8")
        ann = base.replace("</page>", '<t id="900" p="10 10"><s font="3" size="10">Z\u00fcrich</s></t></page>')
        decl = lambda t, enc: t.replace('encoding="UTF-8"', f'encoding="{enc}"')
        variants = {
            "UTF-8 with a byte-order mark": (b"\xef\xbb\xbf" + base.encode(), "BondLength"),
            "UTF-16 little endian": (b"\xff\xfe" + decl(base, "UTF-16").encode("utf-16-le"), "UTF-16"),
            "UTF-16 big endian": (b"\xfe\xff" + decl(base, "UTF-16").encode("utf-16-be"), "UTF-16"),
            "ISO-8859-1 text": (decl(ann, "ISO-8859-1").encode("latin-1"), "ISO-8859-1"),
            "CRLF line ends": (base.replace("\n", "\r\n").encode(), "BondLength"),
            "no XML declaration": (re.sub(r"<\?xml[^>]*\?>\s*", "", base).encode(), "BondLength"),
            "no DOCTYPE": (re.sub(r"<!DOCTYPE[^>]*>", "", base).encode(), "BondLength"),
        }
        for vname, (data, word) in variants.items():
            def var(data=data, word=word):
                got, note = drop(data, "v.cdxml"); same(got, "aromatics.cdxml"); assert word in note, note
            step(f"variant: {vname}", var)
        def pages():
            frag = re.search(r"<page.*</page>", (DATA / "stereo_wedge.cdxml").read_text(encoding="utf-8"), re.S).group(0)
            ids = iter(range(5000, 9000)); m = {}
            second = re.sub(r'(\b(?:id|B|E)=")(\d+)"', lambda mo: mo.group(1) + m.setdefault(mo.group(2), str(next(ids))) + '"', frag)
            got, note = drop(base.replace("</page>", "</page>" + second).encode(), "two.cdxml")
            assert got["atoms"] == 16 + 14 and got["fragments"] == 4, got
        step("two pages: both are drawn", pages)
        def big():
            frag = re.search(r"<fragment.*</fragment>", base, re.S).group(0); ids = iter(range(1000, 90000))
            many = "".join(re.sub(r'(\b(?:id|B|E)=")(\d+)"', lambda mo, m={}: mo.group(1) + m.setdefault(mo.group(2), str(next(ids))) + '"', frag) for _ in range(60))
            data = base.replace(frag, many).encode(); assert len(data) > 70000
            got, note = drop(data, "big.cdxml"); assert got["atoms"] == 60 * 16 and got["fragments"] == 120, got
        step("a 70 kB file with 60 pairs of molecules", big)
        # the paste guard: a single line of CDXML is drawn, any other single line that starts with "<" is a name or junk and stays out of Ketcher
        def one_line():
            got, note = paste(" ".join((DATA / "aromatics.cdxml").read_text(encoding="utf-8").split())); same(got, "aromatics.cdxml"); assert "BondLength" in note, note
        step("paste: one line of CDXML is drawn", one_line)
        def blocked():
            clear(); pg.evaluate(PASTE, "<foo bar>")
            wait_for_js(pg, "!document.getElementById('smi-warn').hidden", timeout=30000)
            assert pg.inner_text("#smi-warn") == "Compound not found and not a SMILES"
            assert pg.evaluate(f"{KJ}.editor.struct().isBlank()"), "the canvas changed"
        step("paste: a single line that starts with < and is not CDXML is blocked", blocked)
        # a file that is not a structure: the notice names it and the canvas stays as it was (the drawing already on it is the same before and after)
        KET = f"{KJ}.getKet()"
        def refused(data, name):
            def one():
                drop((DATA / "aromatics.cdxml").read_bytes(), "aromatics.cdxml")
                before = pg.evaluate(KET); pg.evaluate("document.getElementById('smi-warn').hidden = true")
                pg.evaluate(DROP, [base64.b64encode(data).decode(), name])
                wait_for_js(pg, "!document.getElementById('smi-warn').hidden", timeout=30000)
                note = pg.inner_text("#smi-warn")
                assert note.startswith("Could not read") and name in note, note
                assert pg.evaluate(KET) == before, "the canvas changed"
            return one
        good = (DATA / "aromatics.cdxml").read_text(encoding="utf-8")
        for bname, bdata in {
            "truncated XML": good[: len(good) // 2].encode(),
            "an empty file": b"",
            "text that is not XML": b"this is not a structure\nnor a drawing\n",
            "XML that is not CDXML": b'<?xml version="1.0"?><molecule><atom id="a1"/></molecule>',
        }.items():
            step(f"refused: {bname}", refused(bdata, "bad.cdxml"))
        clear(); drop((DATA / "stereo_wedge.cdxml").read_bytes(), "stereo_wedge.cdxml")
        pg.screenshot(path=os.environ.get("CDXML_SHOT", str(tmp / "cdxml_rdkit.png")))
finally:
    r.close()
shutil.rmtree(tmp, ignore_errors=True)
r.report()
for s in steps: print(" ", s)
if any(v != "ok" for _, v in steps): sys.exit(1)
