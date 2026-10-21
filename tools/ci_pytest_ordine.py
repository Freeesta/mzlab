"""pytest plugin of the CI (`-p ci_pytest_ordine` with PYTHONPATH=tools): the slowest test files, listed in tools/pytest_lenti.txt, are collected first, so
xdist (`--dist loadfile`) starts them at once instead of leaving one to the tail of the run. Only the order changes; no test is added or dropped."""
from pathlib import Path

LISTA = Path(__file__).with_name("pytest_lenti.txt")


def pytest_collection_modifyitems(items):
    posto = {riga.strip(): i for i, riga in enumerate(LISTA.read_text(encoding="utf-8").splitlines()) if riga.strip()}
    fine = len(posto)
    # sort is stable: inside a file and among the other files the order stays the collected one
    items.sort(key=lambda it: posto.get(it.nodeid.split("::", 1)[0], fine))
