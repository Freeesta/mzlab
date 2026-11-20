#!/usr/bin/env python3
"""Quality gates of `mzlab-cli mol draw` (2D layout of `mzlab-mol`): synthetic test sets, the metrics, and the RDKit reference.

  --set-out     write tests/dati/draw_set_a.tsv (200 invented molecules, 9 families) and draw_set_b.tsv (100 E/Z double bonds);
                needs RDKit only to build them (seed 20261120); the committed files are what the tests read
  --reference   write tests/dati/draw_reference.json: RDKit's own clean count per family and its totals on the same metrics
  --report      run the release CLI on both sets and print the gates (a development aid; the test is tests/test_mol_draw_quality.py)

The metrics are pure Python on a scene (atoms with x_a, y_a; bonds; fragments), coordinates scaled so the median bond is 1:
clash = a non-bonded pair closer than 0.45; atom on bond = an atom closer than 0.3 to a bond it is not on; crossing = two bonds with no
shared atom that intersect; bond-length CV; mean ring-angle error on rings of up to 8 atoms; fragment gap; E/Z read from the
coordinates against the expected cis/trans. Clean = no clash, no atom on bond, no crossing, CV < 0.15, ring error < 10 degrees.
RDKit is test-only: never a runtime dependency of mzLab.
"""
import argparse
import json
import math
import random
import statistics
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATI = ROOT / "tests" / "dati"
SEED = 20261120
FAMILIES = [("chain", 20), ("monocycle", 25), ("fused", 40), ("bridged_spiro", 20), ("macrocycle", 20),
            ("charged", 20), ("stereo", 25), ("salt", 20), ("druglike", 10)]

SCAFFOLDS = {
    "chain": ["CCCCCCCC", "CC(C)CCCC", "CCOCCOCC", "CC(=O)NCCC", "CCC#CCC", "C=CCCC=C", "CCSCCN", "CCCCCCCCCCCC", "CC(C)(C)CC",
              "OCCOCCOCCO", "CCN(CC)CC", "CCC(=O)OCC", "CCCC(=O)CC", "C=C=CCCC", "CC(C)CC(C)C", "NCCCCN", "CCCCOCCCC"],
    "monocycle": ["C1CC1", "C1CCC1", "C1CCCC1", "C1CCCCC1", "C1CCCCCC1", "c1ccccc1", "c1ccncc1", "c1ccoc1", "c1ccsc1", "c1cc[nH]c1",
                  "c1cnc[nH]1", "c1cscn1", "c1cncnc1", "C1=CCCCC1", "C1CCNCC1", "C1COCCN1", "C1CCOC1", "C1=CC=CC1", "c1ccnnc1",
                  "C1CCSC1", "C1COC1", "c1ncncn1", "C1=CCC=CC1"],
    "fused": ["c1ccc2ccccc2c1", "c1ccc2[nH]ccc2c1", "c1ccc2ncccc2c1", "c1ccc2occc2c1", "c1ccc2sccc2c1", "C1Cc2ccccc2C1",
              "C1CCc2ccccc2C1", "C1CCC2CCCCC2C1", "c1ncc2nc[nH]c2n1", "c1ccc2[nH]cnc2c1", "c1ccc2cc3ccccc3cc2c1",
              "c1ccc2c(c1)ccc1ccccc12", "C1c2ccccc2-c2ccccc12", "c1ccc2c(c1)[nH]c1ccccc12", "c1ccc2ncncc2c1", "O=c1ccc2ccccc2o1",
              "O=c1ccoc2ccccc12", "C1CCC2C(C1)CCC1C2CCC2CCCC21", "c1cc2ccc3cccc4ccc(c1)c2c34",
              "c1ccc2nc3ccccc3cc2c1", "c1ccn2ccnc2c1", "O=C1C=CC(=O)c2ccccc12", "c1ccc2c(c1)OCCO2", "c1ccc2ocnc2c1",
              "c1ccc2nccnc2c1", "c1ccc2cnccc2c1", "c1ccc2c(c1)Sc1ccccc1N2", "c1ccc2c(c1)oc1ccccc12", "C1CCC2CCCC2C1",
              "c1ccc2c(c1)CCc1ccccc12", "c1ccc2c(c1)CCCC2", "C1CC2CCCC2C1"],
    "bridged_spiro": ["C1CC2CCC1C2", "C1CC2CCC1CC2", "C1C2CC3CC1CC(C2)C3", "C12C3C4C1C5C2C3C45", "C1CN2CCC1CC2", "CN1C2CCC1CC2",
                      "C1CC2CC1C2", "C1CCC2(C1)CCCCC2", "C1CCC2(CC1)CCCCC2", "C1CC2(C1)CCC2",
                      "O=C1Nc2ccccc2C13CCCC3", "C1COC2(O1)CCCC2", "C1CC2CC1C=C2", "C1CCC2CC1C2", "C1CC2(CCC1)CCCC2", "C1CC12CCC2",
                      "C1CC2CCC(C1)N2", "C1OC2(CCCC2)OC1"],
    "macrocycle": ["C1CCCCCCCCCCC1", "C1CCCCCCCCCCCC1", "C1CCCCCCCCCCCCC1", "C1CCCCCCCCCCCCCC1", "C1CCCCCCCCCCCCCCC1", "C1CCCCCCCCCCCCCCCC1",
                   "C1CCCCCCCCCCCCCCCCC1", "C1COCCOCCOCCO1", "C1COCCOCCOCCOCCO1", "C1COCCOCCOCCOCCOCCO1", "O=C1CNC(=O)CNC(=O)CNC(=O)CN1",
                   "O=C1CNC(=O)CNC(=O)CN1", "C1CCCCCCCCCC(=O)OCC1", "C1CCCCCCCc2ccccc2CCCCC1"],
    "charged": ["C[N+](C)(C)C", "CC(=O)[O-]", "c1cc[n+](C)cc1", "[O-][N+](=O)c1ccccc1", "NCC(=O)[O-]", "C[NH3+]", "CCCC[n+]1ccccc1",
                "[O-]C(=O)CCC(=O)[O-]", "C[S+](C)C", "c1ccc([N+](C)(C)C)cc1", "[O-][n+]1ccccc1", "CCOP(=O)([O-])OCC", "NCCC[NH3+]",
                "OC(=O)CC(=O)[O-]", "C[N+]1(C)CCCC1", "[NH3+]CCS(=O)(=O)[O-]"],
    "stereo": ["C[C@H](N)C(=O)O", "C[C@@H](O)CC", "CC(C)[C@@H]1CC[C@@H](C)C[C@H]1O", "C[C@H](O)[C@H](N)C", "OC[C@H]1OC(O)[C@H](O)[C@@H](O)[C@@H]1O",
               "O=C(O)[C@H](O)[C@@H](O)C(=O)O", "N[C@@H](Cc1ccccc1)C(=O)O", "O[C@H]1CCCC[C@@H]1O", "C[C@@H]1CC[C@H](C)CC1", "C[C@H](Cl)[C@@H](Br)C",
               "O=C1[C@H](C)CC[C@@H]1C", "C[C@@H]1CCCC[C@H]1C", "C[C@H](O)[C@@H](C)O", "N[C@@H]1CCCC[C@H]1O", "O[C@@H]1CCC[C@H]1C",
               "C/C=C/[C@H](C)O", "C[C@H]1CCC[C@@H]2CCCC[C@H]12", "OC(=O)[C@@H]1CCCN1", "C[C@@H](c1ccccc1)N", "C[C@H](F)[C@@H](F)C", "CC[C@H](C)[C@@H](C)CC"],
    "salt": ["CC(=O)[O-].[Na+]", "CN(C)C.Cl", "[NH4+].[Cl-]", "c1ccccc1C(=O)[O-].[K+]", "CCN.Cl", "CC(C)NCC(O)c1ccc(O)c(O)c1.Cl",
             "CC(=O)[O-].CC(=O)[O-].[Mg+2]", "C[NH+](C)C.[Cl-]", "OC(=O)CC(O)(CC(=O)O)C(=O)[O-].[Na+]", "c1ccncc1.Cl", "CCCC[N+](C)(C)C.[Br-]",
             "OS(=O)(=O)[O-].[Na+]", "NCCN.Cl.Cl", "CC(=O)O.CCN", "O.CC(=O)[O-].[Na+]"],
    "druglike": ["CC(=O)Oc1ccccc1C(=O)O", "Cn1cnc2c1c(=O)n(C)c(=O)n2C", "CC(C)Cc1ccc(cc1)C(C)C(=O)O", "CC(=O)Nc1ccc(O)cc1",
                 "CN1C(=O)CN=C(c2ccccc2)c2cc(Cl)ccc21", "CC(C)NCC(O)COc1ccccc1", "CN1CCCC1c1cccnc1", "CC(=O)CC(c1ccccc1)c1c(O)c2ccccc2oc1=O",
                 "COc1ccc(CCN(C)C)cc1", "O=C(O)c1ccccc1O"],
}
SUBS = ["C", "O", "N", "F", "Cl", "C(=O)O", "OC", "C(F)(F)F", "CC", "C#N", "N(C)C", "C(C)C", "Br", "C(=O)N", "S"]
# set B: substituents of an E/Z bond, by the atom that carries the double bond
ENDS = {"C": ["C", "CC", "c1ccccc1", "C(=O)O", "CCO", "Cl", "F", "OC", "C#N", "C(C)C", "N(C)C", "Br", "C(=O)OC", "CCCC"],
        "N": ["O", "OC", "N", "NC", "c1ccccc1", "C", "CC", "NC(=O)C"]}


def rdkit():
    try:
        from rdkit import Chem, RDLogger
    except ImportError:
        sys.exit("draw_quality: RDKit is needed for this option (pip install rdkit); the tests do not need it")
    RDLogger.DisableLog("rdApp.*")
    return Chem


def decorate(Chem, smi, rng, k):
    """`smi` with up to `k` substituents on random atoms that have a free hydrogen; stereo centres and double bonds are left alone."""
    m = Chem.MolFromSmiles(smi)
    for _ in range(k):
        for _try in range(30):
            skip = {a.GetIdx() for a in m.GetAtoms() if a.GetChiralTag() != Chem.ChiralType.CHI_UNSPECIFIED or a.GetFormalCharge()}
            for b in m.GetBonds():
                if b.GetStereo() != Chem.BondStereo.STEREONONE:
                    skip |= {b.GetBeginAtomIdx(), b.GetEndAtomIdx()}
            ok = [a.GetIdx() for a in m.GetAtoms() if a.GetTotalNumHs() > 0 and not a.GetNoImplicit() and a.GetIdx() not in skip]
            if not ok:
                break
            at = rng.choice(ok)
            sub = Chem.MolFromSmiles(rng.choice(SUBS))
            em = Chem.RWMol(Chem.CombineMols(m, sub))
            em.AddBond(at, m.GetNumAtoms(), Chem.BondType.SINGLE)
            try:
                Chem.SanitizeMol(em)
            except Exception:
                continue
            m = Chem.MolFromSmiles(Chem.MolToSmiles(em))
            break
    return Chem.MolToSmiles(m)


def build_a(Chem, rng):
    rows = []
    for fam, n in FAMILIES:
        for _ in range(n):
            smi = rng.choice(SCAFFOLDS[fam])
            smi = decorate(Chem, smi, rng, rng.randrange(0, 4))
            rows.append((f"a{len(rows) + 1:03d}", smi, fam))
    return rows


def ez_template(rng):
    kind = rng.choice(["C=C"] * 5 + ["C=N"] * 2 + ["N=N"] + ["ring"] * 2)
    cis = rng.random() < 0.5
    if kind == "ring":
        n = rng.choice([8, 8, 9, 10, 10, 12, 12, 14, 16])
        return "C1" + "C" * (n - 4) + "/C=C" + ("\\" if cis else "/") + "C1"
    mark = "\\" if cis else "/"
    if kind == "C=C":
        extra = rng.choice(["", "", "(C)"])
        return f"{rng.choice(ENDS['C'])}/C{extra}=C{mark}{rng.choice(ENDS['C'])}"
    if kind == "C=N":
        return f"{rng.choice(ENDS['C'])}/C=N{mark}{rng.choice(ENDS['N'])}"
    return f"{rng.choice(ENDS['N'])}/N=N{mark}{rng.choice(ENDS['N'])}"


def stereo_pairs(Chem, smi):
    """`a,b,r0,r1,cis` for every double bond that RDKit reads as stereo: atom indices in SMILES order, r0 on a, r1 on b."""
    m = Chem.MolFromSmiles(smi)
    out = []
    for b in m.GetBonds():
        st = b.GetStereo()
        if st in (Chem.BondStereo.STEREONONE, Chem.BondStereo.STEREOANY):
            continue
        a, c = b.GetBeginAtomIdx(), b.GetEndAtomIdx()
        s0, s1 = b.GetStereoAtoms()
        if m.GetBondBetweenAtoms(a, s0) is None:
            s0, s1 = s1, s0
        cis = st in (Chem.BondStereo.STEREOZ, Chem.BondStereo.STEREOCIS)
        if a > c:
            a, c, s0, s1 = c, a, s1, s0
        out.append(f"{a},{c},{s0},{s1},{'cis' if cis else 'trans'}")
    return out


def build_b(Chem, rng):
    rows = []
    while len(rows) < 100:
        smi = ez_template(rng)
        m = Chem.MolFromSmiles(smi)
        if m is None:
            continue
        if smi.startswith("C1") and rng.random() < 0.6:
            smi = decorate(Chem, smi, rng, rng.randrange(1, 3))
        else:
            smi = Chem.MolToSmiles(m)
        pairs = stereo_pairs(Chem, smi)
        if pairs:
            rows.append((f"b{len(rows) + 1:03d}", smi, ";".join(pairs)))
    return rows


def write_sets():
    Chem = rdkit()
    rng = random.Random(SEED)
    for name, head, rows in (("draw_set_a", "id\tsmiles\tfamily", build_a(Chem, rng)), ("draw_set_b", "id\tsmiles\tez", build_b(Chem, rng))):
        (DATI / f"{name}.tsv").write_text("\n".join([head] + ["\t".join(r) for r in rows]) + "\n", encoding="utf-8")
        print(f"{name}.tsv: {len(rows)} rows")


def read_set(name):
    lines = (DATI / f"{name}.tsv").read_text(encoding="utf-8").splitlines()[1:]
    return [tuple(x.split("\t")) for x in lines]


# ------------------------------------------------------------------ metrics (no RDKit)
def _seg_dist(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2))
    return math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)


def _cross(o, a, b):
    return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])


def _intersect(a, b, c, d):
    return _cross(a, b, c) * _cross(a, b, d) < -1e-12 and _cross(c, d, a) * _cross(c, d, b) < -1e-12


def components(n, bonds):
    comp = list(range(n))

    def find(x):
        while comp[x] != x:
            comp[x] = comp[comp[x]]
            x = comp[x]
        return x
    for a, b in bonds:
        comp[find(a)] = find(b)
    return [find(i) for i in range(n)]


def rings_of(n, bonds):
    """A smallest set of smallest rings (ordered atom cycles): one shortest cycle per bond, then GF(2) independence."""
    adj = [[] for _ in range(n)]
    for a, b in bonds:
        adj[a].append(b)
        adj[b].append(a)
    eid = {tuple(sorted(b)): i for i, b in enumerate(bonds)}
    cands = {}
    for a, b in bonds:
        prev = {a: a}
        queue = [a]
        for x in queue:
            for y in sorted(adj[x]):
                if (x, y) in ((a, b),) or y in prev:
                    continue
                prev[y] = x
                queue.append(y)
        if b in prev:
            path, c = [b], b
            while c != a:
                c = prev[c]
                path.append(c)
            cands.setdefault(tuple(sorted(path)), path)
    target = len(bonds) - n + len(set(components(n, bonds)))
    basis, out = [], []
    for key in sorted(cands, key=lambda k: (len(k), k)):
        ring = cands[key]
        v = 0
        for i in range(len(ring)):
            v ^= 1 << eid[tuple(sorted((ring[i], ring[(i + 1) % len(ring)])))]
        for bv in basis:
            v = min(v, v ^ bv)
        if v:
            basis.append(v)
            out.append(ring)
        if len(out) == target:
            break
    return out


def measure(xy, bonds, frags=None):
    """The metrics of one drawing: xy = [(x, y)], bonds = [(a, b)], frags = piece id per atom (computed when None)."""
    n = len(xy)
    lens = [math.dist(xy[a], xy[b]) for a, b in bonds]
    med = statistics.median(lens) if lens else 1.0
    p = [(x / med, y / med) for x, y in xy]
    bset = {tuple(sorted(b)) for b in bonds}
    frag = frags or components(n, bonds)
    clash = any(math.dist(p[i], p[j]) < 0.45 for i in range(n) for j in range(i) if tuple(sorted((i, j))) not in bset)
    onbond = any(_seg_dist(p[i], p[a], p[b]) < 0.3 for i in range(n) for a, b in bonds if i not in (a, b))
    cross = any(_intersect(p[a], p[b], p[c], p[d]) for k, (a, b) in enumerate(bonds) for c, d in bonds[:k] if len({a, b, c, d}) == 4)
    ls = [x / med for x in lens]
    cv = statistics.pstdev(ls) / statistics.mean(ls) if len(ls) > 1 else 0.0
    errs = []
    for ring in rings_of(n, bonds):
        k = len(ring)
        if k > 8:
            continue
        ideal = (k - 2) * 180.0 / k
        for i in range(k):
            u = (p[ring[i - 1]][0] - p[ring[i]][0], p[ring[i - 1]][1] - p[ring[i]][1])
            w = (p[ring[(i + 1) % k]][0] - p[ring[i]][0], p[ring[(i + 1) % k]][1] - p[ring[i]][1])
            ang = math.degrees(abs(math.atan2(u[0] * w[1] - u[1] * w[0], u[0] * w[0] + u[1] * w[1])))
            errs.append(abs(ang - ideal))
    ring_err = sum(errs) / len(errs) if errs else 0.0
    gaps = [math.dist(p[i], p[j]) for i in range(n) for j in range(i) if frag[i] != frag[j]]
    gap = min(gaps) if gaps else None
    clean = not (clash or onbond or cross) and cv < 0.15 and ring_err < 10.0
    return {"clash": clash, "onbond": onbond, "cross": cross, "cv": cv, "ring_err": ring_err, "gap": gap, "clean": clean}


def ez_wrong(xy, ez):
    """How many of the expected `a,b,r0,r1,cis|trans` items (joined by ;) the coordinates draw the other way."""
    bad = 0
    for item in ez.split(";"):
        a, b, r0, r1, want = item.split(",")
        a, b, r0, r1 = int(a), int(b), int(r0), int(r1)
        ax = (xy[b][0] - xy[a][0], xy[b][1] - xy[a][1])
        s0 = ax[0] * (xy[r0][1] - xy[a][1]) - ax[1] * (xy[r0][0] - xy[a][0])
        s1 = ax[0] * (xy[r1][1] - xy[b][1]) - ax[1] * (xy[r1][0] - xy[b][0])
        bad += (s0 * s1 > 0) != (want == "cis")
    return bad


def summarize(rows):
    """Counts per family and in total from [(family, measure dict)]."""
    fam = {}
    for f, m in rows:
        fam[f] = fam.get(f, 0) + m["clean"]
    keys = ("clash", "onbond", "cross")
    out = {"clean": sum(m["clean"] for _, m in rows), "n": len(rows), "families": fam}
    out.update({k: sum(m[k] for _, m in rows) for k in keys})
    out["fragments_under_1"] = sum(1 for _, m in rows if m["gap"] is not None and m["gap"] < 1.0)
    out["fragments_under_1_5"] = sum(1 for _, m in rows if m["gap"] is not None and m["gap"] < 1.5)
    return out


# ------------------------------------------------------------------ the engines
def rdkit_xy(Chem, smi):
    from rdkit.Chem import rdDepictor
    m = Chem.MolFromSmiles(smi)
    rdDepictor.SetPreferCoordGen(False)
    rdDepictor.Compute2DCoords(m)
    c = m.GetConformer()
    xy = [(c.GetAtomPosition(i).x, c.GetAtomPosition(i).y) for i in range(m.GetNumAtoms())]
    return xy, [(b.GetBeginAtomIdx(), b.GetEndAtomIdx()) for b in m.GetBonds()]


def write_reference():
    Chem = rdkit()
    import rdkit as rk
    a = [(f, measure(*rdkit_xy(Chem, s))) for _, s, f in read_set("draw_set_a")]
    bad = sum(ez_wrong(rdkit_xy(Chem, s)[0], ez) for _, s, ez in read_set("draw_set_b"))
    ref = summarize(a)
    ref.update({"rdkit": rk.__version__, "ez_wrong_b": bad, "ez_total_b": 100})
    (DATI / "draw_reference.json").write_text(json.dumps(ref, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(ref))


def cli_scene(binary, smi):
    p = subprocess.run([binary, "mol", "draw", "smiles:" + smi, "--json"], capture_output=True, encoding="utf-8")
    return p.returncode, json.loads(p.stdout)


def scene_measure(sc):
    xy = [(a["x_a"], a["y_a"]) for a in sc["atoms"]]
    bonds = [(b["a"], b["b"]) for b in sc["bonds"]]
    frag = [0] * len(xy)
    for k, f in enumerate(sc["fragments"]):
        for i in f:
            frag[i] = k
    return xy, bonds, frag


def report(binary):
    ref = json.loads((DATI / "draw_reference.json").read_text(encoding="utf-8")) if (DATI / "draw_reference.json").exists() else None
    rows = []
    for _id, smi, fam in read_set("draw_set_a"):
        code, sc = cli_scene(binary, smi)
        xy, bonds, frag = scene_measure(sc)
        rows.append((fam, measure(xy, bonds, frag)))
    s = summarize(rows)
    print(f"A clean {s['clean']}/{s['n']}  clash {s['clash']} onbond {s['onbond']} cross {s['cross']} frag<1.5 {s['fragments_under_1_5']}")
    for fam, n in FAMILIES:
        r = ref["families"][fam] if ref else "?"
        print(f"  {fam:14s} {s['families'].get(fam, 0):3d}/{n}   rdkit {r}")
    bad = 0
    for _id, smi, ez in read_set("draw_set_b"):
        _, sc = cli_scene(binary, smi)
        bad += ez_wrong(scene_measure(sc)[0], ez)
    print(f"B E/Z wrong {bad}/100")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--set-out", action="store_true")
    ap.add_argument("--reference", action="store_true")
    ap.add_argument("--report", action="store_true")
    ap.add_argument("--bin", default=str(ROOT / "target" / "release" / "mzlab-cli"))
    a = ap.parse_args()
    if a.set_out:
        write_sets()
    if a.reference:
        write_reference()
    if a.report:
        report(a.bin)
    if not (a.set_out or a.reference or a.report):
        ap.print_help()


if __name__ == "__main__":
    main()
