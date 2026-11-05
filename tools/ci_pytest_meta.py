"""pytest plugin of the CI (`-p ci_pytest_meta` with PYTHONPATH=tools): with CI_PYTEST_MITA=a|b only the tests of that half run, so the `rest` shard of
test.yml is split in two parallel jobs. The files of tools/pytest_lenti.txt go to the halves by their measured seconds (slowest first, each to the lighter half);
every other file goes by a hash of its path. The files of DIVISI split their tests between the halves; the two halves cover every test exactly once."""
import os
import zlib
from pathlib import Path

LISTA = Path(__file__).with_name("pytest_lenti.txt")
SOLO_311 = Path(__file__).with_name("pytest_solo_311.txt")
# one file longer than a quarter of its half sets the floor of that half (xdist --dist loadfile keeps a file in one worker): the 8 families of
# test_lettori_parita are 8 cargo builds of mzlab-lettori-wasm, about 14 min on a cold cache
DIVISI = {"tests/test_lettori_parita.py"}


def leggi(testo: str) -> list[tuple[str, float]]:
    """(file, seconds) of pytest_lenti.txt, slowest first; a line without seconds counts 0."""
    out = []
    for riga in testo.splitlines():
        p = riga.split()
        if p:
            out.append((p[0], float(p[1]) if len(p) > 1 else 0.0))
    return out


def assegna(lenti: list[tuple[str, float]], fuori: frozenset = frozenset()) -> dict[str, str]:
    """Half of each slow file: slowest first, each to the half with fewer seconds so far (a DIVISI file weighs half on each side; `fuori`: files the rest shard never runs)."""
    peso = {"a": 0.0, "b": 0.0}
    out = {}
    for f, s in sorted(lenti, key=lambda x: (-x[1], x[0])):
        if f in fuori:
            continue
        if f in DIVISI:
            peso["a"] += s / 2
            peso["b"] += s / 2
            continue
        h = min("ab", key=lambda k: peso[k])
        out[f] = h
        peso[h] += s
    return out


def meta_di(file: str, assegnate: dict[str, str]) -> str:
    """Half ('a' or 'b') that runs this test file (not a DIVISI one)."""
    return assegnate.get(file) or "ab"[zlib.crc32(file.encode()) % 2]


def metà(items_nodeid: list[str], assegnate: dict[str, str]) -> list[str]:
    """Half of each node id, in order: a DIVISI file alternates its tests by sorted node id, any other file goes whole."""
    per_file: dict[str, list[str]] = {}
    for n in items_nodeid:
        per_file.setdefault(n.split("::", 1)[0], []).append(n)
    posto = {n: i for f, ns in per_file.items() if f in DIVISI for i, n in enumerate(sorted(ns))}
    return ["ab"[posto[n] % 2] if n in posto else meta_di(n.split("::", 1)[0], assegnate) for n in items_nodeid]


def fuori_dal_rest() -> frozenset:
    """Whole files of tools/pytest_solo_311.txt: only the solo311 shard runs them."""
    return frozenset(r.strip() for r in SOLO_311.read_text(encoding="utf-8").splitlines() if r.strip() and "::" not in r)


def pytest_collection_modifyitems(config, items):
    mita = os.environ.get("CI_PYTEST_MITA")
    if not mita:
        return
    assegnate = assegna(leggi(LISTA.read_text(encoding="utf-8")), fuori_dal_rest())
    halves = metà([it.nodeid for it in items], assegnate)
    tenuti = [it for it, h in zip(items, halves) if h == mita]
    scartati = [it for it, h in zip(items, halves) if h != mita]
    items[:] = tenuti
    config.hook.pytest_deselected(items=scartati)
