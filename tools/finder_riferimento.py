"""Reference answers of the parts of mzFinder ported to Rust (`crates/mzlab-feat/src/legacy/`) for their parity test
(crates/mzlab-feat/tests/legacy_parity.rs): `api.formula_info`, the names of `api.classify` (`guess_sample`), the kinetic descriptors, the MS2
consensus and the three components of the MS2 similarity; the MSn tree (`hr/msn.py`), its fragment library (`hr/iimn.py`), the localisation
(`hr/localize.py`) and the trees of `fragtree._hr_msn` / `compare_hr` on synthetic MSn files (`TP_Mine/tests/msn_synth.py`).

    python3 tools/finder_riferimento.py [--golden]

Writes tests/golden/feat/finder.json with --golden, else prints its size. Synthetic inputs only (fixed seed), no compound names.
"""
from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path
from types import SimpleNamespace

import numpy as np

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI.parent))
sys.path.insert(0, str(AQUI.parent / "TP_Mine" / "py"))

from tpmine import api  # noqa: E402
from tpmine import chem as CH  # noqa: E402
from tpmine import fragtree as FT  # noqa: E402
from tpmine.engine import guess_sample  # noqa: E402
from tpmine.hr import formula as FO  # noqa: E402
from tpmine.hr import identity as ID  # noqa: E402
from tpmine.hr import iimn  # noqa: E402
from tpmine.hr import kinetics as KN  # noqa: E402
from tpmine.hr import similarity as SM  # noqa: E402
from tpmine.hr import spectra as SP  # noqa: E402
from tpmine.hr import localize as LZ  # noqa: E402
from tpmine.hr import molgraph as MG  # noqa: E402
from tpmine.hr import msn as MS  # noqa: E402
from tpmine.hr.engine import ExperimentHR  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402

sys.path.insert(0, str(AQUI.parent / "TP_Mine" / "tests"))
import msn_synth as MSY  # noqa: E402

FORMULAS = [
    ("C10H12N2O3S", "[M+H]+"), ("C6H6O", "[M+H]+"), ("c1ccccc1O", "[M+H]+"), ("CC(=O)Oc1ccccc1C(=O)O", "[M+Na]+"), ("CCN(CC)CC", "[M-H]-"),
    ("O=C(O)C[NH3+]", "[M+H]+"), ("C1CC1", "[M+H]+"), ("c1ccc2ccccc2c1", "[M+H]+"), ("CS(=O)(=O)N", "[M+H]+"), ("ClC(Cl)Cl", "[M+H]+"),
    ("C[C@H](N)C(=O)O", "[M+H]+"), ("[O-][N+](=O)c1ccccc1", "[M+H]+"), ("C#N", "[M+H]+"), ("OC1=CC=CC=C1", "[M+H]+"), ("Brc1ccccc1", "[M+H]+"),
    ("", "[M+H]+"), ("  ", "[M+H]+"), ("C6H6Xx", "[M+H]+"), ("C6(H6", "[M+H]+"), ("C1CC", "[M+H]+"), ("CC)C", "[M+H]+"), ("C[Zz]C", "[M+H]+"),
    ("C6H6O", "[M+K]+"), ("C6H6O", "[M+Q]+"), ("c1ccccc1", "[M+NH4]+"), ("C(C)(C)(C)(C)C", "[M+H]+"), ("CC(C)C(=O)[O-].[Na+]", "[M+H]+"),
]

NAMES = [
    "blank_01.mzML", "BLK-2.mzML", "bianco.mzML", "STD_12ppm.mzML", "cal 3.mzML", "taratura_1.mzML", "dark_t30.mzML", "ctrl-2.mzML", "buio.mzML",
    "S_t0.mzML", "S_t30.mzML", "S_t2,5h.mzML", "S_t1.5ore.mzML", "run_120min.mzML", "run_2h_b.mzML", "x_t30min(2).mzML", "t15.mzML", "plain.mzML",
    "a.b.mzML", "S_T45_rep1.mzML", "S-t 10 min.mzML", "S_30m.mzML", "S_t30m.mzML", "standardize.mzML", "control.mzML", "blank2.mzML", "x_t05min.mzML",
    "S_3,5ore_x.mzML", "dir/S_t60.mzML", "S_t60min",
]


def _list(a):
    return [float(x) for x in np.asarray(a, float)]


def kinetics_cases(rng):
    out = []
    times_sets = [[-1, 0, 5, 10, 20, 30, 60, 120], [-1, -1, 0, 15, 30, 45, 60, 90, 120, 180], [0, 2, 4, 8, 16, 32], [-1, 10, 20, 40]]
    for i in range(60):
        t = list(times_sets[i % len(times_sets)])
        if i % 7 == 3:
            t = [float(x) for x in rng.permutation(t)]
        tt = np.asarray(t, float)
        peak = rng.uniform(5, 150)
        y = rng.uniform(1e4, 1e7) * np.exp(-((tt - peak) / rng.uniform(5, 80)) ** 2) + rng.uniform(0, 1e4, len(t))
        y[tt < 0] = rng.uniform(0, 3e5) if i % 3 else 0.0
        if i % 11 == 5:
            y[:] = 0.0
        if i % 13 == 6:
            y[tt > 0] = y[tt > 0][::-1]
        d = KN.descriptors(t, y)
        ref = float(y[tt <= 0].max()) if (tt <= 0).any() else 0.0
        tr = y[tt > 0]
        out.append({"times": t, "y": _list(y), "out": None if not d.get("ok") else
                    {k: (_list(v) if isinstance(v, np.ndarray) else v) for k, v in d.items()},
                    "ref_max": ref, "absent": bool(KN.not_in_reference(t, tr, ref)) if len(tr) else None})
    return out


def spectrum(rng, prec, n):
    mz = np.sort(rng.uniform(50, prec + 3, n))
    it = rng.uniform(1, 1000, n) ** 2
    return mz, it


def consensus_cases(rng):
    out = []
    for i in range(30):
        base_mz, base_it = spectrum(rng, 300 + 10 * i, 25)
        spectra = []
        for _ in range(int(rng.integers(1, 8))):
            keep = rng.random(len(base_mz)) < 0.7
            m = base_mz[keep] * (1 + rng.normal(0, 2e-6, keep.sum()))
            o = np.argsort(m)
            spectra.append((m[o], (base_it[keep] * rng.uniform(0.5, 1.5, keep.sum()))[o]))
        if i % 9 == 4:
            spectra.append((np.zeros(0), np.zeros(0)))
        c = SP.ms2_consensus([s[0] for s in spectra], [s[1] for s in spectra])
        out.append({"spectra": [[_list(s[0]), _list(s[1])] for s in spectra],
                    "out": {"mz": _list(c["mz"]), "rel": _list(c["rel"]), "n_found": [int(x) for x in c["n_found"]], "n_scans": c["n_scans"]}})
    return out


def similarity_cases(rng):
    out = []
    for i in range(40):
        pa = float(rng.uniform(200, 500))
        delta = float(rng.choice([15.9949, -14.0157, 2.0157, 0.0, 31.9898]))
        pb = pa + delta
        ma, ia = spectrum(rng, pa, int(rng.integers(0, 45)))
        keep = rng.random(len(ma)) < 0.6
        shift = np.where(rng.random(len(ma)) < 0.4, delta, 0.0)
        mb = np.concatenate([ma[keep] + shift[keep] + rng.normal(0, 1e-3, keep.sum()), rng.uniform(50, pb, int(rng.integers(0, 12)))])
        ib = np.concatenate([ia[keep] * rng.uniform(0.3, 2, keep.sum()), rng.uniform(1, 1e5, len(mb) - keep.sum())])
        o = np.argsort(mb)
        mb, ib = mb[o], ib[o]
        sa, sb = SM.prepare(ma, ia, pa), SM.prepare(mb, ib, pb)
        r = SM.pair_components(sa, pa, sb, pb)
        out.append({"a": [_list(ma), _list(ia), pa], "b": [_list(mb), _list(ib), pb], "prep_a": [_list(sa[0]), _list(sa[1])],
                    "ent_a": _list(SM.entropy_weights(np.asarray(sa[1])[None, :])[0]) if len(sa[1]) else [], "out": json.loads(json.dumps(r, default=float))})
    return out


def _ms2(rng, prec, frag_of=None):
    """A synthetic MS2: random fragments, sometimes the lighter ion of the pair or the loss of NH3 from the precursor."""
    if rng.random() < 0.25:
        return None
    mz = list(rng.uniform(50, prec - 2, int(rng.integers(0, 14))))
    if frag_of is not None and rng.random() < 0.6:
        mz.append(frag_of + rng.normal(0, 4e-4))
    if rng.random() < 0.3:
        mz.append(prec - iimn.NH3 + rng.normal(0, 4e-4))
    if rng.random() < 0.5:
        mz.append(prec + rng.normal(0, 3e-4))
    it = list(rng.uniform(1, 1e5, len(mz)))
    return {"mz": mz, "it": it}


def identity_case(rng, pol, n_mol, S):
    """A synthetic untargeted table: molecules seen as several ions (adducts, isotopes, in-source losses, dimers) that co-elute and follow the same
    profile over the samples, plus noise features; some carry an mzMine ion identity, some mzMine edges come along."""
    forms = ID.FORMS[pol]
    feats, k = [], 0
    for m in range(n_mol):
        M = float(rng.uniform(150, 500))
        rt = float(rng.uniform(1, 20))
        prof = rng.uniform(1e4, 1e7, S) * (rng.random(S) < 0.85)
        ions = [forms[0]] + [forms[int(x)] for x in rng.choice(len(forms) - 1, int(rng.integers(0, 3)), replace=False) + 1]
        base_mz = M + forms[0][1]
        extra = []
        if rng.random() < 0.4:
            extra.append(("iso", base_mz + 1.003355))
        if rng.random() < 0.4:
            nm = list(iimn.LOSSES)[int(rng.integers(0, 8))]
            extra.append(("loss", base_mz - iimn.LOSSES[nm]))
        gid = f"g{m}" if rng.random() < 0.3 else (m if rng.random() < 0.1 else None)
        for name, shift, mult, _ in ions:
            extra.append((name, mult * M + shift))
        for _, mz in extra:
            mz = mz * (1 + rng.normal(0, 1.5e-6))
            f = rng.uniform(0.1, 2) * (1 + rng.normal(0, 0.05, S))
            a = [[int(s), float(prof[s] * f[s])] for s in range(S) if prof[s] > 0 and rng.random() < 0.95]
            fe = {"k": f"F{k}" if k % 7 else k, "id": str(k), "mz": float(mz), "rt": rt + float(rng.normal(0, 0.004)), "a": a, "ms2": _ms2(rng, mz, base_mz)}
            if gid is not None and rng.random() < 0.8:
                fe["iin"] = {"id": gid, "ions": "" if rng.random() < 0.3 else f"ion{k}", "neutral": (M if rng.random() < 0.5 else str(M)) if rng.random() < 0.6 else None}
            elif rng.random() < 0.05:
                fe["iin"] = {"id": "", "ions": "", "neutral": M}
            feats.append(fe)
            k += 1
    for _ in range(int(n_mol * 1.5)):
        feats.append({"k": f"F{k}", "id": str(k), "mz": float(rng.uniform(100, 900)), "rt": float(rng.uniform(1, 20)),
                      "a": [[int(s), float(rng.uniform(1e3, 1e6))] for s in range(S) if rng.random() < 0.7], "ms2": _ms2(rng, 500.0)})
        k += 1
    rng.shuffle(feats)
    edges = []
    for _ in range(int(rng.integers(0, 6))):
        a, b = rng.choice(len(feats), 2, replace=False)
        edges.append({"a": feats[a]["id"], "b": feats[b]["id"], "type": str(rng.choice(["ion identity", "cosine", "", "MS1 annotation"])),
                      "score": float(rng.random()) if rng.random() < 0.5 else None, "annotation": "ann" if rng.random() < 0.5 else ""})
    samples = [f"s{i}" for i in range(S)]
    opt = {"ppm": 5.0, "r_min": float(rng.choice([0.85, 0.5, 0.0])), "apex_scans": 2, "cycle_s": float(rng.choice([1.0, 3.0])), "min_common": 3}
    return {"pol": pol, "samples": samples, "features": feats, "edges": edges, "opt": opt}


def identity_cases(rng):
    out = []
    for i in range(24):
        pol = 1 if i % 3 else -1
        p = identity_case(rng, pol, int(rng.integers(1, 12)), int(rng.integers(0, 9)))
        out.append({"payload": p, "out": json.loads(api.identity_graph(json.dumps(p)))})
    return out


def polarity_cases(rng):
    out = []
    for i in range(12):
        pos, neg, num = [], [], i % 4 == 1           # the keys of one table are all text or all numbers
        for m in range(int(rng.integers(1, 30))):
            M, rt = float(rng.uniform(100, 600)), float(rng.uniform(1, 20))
            for name in ID.PAIR_POS[: int(rng.integers(1, 4))]:
                sh, mult = ID._form_table(1)[name]
                pos.append({"k": len(pos) if num else f"p{len(pos)}", "mz": (M * mult + sh) * (1 + rng.normal(0, 2e-6)), "rt": rt + float(rng.normal(0, 0.05))})
            for name in ID.PAIR_NEG[: int(rng.integers(0, 4))]:
                sh, mult = ID._form_table(-1)[name]
                neg.append({"k": f"n{len(neg)}", "mz": (M * mult + sh) * (1 + rng.normal(0, 2e-6)), "rt": rt + float(rng.normal(0, 0.1))})
        d = {"pos": pos, "neg": neg, "ppm": float(rng.choice([5.0, 2.0, 10.0])), "rt_min": float(rng.choice([0.15, 0.05]))}
        out.append({"payload": d, "out": json.loads(api.polarity_pairs(json.dumps(d)))})
    return out


TREE_SMILES = ["CN1C=NC2=C1C(=O)N(C(=O)N2C)C", "CC(C)NC1=NC(=NC(=N1)Cl)NCC", "NC(=O)N1c2ccccc2C=Cc2ccccc12",
               "OC(=O)Cc1ccccc1Nc1c(Cl)cccc1Cl", "Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1", "CC(=O)Nc1ccc(O)cc1", "OCC(O)CO", "C1CC2CCC1C2"]


def tree_case(rng, smiles, use_ion, use_smiles, lr):
    """A synthetic MS2 of the parent: common losses from the ion and random peaks, for `fragtree._two_level`."""
    neutral = CH.neutral_formula(smiles)
    ion = CH.add(neutral, {"H": 1})
    els = FO.element_order(ion)
    vec = FO.vec(ion, els)
    mz0 = float(FO.ion_mass(vec, els))
    noise = 0.15 if lr else 4e-4
    mz = [mz0 - m + rng.normal(0, noise) for _, m in list(iimn.LOSSES.items())[: int(rng.integers(2, 12))]]
    mz += list(rng.uniform(40, mz0 - 0.5, int(rng.integers(0, 10))))
    mz = [m for m in mz if m > 10]
    rel = list(rng.uniform(0.5, 100, len(mz)))
    if lr:
        mz = [round(m, 1) for m in mz]
    kw = dict(ppm=0.0 if lr else 5.0, abs_da=FT.LR_TOL if lr else 0.002, supposed=lr, mode="lr" if lr else "hr", n_scans=int(rng.integers(1, 9)))
    sm = smiles if use_smiles else None
    out = FT._two_level(vec if use_ion else None, els if use_ion else None, mz0, mz, rel, sm, **kw)
    return {"ion": FO.fmt(vec, els) if use_ion else None, "mz0": mz0, "mzs": mz, "rel": rel, "smiles": sm, **kw, "out": out}


def tree_cases(rng):
    out = []
    for s in TREE_SMILES:
        for use_ion, use_smiles, lr in ((True, True, False), (True, False, False), (False, False, True), (True, True, True), (False, True, False)):
            out.append(tree_case(rng, s, use_ion, use_smiles, lr))
    return out


def origin_case(rng):
    """A report «where does this ion come from?»: the ion and its candidate parent a step apart (adduct, loss, isotope, dimer or none), the apex
    difference, the strongest peaks of the two MS2 and the ratio constancy; any part may be missing."""
    pol = int(rng.choice([1, 1, -1]))
    par = float(rng.uniform(120, 600))
    steps = [21.981944, 17.026549, 37.955882, 18.010565, 43.989829, 46.005479, 27.994915, 35.976678, 1.003355, float(rng.uniform(1, 80))]
    st = float(rng.choice(steps)) * (1 + rng.normal(0, 2e-6))
    kind = int(rng.integers(0, 4))
    mz = par - st if kind == 0 else par + st if kind == 1 else (par - 1.007276) / 2 + 1.007276 if kind == 2 else par + float(rng.uniform(-3, 3))
    rep = {"mz": mz, "parent_mz": par}
    if rng.random() < 0.8:
        rep["profile"] = {"apex_diff_s": None if rng.random() < 0.1 else float(rng.normal(0, 4))}
    if rng.random() < 0.85:
        p, x = _ms2(rng, par, mz if mz < par else None), _ms2(rng, mz, par if par < mz else None)
        if p is not None and rng.random() < 0.3:
            p = [p["mz"], p["it"]]
        rep["ms2_peaks"] = {"p": p, "x": x}
    if rng.random() < 0.8:
        rep["ratio_constancy"] = {"cv": None if rng.random() < 0.1 else float(rng.uniform(0, 4)), "k": int(rng.integers(0, 12))}
    return {"report": rep, "pol": pol if rng.random() < 0.9 else None}


def origin_cases(rng):
    out = []
    for _ in range(60):
        d = origin_case(rng)
        out.append({"payload": d, "out": json.loads(api.origin_evidence(json.dumps(d)))})
    return out


def _plain(x):
    return json.loads(json.dumps(x, default=lambda v: v.tolist() if hasattr(v, "tolist") else float(v)))


def msn_nodes(rng, ion, smiles):
    """A synthetic MSn experiment on the ion: an MS2 of the parent, two MS3 and one MS4, with fragment formulas taken from the substructures."""
    subs = MG.substructures(MG.parse_smiles(smiles))
    els, top = subs.els, FO.vec(ion, subs.els)
    iH, iC = els.index("H"), els.index("C")
    frags = {}
    for row in subs.F:
        v = np.array(row, np.int64)
        v[iH] += 1
        if v[iC] > 0 and np.all(v <= top) and np.all(v >= 0):
            m = float(FO.ion_mass(v, els))
            if 50 < m < float(FO.ion_mass(top, els)) - 10:
                frags[FO.fmt(v, els)] = (m, v)
    names = sorted(frags, key=lambda f: frags[f][0])
    pick = [names[int(i)] for i in sorted(rng.choice(len(names), size=min(7, len(names)), replace=False))]
    nom = lambda f: int(round(frags[f][0]))
    p0 = nom(ion) if ion in frags else int(round(float(FO.ion_mass(top, els))))
    sub = lambda a, pool: [f for f in pool if f != a and np.all(frags[f][1] <= frags[a][1])]
    rel = lambda: float(rng.uniform(3, 100))
    nodes = {(): [(ion, 100.0), (float(rng.uniform(300, 390)), 3.0)], ((p0, 30),): [(ion, 100.0)] + [(f, rel()) for f in pick]}
    big = [f for f in pick[::-1] if len(sub(f, pick)) >= 2][:2]
    for b in big:
        nodes[((p0, 30), (nom(b), 35))] = [(b, 60.0)] + [(f, rel()) for f in sub(b, pick)] + [(float(rng.uniform(60, 300)), 15.0)]
    if big:
        kids = sub(big[0], pick)
        k = kids[-1]
        nodes[((p0, 30), (nom(big[0]), 35), (nom(k), 35))] = [(k, 100.0)] + [(f, rel()) for f in sub(k, kids)]
    return nodes


def msn_scans(run):
    out = []
    for s in run.scans:
        mz, it = run.read(s.index)
        out.append({"level": s.level, "path": [list(x) for x in s.path] if s.path else None, "mz": _list(mz), "it": _list(it)})
    return out


def product_ms2(rng, tree, lib, delta):
    """A synthetic MS2 of a product: library fragments unshifted or shifted by the change (or the change minus water), plus noise."""
    els = tree.els
    tp = FO.vec(tree.nodes[0]["formula"], els) + delta
    mz, rel = [], []
    for f, e in lib.entries.items():
        v = e["vec"] + (delta if rng.random() < 0.5 else 0)
        if rng.random() < 0.2 and delta[els.index("O")] > 0:
            v = v - FO.vec({"H": 2, "O": 1}, els)
        if np.all(v >= 0) and np.all(v <= tp) and v[els.index("C")] > 0:
            mz.append(float(FO.ion_mass(v, els)) * (1 + rng.normal(0, 1e-6)))
            rel.append(float(rng.uniform(1, 100)))
    for _ in range(int(rng.integers(0, 4))):
        mz.append(float(rng.uniform(60, 300)))
        rel.append(float(rng.uniform(1, 30)))
    if rel:
        rel[int(np.argmax(rel))] = 100.0
    return tp, mz, rel


CHANGES = ["+O", "-CH2", "-H2", "+O-H2", "-C2H4", "+O2", "+C"]


def change_vec(c, els):
    v = np.zeros(len(els), np.int64)
    for e, n in CH.parse_delta(c).items():
        v[els.index(e)] += n
    return v


def localize_cases(rng, tree, lib, smiles):
    out = []
    groups = LZ.default_groups(tree.subs.mol)
    ftree = FT._hr_msn(SimpleNamespace(tree=tree, smiles=smiles))
    for c in CHANGES:
        tp, mz, rel = product_ms2(rng, tree, lib, change_vec(c, tree.els))
        tpf = FO.fmt(tp, tree.els)
        tp_mz = float(FO.ion_mass(tp, tree.els))
        r = LZ.localize(tpf, tree.nodes[0]["formula"], mz, rel, lib, FO.FormulaSpace(tp, tree.els), groups=None, tp_mz=tp_mz)
        r["n_atoms"] = tree.subs.mol.n
        if r.get("ok"):
            r["n_groups"] = sum(1 for g in groups.values() if g & set(r["region_atoms"]))
        out.append({"tp": tpf, "mz": mz, "rel": rel, "tp_mz": tp_mz, "out": _plain(r), "compare": _plain(FT.compare_hr(ftree, r))})
    return out


def msn_case(rng, nodes, ion, smiles, calibrate, ppm, seed, dda):
    with tempfile.TemporaryDirectory() as d:
        run = Run(MSY.write_msn(Path(d) / "msn.mzML", nodes, ppm_offset=ppm, n_scans=5, seed=seed))
        scans = msn_scans(run)
        tree = MS.build_tree(run, ion, smiles, calibrate=calibrate)
        run.close()
    case = {"ion": ion, "smiles": smiles, "calibrate": calibrate, "scans": scans, "summary": _plain(tree.summary()), "calibration": tree.cal.as_dict(),
            "tree_summary": _plain(ExperimentHR._tree_summary(SimpleNamespace(tree=tree))), "frag_tree": _plain(FT._hr_msn(SimpleNamespace(tree=tree, smiles=smiles)))}
    if tree.subs is not None:
        lib = iimn.build_library(tree, dda_peaks=dda)
        case["dda"] = None if dda is None else [_list(dda[0]), _list(dda[1])]
        case["library"] = [{"formula": f, "mz": e["mz"], "source": sorted(e["source"]), "rel": e["rel"], "idx": None if e["idx"] is None else _list(e["idx"]),
                            "w": None if e["w"] is None else _list(e["w"])} for f, e in lib.entries.items()]
        case["localize"] = localize_cases(rng, tree, lib, smiles)
    return case


def dda_of(rng, nodes):
    """The DDA MS2 of the parent: the peaks of the MS2 node, slightly moved, some weak."""
    mz, rel = [], []
    for p, r in next(v for k, v in nodes.items() if len(k) == 1):
        if isinstance(p, str):
            mz.append(MSY.mass_of(p) * (1 + rng.normal(0, 1e-6)))
            rel.append(float(min(100.0, r * rng.uniform(0.3, 1.2))))
    mz.append(float(rng.uniform(60, 150)))
    rel.append(0.5)
    return mz, rel


def msn_cases():
    rng = np.random.default_rng(20261014)
    caf = MSY.caffeine_nodes()
    out = [msn_case(rng, caf, MSY.CAFFEINE_ION, MSY.CAFFEINE_SMILES, "auto", -3.0, 3, None),
           msn_case(rng, caf, MSY.CAFFEINE_ION, MSY.CAFFEINE_SMILES, "constant", 4.0, 5, dda_of(rng, caf)),
           msn_case(rng, caf, MSY.CAFFEINE_ION, None, "linear", -3.0, 7, None),
           msn_case(rng, caf, MSY.CAFFEINE_ION, MSY.CAFFEINE_SMILES, "none", 1.0, 9, dda_of(rng, caf))]
    for i, smi in enumerate(TREE_SMILES[1:6]):
        ion_d = CH.add(CH.neutral_formula(smi), {"H": 1})
        ion = FO.fmt(FO.vec(ion_d, FO.element_order(ion_d)), FO.element_order(ion_d))
        nodes = msn_nodes(rng, ion, smi)
        out.append(msn_case(rng, nodes, ion, smi, "auto", float(rng.uniform(-4, 4)), 11 + i, dda_of(rng, nodes) if i % 2 == 0 else None))
    return out


def main(argv):
    rng = np.random.default_rng(20261013)
    ref = {
        "formula_info": [{"text": t, "adduct": a, "out": json.loads(api.formula_info(t, a))} for t, a in FORMULAS],
        "guess": [{"name": n, "out": guess_sample(n)} for n in NAMES],
        "kinetics": kinetics_cases(rng),
        "consensus": consensus_cases(rng),
        "similarity": similarity_cases(rng),
        "identity": identity_cases(rng),
        "polarity": polarity_cases(rng),
        "tree": tree_cases(rng),
        "origin": origin_cases(rng),
        "msn": msn_cases(),
    }
    text = json.dumps(ref, sort_keys=True, allow_nan=True)
    if "--golden" in argv:
        (AQUI.parent / "tests" / "golden" / "feat" / "finder.json").write_text(text + "\n", encoding="utf-8")
    else:
        print(len(text))


if __name__ == "__main__":
    main(sys.argv[1:])
