"""Private validation of the Rust library engine (crates/mzlab-lib) against the data repository: the same checks as tests/test_hr_identificazione.py
(which runs the JS search), through `cargo run --example valida`. Skipped without the data repository or without cargo. The compound is never
named here: the expected library entry is read from the data (the highest-scoring identity of the parent)."""
import importlib.util
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
_spec = importlib.util.spec_from_file_location("hr_ident", ROOT / "tests" / "test_hr_identificazione.py")
hr = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(hr)

pytestmark = [
    pytest.mark.skipif(hr._dati() is None, reason="MZLAB_DATI with librerie/ and HRMS/ not available"),
    pytest.mark.skipif(shutil.which("cargo") is None, reason="cargo not available"),
]


@pytest.fixture(scope="module")
def result(tmp_path_factory):
    d = hr._dati()
    q, allq = hr._queries(d["hrms"])
    tmp = tmp_path_factory.mktemp("rust_ident")
    qf = tmp / "q.json"
    qf.write_text(json.dumps({"queries": q, "all": allq}))
    p = subprocess.run(["cargo", "run", "--release", "--quiet", "-p", "mzlab-lib", "--example", "valida", "--", str(d["lib"]), str(qf)],
                       capture_output=True, encoding="utf-8", check=True, cwd=ROOT)
    return json.loads(p.stdout)


def test_the_parent_is_identified_strongly(result):
    rows = result["ident"]["parent"]
    assert rows and all(r and r["ent"] >= 0.75 and r["shared"] >= 6 and abs(r["dprec"]) <= 5 for r in rows), rows
    assert len({r["name"] for r in rows}) == 1


def test_the_products_have_no_strong_identity(result):
    for k in ("315", "291", "331", "319", "247"):
        rows = result["ident"].get(k) or []
        assert rows, k
        assert not any(r and r["ent"] >= 0.75 and r["shared"] >= 6 for r in rows), (k, rows)


def test_the_analogues_find_the_parent_with_the_expected_shifts(result):
    parent = result["ident"]["parent"][0]["name"]
    for k, (dm, mc) in hr.EXPECT_ANALOG.items():
        top = result["analog"][k]["top"]
        assert top and top[0]["name"] == parent, (k, top)
        assert abs(top[0]["delta"] - dm) <= 0.002 and abs(top[0]["mcos"] - mc) <= 0.05, (k, top[0], dm, mc)
        assert result["analog"][k]["ms"] <= 100, result["analog"][k]


def test_a_whole_file_is_searched_and_the_parent_is_the_most_frequent_hit(result):
    a = result["all"]
    assert a["ms"] <= 3000, a
    parent = result["ident"]["parent"][0]["name"]
    assert a["names"] and max(a["names"], key=a["names"].get) == parent, a["names"]
