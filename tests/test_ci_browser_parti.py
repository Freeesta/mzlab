"""test.yml runs the browser smoke round in the halves of `verifica.py --parte K/N` and keeps the required name `browser (ubuntu-latest, chromium)` on a gate job:
the matrix lists every part of one N, the gate depends on the shard job, and the halves together are exactly the smoke list, each e2e once."""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import verifica  # noqa: E402

TEXT = (ROOT / ".github/workflows/test.yml").read_text(encoding="utf-8")


def job(name: str) -> str:
    m = re.search(rf"^  {name}:\n(.*?)(?=^  [\w-]+:\n|\Z)", TEXT, re.M | re.S)
    assert m, name
    return m.group(1)


def parts() -> list[tuple[int, int]]:
    found = re.search(r'parte: \[([^\]]+)\]', job("browser-shard"))
    assert found
    return [tuple(int(x) for x in p.strip('" ').split("/")) for p in found.group(1).split(",")]


def test_the_required_name_is_a_gate_on_the_shards():
    gate = job("browser")
    assert "name: browser (ubuntu-latest, chromium)" in gate and "needs: browser-shard" in gate and "if: ${{ always() }}" in gate
    assert "name: browser-shard" in job("browser-shard") and "browser (ubuntu-latest, chromium)" not in job("browser-shard")


def test_the_matrix_lists_every_part_of_one_n_and_the_command_passes_it():
    ps = parts()
    n = ps[0][1]
    assert sorted(k for k, _ in ps) == list(range(1, n + 1)) and {m for _, m in ps} == {n} and n >= 2
    assert "--fumo --parte ${{ matrix.parte }}" in job("browser-shard")


def test_the_halves_together_are_the_smoke_list_each_test_once():
    names = sorted(set(verifica.SMOKE) | set(verifica.SMOKE_CHROMIUM_LINUX))
    n = parts()[0][1]
    dealt = [verifica.parte_di(names, k, n) for k in range(1, n + 1)]
    assert sorted(x for part in dealt for x in part) == names


def test_what_must_run_once_runs_in_the_first_part_only():
    shard = job("browser-shard")
    assert shard.count("matrix.parte == '1/2'") == 4  # playwright cache save, .wasm sizes (two steps), cargo cache save
