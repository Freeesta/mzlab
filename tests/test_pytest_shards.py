"""The 3.11 shards of test.yml (`rest` and `solo311`) run every test exactly once (tools/ci_pytest_shards.py)."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import ci_pytest_shards as sh  # noqa: E402


def test_shards_partition_the_collection():
    assert sh.controlla() == []


def test_a_dropped_a_duplicated_or_a_stale_test_is_reported():
    pieno = ["t.py::a", "t.py::b", "t.py::c"]
    assert sh.confronta(pieno, ["t.py::a", "t.py::b"], ["t.py::c"], [], []) == []
    assert any("no shard" in g for g in sh.confronta(pieno, ["t.py::a"], ["t.py::c"], [], []))
    assert any("both shards" in g for g in sh.confronta(pieno, ["t.py::a", "t.py::b"], ["t.py::b", "t.py::c"], [], []))
    assert any("matches no test" in g for g in sh.confronta(pieno, ["t.py::a", "t.py::b", "t.py::c"], [], ["old.py"], []))
    assert any("twice" in g for g in sh.confronta(pieno, pieno, [], [], ["t.py"]))
