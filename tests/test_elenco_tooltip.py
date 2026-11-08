"""The tooltip list generator covers every source and its CSV matches the Markdown (tools/elenco_tooltip.py)."""
import csv, io, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import elenco_tooltip as E


def test_list_is_complete_and_consistent():
    md, cs, st = E.render()
    rows = list(csv.DictReader(io.StringIO(cs)))
    assert len(rows) == st["total"] and len({r["id"] for r in rows}) == len(rows)
    for src in ("site", "svg", "i18n-title", "catalog-only", "flow-step", "flow-param"):
        assert st["sources"].get(src, 0) > 0, src
    assert f"{st['total']} tooltips, {st['flagged']} flagged" in md
    assert st["skipped"] > 0 and all(r["it"] for r in rows)
