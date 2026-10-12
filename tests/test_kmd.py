"""Kendrick mass defect (mzlab/web/kmd-calc.js, run with node): the series of a repeating unit share the KMD, unrelated ions do not join, the retention trend is only evidence."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def node(code: str):
    out = subprocess.run(["node", "-e", f"const K=require({str(WEB / 'kmd-calc.js')!r});" + code], check=True, capture_output=True, text=True, encoding="utf-8").stdout
    return json.loads(out)


# [M-H]- of the perfluoroalkyl carboxylic acids C4..C10 (exact masses of the formulas CnHF(2n-1)O2, minus the electron)
PFCA = [212.97918, 262.97599, 312.97279, 362.96959, 412.96640, 462.96320, 512.96001]
RT = [3.1, 4.6, 5.9, 7.0, 8.0, 8.9, 9.7]


def test_the_units_have_the_exact_masses():
    r = node("console.log(JSON.stringify(K.UNITS.map(u => [u.id, +u.mass.toFixed(5), u.nominal])))")
    assert r == [["CF2", 49.99681, 50], ["CF2O", 65.99172, 66], ["C2F4", 99.99361, 100], ["CH2", 14.01565, 14]]


def test_a_homologous_series_has_one_kmd_and_a_retention_trend():
    feats = [{"id": i, "mz": mz, "rt": rt} for i, (mz, rt) in enumerate(zip(PFCA, RT))] + [{"id": 99, "mz": 301.1234, "rt": 6.0}, {"id": 98, "mz": 455.3141, "rt": 2.0}]
    r = node(f"console.log(JSON.stringify(K.analyse({json.dumps(feats)}, {{unit: 'CF2', tolMDa: 3, minMembers: 3}})))")
    assert len(r["series"]) == 1
    s = r["series"][0]
    assert s["members"] == list(range(7)) and s["distinct"] == 7 and s["steps"] == [1] * 6 and s["consecutive"] == 1
    assert s["spanMDa"] < 1.0 and s["rho"] > 0.99
    assert all(x["series"] is None for x in r["rows"] if x["id"] in (98, 99)), "unrelated ions never join the series"


def test_a_missing_member_shows_as_a_step_of_two_and_a_short_series_is_not_reported():
    gap = [PFCA[0], PFCA[1], PFCA[3], PFCA[4]]
    r = node(f"console.log(JSON.stringify(K.analyse({json.dumps([{'id': i, 'mz': m, 'rt': 1 + i} for i, m in enumerate(gap)])}, {{minMembers: 3}})))")
    assert r["series"][0]["steps"] == [1, 2, 1] and abs(r["series"][0]["consecutive"] - 2 / 3) < 1e-9
    r2 = node(f"console.log(JSON.stringify(K.analyse({json.dumps([{'id': i, 'mz': m} for i, m in enumerate(PFCA[:2])])}, {{minMembers: 3}})))")
    assert r2["series"] == [] and all(x["series"] is None for x in r2["rows"])


def test_another_unit_regroups_the_same_ions_and_spearman_handles_ties():
    r = node("console.log(JSON.stringify([K.kendrick(413.0, K.unitById('CH2')).kmd, K.spearman([1,2,3,4],[1,2,3,4]), K.spearman([1,2,3],[3,2,1]), K.spearman([1,2],[1,2]), K.spearman([1,1,1],[1,2,3])]))")
    assert abs(r[0] - (round(413.0 * 14 / 14.01565006) - 413.0 * 14 / 14.01565006)) < 1e-6 and r[1:] == [1, -1, None, None]
