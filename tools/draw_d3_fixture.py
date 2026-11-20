"""Writes tests/dati/draw_d3.json: synthetic `mol draw --set edit=` requests (a structure and an edit patch) with the scene and the molfile the release CLI answers
(the scene without the newline the CLI prints after JSON; the site, the app and the CLI must give these bytes; read by tests/test_molkit_wasm.py and
crates/mzlab-desktop/tests/draw.rs). Build the release CLI first. Usage: python3 tools/draw_d3_fixture.py"""
import json
import sys
from pathlib import Path

from draw_fixture import ROOT, draw, molfile, picture

# (name, spec, patch); a patch is a list of operations or the object {"ops": [...]}
ALA = "N[C@@H](C)C(=O)O"
CASES = [
    ("methyl on benzene is toluene", "c1ccccc1", [{"op": "atom.add", "el": "C"}, {"op": "bond.add", "a": 0, "b": 6}]),
    ("element", "CCO", [{"op": "atom.set", "id": 2, "el": "S"}]),
    ("charge", "CCN", [{"op": "atom.set", "id": 2, "chg": 1}]),
    ("isotope", "CCO", [{"op": "atom.set", "id": 2, "iso": 18}]),
    ("radical", "CC", [{"op": "atom.set", "id": 1, "rad": 2}]),
    ("label", "c1ccccc1", [{"op": "atom.set", "id": 5, "label": "Ph"}]),
    ("label on a chain end replaces the symbol", "CCO", [{"op": "atom.set", "id": 0, "label": "OMe"}]),
    ("label on the left end runs away from the bond", "OCC", [{"op": "atom.set", "id": 0, "label": "OMe"}]),
    ("label replaces a heteroatom and its hydrogens", "CCN", [{"op": "atom.set", "id": 2, "label": "NHAc"}]),
    ("label on an added atom", "c1ccccc1", [{"op": "atom.add", "el": "C", "label": "Et"}, {"op": "bond.add", "a": 0, "b": 6}]),
    ("bond order", "CC", [{"op": "bond.set", "a": 0, "b": 1, "o": 2}]),
    ("ring closure", "CCC", [{"op": "bond.add", "a": 0, "b": 2}]),
    ("remove atom", "CCO", [{"op": "remove", "atoms": [2]}]),
    ("remove bond", "CCO", [{"op": "remove", "bonds": [[1, 2]]}]),
    ("move bond lengths", "CCO", [{"op": "move", "atoms": [2], "dx": 1, "dy": 0}]),
    ("move angstrom", "CCO", [{"op": "move", "atoms": [2], "dx_a": 0.5, "dy_a": -0.25}]),
    ("merge fragments", "C.CO", [{"op": "merge", "from": 1, "into": 0}]),
    ("merge in a chain", "CC.CC", [{"op": "merge", "from": 2, "into": 1}]),
    ("join fragments by a bond", "C.C", [{"op": "bond.add", "a": 0, "b": 1}]),
    ("atom at a position in bond lengths", "CC", [{"op": "atom.add", "id": 2, "el": "O", "x": 1.5, "y": 1}, {"op": "bond.add", "a": 1, "b": 2}]),
    ("atom at a position in angstrom", "CC", [{"op": "atom.add", "el": "N", "x_a": 2.25, "y_a": 1.3}, {"op": "bond.add", "a": 1, "b": 2}]),
    ("wedge up", ALA, [{"op": "bond.set", "a": 1, "b": 0, "wedge": 1}]),
    ("wedge down", ALA, [{"op": "bond.set", "a": 1, "b": 0, "wedge": -1}]),
    ("wedge on a charged isotope and radical atoms", "[15NH3+][C@@H](C)C[CH2]", [{"op": "atom.set", "id": 4, "rad": 2}, {"op": "bond.set", "a": 1, "b": 2, "wedge": 1}]),
    ("hydrogens given over the valence", "CCO", [{"op": "atom.set", "id": 1, "h": 3}]),
    ("stereo dropped when the centre loses a bond", ALA, [{"op": "remove", "atoms": [2]}]),
    ("double bond stereo dropped", "C/C=C/C", [{"op": "bond.set", "a": 1, "b": 2, "o": 1}]),
    ("chain built from one atom", "C", [{"op": "atom.add", "el": "C"}, {"op": "atom.add", "el": "O"}, {"op": "bond.add", "a": 0, "b": 1}, {"op": "bond.add", "a": 1, "b": 2}]),
    ("a large id", "CC", [{"op": "atom.add", "id": 100, "el": "F"}, {"op": "bond.add", "a": 0, "b": 100}]),
    ("object form", "CCO", {"ops": [{"op": "atom.set", "id": 0, "el": "N"}]}),
    ("empty patch", "CCO", []),
    ("unknown atom", "CCO", [{"op": "atom.set", "id": 0, "chg": 0}, {"op": "bond.add", "a": 1, "b": 9}]),
    ("unknown op", "CCO", [{"op": "atom.zap"}]),
    ("unknown field", "CCO", [{"op": "atom.set", "id": 0, "elem": "N"}]),
    ("units mixed", "CCO", [{"op": "atom.add", "el": "C", "x": 1, "y": 1, "x_a": 1}]),
    ("bond exists", "CCO", [{"op": "bond.add", "a": 0, "b": 1}]),
    ("bad json", "CCO", "[{"),
    ("too many operations", "CCO", [{"op": "x"}] * 1001),
    ("patch text too long", "CCO", "[" + " " * 65536),
    ("too many atoms", "C" * 2000, [{"op": "atom.add", "el": "C"}]),
    ("too many bonds", "C" * 2000, [{"op": "bond.add", "a": 0, "b": k} for k in range(2, 600)]),
    ("too many ids in a list", "CCO", [{"op": "remove", "atoms": [0] * 2001}]),
]


# edited drawings that are also written as svg, png, ket and cdxml (the label cases, charge, added atoms, merged fragments, ...)
RENDER = {
    "label", "label on a chain end replaces the symbol", "label on the left end runs away from the bond", "label replaces a heteroatom and its hydrogens",
    "label on an added atom", "methyl on benzene is toluene", "charge", "element", "bond order", "wedge up", "remove atom", "merge fragments",
    "merge in a chain", "join fragments by a bond", "atom at a position in bond lengths",
}


def main():
    cases = []

    def add(name, spec, patch, stdin=None):
        text = patch if isinstance(patch, str) else json.dumps(patch, separators=(",", ":"))
        sets = ["--set", f"edit={text}"]
        c, scene, err = draw(spec, *sets, stdin=stdin)
        c2, mol, _ = draw(spec, *sets, stdin=stdin, js=False)
        key = None
        if c:
            lines = [x for x in (scene + "\n" + err).splitlines() if x.startswith('{"schema":"error/1"')]
            key = json.loads(lines[0])["error_key"]
        case = {"name": name, "request": {"spec": stdin if stdin is not None else spec, "edit": text}, "exit": c, "error_key": key, "scene": scene.rstrip("\n") if c == 0 else None, "molfile": mol if c2 == 0 else None}
        if name in RENDER:
            assert c == 0, name
            for f in ("ket", "cdxml"):
                case[f] = draw(spec, *sets, "--format", f, stdin=stdin, js=False)[1]
            case["svg"] = picture(spec, "svg", *sets, stdin=stdin)[1]
            case["png"] = picture(spec, "png", *sets, stdin=stdin)[2]
        cases.append(case)

    for name, spec, patch in CASES:
        add(name, "smiles:" + spec, patch)
    base = molfile("CC(=O)Nc1ccc(O)cc1")
    add("molfile keeps its coordinates", "-", [{"op": "atom.add", "el": "Cl"}, {"op": "bond.add", "a": 0, "b": 11}], stdin=base)
    missing = RENDER - {c["name"] for c in cases}
    assert not missing, f"RENDER names that match no case: {sorted(missing)}"
    out = ROOT / "tests" / "dati" / "draw_d3.json"
    out.write_text(json.dumps({"cases": cases}, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{len(cases)} cases, {sum(1 for c in cases if c['exit'])} errors")


if __name__ == "__main__":
    sys.exit(main())
