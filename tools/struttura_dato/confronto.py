"""Study tool for docs/studi/STRUTTURA_DATO.md: what SMILES, molfile and InChI keep or lose on a small real set, and a round trip through the
proposed `mol/1` record (atoms, Kekulé bonds, explicit stereo with status). RDKit and Open Babel are references only (pip install rdkit
openbabel-wheel); nothing here runs in the site. Usage: python3 tools/struttura_dato/confronto.py [--json OUT] [--molfiles DIR]"""

import argparse
import json
import pathlib
import sys

try:
    from rdkit import Chem, RDLogger
    from rdkit.Chem import AllChem
except ImportError:
    sys.exit("needs rdkit: pip install rdkit")
RDLogger.DisableLog("rdApp.*")
try:
    from openbabel import pybel

    pybel.ob.obErrorLog.SetOutputLevel(0)
except ImportError:
    pybel = None

# name, SMILES (CXSMILES enhanced stereo where written), what the case tests
SET = [
    ("timolol_S", "CC(C)(C)NC[C@H](O)COc1nsnc1N1CCOCC1", "drug, one centre"),
    ("timolol_R", "CC(C)(C)NC[C@@H](O)COc1nsnc1N1CCOCC1", "enantiomer"),
    ("timolol_unspec", "CC(C)(C)NCC(O)COc1nsnc1N1CCOCC1", "centre not specified (as in the repository)"),
    ("timolol_rac", "CC(C)(C)NC[C@H](O)COc1nsnc1N1CCOCC1 |&1:6|", "racemate written as enhanced stereo"),
    ("timolol_maleate", "CC(C)(C)NC[C@H](O)COc1nsnc1N1CCOCC1.OC(=O)/C=C\\C(=O)O", "salt"),
    ("timolol_d9", "[2H]C([2H])([2H])C(NC[C@H](O)COc1nsnc1N1CCOCC1)(C([2H])([2H])[2H])C([2H])([2H])[2H]", "labelled standard"),
    ("ephedrine", "CN[C@@H](C)[C@H](O)c1ccccc1", "diastereoisomer pair"),
    ("pseudoephedrine", "CN[C@@H](C)[C@@H](O)c1ccccc1", "diastereoisomer pair"),
    ("cis_dmch_meso", "C[C@@H]1CCCC[C@@H]1C", "meso: two centres, achiral"),
    ("trans_dmch_rel", "C[C@@H]1CCCC[C@H]1C |&1:1,6|", "relative trans, racemic"),
    ("maleic", "OC(=O)/C=C\\C(=O)O", "Z"),
    ("fumaric", "OC(=O)/C=C/C(=O)O", "E"),
    ("butene_unspec", "CC=CC", "double bond not specified"),
    ("hydroxypyridine", "Oc1ccccn1", "tautomer pair"),
    ("pyridone", "O=c1cccc[nH]1", "tautomer pair"),
    ("acetone", "CC(C)=O", "keto/enol pair"),
    ("propenol", "CC(O)=C", "keto/enol pair"),
    ("diclofenac", "OC(=O)Cc1ccccc1Nc1c(Cl)cccc1Cl", "acid"),
    ("diclofenac_na", "[Na+].[O-]C(=O)Cc1ccccc1Nc1c(Cl)cccc1Cl", "sodium salt"),
    ("hexane", "CCCCCC", "no stereo at all"),
]


def ob_can(smi):
    if pybel is None:
        return None
    m = pybel.readstring("smi", smi.split(" |")[0])
    return m.write("can").split("\t")[0].strip()


def ob_inchi(smi):
    if pybel is None:
        return None
    return pybel.readstring("smi", smi.split(" |")[0]).write("inchi").strip()


def stereo_status(m):
    """Every potential stereo element of RDKit with its status: what the proposed record writes and a SMILES cannot."""
    out = []
    for si in Chem.FindPotentialStereo(m):
        kind = "tetra" if si.type == Chem.StereoType.Atom_Tetrahedral else "db"
        spec = str(si.specified).split(".")[-1]
        out.append(f"{kind}{si.centeredOn}:{'defined' if spec == 'Specified' else 'unknown'}")
    return out


# --- mol/1: the proposed record (prototype) ---------------------------------------------------------------------------------------------


def to_record(m):
    """RDKit mol -> mol/1 dict: atoms in canonical order, Kekulé orders + aromatic flag, stereo with explicit refs and status, groups."""
    order = list(Chem.CanonicalRankAtoms(m, breakTies=True))
    perm = sorted(range(m.GetNumAtoms()), key=lambda i: order[i])
    m = Chem.RenumberAtoms(m, perm)
    k = Chem.Mol(m)
    Chem.Kekulize(k, clearAromaticFlags=False)
    atoms = []
    for a in m.GetAtoms():
        atoms.append({"el": a.GetSymbol(), "iso": a.GetIsotope(), "chg": a.GetFormalCharge(), "rad": a.GetNumRadicalElectrons(), "h": a.GetTotalNumHs()})
    bonds = []
    for b in k.GetBonds():
        o = {Chem.BondType.SINGLE: 1, Chem.BondType.DOUBLE: 2, Chem.BondType.TRIPLE: 3}[b.GetBondType()]
        bonds.append({"a": b.GetBeginAtomIdx(), "b": b.GetEndAtomIdx(), "o": o, "arom": b.GetIsAromatic()})
    stereo = []
    for si in Chem.FindPotentialStereo(m):
        defined = str(si.specified).endswith("Specified")
        if si.type == Chem.StereoType.Atom_Tetrahedral:
            c = m.GetAtomWithIdx(si.centeredOn)
            refs = [n.GetIdx() for n in c.GetNeighbors()]
            # parity in neighbour order of the atom as RDKit holds it: the record rewrites it on its own sorted refs
            tag = c.GetChiralTag()
            par = None
            if defined:
                cw = tag == Chem.ChiralType.CHI_TETRAHEDRAL_CW
                srt = sorted(refs)
                # an odd permutation from RDKit's order to the sorted order flips the sense
                perm_ = [refs.index(r) for r in srt]
                inv = sum(1 for i in range(len(perm_)) for j in range(i + 1, len(perm_)) if perm_[i] > perm_[j])
                cw = cw ^ (inv % 2 == 1)
                par = "cw" if cw else "ccw"
                refs = srt
            stereo.append({"t": "tetra", "c": si.centeredOn, "refs": refs, "implicitH": c.GetTotalNumHs() == 1, "par": par, "status": "defined" if defined else "unknown"})
        elif si.type == Chem.StereoType.Bond_Double:
            b = m.GetBondWithIdx(si.centeredOn)
            ent = {"t": "db", "a": b.GetBeginAtomIdx(), "b": b.GetEndAtomIdx(), "status": "defined" if defined else "unknown"}
            if defined:
                ba, ea = b.GetBeginAtomIdx(), b.GetEndAtomIdx()
                r1 = min(n.GetIdx() for n in b.GetBeginAtom().GetNeighbors() if n.GetIdx() != ea)
                r2 = min(n.GetIdx() for n in b.GetEndAtom().GetNeighbors() if n.GetIdx() != ba)
                ent.update({"refs": [r1, r2], "cfg": cis_trans(m, ba, ea, r1, r2)})
            stereo.append(ent)
    groups = []
    for g in m.GetStereoGroups():
        kind = {Chem.StereoGroupType.STEREO_ABSOLUTE: "abs", Chem.StereoGroupType.STEREO_AND: "and", Chem.StereoGroupType.STEREO_OR: "or"}[g.GetGroupType()]
        groups.append({"kind": kind, "atoms": sorted(a.GetIdx() for a in g.GetAtoms())})
    return {"format": "mol/1", "atoms": atoms, "bonds": bonds, "stereo": stereo, "groups": groups}


def cis_trans(m, ba, ea, r1, r2):
    """cis/trans of r1 and r2 across the double bond ba=ea, from 2D coordinates that RDKit draws with the stereo kept."""
    c = Chem.Mol(m)
    AllChem.Compute2DCoords(c)
    p = c.GetConformer().GetPositions()

    def side(i):
        return (p[ea][0] - p[ba][0]) * (p[i][1] - p[ba][1]) - (p[ea][1] - p[ba][1]) * (p[i][0] - p[ba][0])

    return "cis" if side(r1) * side(r2) > 0 else "trans"


def from_record(r):
    """mol/1 dict -> RDKit mol, without reading any SMILES: the record alone must carry the molecule."""
    rw = Chem.RWMol()
    for a in r["atoms"]:
        at = Chem.Atom(a["el"])
        at.SetIsotope(a["iso"])
        at.SetFormalCharge(a["chg"])
        at.SetNumRadicalElectrons(a["rad"])
        at.SetNoImplicit(True)
        at.SetNumExplicitHs(a["h"])
        rw.AddAtom(at)
    for b in r["bonds"]:
        rw.AddBond(b["a"], b["b"], {1: Chem.BondType.SINGLE, 2: Chem.BondType.DOUBLE, 3: Chem.BondType.TRIPLE}[b["o"]])
    for s in r["stereo"]:
        if s["status"] != "defined":
            continue
        if s["t"] == "tetra":
            c = rw.GetAtomWithIdx(s["c"])
            nb = [n.GetIdx() for n in c.GetNeighbors()]
            perm_ = [s["refs"].index(x) for x in nb]
            inv = sum(1 for i in range(len(perm_)) for j in range(i + 1, len(perm_)) if perm_[i] > perm_[j])
            cw = (s["par"] == "cw") ^ (inv % 2 == 1)
            c.SetChiralTag(Chem.ChiralType.CHI_TETRAHEDRAL_CW if cw else Chem.ChiralType.CHI_TETRAHEDRAL_CCW)
    groups = []
    for g in r["groups"]:
        kind = {"abs": Chem.StereoGroupType.STEREO_ABSOLUTE, "and": Chem.StereoGroupType.STEREO_AND, "or": Chem.StereoGroupType.STEREO_OR}[g["kind"]]
        groups.append(Chem.rdchem.CreateStereoGroup(kind, rw, g["atoms"]))
    if groups:
        rw.SetStereoGroups(groups)
    m = rw.GetMol()
    Chem.SanitizeMol(m)
    # double bonds after sanitising, which would otherwise clear a stereo set on atoms that are not yet typed
    for s in r["stereo"]:
        if s["t"] == "db" and s["status"] == "defined":
            b = m.GetBondBetweenAtoms(s["a"], s["b"])
            ref1, ref2 = s["refs"] if b.GetBeginAtomIdx() == s["a"] else s["refs"][::-1]
            b.SetStereoAtoms(ref1, ref2)
            b.SetStereo(Chem.BondStereo.STEREOCIS if s["cfg"] == "cis" else Chem.BondStereo.STEREOTRANS)
    Chem.AssignStereochemistry(m, cleanIt=False, force=True)
    # RDKit's SMILES writer reads Z/E from bond directions: give them, or a kept double bond prints without / and \
    Chem.SetDoubleBondNeighborDirections(m)
    return m


def cx(m):
    return Chem.MolToCXSmiles(m)


def molfile_cases(m):
    """2D molfile with wedges, the same without wedges (only coordinates), and 3D: is the stereo still there?"""
    m2 = Chem.Mol(m)
    AllChem.Compute2DCoords(m2)
    mb = Chem.MolToMolBlock(m2)
    back = Chem.MolFromMolBlock(mb)
    lines = mb.splitlines()
    if "V3000" in lines[3]:
        # RDKit writes V3000 for enhanced stereo; the wedge test is a V2000 one
        back.RemoveAllConformers()
        return {"mol2d_wedges_ok": Chem.MolToCXSmiles(back) == Chem.MolToCXSmiles(m), "mol2d_nowedge_ok": None, "molblock": mb}
    na, nb = int(lines[3][0:3]), int(lines[3][3:6])
    for i in range(4 + na, 4 + na + nb):
        lines[i] = lines[i][:9] + "  0" + lines[i][12:]
    back_nw = Chem.MolFromMolBlock("\n".join(lines))
    iso = Chem.MolToSmiles(m)
    return {
        "mol2d_wedges_ok": back is not None and Chem.MolToSmiles(back) == iso,
        "mol2d_nowedge_ok": back_nw is not None and Chem.MolToSmiles(back_nw) == iso,
        "molblock": mb,
    }


def composti(path):
    """The suspect list of Tabui: does the InChIKey written next to each SMILES describe the same molecule, the same skeleton, or neither?"""
    import csv

    n = full = block = other = bad = withstereo = unknown = frag = 0
    for row in csv.DictReader(open(path, encoding="utf-8")):
        n += 1
        m = Chem.MolFromSmiles(row["smiles"])
        if m is None:
            bad += 1
            continue
        st = [str(si.specified).endswith("Specified") for si in Chem.FindPotentialStereo(m)]
        if st:
            withstereo += 1
            unknown += not all(st)
        frag += "." in row["smiles"]
        k = Chem.MolToInchiKey(m)
        if k == row["inchikey"]:
            full += 1
        elif k[:14] == row["inchikey"][:14]:
            block += 1
        else:
            other += 1
    print(f"{path}: {n} rows; InChIKey equal {full}; same skeleton only {block}; different {other}; SMILES unreadable {bad}; with stereo elements {withstereo}, of which some unspecified {unknown}; with «.» {frag}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--json")
    ap.add_argument("--molfiles")
    ap.add_argument("--composti", help="TP_Mine/data/composti.csv")
    a = ap.parse_args()
    if a.composti:
        composti(a.composti)
        return
    rows = []
    for name, smi, note in SET:
        m = Chem.MolFromSmiles(smi)
        cip = [f"{i}{l}" for i, l in Chem.FindMolChiralCenters(m, includeUnassigned=True, useLegacyImplementation=False)]
        rec = to_record(m)
        back = from_record(json.loads(json.dumps(rec)))
        mf = molfile_cases(m)
        if a.molfiles:
            pathlib.Path(a.molfiles).mkdir(parents=True, exist_ok=True)
            pathlib.Path(a.molfiles, name + ".mol").write_text(mf.pop("molblock"))
        else:
            mf.pop("molblock")
        row = {
            "name": name,
            "note": note,
            "rdkit_smiles": Chem.MolToSmiles(m),
            "rdkit_cx": cx(m),
            "ob_smiles": ob_can(smi),
            "cip": cip,
            "stereo": stereo_status(m),
            "inchi": Chem.MolToInchi(m),
            "inchikey": Chem.MolToInchiKey(m),
            "ob_inchi": ob_inchi(smi),
            "record_roundtrip_cx": cx(back) == cx(m),
            "record_roundtrip_inchi": Chem.MolToInchi(back) == Chem.MolToInchi(m),
            **mf,
        }
        rows.append(row)
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps(rows, indent=1))
    keys = ["name", "rdkit_cx", "ob_smiles", "cip", "stereo", "inchikey", "record_roundtrip_cx", "record_roundtrip_inchi", "mol2d_wedges_ok", "mol2d_nowedge_ok"]
    print("| " + " | ".join(keys) + " |")
    print("|" + "---|" * len(keys))
    for r in rows:
        print("| " + " | ".join((str(r[k]) if not isinstance(r[k], list) else " ".join(r[k])).replace("|", "\\|") for k in keys) + " |")
    print(f"\nRDKit {Chem.rdBase.rdkitVersion}; Open Babel {'missing' if pybel is None else pybel.ob.OBReleaseVersion()}")


if __name__ == "__main__":
    main()
