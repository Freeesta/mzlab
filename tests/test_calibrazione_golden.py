"""The golden files of `quant.curve@1` recomputed from the inputs alone, without the engine: weighted least squares in closed form (1/x, the
golden's weight), weighted R², s_w, unweighted s_yx, residuals, back-calculation, accuracy, ICH and S/N limits, the concentrations of the CSV.
A wrong statistic in the engine (say s_yx from weighted residuals) fails here even if the golden was rewritten with it."""

import csv
import json
import math
from pathlib import Path

DATA = Path(__file__).resolve().parents[1] / "crates/mzlab-cli/tests/data/calibration"
# the plan of `flow_curve_tests.rs` (PARAMS): IS as internal standard, B|std8 left out by the user, defaults otherwise (area, linear, 1/x)
ISTD, EXCLUDED = "IS", {("B", "std8")}


def _close(got, want, what):
    assert math.isclose(got, want, rel_tol=1e-9, abs_tol=1e-12), f"{what}: engine {got} != independent {want}"


def _inputs():
    with open(DATA / "responses.csv", newline="") as fh:
        area = {(r["name"], r["sample"]): float(r["area"]) for r in csv.DictReader(fh)}
        fh.seek(0)
        snr = {(r["name"], r["sample"]): float(r["snr"]) for r in csv.DictReader(fh) if r["snr"]}
    with open(DATA / "standards.csv", newline="") as fh:
        stds = list(csv.DictReader(fh, delimiter=";"))
    for s in stds:
        s["conc"] = float(s["concentration"].replace(",", ".")) if s["concentration"] else None
        s["dilution"] = float(s.get("dilution") or 1)
    return area, snr, stds


def _fit(x, y):
    """Weighted straight line with w = 1/x by the normal equations in closed form: a different route from the engine's QR."""
    w = [1 / v for v in x]
    sw, swx, swy = sum(w), sum(a * b for a, b in zip(w, x)), sum(a * b for a, b in zip(w, y))
    swxx = sum(a * b * b for a, b in zip(w, x))
    swxy = sum(a * b * c for a, b, c in zip(w, x, y))
    b = (sw * swxy - swx * swy) / (sw * swxx - swx * swx)
    a = (swy - b * swx) / sw
    r = [yi - (a + b * xi) for xi, yi in zip(x, y)]
    ybar = swy / sw
    sst = sum(wi * (yi - ybar) ** 2 for wi, yi in zip(w, y))
    ssr_w = sum(wi * ri * ri for wi, ri in zip(w, r))
    dof = len(x) - 2
    return {"a": a, "b": b, "r2": 1 - ssr_w / sst, "s_yx": math.sqrt(sum(ri * ri for ri in r) / dof),
            "s_w": math.sqrt(ssr_w / dof), "w": w, "r": r}


def test_the_golden_json_and_csv_agree_with_an_independent_weighted_fit():
    area, snr, stds = _inputs()
    doc = json.loads((DATA / "golden.json").read_text())
    rows = {(r["analyte"], r["sample"]): r for r in csv.DictReader(open(DATA / "golden.csv", newline=""))}
    assert (doc["weight"], doc["model"], doc["zero"]) == ("1/x", "linear", False)
    assert [c["analyte"] for c in doc["curves"]] == ["A", "B"]
    for c in doc["curves"]:
        an = c["analyte"]
        used = [s for s in stds if s["role"] == "standard" and (an, s["sample"]) not in EXCLUDED]
        x = [s["conc"] for s in used]
        y = [area[(an, s["sample"])] / area[(ISTD, s["sample"])] for s in used]
        f = _fit(x, y)
        assert c["n"] == len(used)
        for k in ("a", "b", "r2", "s_yx", "s_w"):
            _close(c[k], f[k], f"{an} {k}")
        assert c["range"] == [min(x), max(x)]
        lod, loq = ({m["method"]: m["value"] for m in c[k]} for k in ("lod", "loq"))
        _close(lod["ich_3.3_s_yx_over_slope"], 3.3 * f["s_yx"] / f["b"], f"{an} LOD")
        _close(loq["ich_10_s_yx_over_slope"], 10 * f["s_yx"] / f["b"], f"{an} LOQ")
        lo = min(x)
        sn_lo = snr[(an, next(s["sample"] for s in used if s["conc"] == lo))]
        _close(lod["sn_3_lowest_standard"], 3 * lo / sn_lo, f"{an} LOD S/N")
        _close(loq["sn_10_lowest_standard"], 10 * lo / sn_lo, f"{an} LOQ S/N")
        pts = {p["sample"]: p for p in c["points"]}
        for s, xi, yi, wi, ri in zip(used, x, y, f["w"], f["r"]):
            p = pts[s["sample"]]
            assert p["used"] is True
            _close(p["y"], yi, f"{an} {s['sample']} y")
            _close(p["weight"], wi, f"{an} {s['sample']} weight")
            _close(p["residual"], ri, f"{an} {s['sample']} residual")
            back = (yi - f["a"]) / f["b"]
            _close(p["back"], back, f"{an} {s['sample']} back")
            _close(p["accuracy_pct"], 100 * back / xi, f"{an} {s['sample']} accuracy")
            _close(float(rows[(an, s["sample"])]["accuracy_pct"]), 100 * back / xi, f"{an} {s['sample']} CSV accuracy")
        for (a, s) in EXCLUDED:
            if a == an:
                assert pts[s]["used"] is False and pts[s]["reason"] == "excluded_by_user"
        # every sample of the CSV: (ratio − a)/b × dilution, and the range flag of the unknowns
        for s in stds:
            row = rows[(an, s["sample"])]
            ratio = area[(an, s["sample"])] / area[(ISTD, s["sample"])]
            back = (ratio - f["a"]) / f["b"]
            _close(float(row["response"]), ratio, f"{an} {s['sample']} response")
            _close(float(row["concentration"]), back * s["dilution"], f"{an} {s['sample']} concentration")
            if s["role"] == "sample":
                want = "below_range" if back < min(x) else "above_range" if back > max(x) else "ok"
                assert row["note"] == want, (an, s["sample"], row["note"])
