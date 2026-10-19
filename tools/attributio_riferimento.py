#!/usr/bin/env python3
"""Reference runner: executes Attributio 1.12.0 and writes golden JSON files per stage.

Tool only (never shipped in the site). Needs ATTRIBUTIO_SRC=<path of Attributio's src/>.
Stages: campagna, letture, rilevazione, livelli, identita, nomi, membri, proposte, decisioni, sirius, standard, abbinamenti (synthetic campaign) and sorbenti (campaign of the per-sorbent lists).

    ATTRIBUTIO_SRC=../attributio/src python3 tools/attributio_riferimento.py
    ATTRIBUTIO_SRC=../attributio/src python3 tools/attributio_riferimento.py --controlla
"""
from __future__ import annotations

import argparse
import csv
import json
import math
import os
import sys
import warnings
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CAMPAGNA = ROOT / "tests/data/attributio/campagna_sintetica"
USCITA = ROOT / "tests/golden/attributio/sintetica"
LIBRERIE = ROOT / "tests/data/attributio/librerie_sintetiche"
FONTI = ROOT / "tests/data/attributio/fonti_sintetiche"
SORBENTI = ROOT / "tests/data/attributio/campagna_sorbenti"
USCITA_SORBENTI = ROOT / "tests/golden/attributio/sorbenti"
ISTANTANEE = ROOT / "tests/data/attributio/istantanee_sintetiche"
USCITA_TOX = ROOT / "tests/golden/attributio/tox"
POLS = ("POS", "NEG")
LEVEL_COLS = ["_pol", "_id", "level", "level_sub", "level_basis", "name"]


def jsonable(v):
    """Python/pandas/numpy value -> plain JSON value (NaN -> None)."""
    if v is None:
        return None
    if hasattr(v, "item") and not isinstance(v, (list, dict, str)):
        try:
            v = v.item()
        except (ValueError, AttributeError):
            pass
    if isinstance(v, float):
        return None if math.isnan(v) or math.isinf(v) else v
    if isinstance(v, dict):
        return {str(k): jsonable(x) for k, x in v.items()}
    if isinstance(v, (list, tuple, set)):
        return [jsonable(x) for x in v]
    if isinstance(v, (str, int, bool)):
        return v
    return str(v)


def frame_rows(df, cols=None):
    cols = [c for c in (cols or list(df.columns)) if c in df.columns]
    return [{c: jsonable(row[c]) for c in cols} for _, row in df.iterrows()]


def scrivi(uscita: Path, nome: str, dati) -> None:
    uscita.mkdir(parents=True, exist_ok=True)
    testo = json.dumps(jsonable(dati), indent=1, sort_keys=True, ensure_ascii=False)
    (uscita / nome).write_text(testo + "\n", encoding="utf-8")


def carica(src: Path, campagna: Path):
    os.environ["UNTARGET_OFFLINE"] = "1"
    os.environ.setdefault("MPLBACKEND", "Agg")
    sys.path.insert(0, str(src))
    warnings.simplefilter("ignore")
    from attributio import load_campaign
    return load_campaign(campagna / "campaign.toml")


def stadio_campagna(C):
    return {"id": C.id, "name": C.name, "instrument": C.instrument,
            "sites": C.sites, "matrices": C.matrices, "sorbents": list(C.sorbents),
            "site_prefix": C.site_prefix, "polarity_tag": C.polarity_tag,
            "merged": {p: str(C.merged_table_path(p).relative_to(C.root)) for p in POLS},
            "feature_decisions": str(C.feature_decisions_csv.relative_to(C.root))
            if C.feature_decisions_csv else None}


def stadio_letture(C):
    from attributio.readers import mzmine, feature_decisions
    from attributio.readers.gnps import load_gnps_library, load_gnps_network
    out: dict = {"mzmine_tabella": {}, "mzmine_libreria": {}, "gnps_libreria": {},
                 "gnps_rete": {}, "sirius": {}}
    for pol in POLS:
        try:
            t = C.load_feature_table(pol)
            out["mzmine_tabella"][pol] = {"righe": len(t), "colonne": list(t.columns),
                                          "prime5": frame_rows(t.head(5), ["id", "mz", "rt"])}
        except FileNotFoundError:
            out["mzmine_tabella"][pol] = None
        for chiave, fn in (("mzmine_libreria", lambda: mzmine.load_mzmine_library(C, pol)),
                           ("gnps_libreria", lambda: load_gnps_library(C, pol, 15.0)),
                           ("gnps_rete", lambda: load_gnps_network(C, pol))):
            try:
                d = fn()
                out[chiave][pol] = {"righe": len(d), "colonne": list(d.columns), "prime5": frame_rows(d.head(5))}
            except Exception as e:  # missing source for this polarity
                out[chiave][pol] = {"errore": type(e).__name__}
    for tab in ("canopus", "structure", "spectral"):
        try:
            d = C.load_sirius(tab, "POS", missing_ok=True)
            out["sirius"][tab] = {"righe": len(d), "colonne": list(d.columns), "prime5": frame_rows(d.head(5))}
        except Exception as e:
            out["sirius"][tab] = {"errore": type(e).__name__}
    out["standard"] = frame_rows(C.load_standards_library().head(5))
    out["decisioni_colonne"] = list(feature_decisions.load_feature_decisions(C.feature_decisions_csv).columns)
    return out


def stadio_rilevazione(C):
    from attributio.annotate import detection_table
    return {p: frame_rows(detection_table(C, p, list(C.sites))) for p in POLS}


def stadio_livelli(feat):
    from attributio.annotate import level_sub  # noqa: F401  (level_sub is already a column)
    cols = LEVEL_COLS + ["identity_override", "level_decisive", "level_why", "decision_2b"]
    rows = frame_rows(feat, cols)
    for r, (_, f) in zip(rows, feat.iterrows()):
        r["identity"] = jsonable(f.get("identity_override")) or jsonable(f.get("name"))
        r["reasons"] = jsonable(f.get("level_why"))
    return {"features": rows}


def stadio_identita(feat, C):
    from attributio.identity import identity_table, level_counts
    sites = list(C.sites)
    it = identity_table(feat, sites)
    return {"identity_table": frame_rows(it), "level_counts": level_counts(feat, sites)}


def _tabella_curazione(percorso):
    """(pattern, canonical_name, category) rows of a curation table of the campaign, as curation._load_pattern_csv reads them."""
    import pandas as pd
    df = pd.read_csv(percorso, sep=";", keep_default_na=False, dtype=str)
    return list(df[["pattern", "canonical_name", "category"]].itertuples(index=False, name=None))


NOMI = [
    "Caffeine", "caffeine [M+H]+", "Caffeine M-H", "Fictacin B", "Fictamine isomer", "Fictastructin", "TestStandardA", "Fictazine isomer, RT 8.0 min",
    "Caffeine isomer, RT 7.5 min", "Massbank: Foo acid (LM6)|3-methyl-bar", "Foo bar (9CI)", "Foo (tentative) bar", "Atcardil", "ATCARDIL", "tudcabil",
    "hexaethylene glycol", "Polyethylene glycol", "PEG [C12H26O7]", "peg diol", "Oprea1_123", "ncgc00123", "Z1234567", "Compound 12", "Compound12",
    "x [CCS=123.4] CollisionEnergy:30 [M+H]+", "Name CollisionEnergy:20", "Name [M-H]-", "Name M+H+", "Name [M+H+]", "Name (tentative)", "zinc123",
    "Ketamine", "norketamine", "calypsol", "metoprolol", "metoprolol acid", "dutoprol", "xmetoprolol", "EXP-3174", "exp3174", "carboxylosartan",
    "carboxy-losartan acid", "trifloxystrobin cga 321113", "trifloxystrobin cga321113", "7-oxodehydroabietic", "7-oxocallitrisic", "fictbase",
    "fictbaseine", "fictbaseineine", "Fictanoid x", "fictstd-d3", "  ", "", "Atenolol acid", "cid:123", "Sulfamethoxazole (antibiotic)",
]
FORMULE_NOMI = [None, "C6H14O4"]


def stadio_nomi():
    import re
    import attributio.curation as cu
    from attributio import names
    saved = (cu.CEC_MAP, cu.NONCEC_OVERRIDE)
    cu.CEC_MAP = cu.NAME_MAP = _tabella_curazione(CAMPAGNA / "curation" / "name_map.csv")
    cu.NONCEC_OVERRIDE = _tabella_curazione(CAMPAGNA / "curation" / "category_override.csv")
    try:
        rows = []
        for n in NOMI:
            clean = cu.clean_library_name(n)
            r = {"name": n, "clean": clean, "classify": cu.classify_noncec(clean), "match_cec": list(cu.match_cec(clean)),
                 "noncec": cu.match_noncec_override(clean),
                 "identity": list(cu.curate_identity(None, None, clean)),
                 "identity_formula": list(cu.curate_identity(None, None, clean, "C1H2O3")),
                 "raw_canonical": [names.canonical_name(n, f) for f in FORMULE_NOMI],
                 "clean_canonical": [names.canonical_name(clean, f) for f in FORMULE_NOMI]}
            rows.append(r)
        sin_nome = {"none": list(cu.curate_identity(None, None, None)), "formula": list(cu.curate_identity(None, None, None, "C9H9")),
                    "given": list(cu.curate_identity("Given", "ppcp", "Caffeine")), "blank_given": list(cu.curate_identity("  ", None, "caffeine"))}
        return {"names": rows, "unnamed": sin_nome, "audit": cu.audit_cec_map()}
    finally:
        cu.CEC_MAP, cu.NONCEC_OVERRIDE = saved


def stadio_membri(feat, C):
    import attributio.curation as cu
    from attributio.chem import members
    from attributio.identity import identity_table_by_structure
    saved = (cu.CEC_MAP, cu.NONCEC_OVERRIDE)
    cu.CEC_MAP = cu.NAME_MAP = _tabella_curazione(CAMPAGNA / "curation" / "name_map.csv")
    cu.NONCEC_OVERRIDE = _tabella_curazione(CAMPAGNA / "curation" / "category_override.csv")
    try:
        ev = members.feature_evidence(C, feat)
        mem = members.identity_members(ev)
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            st = identity_table_by_structure(feat, list(C.sites))
        ev_cols = [c for c in ev.columns if c not in ("spec_reference",)] + ["spec_reference"]
        return {"evidence": frame_rows(ev, ev_cols), "members": frame_rows(mem), "by_structure": frame_rows(st)}
    finally:
        cu.CEC_MAP, cu.NONCEC_OVERRIDE = saved


def stadio_proposte(C):
    from attributio.review import build
    from attributio.review.settings import ReviewSettings
    p = build.collect(C, ReviewSettings(), fetch=False, verbose=False)
    keep = ("pol", "id", "level", "level_sub", "level_basis", "tier", "proposed", "reasons", "name")
    rows = [{k: jsonable(f.get(k)) for k in keep} for f in p["features"]]
    rows.sort(key=lambda r: (r["pol"], int(r["id"])))
    return {"settings": p.get("settings"), "features": rows}


def stadio_decisioni(C, feat):
    from attributio.readers.feature_decisions import load_feature_decisions, match_decisions
    d = load_feature_decisions(C.feature_decisions_csv)
    m = match_decisions(feat, d)
    return {"decisioni": frame_rows(d), "abbinate": frame_rows(m)}


def stadio_sirius(C):
    from attributio.readers.sirius import load_sirius_tables
    cano, st, sp = load_sirius_tables(C)
    return {"canopus": frame_rows(cano), "structure": frame_rows(st), "spectral": frame_rows(sp)}


def stadio_standard(C):
    from attributio.annotate import detection_table, feature_mgf, standards_settings
    from attributio.readers import ms2
    from attributio.readers.standards import load_decisions, mass_rt_candidates, match_reference_standards
    cfg = standards_settings(C)
    lib = C.load_standards_library()
    out: dict = {"candidati": {}, "abbinamenti": {}, "spettri": {}}
    for p in POLS:
        det = detection_table(C, p, list(C.sites))
        kw = dict(mass_tol_ppm=cfg["mass_tol_ppm"], rt_tol_min=cfg["rt_tol_min"], rt_column=cfg["rt_column"][p],
                  adduct_columns=cfg["adducts"][p])
        out["candidati"][p] = frame_rows(mass_rt_candidates(det, p, lib, **kw))
        fs = ms2.read_feature_spectra(feature_mgf(C, p))
        lj = cfg["ms2_library"][p]
        ls = ms2.read_library_spectra(lj)
        out["spettri"][p] = {"feature": sorted([[k, len(v[0]), v[2]] for k, v in fs.items()]),
                             "libreria": sorted([[n, len(v)] for n, v in ls.items()])}
        m = match_reference_standards(det, p, lib, feature_spectra=fs, library_spectra=ls,
                                      ms2_min_cosine=cfg["ms2_min_cosine"], ms2_min_matched=cfg["ms2_min_matched"],
                                      ms2_tol_da=cfg["ms2_tol_da"], precursor_tol_ppm=cfg["precursor_tol_ppm"],
                                      decisions=load_decisions(cfg["decisions"]), **kw)
        out["abbinamenti"][p] = frame_rows(m)
    return out


def stadio_sorbenti(C):
    from attributio.annotate import detection_table
    from attributio.sorbents import blank_abundance
    df = C.load_feature_table("POS")
    cols = ["id", "mz", "rt", "fragment_scans", "blank_abundance:OA", "blank_abundance:EC", "twin_id"]
    return {"tabella": frame_rows(df, cols), "gemelli": {str(k): v for k, v in C.twin_map("POS").items()},
            "rilevazione": frame_rows(detection_table(C, "POS", list(C.sites))),
            "colonne": list(df.columns),
            "blank_abundance": [[t, blank_abundance(t)] for t in ("", "x (abundance 1.3E3)", "abundance 5)", "(abundance 2) (abundance 7)")]}


def _pool_spettri():
    """Nine invented MS2 spectra around the campaign's standard spectrum (fixed seed): shifted
    m/z, dropped and extra peaks (one below 1 %, one in the precursor region), other precursors."""
    import random
    base = json.loads((CAMPAGNA / "library/ms2_pos.json").read_text(encoding="utf-8"))
    rnd = random.Random(20260101)
    pool = []
    for k in range(9):
        prec = round(base["precursor_mz"] + (0, 14.01565, 0, 28.0313, 0, 15.9949, 0, 79.9568, 2.0157)[k], 4)
        pk = [[round(m + rnd.uniform(-0.006, 0.006) + (0.0 if k % 3 else 10.0), 4),
               round(i * rnd.uniform(0.5, 1.5), 1)] for m, i in base["peaks"]]
        pk = [x for j, x in enumerate(pk) if j not in rnd.sample(range(6), k % 3)]
        pk.append([round(rnd.uniform(60, 90), 4), round(rnd.uniform(0.5, 6.0), 2)])
        pk.append([round(prec - 0.3, 4), round(rnd.uniform(200, 900), 1)])
        pk.append([round(prec + 5.0 - 0.2 * k, 4), 50.0])
        pool.append({"mz": [x[0] for x in pk], "it": [x[1] for x in pk], "prec": prec})
    return pool


def _abbinamento_coppie(R):
    import numpy as np
    pool = _pool_spettri()
    tutte = [(i, j) for i in range(len(pool)) for j in range(len(pool)) if i != j]
    out = []
    for n, (i, j) in enumerate(tutte[::2][:30]):
        a, b = pool[i], pool[j]
        ea, eb = (np.array(a["mz"]), np.array(a["it"]), a["prec"]), (np.array(b["mz"]), np.array(b["it"]), b["prec"])
        shift = round(a["prec"] - b["prec"], 6)
        low = 105.0 if n % 2 == 0 else None
        out.append({"i": i, "j": j, "shift": shift, "low_limit": low,
                    "plain": R.match(ea, eb, 0.02), "shifted": R.match(ea, eb, 0.02, shift=shift),
                    "vis": R.match(ea, eb, 0.02, shift=shift, low_limit=105.0),
                    "compare": R.compare(ea, eb, 0.02, low, shift)})
    return {"pool": pool, "coppie": out}


def _abbinamento_librerie(L):
    import numpy as np
    files = {}
    for p in sorted(list(LIBRERIE.iterdir()) + [CAMPAGNA / "library/ms2_pos.json"]):
        fmt = L.sniff(p)
        recs = [] if fmt not in L.PARSERS else [
            {"name": r["name"], "prec": r["prec"], "adduct": r["adduct"], "pol": r["pol"], "ik1": r["ik1"],
             "entry": r["entry"], "n_peaks": len(r["peaks"])} for r in L.PARSERS[fmt](p)]
        files[p.name] = {"format": fmt, "records": recs}
    found = [[str(p.relative_to(LIBRERIE)), f] for p, f in L.find_library_files(LIBRERIE)]
    adducts = ["[M+H]+", "M+H", "[M + H]+", "[M+Na]+", "[M-H]-", "M-H-", "[M+NH4]+", "[2M+H]+", "[M-H2O+H]+",
               "", "garbage", "[M+FA-H]-", "M+Cl"]
    return {"files": files, "found": found, "adduct_keys": {a: L.adduct_key(a) for a in adducts}}


def _abbinamento_fonti():
    import pandas as pd
    from attributio.review import sources as S

    class FakeC:
        def load_sirius(self, table, pol=None, *, missing_ok=False):
            frames = []
            for p in ([pol] if pol else list(POLS)):
                f = FONTI / f"sirius_{p}" / f"{table}.tsv"
                if not f.exists():
                    continue
                t = pd.read_csv(f, sep="\t", low_memory=False)
                t["_pol"] = p
                idcol = "mappingFeatureId" if "mappingFeatureId" in t.columns else "SIRIUS_mappingFeatureId"
                t["_id"] = pd.to_numeric(t[idcol], errors="coerce")
                frames.append(t.dropna(subset=["_id"]).assign(_id=lambda x: x["_id"].astype(int)))
            if not frames:
                return pd.DataFrame(columns=["_pol", "_id"])
            return pd.concat(frames, ignore_index=True)

        def gnps_output_dir(self, pol):
            return FONTI / f"gnps_{pol}"

    C = FakeC()
    por_chiave = lambda d: [{"pol": k[0], "id": k[1], "items": v} for k, v in sorted(d.items())]
    return {"sirius_hits": frame_rows(S.sirius_hits(C)), "sirius_formulas": por_chiave(S.sirius_formulas(C)),
            "csi_top5": por_chiave(S.csi_top5(C)), "network_edges": frame_rows(S.network_edges(C)),
            "gnps_hits": frame_rows(S.gnps_hits(C))}


def _abbinamento_impostazioni():
    from dataclasses import asdict
    from types import SimpleNamespace
    from attributio.review.settings import ReviewSettings
    testo = ('[review]\nppm_ion = 3.5\ntop_n = 8\nraw_dir = "{lab}/raw"\nexclude_raw_prefixes = ["Po_X", "Qq"]\n'
             'lc_gradient = [[0, 5], [10, 95.5]]\nrt_logp_floor = 1\n')
    import tomllib
    cfg = tomllib.loads(testo)["review"]
    S = ReviewSettings.for_campaign(SimpleNamespace(review_cfg=cfg, config_path=Path("campaign.toml")))
    try:
        ReviewSettings.for_campaign(SimpleNamespace(review_cfg={"ppm_ionn": 1}, config_path=Path("campaign.toml")))
        err = None
    except ValueError as e:
        err = str(e)
    return {"defaults": asdict(ReviewSettings()), "toml": testo, "parsed": asdict(S), "unknown_key_error": err}


def stadio_abbinamenti():
    """Review matching, delta-m formulas, library parsers, hit tables and settings (AT-05)."""
    from attributio.review import libraries as L
    from attributio.review import matching as R
    spettri = _abbinamento_coppie(R)
    pairs = spettri["coppie"]
    deltas = [14.01565, -14.01565, 15.994915, 28.0313, 42.010565, 79.956815, 176.032088, 2.01565, 13.979265,
              58.0, 0.5, -27.994915, 18.010565, 162.052824, 100.0, 14.0, 305.07, -43.99]
    deltas += [p["shift"] for p in pairs]
    formulas = [{"delta": d, "tol_mda": 5.0, "n": 3, "out": R.delta_formulas(d, 5.0, 3)} for d in deltas]
    formulas += [{"delta": d, "tol_mda": 1.0, "n": 2, "out": R.delta_formulas(d, 1.0, 2)} for d in deltas[:6]]
    return {"pool": spettri["pool"], "coppie": pairs, "delta": formulas, "librerie": _abbinamento_librerie(L),
            "fonti": _abbinamento_fonti(), "impostazioni": _abbinamento_impostazioni()}


def stadio_formule():
    """Masses of 20 formulas/adducts, ion m/z, closest common adduct, structure fit and norm_name (AT-02)."""
    from attributio.chem import formula as F
    from attributio.chem.checks import norm_name
    cases = [("C10H12N2O", "[M+H]+"), ("C9H8O4", "[M-H]-"), ("C8H10N4O2", "[M+Na]+"), ("C17H19NO3", "[M+NH4]+"),
             ("C12H22O11", "[M+K]+"), ("C6H12O6", "[M-H2O+H]+"), ("C16H36N+", "[M]+"), ("C7H5NO3S", "[M+Cl]-"),
             ("C21H27N7O14P2", "[M+2H]2+"), ("C2H4O2", "[M+HCOO]-"), ("C15H10O5", "[M-2H]2-"), ("C6H5Cl", "M+H"),
             ("C4H11N", "[2M+H]+"), ("C14H18N2O5", "[M + H3N + H]+"), ("C9H13N5O3", "[M+CH3COO]-"), ("C5H5N5", "[M+ACN+H]+"),
             ("C3H7NO2S", "mz_MH_pos"), ("C20H25N3O", "[M-H2O-H]-"), ("C8H18O3", "[M-e]+"), ("C12H10Br2", "[M-H]-")]
    items = []
    for f, a in cases:
        m = F.formula_mass(f)
        p = F.parse_adduct(a)
        mz = F.ion_mz(m, a) if m is not None else None
        pol = "POS" if (p and p[2] > 0) else "NEG"
        obs = None if mz is None else mz * (1 + 3e-6)
        ba = F.best_common_adduct(obs, F.neutral_equivalent(m, f), pol) if obs else None
        items.append({"formula": f, "adduct": a, "mass": m, "parsed": list(p) if p else None, "mz": mz, "pol": pol,
                      "observed": obs, "closest": list(ba) if ba else None, "charge": F.formula_charge(f),
                      "neutral": None if m is None else F.neutral_equivalent(m, f)})
    fits = []
    for f, a, pre, pol in [("C10H12N2O", "[M+H]+", 177.1022, "POS"), ("C10H12N2O", "[M+Na]+", 177.1022, "POS"),
                           ("C9H8O4", "[M-H]-", 179.0350, "NEG"), ("C16H36N+", "[M]+", 242.2842, "POS"),
                           ("C10H12N2O", "[M+H]+", 300.0, "POS"), (None, "[M+H]+", 100.0, "POS"),
                           ("C10H12N2O", "[M+H]+", None, "POS")]:
        e, ok = F.structure_fit(f, a, pre, pol)
        fits.append({"formula": f, "adduct": a, "precursor": pre, "pol": pol, "ppm": e, "ok": None if ok != ok else bool(ok)})
    names = ["Caffeine (standard)", "Café au lait", "2-Methyl-1H-indole", "  Ibuprofen ", "N,N-Diethyl-m-toluamide (DEET) (x)",
             "Naïve Zoë", "", "α-Tocopherol"]
    return {"items": items, "fits": fits, "names": [{"in": n, "out": norm_name(n)} for n in names]}


def _gauss(rt, c, w, h):
    return [h * math.exp(-0.5 * ((t - c) / w) ** 2) for t in rt]


def stadio_spiegazioni():
    """Review proofs: sub-formula explanation, isotope pattern and comparison, MS2 purity, raw proofs and RT versus logP (AT-06)."""
    import numpy as np
    from attributio.review import explain as X
    from attributio.review import isotopes as ISO
    from attributio.review import purity as PU
    from attributio.review import raw as RAW
    from attributio.review import rtlogp as RL
    from attributio.review.settings import ReviewSettings
    S = ReviewSettings()
    losses = [18.010565, 27.994915, 17.026549, 46.005479, 44.026215, 28.0313, 56.0626, 31.98983, 79.9568, 14.01565]
    cases = [("C10H12N2O", "POS"), ("C17H19NO3", "POS"), ("C8H10N4O2", "POS"), ("C15H15ClN2O", "POS"), ("C9H8O4", "NEG"),
             ("C12H10N2O3S", "POS"), ("C6H5NO3", "NEG"), ("C21H23ClN2O3", "POS"), ("C16H16Br2O2", "NEG"), ("C9H13N5O3", "POS")]
    expl = []
    for k, (f, pol) in enumerate(cases):
        fd = X.parse_formula(f)
        ion = X.ion_mass(fd, pol)
        mz = ion * (1 + (k % 3 - 1) * 1.5e-6)
        raw_mz, raw_it = [], []
        for j, loss in enumerate(losses):
            m = ion - loss
            if m > 40:
                raw_mz.append(m * (1 + (j % 4 - 1.5) * 2e-6))
                raw_it.append(1000.0 / (j + 1) + 7 * k)
        raw_mz += [ion, ion + 1.00335, 61.0123 + k * 0.01, 77.04 + k, 151.5 + k, 90.0002 + k * 3.1]
        raw_it += [800.0, 120.0, 40.0, 9.0, 30.0, 0.5]
        order = np.argsort(raw_mz)
        raw_mz = [round(float(raw_mz[i]), 6) for i in order]
        raw_it = [float(raw_it[i]) for i in order]
        em, ei, idx = X.prep_ms2(raw_mz, raw_it, mz)
        ex = X.explain(fd, mz, pol, (em, ei), S)
        expl.append({"formula": f, "pol": pol, "mz": mz, "raw_mz": raw_mz, "raw_it": raw_it,
                     "prep_mz": [float(v) for v in em], "prep_it": [float(v) for v in ei],
                     "removed": sorted([i, w] for i, w in X.removed_reasons(raw_mz, raw_it, mz).items()),
                     "iso": list(X.iso_pattern(ex["ion"])) if ex["ion"] else None,
                     "adduct": ex["adduct"], "ion_formula": ex["ion_formula"], "ion_ppm": ex["ion_ppm"],
                     "n_expl": ex["n_expl"], "frac_expl": ex["frac_expl"], "decoy_frac": ex["decoy_frac"],
                     "frags": [None if h is None else h for h in ex["frags"]]})
    iso = []
    for f, pol in cases + [("C6H12O6", "POS"), ("C24H40N6O12S2", "POS")]:
        fd = X.parse_formula(f)
        ion = X.ion_mass(fd, pol)
        ionf = dict(fd)
        ionf["H"] = ionf.get("H", 0) + (1 if pol == "POS" else -1)
        sticks = ISO.pattern(ionf, pol)
        obs_mz = [m + 0.0006 for m, _ in sticks[:5]]
        obs_it = [a * (1 + 0.04 * i) * 1000 for i, (_, a) in enumerate(sticks[:5])]
        cmp_ = ISO.compare(ionf, pol, obs_mz, obs_it, sticks[0][0] + 0.0006, 0.06, 0.35, 10.0, 12.0)
        iso.append({"ion": ionf, "pol": pol, "sticks": [list(x) for x in sticks], "clusters": ISO.clusters(sticks),
                    "obs_mz": obs_mz, "obs_it": obs_it, "prec": sticks[0][0] + 0.0006, "cmp": cmp_})
    return {"spiegazioni": expl, "isotopi": iso, "purezza": _spiegazioni_purezza(PU, RAW, ISO, X, S),
            "rtlogp": _spiegazioni_rtlogp(RL)}


def _spiegazioni_purezza(PU, RAW, ISO, X, S):
    import numpy as np
    prec = 301.1415
    mz = [299.12, 300.1389, prec, prec + 1.00336, prec + 2.0067, 302.35, 303.0102, 301.6]
    it = [12.0, 3.0, 1000.0, 168.0, 21.0, 90.0, 150.0, 4.0]
    iso = PU.isolation_purity(mz, it, prec, prec, 1.0, 1.0, [(prec, 100.0), (prec + 1.00336, 17.0)], "POS")
    iso2 = PU.isolation_purity(mz, it, prec, prec, 1.5, 1.5, None, "POS")
    pm = [41.04, 55.05, 77.04, 91.054, 105.07, 119.086, 133.1, 150.2, 195.1, 226.3, 301.14, 340.0]
    pi = [20.0, 30.0, 100.0, 80.0, 60.0, 10.0, 5.0, 50.0, 3.0, 40.0, 500.0, 70.0]
    imp = PU.impossible_losses(pm, pi, prec, 0.05, 0.005, None)
    em, ei = [77.04, 91.054, 105.07, 119.086, 150.2], [100.0, 80.0, 60.0, 10.0, 50.0]
    rm, ri = [77.0402, 91.0541, 105.0702, 133.0999, 150.19, 280.0], [90.0, 100.0, 40.0, 30.0, 20.0, 15.0]
    expl = [True, False, True, False, False]
    qs = [PU.candidate_scores((em, ei, prec), (rm, ri, prec), 0.02, low, sh, expl) for low, sh in ((None, 0.0), (70.0, 0.0), (70.0, 14.01565))]
    unex = [PU.unexplained(ei, expl, None), PU.unexplained(ei, expl, np.array([False, True, False, False, False]))]
    chance = [PU.chance_probability(n, r, m, 0.02, 400.0) for n, r, m in ((5, 6, 3), (30, 40, 12), (8, 8, 8), (1, 1, 0))]
    coel = [{"mz": 302.35, "rel": 0.09, "r": 0.93, "offset": 0.02}, {"mz": 303.01, "rel": 0.15, "r": 0.2, "offset": 0.4}]
    bad = PU.badge(iso, coel, 0.55, imp, qs[0])
    good = PU.badge(None, [], 0.1, [], qs[1])
    low = [PU.low_information(q) for q in qs]
    rt = [4.0 + 0.01 * i for i in range(61)]
    g1, g2 = _gauss(rt, 4.3, 0.04, 1e5), _gauss(rt, 4.34, 0.05, 4e4)
    co = [RAW.coelution(rt, g1, g2, 4.3), RAW.coelution(rt, g1, [0.0] * 61, 4.3), RAW.coelution(rt[:3], g1[:3], g2[:3], 4.3)]
    y = [v + 300 * (1 + (i % 3)) for i, v in enumerate(_gauss(rt, 4.3, 0.04, 1e5))]
    br = [v * 0.001 for v in y]
    pq = [RAW.peak_quality(rt, y, 4.3, rt, br), RAW.peak_quality(rt, y, 4.3), RAW.peak_quality(rt, [0.0] * 61, 4.3)]
    ion = {"C": 17, "H": 20, "N": 1, "O": 3}
    info = {"low": 50.0, "high": 700.0, "target": prec, "lo": 1.0, "hi": 1.0,
            "ms1_mz": [p for p in ISO.pattern(ion, "POS")[0:1] for p in [prec]] + [prec + 1.00336, prec + 2.0067, 302.2, 301.6],
            "ms1_it": [1000.0, 190.0, 25.0, 130.0, 70.0]}
    m1 = [RAW.ms1_metrics(info, prec, ion, S), RAW.ms1_metrics(info, 400.0, ion, S), RAW.ms1_metrics(info, prec, None, S)]
    return {"iso": iso, "iso2": iso2, "imposs": imp, "qs": qs, "unexplained": unex, "chance": chance, "coel_rows": coel,
            "badge_bad": bad, "badge_good": good, "low": low, "coelution": co, "peak_quality": pq, "ms1_info": info,
            "ms1_ion": ion, "ms1": m1, "inputs": {"mz": mz, "it": it, "prec": prec, "pm": pm, "pi": pi, "em": em, "ei": ei, "rm": rm,
                                                    "ri": ri, "expl": expl, "rt": rt, "g1": g1, "g2": g2, "y": y, "blank": br}}


def _spiegazioni_rtlogp(RL):
    x = [0.2 + 0.31 * i for i in range(24)]
    y = [1.1 + 0.9 * v + 0.15 * math.sin(3 * i) for i, v in enumerate(x)]
    X = [[v, 150 + 12 * i, 20 + (i * 7) % 40, i % 6, i % 4, (i * 3) % 7] for i, v in enumerate(x)]
    pts = [{"xlogp3": v, "crippen": v + 0.2 * math.cos(i), "y": y[i], "class": ("acid", "base", "neutral")[i % 3],
            "mw": X[i][1], "tpsa": X[i][2], "nrot": X[i][3], "hbd": X[i][4], "hba": X[i][5]} for i, v in enumerate(x)]
    model = RL.calibrate(pts, 3.0, 0.0)
    model_small = RL.calibrate(pts[:10], 3.0, 0.0)
    lp = {"xlogp3": 2.5, "crippen": 2.4, "class": "base", "mw": 220, "tpsa": 30, "nrot": 3, "hbd": 1, "hba": 4}
    checks = [RL.check(model, lp, 3.6, 3.0, 0.0), RL.check(model, lp, 9.0, 3.0, 0.0), RL.check(model, {"xlogp3": None, "crippen": None}, 3.0, 3.0, 0.0),
              RL.check(model_small, lp, 3.0, 3.0, 0.0)]
    grad = [[0.0, 5.0], [2.0, 5.0], [12.0, 95.0], [14.0, 95.0], [14.1, 5.0]]
    return {"line": RL.fit_line(x, y), "line_short": RL.fit_line(x[:2], y[:2]), "mlr": RL.fit_mlr(X, y), "mlr_short": RL.fit_mlr(X[:5], y[:5]),
            "points": pts, "model": model, "model_small": model_small, "lp": lp, "checks": checks,
            "organic": [[t, RL.organic_fraction(t, grad, 0.4)] for t in (0.1, 1.0, 2.0, 5.5, 13.0, 14.05, 20.0)], "gradient": grad}


FRAGMENTI_SMILES = [
    "Cn1cnc2c1c(=O)n(C)c(=O)n2C", "CC(C)Cc1ccc(cc1)C(C)C(=O)O", "OC(=O)Cc1ccccc1Nc1c(Cl)cccc1Cl", "NC(=O)N1c2ccccc2C=Cc2ccccc12",
    "CCNc1nc(Cl)nc(NC(C)C)n1", "CC(C)NCC(O)COc1cccc2ccccc12", "Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1", "CC(=O)Nc1ccc(O)cc1",
    "COc1ccc2cc(ccc2c1)C(C)C(=O)O", "CC(C(=O)c1cccc(c1)C(=O)c1ccccc1)c1ccccc1", "CC(C)(c1ccc(O)cc1)c1ccc(O)cc1",
    "NCCc1ccccc1", "COc1ccccc1OCC(O)CN", "COCCc1ccc(OCC(O)CNC(C)C)cc1", "CCN(CC)CC(=O)Nc1c(C)cccc1C",
    "[O-][N+](=O)NC1=NCCN1Cc1ccc(Cl)nc1", "CN(C)CC1CCCCC1(O)c1cccc(OC)c1", "CC(C)NCC(O)c1ccc(O)c(O)c1",
    "OC(=O)c1ccccc1OC(C)=O", "Clc1ccc(Cl)c(c1)Oc1ccc(Cl)cc1O", "CCOC(=O)c1ccc(N)cc1", "NC(=O)c1cccnc1",
    "OC(=O)CCCCC(=O)O", "CCCCOC(=O)c1ccc(O)cc1", "COC(=O)c1ccccc1C(=O)OC", "CC(=O)OCC(=O)C1CCC2C1(C)CCC1C2CCC2=CC(=O)CCC12C",
    "c1ccc(cc1)-c1ccccc1", "OC(=O)C(O)C(O)C(O)=O", "CC(N)Cc1ccccc1", "O=C(O)c1ccccc1-c1ccccc1", "CN1CCC(CC1)Oc1ccccc1",
    "Nc1ccc(cc1)S(N)(=O)=O", "CC(C)(C)NCC(O)c1ccc(O)c(CO)c1", "OCCN(CCO)c1ccccc1", "CCCN(CCC)c1c(cc(cc1[N+](=O)[O-])C(F)(F)F)[N+](=O)[O-]",
    "c1ccc2c(c1)[nH]c1ccccc12", "Oc1ccc(cc1)C(=O)OCC", "NC1=NC(=O)c2[nH]cnc2N1", "CC1=C(C(=O)NC2=CC=CC=C2)SCCO1",
]


def stadio_frammenti():
    """Systematic bond cleavage of 40 structures against peaks built from their own fragments (AT-06; needs RDKit)."""
    from attributio.review.fragmenter import fragment_molecule
    out = []
    for k, smi in enumerate(FRAGMENTI_SMILES):
        pol = "POS" if k % 4 else "NEG"
        base = fragment_molecule(smi, pol=pol, exp_peaks=[], ppm_tol=10.0)
        if not base.get("ok"):
            out.append({"smiles": smi, "pol": pol, "ok": False})
            continue
        sides = [fr for b in base["bonds"] for fr in (b["frag1"], b["frag2"]) if fr]
        peaks = []
        for j, fr in enumerate(sides):
            if j % 2 == 0 or not peaks:
                mz = fr["mz_pos"] if pol == "POS" else fr["mz_neg"]
                if "O" in fr["formula"] and j % 3 == 0:
                    mz -= 18.010565
                peaks.append(round(mz * (1 + ((j % 5) - 2) * 1.8e-6), 5))
        peaks += [round(103.7 + k * 1.3, 4), round(251.234 + k, 4)]
        peaks = sorted(set(peaks))
        r = fragment_molecule(smi, pol=pol, exp_peaks=peaks, ppm_tol=10.0)
        out.append({"smiles": smi, "pol": pol, "ok": True, "peaks": peaks, "n_bonds": len(r["bonds"]),
                    "bonds": [[b["label"], (b["frag1"] or {}).get("formula"), (b["frag2"] or {}).get("formula")] for b in r["bonds"]],
                    "explained": [[e["peak"], e["formula"], e["mz_th"], e["ppm"], e["loss"], e["bond_label"]] for e in r["explained_peaks"]]})
    return {"molecole": out}


ESPORTAZIONE = ROOT / "tests/data/attributio/campagna_esportazione"
GRADIENTE = ROOT / "tests/data/attributio/campagna_gradiente"
USCITA_ESPORTAZIONE = ROOT / "tests/golden/attributio/esportazione"


def foglio(nome, df):
    """DataFrame -> {name, head, rows}: a missing value is null, the numbers stay numbers."""
    import pandas as pd
    righe = [[None if (not isinstance(v, (list, dict, tuple, set)) and pd.isna(v)) else jsonable(v) for v in r]
             for r in df.itertuples(index=False, name=None)]
    return {"name": nome, "head": [str(c) for c in df.columns], "rows": righe}


def campagna_esportazione(src: Path):
    """A temporary copy of the synthetic campaign with the overlay of the exports (calibration, decisions with evidence), loaded."""
    import shutil
    import tempfile
    tmp = Path(tempfile.mkdtemp())
    shutil.copytree(CAMPAGNA, tmp / "c")
    shutil.copytree(ESPORTAZIONE, tmp / "c", dirs_exist_ok=True)
    return carica(src, tmp / "c"), tmp


def stadio_quantificazione(src):
    from attributio.annotate import load_annotations
    from attributio.quantify import quantify_lvl1, quantify_lvl1_environmental
    C, _ = campagna_esportazione(src)
    feat = load_annotations(C)
    q = quantify_lvl1(C, feat)
    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")
        e = quantify_lvl1_environmental(C, feat)
    avvisi = [str(x.message) for x in w if "quantify_lvl1_environmental" in str(x.message)]
    return {"lvl1": foglio("lvl1", q), "env": foglio("env", e), "avviso": avvisi[0] if avvisi else None}


def stadio_schymanski(src):
    import pandas as pd
    from attributio.diagnostics import schymanski_export as se
    from attributio.identity import identity_table
    C, _ = campagna_esportazione(src)
    diag = C.cfg.get("diagnostics", {})
    matrix = diag.get("matrix") or next(iter(C.matrices))
    sites = C.matrix_sites(matrix)
    short = {s: C.site_meta[s]["short"] for s in sites}
    feat = se.build_feature_frame(C, sites)
    it_all = identity_table(feat, sites, drop_groups=())
    blocks = {"id": [], "feat": [], "exc": []}
    for level in (1, 2, 3):
        by_id, features, excluded = se.build_identity_sheet(C, feat, it_all, level, sites, short)
        if level == 1:
            by_id = se.attach_lvl1_quant(C, by_id, feat, sites, short)
        for d in (by_id, features, excluded):
            if not d.empty:
                d.insert(0, "schymanski_level", se.LEVEL_TAG[level])
        blocks["id"].append(by_id)
        blocks["feat"].append(features)
        blocks["exc"].append(excluded)

    def cat(bs):
        bs = [b for b in bs if len(b)]
        return pd.concat(bs, ignore_index=True) if bs else pd.DataFrame()

    by_identity, features, excluded = cat(blocks["id"]), cat(blocks["feat"]), cat(blocks["exc"])
    if not by_identity.empty:
        by_identity = by_identity[[c for c in se._FRONT if c in by_identity.columns]
                                  + [c for c in by_identity.columns if c not in se._FRONT]]
    hojas = [foglio(n, d if not d.empty else pd.DataFrame({e: []}))
             for n, d, e in (("by_identity", by_identity, "(no identities)"), ("features", features, "(no features)"),
                             ("excluded", excluded, "(no excluded identities)"))]
    hojas.append(foglio("notes", se._notes(C, sites, C.calibration_csv is not None)))
    return {"sheets": hojas}


def _senza_dati_di_pericolo(pt):
    """Stand-in of the hazard record without snapshot data: the record of a name that the registry does not know."""
    from attributio.ecotox import hazard as hz

    def record(name, campaign_id=None, level=None):
        rec = hz.HazardRecord(identity=name)
        caveats = []
        if hz.ISOMER.search(name):
            caveats.append("RT isomer: hazard of the named structure, the peak may be another isomer")
        if level == 3:
            caveats.append("Level 3: hazard of the CSI:FingerID candidate structure")
        caveats.append("no structure in the registry")
        rec.caveat = "; ".join(caveats)
        return rec
    pt._hazard.record = record


def stadio_pubblicazione(src):
    import pandas as pd
    from attributio.diagnostics import publication_table as pt
    C, _ = campagna_esportazione(src)
    _senza_dati_di_pericolo(pt)
    vuoto = (pd.DataFrame(columns=["inchikey_first_block"]), pd.DataFrame(columns=["name", "first_block", "campaign"]))
    res = pt.build(C, "all", registry=vuoto)
    rd = pt.readme(res, "Title", "Description", {})
    return {"identities": foglio("Identities", res["Identities"]), "excluded": foglio("Excluded", res["Excluded"]),
            "readme": foglio("README", rd), "counts": res["counts"], "sites": res["sites"], "short": res["short"],
            "csv": res["Identities"].to_csv(sep=";", decimal=".", index=False, encoding="utf-8")}


def stadio_gradiente(src):
    from attributio import gradient as gr
    from attributio.annotate import load_annotations
    from attributio.diagnostics import publication_table as pt
    import pandas as pd
    C = carica(src, GRADIENTE)
    sites = list(C.sites)
    vero = pt.rt_logp_flags

    def senza_l3(feat):          # the reference stops when no Level 3 feature rests on CSI:FingerID: no flags then
        try:
            return vero(feat)
        except ValueError:
            return pd.DataFrame(columns=["_pol", "_id", "rt_logp_resid", "rt_logp_z", "rt_logp_outlier", "rt_logp_note"])
    pt.rt_logp_flags = senza_l3
    feat = load_annotations(C)
    gt, dd = pt.canonical_identities(feat, sites)
    fr = pt.feature_frame(C, feat, gt, sites)
    ident = dd.rename(columns={"nome_comune": "canonical_name"})
    fi = fr.rename(columns={"nome_comune": "canonical_name"})[["_pol", "_id", "rt", "canonical_name"]]
    out = {"sites": sites, "identities": foglio("identities", ident), "feature_identities": foglio("fi", fi)}
    for nome, same in (("stesso_sorbente", None), ("senza_sorbente", False)):
        tab, summ = gr.identity_gradient(C, feat, ident, sites[0], sites[1], 3.0,
                                         feature_identities_fn=lambda _f: fi, same_sorbent=same)
        out[nome] = {"table": foglio(nome, tab), "summary": jsonable(summ.__dict__)}
    nuvola = gr.feature_cloud_gradient(C, feat, sites[0], sites[1])
    cols = ["_pol", "_id", "area_us", "area_ds", "status", "log2FC_us_ds", "abs_area"]
    out["cloud"] = foglio("cloud", nuvola[cols])
    ar = gr.compute_gradient_areas(C, tuple(sites))
    out["areas"] = {f"{p}|{s}": {str(k): v for k, v in sorted(m.items())} for (p, s), m in sorted(ar.items())}
    ab = gr.compute_gradient_areas_by_sorbent(C, tuple(sites))
    out["areas_sorbent"] = {f"{p}|{s}|{b}": {str(k): v for k, v in sorted(m.items())} for (p, s, b), m in sorted(ab.items())}
    out["same_sorbent_default"] = gr.same_sorbent_default(C)
    return out


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


def stadio_tox_istantanee(dir_: Path):
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


def controlla(uscita: Path, campagna: Path) -> int:
    got = json.loads((uscita / "livelli.json").read_text(encoding="utf-8"))["features"]
    with open(campagna / "livelli_attesi.csv", encoding="utf-8", newline="") as fh:
        att = list(csv.DictReader(fh, delimiter=";"))
    if len(got) != len(att):
        print(f"FAIL: {len(got)} features, {len(att)} expected")
        return 1
    bad = 0
    for g, a in zip(got, att):
        g = {k: ("" if v is None else str(v)) for k, v in g.items()}
        for gk, ak in (("level", "level"), ("level_sub", "level_sub"),
                       ("level_basis", "level_basis"), ("name", "name")):
            if g[gk] != a[ak]:
                print(f"DIFF {g['_pol']} {g['_id']} {gk}: {g[gk]!r} != {a[ak]!r}")
                bad += 1
    print("OK" if not bad else f"{bad} differences")
    return 1 if bad else 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--campagna", type=Path, default=CAMPAGNA)
    ap.add_argument("--uscita", type=Path, default=USCITA)
    ap.add_argument("--sorbenti", type=Path, default=SORBENTI)
    ap.add_argument("--uscita-sorbenti", type=Path, default=USCITA_SORBENTI)
    ap.add_argument("--uscita-esportazione", type=Path, default=USCITA_ESPORTAZIONE)
    ap.add_argument("--solo", nargs="+", metavar="STADIO", help="write only these stages (e.g. spiegazioni frammenti)")
    ap.add_argument("--controlla", action="store_true",
                    help="compare livelli.json with livelli_attesi.csv (exit 1 on difference)")
    a = ap.parse_args()
    src = os.environ.get("ATTRIBUTIO_SRC")
    if not src:
        print("ATTRIBUTIO_SRC=<path of Attributio's src/> is required", file=sys.stderr)
        return 2
    root = Path(src).expanduser().resolve()
    C = carica(root, a.campagna.resolve())
    from attributio.annotate import load_annotations
    feat = load_annotations(C)
    esp = a.uscita_esportazione
    stadi = {
        "quantificazione": lambda: scrivi(esp, "quantificazione.json", stadio_quantificazione(root)),
        "schymanski": lambda: scrivi(esp, "schymanski.json", stadio_schymanski(root)),
        "pubblicazione": lambda: scrivi(esp, "pubblicazione.json", stadio_pubblicazione(root)),
        "gradiente": lambda: scrivi(esp, "gradiente.json", stadio_gradiente(root)),
        "campagna": lambda: scrivi(a.uscita, "campagna.json", stadio_campagna(C)),
        "letture": lambda: scrivi(a.uscita, "letture.json", stadio_letture(C)),
        "rilevazione": lambda: scrivi(a.uscita, "rilevazione.json", stadio_rilevazione(C)),
        "livelli": lambda: scrivi(a.uscita, "livelli.json", stadio_livelli(feat)),
        "identita": lambda: scrivi(a.uscita, "identita.json", stadio_identita(feat, C)),
        "nomi": lambda: scrivi(a.uscita, "nomi.json", stadio_nomi()),
        "membri": lambda: scrivi(a.uscita, "membri.json", stadio_membri(feat, C)),
        "proposte": lambda: scrivi(a.uscita, "proposte.json", stadio_proposte(C)),
        "decisioni": lambda: scrivi(a.uscita, "decisioni.json", stadio_decisioni(C, feat)),
        "abbinamenti": lambda: scrivi(a.uscita, "abbinamenti.json", stadio_abbinamenti()),
        "formule": lambda: scrivi(a.uscita, "formule.json", stadio_formule()),
        "sirius": lambda: scrivi(a.uscita, "sirius.json", stadio_sirius(C)),
        "standard": lambda: scrivi(a.uscita, "standard.json", stadio_standard(C)),
        "spiegazioni": lambda: scrivi(a.uscita, "spiegazioni.json", stadio_spiegazioni()),
        "frammenti": lambda: scrivi(a.uscita, "frammenti.json", stadio_frammenti()),
        "tox-ghs": lambda: scrivi(USCITA_TOX, "ghs.json", stadio_tox_ghs()),
        "tox-istantanee": lambda: scrivi(USCITA_TOX, "istantanee.json", stadio_tox_istantanee(ISTANTANEE)),
        "tox-registro": lambda: scrivi(USCITA_TOX, "registro.json", stadio_tox_registro()),
        "sorbenti": lambda: scrivi(a.uscita_sorbenti, "sorbenti.json", stadio_sorbenti(carica(root, a.sorbenti.resolve()))),
    }
    for nome in a.solo or list(stadi):
        stadi[nome]()
    return controlla(a.uscita, a.campagna) if a.controlla else 0


if __name__ == "__main__":
    sys.exit(main())
