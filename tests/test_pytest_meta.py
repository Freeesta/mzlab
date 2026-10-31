"""The two halves of the `rest` shard (tools/ci_pytest_meta.py) split the test files without overlap."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import ci_pytest_meta as m  # noqa: E402


def test_halves_partition_files():
    lenti = ["tests/slow_a.py", "tests/slow_b.py", "tests/slow_c.py"]
    files = lenti + [f"tests/test_{i}.py" for i in range(200)]
    halves = {f: m.meta_di(f, lenti) for f in files}
    assert set(halves.values()) == {"a", "b"}
    assert [halves[f] for f in lenti] == ["a", "b", "a"]
    assert 60 < sum(v == "a" for v in halves.values()) < 140
