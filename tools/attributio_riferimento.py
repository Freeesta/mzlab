#!/usr/bin/env python3
"""Reference runner: executes Attributio 1.12.0 and writes golden JSON files per stage.

Tool only (never shipped in the site). Needs ATTRIBUTIO_SRC=<path of Attributio's src/>.
Stages: campagna, letture, rilevazione, livelli, identita, proposte, decisioni, sirius, standard, abbinamenti (synthetic campaign) and sorbenti (campaign of the per-sorbent lists).

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
    ap.add_argument("--controlla", action="store_true",
                    help="compare livelli.json with livelli_attesi.csv (exit 1 on difference)")
    a = ap.parse_args()
    src = os.environ.get("ATTRIBUTIO_SRC")
    if not src:
        print("ATTRIBUTIO_SRC=<path of Attributio's src/> is required", file=sys.stderr)
        return 2
    C = carica(Path(src).expanduser().resolve(), a.campagna.resolve())
    from attributio.annotate import load_annotations
    feat = load_annotations(C)
    scrivi(a.uscita, "campagna.json", stadio_campagna(C))
    scrivi(a.uscita, "letture.json", stadio_letture(C))
    scrivi(a.uscita, "rilevazione.json", stadio_rilevazione(C))
    scrivi(a.uscita, "livelli.json", stadio_livelli(feat))
    scrivi(a.uscita, "identita.json", stadio_identita(feat, C))
    scrivi(a.uscita, "proposte.json", stadio_proposte(C))
    scrivi(a.uscita, "decisioni.json", stadio_decisioni(C, feat))
    scrivi(a.uscita, "abbinamenti.json", stadio_abbinamenti())
    scrivi(a.uscita, "formule.json", stadio_formule())
    scrivi(a.uscita, "sirius.json", stadio_sirius(C))
    scrivi(a.uscita, "standard.json", stadio_standard(C))
    scrivi(a.uscita_sorbenti, "sorbenti.json", stadio_sorbenti(carica(Path(src).expanduser().resolve(), a.sorbenti.resolve())))
    return controlla(a.uscita, a.campagna) if a.controlla else 0


if __name__ == "__main__":
    sys.exit(main())
