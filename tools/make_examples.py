"""Builds the two example drawings of the Disegno tab WITH the program itself (Ketcher + the automatic labels + the PNG export of the tab).

    python3 tools/make_examples.py            # writes mzlab/web/esempio-trasformazione.png and esempio-frammentazione.png

Why a script: the pictures must be exactly what a student can export, so they are never drawn by hand. The structures are loaded into Ketcher as
SMILES, placed on one canvas (.ket built in the page), the text objects above the structures are the ones a student writes with Ketcher's text
tool, and the export is the page's own `TPDraw.image("png")`.
The drug is paracetamol (a common drug, not one of the pollutants of the course). Its masses are checked against /api/formula (server side
element table) before drawing: nothing is typed by memory.
"""
import base64, json, os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tests_e2e"))
from lib import *

# (key, SMILES, centre x, centre y) in Ketcher units (bond length 1, y up); text above = (line with "m/z" in italics)
SCHEME_T = {
    "mols": [
        ("P", "CC(=O)Nc1ccc(O)cc1", 0.0, 0.0),
        ("TP1", "CC(=O)Nc1ccc(O)c(O)c1", 15.0, 2.4),
        ("TP2", "CC(=O)Nc1cc(O)c(O)cc1O", 30.0, 2.4),
        ("TP3", "Nc1ccc(O)cc1", 24.0, -3.4),
    ],
    # arrows: from key -> to key
    "arrows": [("P", "TP1"), ("TP1", "TP2"), ("P", "TP3")],
    "ion": "",
}
SCHEME_F = {
    "mols": [
        ("F152", "CC(=[OH+])Nc1ccc(O)cc1", 0.0, 0.0),
        ("F110", "Oc1ccc([NH3+])cc1", 11.0, 0.0),
        ("F93", "Oc1cc[c+]cc1", 20.0, 0.0),
        ("F65", "[CH+]1C=CC=C1", 28.0, 0.0),
    ],
    "arrows": [("F152", "F110"), ("F110", "F93"), ("F93", "F65")],
    "ion": "",
}

BUILD = r"""
async ({scheme, texts}) => {
  const K = document.getElementById('kframe').contentWindow.ketcher;
  const box = {}, kets = {};
  for (const [key, smi, cx, cy] of scheme.mols) {
    await K.setMolecule(smi);
    const j = JSON.parse(await K.getKet()), m = j.mol0;
    const xs = m.atoms.map(a => a.location[0]), ys = m.atoms.map(a => a.location[1]);
    const mx = (Math.min(...xs) + Math.max(...xs)) / 2, my = (Math.min(...ys) + Math.max(...ys)) / 2;
    m.atoms.forEach(a => { a.location = [a.location[0] - mx + cx, a.location[1] - my + cy, 0]; });
    const X = m.atoms.map(a => a.location[0]), Y = m.atoms.map(a => a.location[1]);
    box[key] = { x0: Math.min(...X), x1: Math.max(...X), y0: Math.min(...Y), y1: Math.max(...Y), cx, cy };
    kets[key] = m;
  }
  const out = { root: { nodes: [] } }; let n = 0;
  for (const [key] of scheme.mols) { out["mol" + n] = kets[key]; out.root.nodes.push({ "$ref": "mol" + n }); n++; }
  for (const [a, b] of scheme.arrows) {            // arrow between the facing sides of two structures, on the line joining their centres
    const A = box[a], B = box[b], dx = B.cx - A.cx, dy = B.cy - A.cy, L = Math.hypot(dx, dy), ux = dx / L, uy = dy / L;
    const pad = (S, sgn) => { const hw = (S.x1 - S.x0) / 2 + 0.9, hh = (S.y1 - S.y0) / 2 + 0.9; return Math.min(hw / Math.abs(ux || 1e-9), hh / Math.abs(uy || 1e-9)) ; };
    const s = pad(A), e = pad(B);
    out.root.nodes.push({ type: "arrow", data: { mode: "open-angle", pos: [{ x: A.cx + ux * s, y: A.cy + uy * s, z: 0 }, { x: B.cx - ux * e, y: B.cy - uy * e, z: 0 }] } });
  }
  for (const t of texts) {                          // text objects as the student writes them: "TP1 m/z 168", m/z in italics
    const S = box[t.key], full = t.parts.map(p => p[0]).join(""), px = t.px || 18;
    const ranges = [{ offset: 0, length: full.length, style: "CUSTOM_FONT_SIZE_" + px + "px" }]; let off = 0;
    for (const [s, st] of t.parts) { if (st) ranges.push({ offset: off, length: s.length, style: st }); off += s.length; }
    const w = full.length * px / 40 * 0.52;         // width in Ketcher units (40 px per unit)
    const block = { key: "t" + Math.random().toString(36).slice(2, 7), text: full, type: "unstyled", depth: 0, inlineStyleRanges: ranges, entityRanges: [], data: {} };
    out.root.nodes.push({ type: "text", data: { content: JSON.stringify({ blocks: [block], entityMap: {} }), position: { x: S.cx - w / 2, y: S.y1 + (t.up || 1.6), z: 0 } } });
  }
  await K.setMolecule(JSON.stringify(out));
  return box;
}
"""

def png_b64(pg):
    return pg.evaluate("""async () => { const b = await TPDraw.image('png'); const buf = new Uint8Array(await b.arrayBuffer()); let s = ''; for (let i = 0; i < buf.length; i += 8192) s += String.fromCharCode(...buf.subarray(i, i + 8192)); return btoa(s); }""")

def main():
    import urllib.request
    r = Run(port=8822, wd="/tmp/wdF_ex")
    web = ROOT / "mzlab" / "web"
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            # masses from the server's own element table (mzlab/chem/elements.py), not from memory
            def fm(f, ad):
                return json.load(urllib.request.urlopen(f"http://127.0.0.1:8822/api/formula?f={f}&adduct={urllib.parse.quote(ad)}"))
            import urllib.parse
            for f in ["C8H9NO2", "C8H9NO3", "C8H9NO4", "C6H7NO"]:
                x = fm(f, "[M+H]+"); print(f, "M", round(x["neutral"], 4), "[M+H]+", round(x["mz"], 4), "unit", x["nominal"])
            pg.evaluate("setView('draw')"); pg.wait_for_function("window.TPDraw && TPDraw.ready()", timeout=60000); pg.wait_for_timeout(1500)
            pg.check("#lb-f"); pg.check("#lb-m"); pg.select_option("#lb-dec", "0")
            I = lambda s: [s, "ITALIC"]
            # 1) transformation scheme: molecules; text above = name or "TP<n> m/z <ion chosen>"
            tx = [{"key": "P", "parts": [["Paracetamolo", "BOLD"]]},
                  {"key": "TP1", "parts": [["TP1", "BOLD"], [" ", ""], ["m/z", "ITALIC"], [" 168", ""]]},
                  {"key": "TP2", "parts": [["TP2", "BOLD"], [" ", ""], ["m/z", "ITALIC"], [" 184", ""]]},
                  {"key": "TP3", "parts": [["TP3", "BOLD"], [" ", ""], ["m/z", "ITALIC"], [" 110", ""]]}]
            pg.evaluate(BUILD, {"scheme": SCHEME_T, "texts": tx}); pg.wait_for_timeout(1800)
            pg.screenshot(path=SH + "F_ex_T_canvas.png")
            LBL = "[...document.getElementById('kframe').contentWindow.document.querySelectorAll('#qqq-labels text')].map(t=>t.textContent)"
            print("labels T:", pg.evaluate(LBL))
            open(web / "esempio-trasformazione.png", "wb").write(base64.b64decode(png_b64(pg)))
            # 2) fragmentation scheme: all ions drawn with their charge
            pg.evaluate(BUILD, {"scheme": SCHEME_F, "texts": []}); pg.wait_for_timeout(1800)
            pg.screenshot(path=SH + "F_ex_F_canvas.png")
            print("labels F:", pg.evaluate(LBL))
            open(web / "esempio-frammentazione.png", "wb").write(base64.b64decode(png_b64(pg)))
    finally:
        r.close()
    r.report()

if __name__ == "__main__":
    main()
