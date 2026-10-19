"""Reference stages of the Attributio port for the registry, hazard and regulatory-list rules (tox/, reglists/).

Tool only (never shipped in the site). Used by tools/attributio_riferimento.py, which passes its own helpers.
"""
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ISTANTANEE = ROOT / "tests/data/attributio/istantanee_sintetiche"
USCITA_TOX = ROOT / "tests/golden/attributio/tox"


def stadio_tox_ghs():
    """Thresholds of ecotox/ghs.py on a grid of values, taxon sets and H-statements."""
    from attributio.ecotox import ghs
    vals = [None, float("nan"), 0.005, 0.01, 0.05, 0.1, 0.5, 1, 5, 10, 50, 100, 500]
    flags = (False, True)
    out = {"acute": [[v, ghs.acute_from_value(v)] for v in vals],
           "noec": [[v, r, ghs.chronic_from_noec(v, r)] for v in vals for r in flags],
           "from_acute": [[v, r, k, b, ghs.chronic_from_acute(v, r, k, b)]
                          for v in vals for r in flags for k in (None, 3.9, 4.0) for b in (None, 100, 500)]}
    cases = [({"fish": 0.5}, {}), ({"fish": 5, "algae": 50}, {}), ({}, {}), ({"fish": 0.5, "crustacean": 20}, {"fish": 0.5}),
             ({"fish": 50, "crustacean": 0.8}, {"fish": 2, "algae": 3}), ({"fish": 5}, {"fish": 0.005, "crustacean": 0.3, "algae": 4})]
    out["classify"] = [[a, c, r, ghs.classify(a, c, r, 4.2, None)] for a, c in cases for r in flags]
    codes = [[], ["H400"], ["H401", "H402"], ["H410", "H413"], ["H412"], ["H400", "H411", "H413", "H319"], ["H402", "H401"]]
    out["h_codes"] = [[c, ghs.from_h_codes(c)] for c in codes]
    return out


def stadio_tox_istantanee(dir_: Path, frame_rows):
    """The snapshot readers (registry, membership, ECOTOX values, ECOSAR endpoints) on the synthetic snapshots."""
    from attributio.chem import registry
    from attributio.ecotox import hazard
    from attributio.reglists import schema
    registry.registry_path = lambda name="registry": dir_ / registry.FILES[name]
    hazard._paths.ecotox_dir = lambda: dir_
    hazard.clear_caches()
    snap = hazard.snapshot()
    return {"registry": frame_rows(registry.load_registry()),
            "membership": frame_rows(schema.read_membership(dir_ / "reglist_membership.csv")),
            "values": frame_rows(snap["values"]), "ecosar": frame_rows(snap["ecosar"])}


def stadio_tox_registro():
    """Pure registry rules: representative CID, synonyms, aquatic H-codes, PubChemLite categories, predicted NORMAN toxicity."""
    from attributio.chem import norman, pubchem, pubchemlite
    props = {5: {"InChIKey": "AAAAAAAAAAAAAA-UHFFFAOYSA-N"}, 3: {"InChIKey": "AAAAAAAAAAAAAA-ABCDEFGHIJ-N"},
             7: {"InChIKey": "AAAAAAAAAAAAAA-ZZZZZZZZZZ-O"}, 9: {"InChIKey": "BBBBBBBBBBBBBB-ZZZZZZZZZZ-O"},
             2: {"InChIKey": "CCCCCCCCCCCCCC-QQQQQQQQQQ-M"}, 4: {"InChIKey": "CCCCCCCCCCCCCC-RRRRRRRRRR-L"}}
    reps = []
    for block, prefer in (("AAAAAAAAAAAAAA", None), ("AAAAAAAAAAAAAA", "AAAAAAAAAAAAAA-ABCDEFGHIJ-N"), ("BBBBBBBBBBBBBB", None),
                          ("CCCCCCCCCCCCCC", None), ("DDDDDDDDDDDDDD", None)):
        reps.append([block, prefer, pubchem.representative_cid(block, props, prefer)])
    syn = [["Foo", "DTXSID123", "DTXSID1234567", "50-00-0", "1234567-89-0", "12345678-90-1"], [], ["CAS 50-00-0", "7732-18-5"]]
    out = {"props": {str(k): v["InChIKey"] for k, v in props.items()}, "representative": reps,
           "synonyms": [[s, pubchem.dtxsid_from_synonyms(s), pubchem.cas_from_synonyms(s)] for s in syn]}
    texts = [["Warning H400 Very toxic", "H410", "xH411", "H4120", "H413."], ["H401, H402 and H319"], ["nothing"], ["H410 H410 H400"]]
    codes = []
    for t in texts:
        pubchem.fetch = lambda url, offline=None, _t=t: {"A": [{"String": x} for x in _t], "n": {"String": 3}}
        codes.append([t, pubchem.ghs_aquatic(1)])
    out["h_codes"] = codes
    rows = [{c: 0 for c in pubchemlite.CATEGORIES}, {**{c: "" for c in pubchemlite.CATEGORIES}, "AgroChemInfo": "3", "ToxicityInfo": "x", "KnownUse": "1"},
            {**{c: "0" for c in pubchemlite.CATEGORIES}, "FoodRelated": "0.5", "Identification": "12"}]
    out["categories"] = [[[r[c] for c in pubchemlite.CATEGORIES], pubchemlite.categories(r)] for r in rows]
    tox = [{}, {"LC50_48_hr_ug/L": "5000"}, {"LC50_48_hr_ug/L": "5000", "EC50_72_hr_ug/L": "200", "LC50_96_hr_ug/L": "200"},
           {"LC50_96_hr_ug/L": "abc", "EC50_72_hr_ug/L": "1000000"}, {"LC50_48_hr_ug/L": "nan"}, {"LC50_48_hr_ug/L": "50000"}, {"LC50_48_hr_ug/L": "500"}]
    cols = list(norman.TOX_COLUMNS)
    pred = []
    for r in tox:
        v, w = norman.predicted_lowest_lc50_mg_l(r)
        pred.append([[r.get(c) for c in cols], v, w, norman.ghs_acute_category(v)])
    out["predicted"] = pred
    return out


def stadio_tox_pericolo(dir_: Path):
    """Hazard records of the synthetic snapshots: tiers, cross-checks, caveats, ECOSAR rules, coverage."""
    import pandas as pd
    from attributio.chem import registry
    from attributio.ecotox import hazard
    registry.registry_path = lambda name="registry": dir_ / registry.FILES[name]
    hazard._paths.ecotox_dir = lambda: dir_
    hazard.CURATION_DATA_DIR = dir_
    hazard.clear_caches()
    cases = [("Alphazol (herbicide)", None), ("Alphazol", None), ("Alphazol (herbicide) isomer, RT 5.2 min", None), ("Betamine", None), ("Gammaxin", None),
             ("Deltacide", 3), ("Epsilon", None), ("Zetafen", None), ("Unknown glycol [C4H10O3]", None), ("Thetadecane", None),
             ("Mystery compound", None), ("Nothing known", 3), ("Alphazol", 2)]
    recs = [hazard.record(n, "SYN" if i % 2 == 0 else None, lv) for i, (n, lv) in enumerate(cases)]
    table = pd.DataFrame([r.as_dict() for r in recs])
    return {"cases": [[n, lv, "SYN" if i % 2 == 0 else None] for i, (n, lv) in enumerate(cases)],
            "records": [r.as_dict() for r in recs],
            "coverage": hazard.coverage(table, pd.Series([True, False] * 6 + [True]))}


def stadio_tox_amenabilita():
    """LC-ESI amenability rules on hand cases (every branch of the decision list)."""
    from attributio.reglists import amenability as A
    nan = float("nan")
    C = [  # smiles, mono_mass, formula, charge, xlogp, skeleton
        ("", 100.0, "C5H5N", None, 1.0, "AAAAAAAAAAAAAA"), ("nan", 100.0, "C5H5N", None, 1.0, "AAAAAAAAAAAAAA"), (None, 100.0, "C5H5N", None, 1.0, ""),
        ("CBr", 93.9, "CH3Br", None, 1.2, "GZUXJHMPEANEGY"), ("C[n+]1ccc(cc1)-c1cc[n+](C)cc1", 186.1, "C12H14N2", 2, None, "INFDPOAKFNIJBF"),
        ("CCO", None, "C2H6O", None, 0.0, ""), ("CCO", 46.04, "C2H6O", None, 0.0, ""), ("C" * 120, 1700.0, "C120H242", None, 1.0, ""),
        ("O=O", 100.0, "O2", None, 0.0, ""), ("[Hg]C", 216.0, "CH3Hg", None, 1.0, ""), ("CC(=O)[O-].[Mn+2]", 200.0, "C2H3MnO2", None, -1.0, ""),
        ("CC(=O)Nc1ccc(O)cc1", 151.06, "C8H9NO2", None, 0.5, ""), ("CC(=O)Nc1ccc(O)cc1", 151.06, "C8H9NO2", 0, 0.5, ""),
        ("c1ccc(cc1)C(=O)[O-].[Na+]", 144.0, "C7H5O2Na", None, 1.0, ""), ("CC(=O)Nc1ccc(O)cc1.Cl", 187.0, "C8H10ClNO2", None, 0.5, ""),
        ("CCCCCCCCCCCCCCCC[N+](C)(C)C.[Cl-]", 284.3, "C19H42N.Cl", None, 6.0, ""), ("C[N+](C)(C)C", 74.1, "C4H12N", None, -3.0, ""),
        ("CCCCCCCCCCCCCCCC[N+](C)(C)C", 284.3, "C19H42N", None, 6.0, ""), ("c1ccccc1", 78.05, "C6H6", None, 2.1, ""), ("ClC(Cl)(Cl)C(Cl)(Cl)Cl", 233.8, "C2Cl6", None, 4.0, ""),
        ("CC", 30.0, "C2H6", None, 1.8, ""), ("CCCCCCCCCCCC", 170.2, "C12H26", None, 6.0, ""),
        ("OP(=O)(O)CNCC(=O)O", 169.0, "C3H8NO5P", None, -3.4, ""), ("NCC(=O)O", 75.03, "C2H5NO2", None, -3.2, ""), ("OCC(O)C(O)C(O)C(O)CO", 182.08, "C6H14O6", None, -3.1, ""),
        ("OC(=O)CC(O)(CC(=O)O)C(=O)O", 192.03, "C6H8O7", None, -1.7, ""), ("NC(CO)CO.OC(=O)C(F)(F)F", 205.0, "C5H8F3NO4", None, -2.5, ""),
        ("FC(F)(F)C(F)(F)C(F)(F)F", 200.0, "C6F14", None, 3.0, ""), ("CS(=O)(=O)N", 95.0, "CH5NO2S", -1, -0.5, ""), ("CC(=O)[O-]", 59.0, "C2H3O2", -1, -0.5, ""),
        ("CC[NH3+]", 46.0, "C2H8N", 1, 0.0, ""), ("CC[NH3+]", 246.0, "C2H8N", 1, 0.0, ""), ("OC(=O)CC.OC(=O)CCC", 164.0, "C7H12O4", None, 0.1, ""),
        ("c1ccccc1O", 94.04, "C6H6O", None, nan, ""), ("c1ccccc1O", 94.04, None, None, 1.5, ""), ("c1ccccc1O", 94.04, "", None, 1.5, ""), ("CN(C)C.Cl", 59.07, "C3H9N.ClH", None, 0.3, ""),
        ("CC(C)(C)N.[K+]", 73.1, "C4H11NK", None, 0.2, ""), ("CCN.[Mg+2]", 45.0, "C2H7NMg", None, 0.2, ""), ("CC[N+](C)(C)C.[Br-]", 102.0, "C5H14N.Br", None, 0.0, ""),
    ]
    return {"cases": [[c[0], None if c[1] is None else c[1], c[2], c[3], None if c[4] is None else (None if c[4] != c[4] else c[4]), c[5], list(A.lc_esi_amenable("n", c[0], c[1], c[2], c[3], c[4], c[5]))] for c in C]}


def stadio_tox_b1(dir_: Path, frame_rows, carica_tox):
    """B1 of the synthetic campaign against the invented membership table: inputs (identities, structures, list entries) and every output table."""
    import pandas as pd
    from attributio.chem import registry
    from attributio.reglists import catalog as K
    from attributio.reglists import identities as I
    from attributio.reglists import match as M
    from attributio.reglists.schema import read_membership
    registry.registry_path = lambda name="registry": dir_ / registry.FILES[name]
    C = carica_tox()
    out = {}
    for window in (None, [100.0, 400.0]):
        C.cfg.setdefault("reglists", {})
        if window:
            C.cfg["reglists"]["mz_window"] = window
        else:
            C.cfg["reglists"].pop("mz_window", None)
        sites = C.matrix_sites(I.reglists_config(C)["matrix"])
        base, _feat, _n2nc = I.campaign_identities(C, sites)
        b = M.match_campaign(C, mem_path=dir_ / "reglist_membership.csv")
        mem = read_membership(dir_ / "reglist_membership.csv")
        subs = M.submatrices(C, sites)
        out["window" if window else "nowindow"] = {
            "window": window, "sites": sites, "submatrices": subs,
            "identities_in": frame_rows(base, ["nome_comune", "group", "best_level", *[f"in_{x}" for x in sites]]),
            "structures": frame_rows(b.structures), "membership": frame_rows(mem),
            "ident": frame_rows(b.ident.drop(columns=[c for c in b.ident.columns if c in ("raw", "is_anthro", "formula")])),
            "long": frame_rows(b.long), "summary": frame_rows(b.summary), "coverage": frame_rows(b.coverage), "lists": b.lists,
            "catalog": [[l, K.spec(l).label, K.group_of(l), K.spec(l).question] for l in b.lists]}
    from attributio.reglists import compare as CMP
    mp = dir_ / "reglist_membership.csv"
    b_all, b_h, b_w = (M.match_campaign(C, mem_path=mp, lists=l) for l in (None, ["HHP_PAN", "EU_PS_2026_805"], ["WHO_MIA", "EU_PS_2026_805"]))
    for tag, res in (("two", {"ALL": b_all, "H": b_h}), ("three", {"ALL": b_all, "H": b_h, "W": b_w}), ("one", {"W": b_w})):
        t, sm = CMP.cross_table(res)
        out["p3_" + tag] = {"campaigns": list(res), "table": frame_rows(t), "summary": frame_rows(sm)}
    out["ordered"] = [K.ordered(x) for x in (["WHO_MIA", "ZZ", "HHP_PAN", "AA", "WHO_MIA"], [], ["EU_PS_2013_39"])]
    return out


def stadio_tox_schema(dir_: Path, frame_rows):
    """Schema A1 and the membership tables: read, normalise, collapse by first block, write, long and wide join tables."""
    import io
    import pandas as pd
    from attributio.reglists import schema as S
    raw = S.read_list(dir_ / "list_a1.csv")
    norm = S.normalise(raw)
    col = S.collapse_by_skeleton(raw)
    buf = io.StringIO()
    S.normalise(raw).sort_values(["inchikey_skeleton", "name"], kind="mergesort").to_csv(buf, sep=";", index=False)
    lists = {"HHP_PAN": col, "WHO_MIA": col.assign(list_id="WHO_MIA")}
    long = S.membership(lists, {"HHP_PAN": "pesticides", "WHO_MIA": "drugs"})
    wide = S.membership_wide(long, ["WHO_MIA", "EU_X"])
    mbuf = io.StringIO()
    long.to_csv(mbuf, sep=";", index=False)
    return {"read": frame_rows(raw), "normalised": frame_rows(norm), "collapsed": frame_rows(col), "written": buf.getvalue(),
            "long": frame_rows(long), "long_csv": mbuf.getvalue(), "wide": frame_rows(wide), "wide_columns": list(wide.columns)}


def _b2_ambiente(dir_: Path, frame_rows):
    """Invented features for B2/B3: the calibration of RT against logP, every adduct and class, the isolated adducts, the already identified suspects."""
    import numpy as np
    import pandas as pd
    from types import SimpleNamespace as NS
    from attributio.chem import registry
    from attributio.reglists import screen as SC
    registry.registry_path = lambda name="registry": dir_ / registry.FILES[name]
    P = 1.007276
    sites, sorbents = ["SiteA", "SiteB"], ["PA", "PE"]
    rows = []

    def add(pol, i, mz, rt, level=4, xlogp=None, formula="", a=1, b=0, pa=1, pe=0):
        rows.append({"_pol": pol, "_id": i, "mz": mz, "rt": rt, "level": level, "xlogp": xlogp, "formula": formula,
                     "in_SiteA": bool(a), "in_SiteB": bool(b), "any_PA": bool(pa), "any_PE": bool(pe)})
    noise = [0.2, -0.3, 0.4, -0.2, 0.1, -0.4, 0.3, -0.1]
    for n, (lp, e) in enumerate(zip([0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.5], noise)):
        add("POS", 101 + n, 500.1 + n, 2.0 * lp + 1.0 + e, level=2 + n % 2, xlogp=lp, formula="C20H30")
    add("POS", 109, 520.2, 20.0, level=3, xlogp=1.0)
    k, q, _, _ = SC.rt_model(pd.DataFrame(rows).assign(rt=lambda d: d["rt"].astype(float)))
    pred = lambda lp: k * lp + q
    add("POS", 1, 162.0793 + P + 0.0001, pred(1.5), formula="C9H10N2O", a=1, b=1, pa=1, pe=1)
    add("POS", 2, 162.0793 + 18.033823, pred(1.5) + 0.05, a=1)
    add("POS", 10, 162.0793 + 22.989218, pred(1.5) + 0.5, formula="C9H10N2O", b=1)
    add("POS", 3, 110.0368 + P, pred(2.1) + 5.0, formula="C6H6O2", pa=0, pe=1)
    add("POS", 4, 110.0368 + P + 0.0003, pred(2.1) + 0.1, pa=0, pe=0)
    add("POS", 5, 178.0783 + P, pred(4.5), level=3, formula="C14H10")
    add("NEG", 6, 132.0575 - P, pred(2.0) + 4.0, formula="C9H8O")
    add("NEG", 7, 132.0575 + 34.969402, pred(2.0) - 6.0)
    add("POS", 8, 112.0080 + P, pred(3.0), formula="C6H4Cl")
    add("POS", 9, 161.9139 + 22.989218, pred(1.0))
    add("POS", 11, 162.0793 + P + 0.01, pred(1.5))
    feat = pd.DataFrame(rows)
    info = {("POS", i): (i not in (4, 11), "2/3" if i == 3 else "0") for i in range(1, 12) if i != 9}
    info.update({("NEG", 6): (True, "1/3"), ("NEG", 7): (True, "0")})
    ann = {("POS", 5): "Other compound"}
    SC.ms2_and_blank = lambda C, s: info
    SC._annotated = lambda C, f, b1: ann
    import attributio.annotate as AN
    AN.load_annotations_cached = lambda C, s, verbose=False: feat
    structures = pd.DataFrame({"nome_comune": ["Fictazine", "Betamine", "Betamine"], "first_block": ["IIIIIIIIIIIIII", "BBBBBBBBBBBBBB", None]})
    b1 = NS(sites=sites, structures=structures)
    C = NS(id="synthetic", sorbents=sorbents)
    out = {"sites": sites, "sorbents": sorbents,
           "features": frame_rows(feat), "info": [[p, i, bool(m), b] for (p, i), (m, b) in info.items()],
           "annotated": [[p, i, n] for (p, i), n in ann.items()], "structures": frame_rows(structures), "cases": []}
    return SC, C, b1, out


B2_CASI = (([100.0, 400.0], ["HHP_PAN", "WHO_MIA", "EU_PS_2026_805"], {"PA": "HLB"}), (None, ["WHO_MIA"], {}))


def stadio_tox_b2(dir_: Path, frame_rows):
    """B2 on the invented features, with and without the MS1 window."""
    SC, C, b1, out = _b2_ambiente(dir_, frame_rows)
    for window, lists, labels in B2_CASI:
        cfg = {"b2_lists": lists, "mz_window": window, "sorbent_labels": labels}
        SC.reglists_config = lambda C, _c=cfg: _c
        b = SC.screen_campaign(C, b1, mem_path=dir_ / "b2_membership.csv")
        assert "Crippen" not in set(b.suspects["logp_source"]), (b.suspects[["name", "logp_source"]], "Crippen logP is not ported")
        out["cases"].append({"cfg": cfg, "suspects": frame_rows(b.suspects), "already": frame_rows(b.already), "hits": frame_rows(b.hits),
                             "summary": frame_rows(b.summary), "rt_model": list(b.rt_model), "lists": b.lists,
                             "method": SC.method_lines(b, window)})
    out["m2"] = [[f, SC.m2_ratio(f)] for f in ["C6H5Cl", "C2H2Cl2Br", "C4H3BrS2", "C5H5N", "CSi", "NaCl", "C3H8Sn", "Cl10"]]
    return out


def stadio_tox_b3(dir_: Path, frame_rows):
    """B3 on the B2 candidates: SIRIUS tables (B3 export for POS, the campaign's top-1 for NEG), a MetFrag summary and the chemist's decisions."""
    import tempfile
    import pandas as pd
    from types import SimpleNamespace as NS
    from attributio.reglists import confirm as CF
    SC, C, b1, out = _b2_ambiente(dir_, frame_rows)
    window, lists, labels = B2_CASI[0]
    cfg = {"b2_lists": lists, "mz_window": window, "sorbent_labels": labels}
    SC.reglists_config = lambda C, _c=cfg: _c
    b2 = SC.screen_campaign(C, b1, mem_path=dir_ / "b2_membership.csv")
    st = [{"mappingFeatureId": 1, "structurePerIdRank": 1, "InChIkey2D": "AAAAAAAAAAAAAA", "name": "Alphazol", "ConfidenceScoreApproximate": 0.9},
          {"mappingFeatureId": 8, "structurePerIdRank": 3, "InChIkey2D": "MMMMMMMMMMMMMM", "name": "Other", "ConfidenceScoreApproximate": 0.3},
          {"mappingFeatureId": 8, "structurePerIdRank": 1, "InChIkey2D": "ZZZZZZZZZZZZZZ", "name": "Other", "ConfidenceScoreApproximate": 0.3},
          {"mappingFeatureId": 2, "structurePerIdRank": 7, "InChIkey2D": "AAAAAAAAAAAAAA", "name": "Alphazol", "ConfidenceScoreApproximate": 0.1},
          {"mappingFeatureId": 2, "structurePerIdRank": 1, "InChIkey2D": "ZZZZZZZZZZZZZZ", "name": "Other", "ConfidenceScoreApproximate": 0.1},
          {"mappingFeatureId": 10, "structurePerIdRank": 2, "InChIkey2D": "AAAAAAAAAAAAAA", "name": "Alphazol", "ConfidenceScoreApproximate": 0.5},
          {"mappingFeatureId": 10, "structurePerIdRank": 1, "InChIkey2D": "ZZZZZZZZZZZZZZ", "name": "Other", "ConfidenceScoreApproximate": 0.5},
          {"mappingFeatureId": 3, "structurePerIdRank": 1, "InChIkey2D": "DDDDDDDDDDDDDD", "name": "Deltacide", "ConfidenceScoreApproximate": 0.7}]
    fo = [{"mappingFeatureId": 1, "formulaRank": 1, "molecularFormula": "C9H10N2O"}, {"mappingFeatureId": 8, "formulaRank": 1, "molecularFormula": "C6H5ClO"},
          {"mappingFeatureId": 8, "formulaRank": 2, "molecularFormula": "ClC6H5"}, {"mappingFeatureId": 10, "formulaRank": 1, "molecularFormula": "N2OC9H10"},
          {"mappingFeatureId": 3, "formulaRank": 1, "molecularFormula": "C6H6O2"}]
    sm = [{"mappingFeatureId": 1, "InChIkey2D": "AAAAAAAAAAAAAA", "similarity": 0.85, "sharedPeaks": 8, "analogHit": "false"},
          {"mappingFeatureId": 1, "InChIkey2D": "AAAAAAAAAAAAAA", "similarity": 0.75, "sharedPeaks": 12, "analogHit": "False"},
          {"mappingFeatureId": 2, "InChIkey2D": "AAAAAAAAAAAAAA", "similarity": 0.9, "sharedPeaks": 9, "analogHit": "true"},
          {"mappingFeatureId": 3, "InChIkey2D": "DDDDDDDDDDDDDD", "similarity": 0.8, "sharedPeaks": 5, "analogHit": "false"}]
    neg = {"structure_identifications": pd.DataFrame([{"_id": 6, "InChIkey2D": "KKKKKKKKKKKKKK", "name": "Standard A"}]),
           "formula_identifications": pd.DataFrame([{"_id": 6, "molecularFormula": "C9H8O"}])}
    tables = {"structure_identifications_top-5.tsv": pd.DataFrame(st), "formula_identifications_top-5.tsv": pd.DataFrame(fo),
              "spectral_matches_top-5.tsv": pd.DataFrame(sm)}
    metfrag = [{"pol": "POS", "fid": 8, "adduct": "[M+H]+", "suspect": "Chloroxen", "mf_rank": 2, "expl": 5, "npk": 7, "n_cand": 40}]
    decisions = [{"suspect": "Alphazol", "polarity": "", "feature_id": "", "verdetto": "respinto", "nota": "general", "decided_on": "2026-10-01", "decided_by": "fc"},
                 {"suspect": "Alphazol", "polarity": "POS", "feature_id": "10", "verdetto": "Livello 3 con riserva", "nota": "RT anchor",
                  "decided_on": "2026-10-02", "decided_by": "fc"},
                 {"suspect": "Deltacide", "polarity": "NEG", "feature_id": "", "verdetto": "rejected", "nota": "wrong polarity", "decided_on": "", "decided_by": ""}]
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        (tmp / "metfrag").mkdir()
        pd.DataFrame(metfrag).to_csv(tmp / "metfrag" / "metfrag_summary.csv", sep=";", index=False)
        pd.DataFrame(decisions).to_csv(tmp / "dec.csv", sep=";", index=False)
        CF.b3_dir = lambda C: tmp
        CF.decisions_path = lambda C: tmp / "dec.csv"
        CF._b3_exports = lambda C, pol: (Path("synthetic"), "B3 v2") if pol == "POS" else (None, "")
        CF._read_tsv = lambda d, names: tables.get(names[0], pd.DataFrame())
        C.load_sirius = lambda table, pol, missing_ok=False: neg.get(table, pd.DataFrame()) if pol == "NEG" else pd.DataFrame()
        ev, info = CF.evaluate(C, b2)
    acc = CF.accepted(ev)
    return {"hits": frame_rows(b2.hits), "sirius": {"POS": {"kind": "B3 v2", "structures": st, "formulas": fo, "matches": sm},
                                                     "NEG": {"kind": info["sirius"]["NEG"], "structures": frame_rows(neg["structure_identifications"]),
                                                             "formulas": frame_rows(neg["formula_identifications"]), "matches": []}},
            "metfrag": metfrag, "decisions": decisions, "rows": frame_rows(ev), "info": info,
            "accepted": [[r["suspect"], r["polarity"], int(r["feature_id"])] for _, r in acc.iterrows()], "method": CF.METHOD,
            "formulas": [[f, CF.parse_formula(f)] for f in ["C9H10N2O", "N2OC9H10", "ClC6H5", "C6H5Cl", "Cl10", "", "H2O", "NaCl", "CSi"]]}


def stadi_tox(frame_rows, scrivi, carica_tox) -> dict:
    """The stage table: name -> function writing its golden file."""
    return {
        "tox-ghs": lambda: scrivi(USCITA_TOX, "ghs.json", stadio_tox_ghs()),
        "tox-istantanee": lambda: scrivi(USCITA_TOX, "istantanee.json", stadio_tox_istantanee(ISTANTANEE, frame_rows)),
        "tox-amenabilita": lambda: scrivi(USCITA_TOX, "amenabilita.json", stadio_tox_amenabilita()),
        "tox-schema": lambda: scrivi(USCITA_TOX, "schema.json", stadio_tox_schema(ISTANTANEE, frame_rows)),
        "tox-b2": lambda: scrivi(USCITA_TOX, "b2.json", stadio_tox_b2(ISTANTANEE, frame_rows)),
        "tox-b3": lambda: scrivi(USCITA_TOX, "b3.json", stadio_tox_b3(ISTANTANEE, frame_rows)),
        "tox-b1": lambda: scrivi(USCITA_TOX, "b1.json", stadio_tox_b1(ISTANTANEE, frame_rows, carica_tox)),
        "tox-pericolo": lambda: scrivi(USCITA_TOX, "pericolo.json", stadio_tox_pericolo(ISTANTANEE)),
        "tox-registro": lambda: scrivi(USCITA_TOX, "registro.json", stadio_tox_registro()),
    }
