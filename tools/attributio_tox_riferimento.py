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


def stadi_tox(frame_rows, scrivi, carica_tox) -> dict:
    """The stage table: name -> function writing its golden file."""
    return {
        "tox-ghs": lambda: scrivi(USCITA_TOX, "ghs.json", stadio_tox_ghs()),
        "tox-istantanee": lambda: scrivi(USCITA_TOX, "istantanee.json", stadio_tox_istantanee(ISTANTANEE, frame_rows)),
        "tox-amenabilita": lambda: scrivi(USCITA_TOX, "amenabilita.json", stadio_tox_amenabilita()),
        "tox-b1": lambda: scrivi(USCITA_TOX, "b1.json", stadio_tox_b1(ISTANTANEE, frame_rows, carica_tox)),
        "tox-pericolo": lambda: scrivi(USCITA_TOX, "pericolo.json", stadio_tox_pericolo(ISTANTANEE)),
        "tox-registro": lambda: scrivi(USCITA_TOX, "registro.json", stadio_tox_registro()),
    }
