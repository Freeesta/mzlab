"""The two halves of the `rest` shard (tools/ci_pytest_meta.py) split the tests without overlap and with about the same measured seconds."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import ci_pytest_meta as m  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]


def test_slow_files_go_to_the_lighter_half():
    slow = [("tests/s1.py", 100.0), ("tests/s2.py", 60.0), ("tests/s3.py", 50.0), ("tests/s4.py", 5.0), ("tests/solo.py", 500.0)]
    assert m.assegna(slow, frozenset({"tests/solo.py"})) == {"tests/s1.py": "a", "tests/s2.py": "b", "tests/s3.py": "b", "tests/s4.py": "a"}


def test_halves_partition_the_tests():
    assigned = {"tests/s1.py": "a", "tests/s2.py": "b"}
    split = next(iter(m.DIVISI))
    ids = ([f"tests/s{i}.py::t{j}" for i in (1, 2) for j in range(3)] + [f"tests/test_{i}.py::t" for i in range(200)]
           + [f"{split}::t[{k}]" for k in "dcba"])
    halves = m.metà(ids, assigned)
    assert len(halves) == len(ids) and set(halves) == {"a", "b"}
    assert halves[:6] == ["a"] * 3 + ["b"] * 3
    assert 60 < halves[6:206].count("a") < 140
    assert halves[206:] == ["b", "a", "b", "a"]   # by sorted node id: t[a] a, t[b] b, t[c] a, t[d] b


def test_the_list_parses_with_and_without_seconds():
    assert m.leggi("tests/a.py\t12.5\n\ntests/b.py\n") == [("tests/a.py", 12.5), ("tests/b.py", 0.0)]


def test_the_committed_list_gives_balanced_halves():
    slow = m.leggi(m.LISTA.read_text(encoding="utf-8"))
    outside = m.fuori_dal_rest()
    assigned = m.assegna(slow, outside)
    weight = {"a": 0.0, "b": 0.0}
    for f, s in slow:
        if f in m.DIVISI:
            weight["a"] += s / 2
            weight["b"] += s / 2
        elif f not in outside:
            weight[assigned[f]] += s
    assert abs(weight["a"] - weight["b"]) <= 0.1 * max(weight.values()), weight


def test_divided_files_exist_and_are_not_solo_311():
    for f in m.DIVISI:
        assert (ROOT / f).is_file() and f not in m.fuori_dal_rest(), f
