"""pytest plugin of the CI (`-p ci_pytest_meta` with PYTHONPATH=tools): with CI_PYTEST_MITA=a|b only the test files of that half run, so the `rest` shard of
test.yml is split in two parallel jobs. The slow files of tools/pytest_lenti.txt alternate between the halves; every other file goes by a hash of its path.
Whole files stay together (xdist `--dist loadfile`); the two halves cover every test exactly once."""
import os
import zlib
from pathlib import Path

LISTA = Path(__file__).with_name("pytest_lenti.txt")


def meta_di(file: str, lenti: list[str]) -> str:
    """Half ('a' or 'b') that runs this test file."""
    n = lenti.index(file) if file in lenti else zlib.crc32(file.encode())
    return "ab"[n % 2]


def pytest_collection_modifyitems(config, items):
    mita = os.environ.get("CI_PYTEST_MITA")
    if not mita:
        return
    lenti = [r.strip() for r in LISTA.read_text(encoding="utf-8").splitlines() if r.strip()]
    tenuti = [it for it in items if meta_di(it.nodeid.split("::", 1)[0], lenti) == mita]
    scartati = [it for it in items if it not in tenuti]
    items[:] = tenuti
    config.hook.pytest_deselected(items=scartati)
