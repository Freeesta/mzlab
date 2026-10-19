"""The public example files stay small and the research compound name never appears in public files."""
import hashlib
import os
import re
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from dati_percorsi import radice  # noqa: E402
ESEMPI = ROOT / "mzlab" / "web" / "esempi"
MAX_BYTES = 10 * 1024 * 1024


def _nomi():
    """Forbidden names: MZLAB_NOMI_VIETATI (comma separated) or nomi_vietati.txt in the private data repository."""
    env = os.environ.get("MZLAB_NOMI_VIETATI")
    if env:
        return [n.strip().lower() for n in env.split(",") if n.strip()]
    r = radice()
    f = r / "nomi_vietati.txt" if r else None
    if f and f.is_file():
        return [n.strip().lower() for n in f.read_text(encoding="utf-8").splitlines() if n.strip() and not n.startswith("#")]
    return []


def _word(n):
    """Whole-word, case-insensitive match (`low` is already lowercase): a forbidden name inside a longer common word is not a hit."""
    return re.compile(rb"(?<![a-z0-9])" + re.escape(n.encode("utf-8")) + rb"(?![a-z0-9])")


# Surnames that stay on purpose: CC BY attributions of third-party records and authors cited in the Teoria (see STATO.md in the private data repository). {file: {sha256(name)[:12]: exact number of occurrences}}; the names themselves must not appear here. Any extra occurrence, or one in another file, is still a hit.
ALLOWED = {
    "LICENZE-TERZI.md": {"b5ffe96ecbf4": 1},
    "tools/librerie_pacchetti.py": {"b5ffe96ecbf4": 1},
    "mzlab/web/teoria/02-fotocatalisi.html": {"b5ffe96ecbf4": 2, "d69a0ae9ffe9": 3},
    "mzlab/web/teoria/18-strategia.html": {"d69a0ae9ffe9": 1},
}


def _id(n):
    return hashlib.sha256(n.encode("utf-8")).hexdigest()[:12]


def test_no_example_file_exceeds_10_mb():
    big = {p.name: p.stat().st_size for p in ESEMPI.iterdir() if p.is_file() and p.stat().st_size > MAX_BYTES}
    assert not big, big


def test_the_forbidden_names_are_not_in_public_files():
    nomi = _nomi()
    if not nomi:
        pytest.skip("MZLAB_NOMI_VIETATI or nomi_vietati.txt in the data repository not available")
    out = subprocess.run(["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, check=True).stdout.decode("utf-8")
    hits = []
    for rel in filter(None, out.split("\0")):
        p = ROOT / rel
        if not p.is_file() or "/vendor/" in rel or rel.startswith("tests/golden/librerie/massbank_unito/"):      # the libraries may hold any compound (the records of the UniTO standards)
            continue
        low = (rel + "\n").lower().encode("utf-8") + p.read_bytes().lower()
        hits += [(rel, n) for n in nomi if len(_word(n).findall(low)) != ALLOWED.get(rel, {}).get(_id(n), 0) and _word(n).search(low)]
    assert not hits, [h[0] for h in hits]
