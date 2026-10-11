# TPMINE-PRIVATE
"""Evidence on the link between two co-eluting ions of the same identity: adduct or in-source fragment?

The rules are those of Attributio (by Federico Cristaudo, `LICENZE-TERZI.md`): the steps below, co-elution within 0.05 min, tolerance
max(0.002 Da, 5 ppm). For a pair (X lighter, Y heavier) the module lists the evidence found and gives a label that is a SIGNAL for the chemist,
never a decision:
  - "adduct": the m/z step is in the adduct table and, when the MS2 allow the test, the fragments share a skeleton or the typical loss shows
    (-17.027 from [M+NH4]+); sodiated and potassiated ions that do not fragment are only a hint;
  - "in-source fragment?": the step is a neutral loss, the m/z of X is among the MS2 fragments of the heavier co-eluting ion Y AND the ratio Y/X
    is stable across the samples (CV shown);
  - "not distinguishable": anything else (both readings fit, or neither is supported).
The ratio constancy comes from `ionfamily.ratio_constancy` and, for a report of `origin_report`, from `isf.features` (one source of numbers, no copy).
Pure numpy; spectra are {mz: [...], it: [...]} (or (mz, intensity) arrays)."""
from __future__ import annotations

import math

import numpy as np

from mzlab import ionfamily as F

from .. import isf
from . import iimn

ABS_DA, PPM = 0.002, 5.0
COELUTION_MIN = 0.05                 # co-elution window of the edge evidence (minutes)
CV_STABLE = 0.30                     # ratio Y/X constant across samples when CV <= 30 %
MIN_SAMPLES = 3
SKELETON_MIN = 0.5                   # fraction of the fragments of X that Y also has (same m/z or shifted by the m/z step)
MAX_PEAKS = 40                       # peaks of a spectrum kept for the evidence (the page sends the strongest)
NH3 = iimn.NH3

# Steps of Attributio (physical constants): name -> m/z step (ion Y minus ion X, Y heavier) for each polarity. Every one is also read by
# `iimn.relations` (adducts of the base ion and neutral losses of the heavier ion; the opposite direction is the same step seen from the other ion).
STEPS = {
    1: {"adduct": {"Na-H": 21.981944, "NH4-H": 17.026549, "K-H": 37.955882}, "loss": {"H2O": 18.010565, "CO2": 43.989829, "HCOOH": 46.005479, "CO": 27.994915}},
    -1: {"adduct": {"HCOO": 46.005479, "Cl+H": 35.976678}, "loss": {"H2O": 18.010565, "CO2": 43.989829}},
}
TYPICAL_LOSS = {"[M+NH4]+": ("NH3", NH3)}              # neutral loss that an adduct shows in its own MS2
NO_FRAG_ADDUCTS = ("[M+Na]+", "[M+K]+", "[M+Na-2H]-")   # sodiated / potassiated: they often do not fragment (a hint, not a proof)
KIND_TEXT = {"adduct": "adduct", "isf": "in-source fragment?", "undetermined": "not distinguishable", "other": ""}


def tol(mz: float) -> float:
    return max(ABS_DA, abs(mz) * PPM * 1e-6)


def _spec(s):
    """-> (mz array, intensity array) of the strongest MAX_PEAKS peaks, or None."""
    if s is None:
        return None
    if isinstance(s, dict):
        a, i = s.get("mz"), s.get("it")
    else:
        a, i = s
    if a is None or i is None or len(a) == 0:
        return None
    a, i = np.asarray(a, float), np.asarray(i, float)
    if len(a) > MAX_PEAKS:
        k = np.argsort(-i)[:MAX_PEAKS]
        a, i = a[k], i[k]
    return a, i


def _near(spec, mz: float, prec: float | None = None):
    """The peak of `spec` closest to `mz` within the tolerance (the precursor's own peak excluded): (m/z, relative intensity %) or None."""
    a, i = spec
    keep = np.ones(len(a), bool) if prec is None else np.abs(a - prec) > 1.5 * tol(prec)
    a, i = a[keep], i[keep]
    if len(a) == 0:
        return None
    j = int(np.argmin(np.abs(a - mz)))
    return (float(a[j]), float(100 * i[j] / max(float(i.max()), 1e-300))) if abs(a[j] - mz) <= tol(mz) else None


def _ev(key: str, text: str, strength: str, **kw) -> dict:
    """One piece of evidence: strength 'evidence' (supports the reading), 'against', 'hint' (suggests, proves nothing) or 'info'."""
    return {"key": key, "text": text, "strength": strength, **kw}


def relations_of(mz_lo: float, mz_up: float, pol: int) -> list[dict]:
    """The readings of the step mz_up - mz_lo: role adduct / loss / isotope / dimer with the name and the error in ppm."""
    adducts = iimn.ADDUCT_FROM_H if pol > 0 else iimn.ADDUCT_FROM_DEPROT
    delta, out = mz_up - mz_lo, []
    for r in iimn.relations(delta, mz_lo, PPM, ABS_DA, adducts=adducts):
        if r["of"] == "x" and r["role"] in ("adduct", "isotope"):
            out.append({"role": r["role"], "name": r["name"], "ppm": (delta - r["delta"]) / mz_lo * 1e6, "step": r["delta"]})
        elif r["of"] == "y" and r["role"] == "loss":
            out.append({"role": "loss", "name": r["name"], "ppm": (delta - r["delta"]) / mz_lo * 1e6, "step": r["delta"]})
    if iimn.dimer_of(mz_lo, mz_up, PPM, ABS_DA, pol):
        out.append({"role": "dimer", "name": "[2M+H]+" if pol > 0 else "[2M-H]-", "ppm": 0.0, "step": delta})
    return out


def ratio_stats(areas_lo, areas_up, min_samples: int = MIN_SAMPLES) -> dict:
    """Ratio heavy/light over the samples where both are positive: {n, cv, mean, stable}; cv is None below `min_samples` samples."""
    lo, up = np.asarray(areas_lo, float), np.asarray(areas_up, float)
    ok = (lo > 0) & (up > 0)
    if int(ok.sum()) < min_samples:
        return {"n": int(ok.sum()), "cv": None, "stable": None}
    rc = F.ratio_constancy(list(up[ok] / lo[ok]))
    cv = rc.get("cv")
    cv = float(cv) if cv is not None and math.isfinite(cv) else None
    return {"n": int(ok.sum()), "cv": cv, "mean": float(np.mean(up[ok] / lo[ok])), "stable": None if cv is None else cv <= CV_STABLE}


def skeleton(spec_lo, spec_up, mz_lo: float, mz_up: float) -> dict:
    """Do the fragments of X also appear in Y, at the same m/z or shifted by the m/z step? -> {n, shared, frac}. The precursors' own peaks are not counted."""
    a, i = spec_lo
    a2, _ = spec_up
    keep = np.abs(a - mz_lo) > 1.5 * tol(mz_lo)
    a, i = a[keep], i[keep]
    shift = mz_up - mz_lo
    sig = np.sqrt(i)
    tot = float(sig.sum())
    shared = 0.0
    n_sh = 0
    for m, w in zip(a, sig):
        if len(a2) and (np.abs(a2 - m).min() <= tol(m) or np.abs(a2 - (m + shift)).min() <= tol(m)):
            shared += float(w)
            n_sh += 1
    return {"n": int(len(a)), "shared": n_sh, "frac": shared / tot if tot > 0 else None}


def edge_evidence(mz_lo: float, mz_up: float, pol: int, *, ms2_lo=None, ms2_up=None, areas_lo=None, areas_up=None, drt_min: float | None = None) -> dict:
    """Evidence and label of the link between X (lighter, mz_lo) and Y (heavier, mz_up). Returns {kind, label, text, relations, evidence, summary}.
    `ms2_*`: spectra of the two features (or None); `areas_*`: areas over the same samples (aligned lists); `drt_min`: |RT(Y) - RT(X)| in minutes."""
    rels = relations_of(mz_lo, mz_up, pol)
    ev: list[dict] = []
    adduct_rels = [r for r in rels if r["role"] in ("adduct", "dimer")]
    loss_rels = [r for r in rels if r["role"] == "loss"]
    if not adduct_rels and not loss_rels:
        return {"kind": "other", "label": "", "text": "", "relations": rels, "evidence": [], "summary": ""}
    s_lo, s_up = _spec(ms2_lo), _spec(ms2_up)

    # ---- the step and the co-elution
    for r in rels:
        if r["role"] in ("adduct", "dimer", "loss"):
            what = {"adduct": "adduct", "dimer": "dimer", "loss": "loss"}[r["role"]]
            ev.append(_ev("dm", f"Δm/z {mz_up - mz_lo:.4f} = {what} {r['name']} ({r['ppm']:+.1f} ppm)", "info", name=r["name"], role=r["role"], ppm=r["ppm"]))
    if drt_min is not None:
        ev.append(_ev("coelution", f"ΔRT {drt_min * 60:.1f} s ({'within' if drt_min <= COELUTION_MIN else 'beyond'} {COELUTION_MIN} min)",
                      "evidence" if drt_min <= COELUTION_MIN else "against", drt_min=drt_min))

    # ---- in-source fragment: X in the MS2 of Y, and a stable ratio Y/X
    isf_ok = False
    frag_in = None
    if loss_rels:
        if s_up is None:
            ev.append(_ev("frag_ms2", "no MS2 of the heavier ion: its fragments are unknown", "info"))
        else:
            frag_in = _near(s_up, mz_lo, mz_up)
            if frag_in:
                ev.append(_ev("frag_ms2", f"m/z {mz_lo:.4f} is a fragment in the MS2 of {mz_up:.4f} ({(frag_in[0] - mz_lo) / mz_lo * 1e6:+.1f} ppm, {frag_in[1]:.0f} % of the base peak)", "evidence", rel=frag_in[1]))
            else:
                ev.append(_ev("frag_ms2", f"m/z {mz_lo:.4f} is not among the fragments of the MS2 of {mz_up:.4f}", "against"))
    rs = None
    if areas_lo is not None and areas_up is not None:
        rs = ratio_stats(areas_lo, areas_up)
        if rs["cv"] is None:
            ev.append(_ev("ratio", f"ratio {mz_up:.4f}/{mz_lo:.4f}: fewer than {MIN_SAMPLES} common samples", "info", n=rs["n"]))
        else:
            ev.append(_ev("ratio", f"ratio {mz_up:.4f}/{mz_lo:.4f} {'stable' if rs['stable'] else 'not stable'}: CV {100 * rs['cv']:.0f} % over {rs['n']} samples",
                          "evidence" if rs["stable"] else "against", cv=rs["cv"], n=rs["n"]))
    if loss_rels:
        isf_ok = bool(frag_in) and bool(rs and rs["stable"])

    # ---- adduct: skeleton or typical loss from the MS2
    pos = neg = tl_hit = False
    if adduct_rels:
        name = adduct_rels[0]["name"]
        if s_lo is not None and s_up is not None:
            sk = skeleton(s_lo, s_up, mz_lo, mz_up)
            if sk["frac"] is None:
                ev.append(_ev("skeleton", "the lighter ion has no fragment besides its own precursor", "info"))
            elif sk["frac"] >= SKELETON_MIN:
                pos = True
                ev.append(_ev("skeleton", f"common skeleton: {sk['shared']} of {sk['n']} fragments of {mz_lo:.4f} also in {mz_up:.4f} (same m/z or shifted by Δm/z)", "evidence", frac=sk["frac"]))
            else:
                neg = True
                ev.append(_ev("skeleton", f"no common skeleton: {sk['shared']} of {sk['n']} fragments of {mz_lo:.4f} also in {mz_up:.4f}", "against", frac=sk["frac"]))
        if name in TYPICAL_LOSS and s_up is not None:
            ln, lm = TYPICAL_LOSS[name]
            hit = _near(s_up, mz_up - lm, mz_up)
            if hit:
                pos, neg, tl_hit = True, False, True
                ev.append(_ev("typical_loss", f"typical loss of {name}: -{lm:.3f} ({ln}) seen in the MS2 of {mz_up:.4f}", "evidence"))
            else:
                ev.append(_ev("typical_loss", f"typical loss of {name} (-{lm:.3f}, {ln}) not seen in the MS2 of {mz_up:.4f}", "against"))
                neg = neg or s_lo is None
        if name in NO_FRAG_ADDUCTS and s_up is not None:
            others = _spec_peaks_besides(s_up, mz_up)
            if others == 0:
                ev.append(_ev("no_fragmentation", f"{name} does not fragment in its MS2 (a hint, not a proof)", "hint"))

    if tl_hit and all(r["name"] == "NH3" for r in loss_rels):
        isf_ok = False               # the loss of NH3 in the MS2 is the typical loss of the adduct: the same peak cannot also prove the in-source fragment
    # ---- the label
    a_rel, l_rel = bool(adduct_rels), bool(loss_rels)
    if a_rel and not l_rel:
        kind = "undetermined" if neg and not pos else "adduct"
    elif l_rel and not a_rel:
        kind = "isf" if isf_ok else "undetermined"
    else:
        kind = "adduct" if pos and not isf_ok else "isf" if isf_ok and not pos else "undetermined"
    shown = [e["text"] for e in ev if e["key"] != "coelution" or e["strength"] == "against"]
    label = KIND_TEXT[kind]
    return {"kind": kind, "label": label, "text": f"{label} (evidence: {'; '.join(shown)})" if kind != "undetermined" else (f"{label}" + (f" ({'; '.join(shown)})" if shown else "")),
            "relations": rels, "evidence": ev, "summary": "; ".join(shown)}


def _spec_peaks_besides(spec, prec: float) -> int:
    a, i = spec
    return int((np.abs(a - prec) > 1.5 * tol(prec)).sum())


def feature_areas(feats_lo: dict, feats_up: dict, n_samples: int):
    """Areas of two payload features over all the samples as two aligned lists."""
    out = []
    for f in (feats_lo, feats_up):
        v = [0.0] * n_samples
        for s, a in f.get("a") or []:
            if 0 <= s < n_samples:
                v[s] = float(a)
        out.append(v)
    return out


def report_evidence(rep: dict, pol: int) -> list[dict]:
    """The same evidence for the report «where does this ion come from?» (`ionfamily.origin_report` + the strongest peaks of the MS2 of X and of P in
    `rep['ms2_peaks']`): a list of pieces of evidence only, no label. X = ion, P = candidate parent. In order: the m/z step, the co-elution, the MS2
    of the parent, the ratio across samples (CV, from `isf.features`)."""
    mz, par = float(rep["mz"]), float(rep["parent_mz"])
    lo, up = (mz, par) if mz < par else (par, mz)
    rels = relations_of(lo, up, pol)
    ev: list[dict] = []
    for r in rels:
        if r["role"] in ("adduct", "dimer", "loss", "isotope"):
            ev.append(_ev("dm", f"Δm/z {up - lo:.4f} = {r['role']} {r['name']} ({r['ppm']:+.1f} ppm)", "info"))
    pr = rep.get("profile") or {}
    d = pr.get("apex_diff_s")
    if isinstance(d, (int, float)) and math.isfinite(d):
        ev.append(_ev("coelution", f"apex difference {abs(d):.1f} s ({'within' if abs(d) <= COELUTION_MIN * 60 else 'beyond'} {COELUTION_MIN} min)", "evidence" if abs(d) <= COELUTION_MIN * 60 else "against"))
    pk = rep.get("ms2_peaks") or {}
    sp, sx = _spec(pk.get("p")), _spec(pk.get("x"))
    if sp is not None and mz < par:
        hit = _near(sp, mz, par)
        ev.append(_ev("frag_ms2", (f"m/z {mz:.4f} is a fragment in the MS2 of {par:.4f} ({(hit[0] - mz) / mz * 1e6:+.1f} ppm, {hit[1]:.0f} % of the base peak)" if hit
                                    else f"m/z {mz:.4f} is not among the fragments of the MS2 of {par:.4f}"), "evidence" if hit else "against"))
    if sp is not None and sx is not None:
        sk = skeleton(sx, sp, mz, par) if mz < par else skeleton(sp, sx, par, mz)
        if sk["frac"] is not None:
            ev.append(_ev("skeleton", f"{sk['shared']} of {sk['n']} fragments of the lighter ion also in the heavier (same m/z or shifted by Δm/z)", "evidence" if sk["frac"] >= SKELETON_MIN else "against"))
    f = isf.features(rep)
    cv = f.get("const_cv")
    n = (rep.get("ratio_constancy") or {}).get("k")
    if cv is not None and math.isfinite(cv):
        ev.append(_ev("ratio", f"ratio ion/parent {'stable' if cv <= CV_STABLE else 'not stable'}: CV {100 * cv:.0f} % over {n} samples", "evidence" if cv <= CV_STABLE else "against"))
    for r in rels:
        if r["role"] == "adduct" and r["name"] in TYPICAL_LOSS and sx is not None and mz > par:
            ln, lm = TYPICAL_LOSS[r["name"]]
            hit = _near(sx, mz - lm, mz) if sx is not None else None
            ev.append(_ev("typical_loss", f"typical loss of {r['name']} (-{lm:.3f}, {ln}) {'seen' if hit else 'not seen'} in the MS2 of the ion", "evidence" if hit else "against"))
    return ev
