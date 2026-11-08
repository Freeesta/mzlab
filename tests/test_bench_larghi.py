"""Smeared-peak endpoints of the benchmark (valida.abbina.match_wide, bench.metriche.larghi, bench.confronto_larghi): hand-built cases for every endpoint, the
narrow matching unchanged, an oracle table that must score perfectly."""
import json
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
from bench import confronto_larghi as CL, metriche as M  # noqa: E402
from bench.verita import da_json, from_rows  # noqa: E402
from valida import sintetici_mzml as SM  # noqa: E402
from valida.abbina import match_points, match_wide  # noqa: E402

NAN = float("nan")


def _real(id_, role, mz, rt, fwhm_s, rt_min, rt_max, area, morf, su=None):
    r = {"id": id_, "ruolo": role, "mz": mz, "rt": rt, "fwhm_s": fwhm_s, "rt_min": rt_min, "rt_max": rt_max, "area_attesa": area, "morfologia": morf, "presente": 1}
    return r if su is None else dict(r, su=su)


TRUTH = from_rows([
    _real("B1", "broad", 300.0, 10.0, 120, 8.0, 12.0, [0, 100, 100], "a_gauss"),
    _real("B2", "broad", 400.0, 15.0, 60, 14.0, 16.0, [0, 50, 50], "b_doppio"),
    _real("N1", "narrow", 500.0, 20.0, 8, 19.9, 20.1, [0, 10, 10], "stretto"),
    _real("N2", "narrow", 300.0005, 10.2, 8, 10.1, 10.3, [0, 10, 10], "stretto_su_banda", su="B1"),
])


def _row(mz, rt, aree, rt_min=None, rt_max=None):
    return {"mz": mz, "rt": rt, "rt_min": rt_min, "rt_max": rt_max, "aree": aree, "altezze": None, "candidato": None, "livello": None, "punteggio": None}


def _tab(rows, name="t"):
    return {"formato": "bench/1", "strumento": name, "dataset": "L", "config": "c", "file": ["blank", "a", "b"], "righe": rows}


# B1 is found (r0) and split (r2: overlaps the interval), N2 has its own row (r1), B2 and N1 are lost, r3 is spurious
TAB = _tab([_row(300.0, 10.4, [0, 90, 130], 8.5, 12.5), _row(300.0005, 10.2, [0, 9, 9], 10.1, 10.3), _row(300.0, 9.2, [0, 5, 5], 8.0, 10.0), _row(250.0, 5.0, [0, 1, 1])])


def test_p1_narrow_and_wide_separated():
    m = M.larghi(TRUTH, TAB)
    assert m["P1_narrow"]["richiamo"] == 0.5 and m["P1_narrow"]["n"] == 2
    assert m["P1_narrow"]["per_morfologia"] == {"stretto": {"n": 1, "richiamo": 0.0}, "stretto_su_banda": {"n": 1, "richiamo": 1.0}}
    assert m["P1_broad"]["richiamo"] == 0.5 and m["P1_broad"]["per_morfologia"]["b_doppio"]["richiamo"] == 0.0


def test_p2_median_relative_area_error():
    p2 = M.larghi(TRUTH, TAB)["P2_broad"]
    assert p2["per_composto"] == [pytest.approx(0.10), None] and p2["mediana"] == pytest.approx(0.10) and p2["n_misurati"] == 1 and p2["n"] == 2


def test_p3_rows_per_band_without_the_narrow_peak_on_top():
    p3 = M.larghi(TRUTH, TAB)["P3_split"]
    assert p3["per_composto"] == [2, 0] and p3["media"] == 1.0 and p3["mediana"] == 1.0 and p3["divise"] == 0.5


def test_p5_spurious_rows():
    assert M.larghi(TRUTH, TAB)["P5_fp"] == {"righe": 4, "spurie": 1, "frazione": 0.25}


def test_without_rows_and_without_compounds():
    m = M.larghi(TRUTH, _tab([]))
    assert m["P1_broad"]["richiamo"] == 0.0 and m["P2_broad"]["mediana"] is None and m["P3_split"]["per_composto"] == [0, 0] and m["P5_fp"]["frazione"] is None
    m = M.larghi([], TAB)
    assert m["P1_broad"]["richiamo"] is None and m["P5_fp"]["spurie"] == 4


def test_window_of_the_wide_ones():
    # band FWHM 2 min at RT 10, interval 8-12: tolerance max(0.15, 0.7) = 0.7 min
    def find(rt, rt_min=NAN, rt_max=NAN, wide_one=True):
        ia, _ = match_wide([300.0], [10.0], [2.0], [wide_one], [8.0], [12.0], [300.0], [rt], [rt_min], [rt_max])
        return len(ia) == 1
    assert find(10.69) and not find(10.75)                       # 0.35 * FWHM
    assert find(8.2, 8.0, 10.0) and not find(8.2, 8.0, 8.9)       # beyond the tolerance: only the overlap (IoU 0.5 vs 0.225) accepts
    assert find(10.1, wide_one=False) and not find(10.2, wide_one=False)     # a narrow truth keeps +-0.15


def test_narrow_ones_stay_as_abbina():
    rng = np.random.default_rng(3)
    for _ in range(40):
        na, nb = int(rng.integers(1, 12)), int(rng.integers(1, 12))
        mz_a, rt_a = 300 + rng.uniform(0, 0.004, na), rng.uniform(5, 5.4, na)
        mz_b, rt_b = 300 + rng.uniform(0, 0.004, nb), rng.uniform(5, 5.4, nb)
        a = match_points(mz_a, rt_a, mz_b, rt_b)
        b = match_wide(mz_a, rt_a, np.full(na, NAN), np.zeros(na, bool), np.full(na, NAN), np.full(na, NAN), mz_b, rt_b, np.full(nb, NAN), np.full(nb, NAN))
        assert all(np.array_equal(x, y) for x, y in zip(a, b))


def test_oracle_table_scores_the_maximum():
    rows, _, _ = SM.build_wide(5, n_larghi=18, n_stretti=6, n_fondo=10, n_campioni=2)
    verita = from_rows(rows)
    oracle = _tab([_row(r["mz"], r["rt"], r["area_attesa"], r["rt_min"], r["rt_max"]) for r in rows])
    m = M.larghi(verita, oracle)
    assert m["P1_narrow"]["richiamo"] == 1.0 and m["P1_broad"]["richiamo"] == 1.0
    assert m["P2_broad"]["mediana"] == 0.0 and m["P3_split"]["media"] == 1.0 and m["P3_split"]["divise"] == 0.0 and m["P5_fp"]["spurie"] == 0


def test_three_column_comparison_table(tmp_path):
    (tmp_path / "v.json").write_text(json.dumps({"ioni": [dict(r, rt_obs=[r["rt"]]) for r in TRUTH]}))
    verita = da_json(tmp_path / "v.json")
    assert verita[0]["fwhm_s"] is not None and verita[0]["rt_min"] is not None            # the lot's fields survive the loader
    md = CL.a_markdown(CL.comparison(verita, {"MZmine default": TAB, "MZmine wide": _tab([]), "mzLab default": TAB}))
    rows = md.strip().split("\n")
    assert rows[0] == "| endpoint | MZmine default | MZmine wide | mzLab default |" and len(rows) == 2 + len(CL.RIGHE)
    assert "| P1_broad (recall) | 0.50 | 0.00 | 0.50 |" in rows and "| P2_broad (median rel. area error) | 0.10 | - | 0.10 |" in rows
