"""Study tool for docs/studi/STRUTTURA_DATO.md: what SMILES, molfile and InChI keep or lose on a small real set, and a round trip through the
proposed `mol/1` record (atoms, Kekulé bonds, explicit stereo with status). RDKit and Open Babel are references only (pip install rdkit
openbabel-wheel); nothing here runs in the site. Usage: python3 tools/struttura_dato/confronto.py [--json OUT] [--molfiles DIR] [--composti CSV] [--golden OUT]"""

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


def to_record(m, renumber=True):
    """RDKit mol -> mol/1 dict (the golden data of crates/mzlab-mol). Atoms in RDKit canonical order (renumber=True) or as written; Kekulé orders;
    bonds and double-bond centres with a < b; tetrahedral refs ascending, parity = RDKit chiral tag for the refs in that order with the implicit
    H or lone pair last; unknown elements carry no refs/parity/cfg."""
    if renumber:
        order = list(Chem.CanonicalRankAtoms(m, breakTies=True))
        m = Chem.RenumberAtoms(m, sorted(range(m.GetNumAtoms()), key=lambda i: order[i]))
    k = Chem.Mol(m)
    Chem.Kekulize(k, clearAromaticFlags=False)
    atoms = [{"el": a.GetSymbol(), "iso": a.GetIsotope(), "chg": a.GetFormalCharge(), "rad": a.GetNumRadicalElectrons(), "h": a.GetTotalNumHs()} for a in m.GetAtoms()]
    orders = {Chem.BondType.SINGLE: 1, Chem.BondType.DOUBLE: 2, Chem.BondType.TRIPLE: 3}
    bonds = sorted(
        ({"a": min(b.GetBeginAtomIdx(), b.GetEndAtomIdx()), "b": max(b.GetBeginAtomIdx(), b.GetEndAtomIdx()), "o": orders[b.GetBondType()]} for b in k.GetBonds()),
        key=lambda b: (b["a"], b["b"]),
    )
    stereo = []
    for si in Chem.FindPotentialStereo(m):
        defined = str(si.specified).endswith("Specified")
        status = "defined" if defined else "unknown"
        if si.type == Chem.StereoType.Atom_Tetrahedral:
            c = m.GetAtomWithIdx(si.centeredOn)
            ent = {"t": "tetra", "c": si.centeredOn}
            if defined:
                refs = [n.GetIdx() for n in c.GetNeighbors()]
                srt = sorted(refs)
                perm_ = [refs.index(r) for r in srt]
                inv = sum(1 for i in range(len(perm_)) for j in range(i + 1, len(perm_)) if perm_[i] > perm_[j])
                cw = (c.GetChiralTag() == Chem.ChiralType.CHI_TETRAHEDRAL_CW) ^ (inv % 2 == 1)
                ent.update({"refs": srt, "implicitH": c.GetTotalNumHs() == 1, "par": "cw" if cw else "ccw"})
            ent["status"] = status
            stereo.append(ent)
        elif si.type == Chem.StereoType.Bond_Double:
            b = m.GetBondWithIdx(si.centeredOn)
            ba, ea = sorted((b.GetBeginAtomIdx(), b.GetEndAtomIdx()))
            ent = {"t": "db", "a": ba, "b": ea}
            if defined:
                r1 = min(n.GetIdx() for n in m.GetAtomWithIdx(ba).GetNeighbors() if n.GetIdx() != ea)
                r2 = min(n.GetIdx() for n in m.GetAtomWithIdx(ea).GetNeighbors() if n.GetIdx() != ba)
                ent.update({"refs": [r1, r2], "cfg": cis_trans(m, ba, ea, r1, r2)})
            ent["status"] = status
            stereo.append(ent)
    stereo.sort(key=lambda e: (e["t"] != "tetra", e.get("c", e.get("a"))))
    groups = []
    kinds = {Chem.StereoGroupType.STEREO_ABSOLUTE: "abs", Chem.StereoGroupType.STEREO_AND: "and", Chem.StereoGroupType.STEREO_OR: "or"}
    for g in m.GetStereoGroups():
        groups.append({"kind": kinds[g.GetGroupType()], "atoms": sorted(a.GetIdx() for a in g.GetAtoms())})
    groups.sort(key=lambda g: g["atoms"])
    return {"format": "mol/1", "atoms": atoms, "bonds": bonds, "stereo": stereo, "groups": groups}


def core_sha256(rec):
    """SHA-256 of the compact JSON of the core (atoms, bonds, stereo, groups): the same bytes crates/mzlab-mol hashes."""
    import hashlib

    core = {k: rec[k] for k in ("atoms", "bonds", "stereo", "groups")}
    return hashlib.sha256(json.dumps(core, separators=(",", ":")).encode()).hexdigest()


def golden(path):
    """Golden data for the Rust tests: per case the input, the RDKit answers (CXSMILES, InChIKey) and the record in written and canonical order."""
    out = []
    for name, smi, _note in SET:
        m = Chem.MolFromSmiles(smi)
        recs = {"input_order": to_record(m, renumber=False), "rdkit_canonical": to_record(m)}
        out.append(
            {
                "name": name,
                "input": smi,
                "rdkit_cx": cx(m),
                "inchikey": Chem.MolToInchiKey(m),
                "aromatic": aromatic_pairs(m),
                "canonical_perm": canonical_perm(m),
                "records": recs,
                "core_sha256": {k: core_sha256(v) for k, v in recs.items()},
            }
        )
    doc = {"generator": "tools/struttura_dato/confronto.py --golden", "rdkit": Chem.rdBase.rdkitVersion, "cases": out, "smiles_cases": smiles_cases()}
    doc["composti"] = composti_rows(str(pathlib.Path(__file__).resolve().parents[2] / "TP_Mine" / "data" / "composti.csv"))
    pathlib.Path(path).write_text(json.dumps(doc, separators=(",", ":")) + "\n")


# SMILES reader cases beyond SET: aromatic systems, radicals, charges, isotopes, stereo written in many ways, enhanced-stereo groups
SMILES_EXTRA = [
    "c1ccccc1", "c1ccncc1", "c1cc[nH]c1", "c1ccoc1", "c1ccsc1", "c1cnc[nH]1", "c1nnn[nH]1", "c1ccc2[nH]ccc2c1", "c1ccc2ccccc2c1",
    "Cn1cnc2c1c(=O)n(C)c(=O)n2C", "O=c1cccc[nH]1", "Cc1ccccc1-c1ccccc1", "c1cc[n+](C)cc1", "[nH]1cccc1", "C1=CC=CC=C1", "c1ccc2c(c1)cc[nH]2",
    "[CH3]", "C[CH2]", "[NH4+]", "C[N+](=O)[O-]", "[O-][N+](=O)c1ccccc1", "[13CH4]", "[2H]O[2H]", "[Na+].[Cl-]", "CS(=O)(=O)C", "CP(=O)(O)O",
    "[Fe+2]", "C[Si](C)(C)C", "[Se]1C=CC=C1", "OS(=O)(=O)O", "C#N", "C=C=C", "[Cl-]", "[OH-]", "[O]",
    "C[C@H]1CC[C@@H](C)CC1", "F[C@](Cl)(Br)I", "F[C@@](Cl)(Br)I", "[C@H](F)(Cl)Br", "[C@@H](F)(Cl)Br", "[C@@H]1(C)CCCO1", "N[C@@]1(C)CCCO1",
    "C[S@](=O)CC", "C[S@@](=O)CC", "C[C@H](N)C(=O)O", "C[C@@H](N)C(=O)O", "OC[C@H]1O[C@@H](O)[C@H](O)[C@@H](O)[C@@H]1O", "C[C@]12CC[C@H]3[C@@H](CCC4=CC(=O)CC[C@@]43C)[C@@H]1CC[C@@H]2O",
    "F/C=C/F", "F/C=C\\F", "C(/F)=C/F", "F\\C=C\\F", "F/C=C/C=C/F", "F/C=C\\C=C/F", "C/C=C/C(=O)O", "C/C(CC)=C/C", "CC(/CC)=C/C",
    "C1CCC/C=C/CC1", "C1CCC/C=C\\CC1", "C/1=C/CCCCCCC1", "C/C=C(/CC)C", "Cl/C=C/C=C\\Br", "c1ccccc1/C=C/c1ccccc1", "O/N=C/C", "O/N=C\\C",
    "C[C@H](F)[C@@H](Cl)Br |a:1,3|", "C[C@H](F)[C@@H](Cl)Br |o1:1,3|", "C[C@H](F)[C@@H](Cl)Br |&1:1,3|", "C[C@H](F)[C@@H](Cl)Br |&1:1,&2:3|",
    "C[C@H](F)[C@@H](Cl)Br |a:1,&1:3|", "C[C@H](F)[C@@H](Cl)Br |$;;;;;$,&1:1,3,r|", "C[C@H](F)[C@@H](Cl)Br",
]


def canonical_perm(m):
    """new index of every atom when the atoms are put in RDKit canonical order (the permutation `to_record` applies)."""
    order = list(Chem.CanonicalRankAtoms(m, breakTies=True))
    by_rank = sorted(range(m.GetNumAtoms()), key=lambda i: order[i])
    new = [0] * len(by_rank)
    for j, old in enumerate(by_rank):
        new[old] = j
    return new


def aromatic_pairs(m):
    return sorted([min(b.GetBeginAtomIdx(), b.GetEndAtomIdx()), max(b.GetBeginAtomIdx(), b.GetEndAtomIdx())] for b in m.GetBonds() if b.GetIsAromatic())


def smiles_cases():
    """Per SMILES: the RDKit record in written atom order, its aromatic bonds (the reader's Kekulé choice may differ there) and the CXSMILES."""
    out = []
    for smi in SMILES_EXTRA:
        m = Chem.MolFromSmiles(smi)
        out.append({"input": smi, "record": to_record(m, renumber=False), "aromatic": aromatic_pairs(m)})
    return out


def composti_rows(path):
    """Per row of composti.csv: short digests of the RDKit atoms and defined stereo elements (written atom order) and the counts of potential and
    unspecified elements: the external truth for the reader on real data (the study quotes 286 rows with stereo elements, 111 with one unspecified)."""
    import csv
    import hashlib

    def digest(v):
        return hashlib.sha256(json.dumps(v, separators=(",", ":")).encode()).hexdigest()[:16]

    rows = []
    for row in csv.DictReader(open(path, encoding="utf-8")):
        m = Chem.MolFromSmiles(row["smiles"])
        rec = to_record(m, renumber=False)
        defined = [s for s in rec["stereo"] if s["status"] == "defined"]
        arom = sum(b.GetIsAromatic() for b in m.GetBonds())
        rows.append([digest(rec["atoms"]), len(rec["bonds"]), digest(defined), len(rec["stereo"]), len(rec["stereo"]) - len(defined), arom])
    return rows


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
    ap.add_argument("--golden", help="write the golden data of crates/mzlab-mol (crates/mzlab-mol/tests/data/golden.json)")
    a = ap.parse_args()
    if a.golden:
        golden(a.golden)
        return
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
