"""Reference stages of the Attributio port for the registry, hazard and regulatory-list rules (tox/, reglists/).

Tool only (never shipped in the site). Used by tools/attributio_riferimento.py, which passes its own helpers.
"""
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ISTANTANEE = ROOT / "tests/data/attributio/istantanee_sintetiche"
USCITA_TOX = ROOT / "tests/golden/attributio/tox"


def stadio_tox_ghs():
    """Thresholds of ecotox/ghs.py on a grid of values, taxon sets and H-statements."""
    from attributio.ecotox import ghs
    vals = [None, float("nan"), 0.005, 0.01, 0.05, 0.1, 0.5, 1, 5, 10, 50, 100, 500]
    flags = (False, True)
    out = {"acute": [[v, ghs.acute_from_value(v)] for v in vals],
           "noec": [[v, r, ghs.chronic_from_noec(v, r)] for v in vals for r in flags],
           "from_acute": [[v, r, k, b, ghs.chronic_from_acute(v, r, k, b)]
                          for v in vals for r in flags for k in (None, 3.9, 4.0) for b in (None, 100, 500)]}
    cases = [({"fish": 0.5}, {}), ({"fish": 5, "algae": 50}, {}), ({}, {}), ({"fish": 0.5, "crustacean": 20}, {"fish": 0.5}),
             ({"fish": 50, "crustacean": 0.8}, {"fish": 2, "algae": 3}), ({"fish": 5}, {"fish": 0.005, "crustacean": 0.3, "algae": 4})]
    out["classify"] = [[a, c, r, ghs.classify(a, c, r, 4.2, None)] for a, c in cases for r in flags]
    codes = [[], ["H400"], ["H401", "H402"], ["H410", "H413"], ["H412"], ["H400", "H411", "H413", "H319"], ["H402", "H401"]]
    out["h_codes"] = [[c, ghs.from_h_codes(c)] for c in codes]
    return out


def stadio_tox_istantanee(dir_: Path, frame_rows):
    """The snapshot readers (registry, membership, ECOTOX values, ECOSAR endpoints) on the synthetic snapshots."""
    from attributio.chem import registry
    from attributio.ecotox import hazard
    from attributio.reglists import schema
    registry.registry_path = lambda name="registry": dir_ / registry.FILES[name]
    hazard._paths.ecotox_dir = lambda: dir_
    hazard.clear_caches()
    snap = hazard.snapshot()
    return {"registry": frame_rows(registry.load_registry()),
            "membership": frame_rows(schema.read_membership(dir_ / "reglist_membership.csv")),
            "values": frame_rows(snap["values"]), "ecosar": frame_rows(snap["ecosar"])}


def stadi_tox(frame_rows, scrivi) -> dict:
    """The stage table: name -> function writing its golden file."""
    return {
        "tox-ghs": lambda: scrivi(USCITA_TOX, "ghs.json", stadio_tox_ghs()),
        "tox-istantanee": lambda: scrivi(USCITA_TOX, "istantanee.json", stadio_tox_istantanee(ISTANTANEE, frame_rows)),
    }
