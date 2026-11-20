#!/usr/bin/env python3
"""Writes the synthetic CDXML fixtures of tests/dati/cdxml/ with RDKit and the file expected.json (what RDKit itself reads back from each file).

Dev tool, never run by the tests: the e2e test `e2e_disegno_cdxml_rdkit` only reads the committed files. Needs `pip install rdkit` (2024.09+, built with
the ChemDraw parser). Molecules are textbook ones, no real data. The RDKit writer has no text labels and no reaction output, so the reaction scheme and the
abbreviation labels are composed here from the CDXML element names (arrow, scheme/step, t/s, fragment nodes) around RDKit's own drawing: the notes in
expected.json say which. Rewrite with `python3 tools/cdxml_fixtures.py` (prints what it wrote).
"""
import itertools, json, re, sys
from pathlib import Path
import xml.etree.ElementTree as ET
from rdkit import Chem, rdBase
from rdkit.Chem import AllChem

OUT = Path(__file__).resolve().parent.parent / "tests" / "dati" / "cdxml"
HEAD = '<?xml version="1.0" encoding="UTF-8" ?>\n<!DOCTYPE CDXML SYSTEM "https://static.chemistry.revvitycloud.com/cdxml/CDXML.dtd" >\n'

# name -> (SMILES of the source, what it tests)
SIMPLE = {
    "aromatics": ("Cc1ccncc1.c1ccc2[nH]ccc2c1", "aromatic rings written as alternating double bonds, a heteroaromatic ring and a fused pair"),
    "stereo_wedge": ("C[C@H](N)C(=O)O.C[C@H]1CCCC[C@@H]1O", "wedge and hash bonds on chain and ring stereocentres"),
    "stereo_ez": ("C/C=C/C(=O)O.C/C=C\\C(=O)O", "E and Z double bonds from coordinates only"),
    "charges": ("C[N+](C)(C)C.CC(=O)[O-].[NH3+]CC(=O)[O-]", "positive, negative and zwitterionic atoms"),
    "isotopes": ("[13CH3]O.[2H]C(Cl)Cl.[18F]c1ccccc1", "isotope attributes on carbon, hydrogen and fluorine"),
    "radicals": ("C[CH2].[CH3].CC(C)(C)[O]", "singlet radical atoms"),
    "rings": ("C1CCCCC1.C1CCC2(CC1)CCCC2.C1CC2CCC1C2.C1CCCCCCCCCCC1", "monocycle, spiro, bridged and a twelve-membered ring"),
    "fragments": ("CC(=O)[O-].[Na+].c1ccccc1.CCO", "four separate fragments on one page"),
    "explicit_h": ("[H]C([H])([H])O[H]", "hydrogens drawn as atoms"),
}


def mol_of(smiles):
    m = Chem.MolFromSmiles(smiles)
    AllChem.Compute2DCoords(m)
    return m


def spread(m):
    """Moves each fragment to its own place on the page so none overlaps (RDKit lays all of them out around the origin)."""
    frags = Chem.GetMolFrags(m)
    conf = m.GetConformer()
    x = 0.0
    for f in frags:
        xs = [conf.GetAtomPosition(i).x for i in f]
        dx = x - min(xs)
        for i in f:
            p = conf.GetAtomPosition(i); conf.SetAtomPosition(i, (p.x + dx, p.y, 0))
        x = max(xs) + dx + 2.0
    return m


def write(name, block):
    (OUT / f"{name}.cdxml").write_text(block, encoding="utf-8", newline="\n")


def reading(path):
    """What RDKit reads from the file: fragments, atoms, bonds, charge, radical electrons, InChI of everything on the page."""
    mols = list(Chem.MolsFromCDXMLFile(str(path), sanitize=False, removeHs=False))
    out = {"fragments": sum(len(Chem.GetMolFrags(m)) for m in mols), "atoms": 0, "bonds": 0, "charge": 0, "radicals": 0, "isotopes": 0, "inchi": None}
    ok = []
    for m in mols:
        out["atoms"] += m.GetNumAtoms(); out["bonds"] += m.GetNumBonds()
        out["charge"] += sum(a.GetFormalCharge() for a in m.GetAtoms())
        out["radicals"] += sum(a.GetNumRadicalElectrons() for a in m.GetAtoms())
        out["isotopes"] += sum(1 for a in m.GetAtoms() if a.GetIsotope())
        try:
            Chem.SanitizeMol(m); Chem.AssignStereochemistry(m, cleanIt=True, force=True); ok.append(m)
        except Exception:
            pass
    if mols and len(ok) == len(mols):
        c = ok[0]
        for m in ok[1:]: c = Chem.CombineMols(c, m)
        out["inchi"] = Chem.MolToInchi(c)
    return out


def source_inchi(smiles):
    p = Chem.SmilesParserParams(); p.removeHs = False
    return Chem.MolToInchi(Chem.MolFromSmiles(smiles, p))


# --- composed files: RDKit has no writer for these ---------------------------------------------------------------------------------------------
PRE = '<fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable><colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>'


def doc(body, extra=""):
    return HEAD + f'<CDXML CreationProgram="synthetic fixture" LabelFont="3" LabelSize="10">{PRE}<page id="1">{body}</page></CDXML>\n'


def lbl(i, x, y, text):
    return f'<t id="{i}" p="{x} {y}" LabelJustification="Left"><s font="3" size="10">{text}</s></t>'


def reaction():
    """Acetic acid + ethanol -> ethyl acetate: RDKit draws the three molecules, the arrow and the step are composed here."""
    cnt = itertools.count(10)
    frags, ids = [], []
    for smi, dx in [("CC(=O)O", 0), ("CCO", 90), ("CC(=O)OCC", 330)]:
        m = mol_of(smi)
        frag = re.search(r"<fragment.*?</fragment>", Chem.MolToCDXMLBlock(m), re.S).group(0)
        remap = {}
        frag = re.sub(r'(\b(?:id|B|E)=")(\d+)"', lambda mo: mo.group(1) + remap.setdefault(mo.group(2), str(next(cnt))) + '"', frag)
        frag = re.sub(r'\bp="(-?[\d.]+) (-?[\d.]+)"', lambda mo: f'p="{float(mo.group(1)) + 100 + dx:.2f} {float(mo.group(2)) + 100:.2f}"', frag)
        frags.append(frag); ids.append(re.search(r'id="(\d+)"', frag).group(1))
    arrow = next(cnt); step = next(cnt)
    body = "".join(frags) + (f'<arrow id="{arrow}" FillType="None" ArrowheadHead="Full" ArrowheadType="Solid" HeadSize="1000" ArrowheadCenterSize="875" ArrowheadWidth="250"'
                              f' Head3D="320 100 0" Tail3D="245 100 0"/>' + lbl(next(cnt), 270, 90, "H+") +
                              f'<scheme id="{next(cnt)}"><step id="{step}" ReactionStepReactants="{ids[0]} {ids[1]}" ReactionStepProducts="{ids[2]}" ReactionStepArrows="{arrow}"/></scheme>')
    return doc(body)


def atom(i, x, y, el=6, h=None, label=None):
    attrs = f'id="{i}" p="{x} {y}" Element="{el}"' + (f' NumHydrogens="{h}"' if h is not None else "")
    return f"<n {attrs}>{lbl(i + 500, x - 4, y + 3, label)}</n>" if label else f"<n {attrs}/>"


def bond(i, b, e, order=1):
    return f'<b id="{i}" B="{b}" E="{e}"' + (f' Order="{order}"' if order != 1 else "") + "/>"


def group(i, x, y, text, atoms, bonds_):
    """A node that holds its own fragment (the abbreviation expanded) and an external connection point at its last atom; `atoms` are (element, dx, dy)."""
    inner = "".join(atom(i + 10 + k, x + dx, y + dy, el) for k, (el, dx, dy) in enumerate(atoms))
    inner += f'<n id="{i + 90}" p="{x - 14} {y}" NodeType="ExternalConnectionPoint"/>' + "".join(bond(i + 50 + k, i + 10 + a, i + 10 + c if c < 90 else i + 90, o) for k, (a, c, o) in enumerate(bonds_))
    return f'<n id="{i}" p="{x} {y}" NodeType="Fragment"><fragment id="{i + 1}">{inner}</fragment>{lbl(i + 2, x - 4, y + 3, text)}</n>'


def labels():
    """Ways a drawing names a group: ordinary element labels (OH, NH, NH2) and the expanded forms ChemDraw writes for Ph and OMe (a node holding its own
    fragment and an external connection point). Composed from the CDXML element names, not written by RDKit."""
    ph = group(100, 86, 108, "Ph", [(6, 0, 0), (6, 6, 6), (6, 18, 6), (6, 24, 0), (6, 18, -6), (6, 6, -6)], [(0, 1, 2), (1, 2, 1), (2, 3, 2), (3, 4, 1), (4, 5, 2), (5, 0, 1), (0, 99, 1)])
    ome = group(300, 386, 108, "OMe", [(8, 0, 0), (6, 10, 0)], [(0, 1, 1), (0, 99, 1)])
    body = ('<fragment id="10">' + atom(11, 100, 100) + atom(12, 114, 108, 8, 1, "OH") + ph + bond(13, 11, 12) + bond(14, 11, 100) + "</fragment>"
            '<fragment id="20">' + atom(21, 200, 100) + atom(22, 214, 108, 7, 1, "NH") + atom(23, 228, 100) + bond(25, 21, 22) + bond(26, 22, 23) + "</fragment>"
            '<fragment id="30">' + atom(31, 400, 100) + atom(32, 414, 108) + atom(33, 414, 124, 8) + ome + bond(34, 31, 32) + bond(35, 32, 33, 2) + bond(36, 32, 300) + "</fragment>"
            '<fragment id="40">' + atom(41, 500, 100) + atom(42, 514, 108) + atom(43, 528, 100, 7, 2, "NH2") + bond(44, 41, 42) + bond(45, 42, 43) + "</fragment>")
    return doc(body)


def generic_label():
    """CH3-NH-Boc with Boc as a label-only node (NodeType Unspecified): no structure behind the word."""
    return doc('<fragment id="10">' + atom(11, 100, 100) + atom(12, 114, 108, 7, 1, "NH") + f'<n id="13" p="128 100" NodeType="Unspecified">{lbl(14, 124, 103, "Boc")}</n>' + bond(15, 11, 12) + bond(16, 12, 13) + "</fragment>")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    expected = {"rdkit_version": rdBase.rdkitVersion, "files": {}}
    for name, (smi, what) in SIMPLE.items():
        if name == "explicit_h":
            m = Chem.AddHs(Chem.MolFromSmiles("CO")); AllChem.Compute2DCoords(m); m = spread(m)
            smi = "[H]C([H])([H])O[H]"
        else:
            m = spread(mol_of(smi))
        write(name, Chem.MolToCDXMLBlock(m))
        e = {"what": what, "source": smi, "source_inchi": source_inchi(smi), "reads": reading(OUT / f"{name}.cdxml")}
        expected["files"][f"{name}.cdxml"] = e
    for name, text, smi, what in [("reaction", reaction(), "CC(=O)O.CCO>>CC(=O)OCC", "a reaction scheme: two reactants, an arrow with a text, one product (arrow and step composed here)"),
                                  ("labels", labels(), "OCc1ccccc1.CNC.COC(C)=O.CCN", "atom labels and abbreviations: OH, NH, NH2, and Ph and OMe as expanded nodes (composed here)"),
                                  ("generic_label", generic_label(), "CNC(=O)OC(C)(C)C", "a label-only node, Boc, that holds no structure (composed here)")]:
        write(name, text)
        e = {"what": what, "source": smi, "source_inchi": source_inchi(smi.split(">>")[-1] if ">>" in smi else smi), "reads": reading(OUT / f"{name}.cdxml")}
        expected["files"][f"{name}.cdxml"] = e
        if name == "reaction":                    # the steps RDKit finds: how many molecules on each side of the arrow
            from rdkit.Chem import rdChemReactions
            steps = rdChemReactions.ReactionsFromCDXMLFile(str(OUT / "reaction.cdxml"))
            e["reaction"] = {"steps": len(steps), "reactants": steps[0].GetNumReactantTemplates(), "products": steps[0].GetNumProductTemplates(), "agents": steps[0].GetNumAgentTemplates()}
    (OUT / "expected.json").write_text(json.dumps(expected, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print("wrote", len(expected["files"]), "fixtures to", OUT)


if __name__ == "__main__":
    sys.exit(main())
