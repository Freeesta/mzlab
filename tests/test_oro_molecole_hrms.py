"""Gold m/z of the HRMS substance list: the file stays in step with the JS core until the molecules engine replaces it."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
ORO = ROOT / "tests" / "dati" / "molecole_hrms_oro.json"


def test_oro_shape():
    d = json.loads(ORO.read_text())
    n = sum(1 for l in (ROOT / "mzlab/web/sostanze_hrms.csv").read_text(encoding="utf-8-sig").splitlines()
            if l.strip() and not l.startswith("#")) - 1
    assert len(d["rows"]) >= n - 5
    assert all(len(r) == len(d["columns"]) for r in d["rows"])


@pytest.mark.skipif(shutil.which("node") is None, reason="node missing")
def test_oro_matches_current_js_core():
    before = ORO.read_bytes()
    subprocess.run(["node", str(ROOT / "tools/oro_molecole_hrms.js")], check=True, capture_output=True)
    try:
        assert ORO.read_bytes() == before
    finally:
        ORO.write_bytes(before)
