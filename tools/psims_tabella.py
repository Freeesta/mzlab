"""Builds `crates/mzlab-core/data/psims-cv.json`, the vocabulary table of the mzML validator (`mzlab_core::mzmlvalida`), from the PSI-MS ontology `psi-ms.obo`
(HUPO-PSI/psi-ms-CV, CC BY 4.0, declared in LICENZE-TERZI.md). The obo is pinned by tag and SHA-256, so the table is reproducible.

    python3 tools/psims_tabella.py                 # fetch the pinned obo (cached in the temp folder), rewrite the table
    python3 tools/psims_tabella.py --obo FILE      # use a local copy of the obo (it must have the pinned SHA-256)
    python3 tools/psims_tabella.py --controlla     # exit 1 if the committed table is not the one generated from the pinned obo

The table: `terms` (accession, name, obsolete 0/1), `units` (term -> the units its `has_units` allows), `unit_names` (names of the `UO:` units that
`has_units` cites) and `classes` (the accessions below each of the roots the validator asks "is this term a ...?" about).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TABLE = ROOT / "crates" / "mzlab-core" / "data" / "psims-cv.json"
TAG = "v4.2.6"
URL = f"https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/{TAG}/psi-ms.obo"
OBO_SHA256 = "d0144c7a190d382c568980dacc497bc52873460f5bbd8217ed63a87e9377c8d0"
# name of the class -> root term (the root belongs to its class)
CLASSES = {
    "spectrum_type": "MS:1000559",
    "spectrum_repr": "MS:1000525",
    "array_type": "MS:1000513",
    "binary_type": "MS:1000518",
    "compression": "MS:1000572",
    "chrom_type": "MS:1000626",
}


def fetch(path: Path | None) -> bytes:
    if path is None:
        path = Path(tempfile.gettempdir()) / f"psi-ms-{TAG}.obo"
        if not path.exists():
            with urllib.request.urlopen(URL, timeout=60) as r:
                path.write_bytes(r.read())
    data = path.read_bytes()
    got = hashlib.sha256(data).hexdigest()
    if got != OBO_SHA256:
        sys.exit(f"error: {path} has SHA-256 {got}, the pinned {TAG} has {OBO_SHA256}")
    return data


def stanzas(text: str):
    """The `[Term]` stanzas as lists of (tag, value)."""
    cur = None
    for line in text.splitlines():
        if line.startswith("["):
            if cur is not None:
                yield cur
            cur = [] if line == "[Term]" else None
        elif cur is not None and ": " in line:
            tag, _, value = line.partition(": ")
            cur.append((tag, value))
    if cur is not None:
        yield cur


def build(obo: bytes) -> dict:
    terms, units, unit_names, parents = {}, {}, {}, {}
    for st in stanzas(obo.decode("utf-8")):
        d = {}
        for tag, value in st:
            d.setdefault(tag, []).append(value)
        acc = d["id"][0]
        if not acc.startswith("MS:"):
            continue
        terms[acc] = (d["name"][0], int("true" in d.get("is_obsolete", [])))
        parents[acc] = [v.split(" ")[0] for v in d.get("is_a", [])]
        for rel in d.get("relationship", []):
            head, _, rest = rel.partition(" ")
            if head != "has_units":
                continue
            unit, _, name = rest.partition(" ! ")
            units.setdefault(acc, []).append(unit)
            if unit.startswith("UO:"):
                unit_names[unit] = name
    children = {}
    for acc, ps in parents.items():
        for p in ps:
            children.setdefault(p, []).append(acc)
    classes = {}
    for name, root in CLASSES.items():
        if root not in terms:
            sys.exit(f"error: the root {root} of {name} is not in the obo")
        seen, todo = set(), [root]
        while todo:
            t = todo.pop()
            if t not in seen:
                seen.add(t)
                todo += children.get(t, [])
        classes[name] = sorted(seen)
    return {
        "source": "psi-ms.obo",
        "tag": TAG,
        "obo_sha256": OBO_SHA256,
        "terms": [[a, terms[a][0], terms[a][1]] for a in sorted(terms)],
        "units": {a: sorted(set(u)) for a, u in sorted(units.items())},
        "unit_names": dict(sorted(unit_names.items())),
        "classes": classes,
    }


def render(table: dict) -> str:
    """One term per line, so that a change of the vocabulary is a readable diff."""
    out = ["{"]
    for k in ("source", "tag", "obo_sha256"):
        out.append(f'"{k}":{json.dumps(table[k])},')
    out.append('"terms":[')
    out.append(",\n".join(json.dumps(t, ensure_ascii=False, separators=(",", ":")) for t in table["terms"]))
    out.append("],")
    for k in ("units", "unit_names", "classes"):
        out.append(f'"{k}":{json.dumps(table[k], ensure_ascii=False, separators=(",", ":"))}' + ("," if k != "classes" else ""))
    out.append("}")
    return "\n".join(out) + "\n"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--obo", type=Path, help="local copy of the pinned psi-ms.obo")
    ap.add_argument("--controlla", action="store_true", help="check the committed table instead of writing it")
    args = ap.parse_args()
    text = render(build(fetch(args.obo)))
    if args.controlla:
        if not TABLE.exists() or TABLE.read_text(encoding="utf-8") != text:
            print(f"FAIL {TABLE.relative_to(ROOT)} is not the table generated from the pinned obo: run python3 tools/psims_tabella.py", file=sys.stderr)
            return 1
        print("ok: the committed vocabulary table is the one generated from the pinned psi-ms.obo")
        return 0
    TABLE.parent.mkdir(parents=True, exist_ok=True)
    TABLE.write_text(text, encoding="utf-8")
    print(f"wrote {TABLE.relative_to(ROOT)} ({len(text) // 1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
