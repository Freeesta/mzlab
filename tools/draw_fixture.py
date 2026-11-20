"""Writes tests/dati/draw_d1.json: 30 synthetic `mol draw` requests with the scene, the molfile, the SVG and the PNG (length and CRC-32) the release CLI answers (the scene without the newline the
CLI prints after JSON; the site, the app and the CLI must give these bytes), and `images`: picture requests with options (scale, CIP marks, highlights, the draw-image/1 wrapper). Read by
tests/test_molkit_wasm.py and crates/mzlab-desktop/tests/draw.rs. Usage: python3 tools/draw_fixture.py (MZLAB_CLI=path to use another build of the CLI)"""
import json
import os
import re
import subprocess
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CLI = Path(os.environ.get("MZLAB_CLI") or ROOT / "target" / "release" / "mzlab-cli")
CATALOG = ROOT / "mzlab" / "web" / "molecole" / "catalogo.csv"
SMILES = [
    "CCO", "C/C=C/C(=O)O", "C/C=C\\C(=O)O", "c1ccccc1", "c1ccc2ccccc2c1", "Cn1cnc2c1c(=O)n(C)c(=O)n2C", "N[C@@H](C)C(=O)O",
    "N[C@H](C)C(=O)O", "C1CCC2C(C1)CCC1C2CCC2CCCC12", "C1CCC2(CC1)CCCC2", "C1CC2CCC1C2", "C1C2CC3CC1CC(C2)C3", "C1CCCCCCCCCCC1", "[Na+].[Cl-]",
    "CC(=O)[O-].[Na+].O", "[13CH4]", "C#CC#N", "OC[C@H]1OC(O)[C@H](O)[C@@H](O)[C@@H]1O",
]
NAMES = ["caffeine", "aspirin", "caffein", "cafeine", "RZVAJINKPMORJF-UHFFFAOYSA-N"]


def draw(spec, *sets, stdin=None, js=True):
    cmd = [str(CLI), "mol", "draw", spec, *sets] + (["--json"] if js else [])
    p = subprocess.run(cmd, input=stdin, capture_output=True, encoding="utf-8")
    return p.returncode, p.stdout, p.stderr


def picture(spec, fmt, *sets, stdin=None, wrap=False):
    """(exit, text or None, png {len, crc32} or None) of the CLI for a picture."""
    cmd = [str(CLI), "mol", "draw", spec, "--format", fmt, *sets] + (["--json"] if wrap else [])
    p = subprocess.run(cmd, input=stdin.encode() if stdin is not None else None, capture_output=True)
    if p.returncode:
        return p.returncode, None, None
    if fmt == "png" and not wrap:
        return 0, None, {"len": len(p.stdout), "crc32": zlib.crc32(p.stdout)}
    text = p.stdout.decode("utf-8")
    return 0, text[:-1] if wrap else text, None


# picture requests with options: (name, request)
IMAGES = [
    ("svg highlight", {"spec": "smiles:CC(=O)Oc1ccccc1C(=O)O", "format": "svg", "highlight": "0,1,2,0-1,1-2@red;5,6@blue"}),
    ("svg scale 40 title", {"spec": "smiles:N[C@@H](C)C(=O)[O-].[Na+]", "format": "svg", "scale": "40", "name": "alanine sodium"}),
    ("svg cip off", {"spec": "smiles:C/C=C\\C(=O)O", "format": "svg", "cip": "off"}),
    ("svg wrapped", {"spec": "smiles:OC[C@H]1OC(O)[C@H](O)[C@@H](O)[C@@H]1O", "format": "svg", "json": True}),
    ("png highlight", {"spec": "smiles:Cn1cnc2c1c(=O)n(C)c(=O)n2C", "format": "png", "highlight": "0-1,1-2@green"}),
    ("png wrapped scale 16", {"spec": "smiles:CCO", "format": "png", "scale": "16", "json": True}),
    ("svg too wide", {"spec": "smiles:" + "C" * 400, "format": "svg", "json": True}),
]


def molfile(smiles):
    return draw("smiles:" + smiles, js=False)[1]


def main():
    lines = CATALOG.read_text(encoding="utf-8").splitlines()
    keep = [lines[0]] + [x for x in lines[1:] if x.split(";")[1] in ("caffeine", "acetylsalicylic acid", "acetaminophen", "ethanol")]
    db = "\n".join(keep) + "\n"
    dbfile = ROOT / "tests" / "dati" / ".draw_db.tmp"
    dbfile.write_text(db, encoding="utf-8")
    cases = []

    def add(name, spec, opts=None, stdin=None):
        opts = opts or {}
        sets = [f"{k}={v}" for k, v in opts.items() if k != "db"]
        sets = [x for s in sets for x in ("--set", s)]
        if "db" in opts:
            sets += ["--set", f"db={dbfile}"]
        c, scene, err = draw(spec, *sets, stdin=stdin)
        c2, mol, _ = draw(spec, *sets, stdin=stdin, js=False)
        req = {"spec": stdin if stdin is not None else spec, **({"db": db} if "db" in opts else {}), **{k: v for k, v in opts.items() if k != "db"}}
        key = None
        if c:
            lines = [x for x in (scene + "\n" + err).splitlines() if x.startswith('{"schema":"error/1"')]
            key = json.loads(lines[0])["error_key"]
        cases.append({"name": name, "request": req, "exit": c, "error_key": key, "scene": scene.rstrip("\n") if c == 0 else None, "molfile": mol if c2 == 0 else None,
                      "svg": picture(spec, "svg", *sets, stdin=stdin)[1] if c == 0 else None, "png": picture(spec, "png", *sets, stdin=stdin)[2] if c == 0 else None})

    for i, s in enumerate(SMILES):
        add(f"smiles {i}", "smiles:" + s)
    base = molfile("CC(=O)Nc1ccc(O)cc1")
    flat = re.sub(r"(?m)^( *-?\d+\.\d{4} +-?\d+\.\d{4}) +-?\d+\.\d{4}( [A-Z])", lambda m: "    0.0000    0.0000    0.0000" + m.group(2), base)
    shifted = re.sub(r"(?m)^ *(-?\d+\.\d{4})( +-?\d+\.\d{4} +-?\d+\.\d{4} [A-Z])", lambda m: f"{float(m.group(1)) + 10:10.4f}" + m.group(2), base)
    solid = re.sub(r"(?m)^( *-?\d+\.\d{4} +-?\d+\.\d{4}) +-?\d+\.\d{4}( [A-Z])", lambda m: m.group(1) + "    1.2500" + m.group(2), base)
    add("molfile 3d", "-", stdin=solid)
    add("molfile keep", "-", stdin=base)
    add("molfile new", "-", {"layout": "new"}, stdin=base)
    add("molfile origin", "-", stdin=flat)
    add("molfile shifted", "-", stdin=shifted)
    for k, n in enumerate(NAMES):
        add(f"name {k}", ("inchikey:" if "-" in n and n.isupper() else "name:") + n, {"db": 1})
    add("bad smiles", "smiles:C1CC")
    add("name without db", "name:caffeine")
    assert len(cases) == 30, len(cases)
    dbfile.unlink()
    out = ROOT / "tests" / "dati" / "draw_d1.json"
    images = []
    for name, r in IMAGES:
        spec = r["spec"]
        sets = [x for k in ("scale", "cip", "highlight", "name") if k in r for x in ("--set", f"{k}={r[k]}")]
        c, text, png = picture(spec, r["format"], *sets, wrap=bool(r.get("json")))
        assert c == 0, name
        images.append({"name": name, "request": r, "text": text, "png": png})
    out.write_text(json.dumps({"db": db, "cases": cases, "images": images}, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{len(cases)} cases, {sum(1 for c in cases if c['exit'])} errors")


if __name__ == "__main__":
    sys.exit(main())
