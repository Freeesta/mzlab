"""Gold m/z of the HRMS substance list (numbers only): the molecules engine must give the same ones, proved by crates/mzlab-core/tests/molecules.rs."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ORO = ROOT / "tests" / "dati" / "molecole_hrms_oro.json"


def test_oro_shape():
    d = json.loads(ORO.read_text())
    n = sum(1 for l in (ROOT / "tests/data/liste/sostanze_hrms.csv").read_text(encoding="utf-8-sig").splitlines()
            if l.strip() and not l.startswith("#")) - 1
    assert len(d["rows"]) >= n - 5
    assert all(len(r) == len(d["columns"]) for r in d["rows"])
