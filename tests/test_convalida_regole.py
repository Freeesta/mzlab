"""The validation of the fragmentation rules on a handful of public CC BY MassBank records (`tests/data/pubblici_gemini/`): the lines of
`mzlab-cli msn rules`, summed, give the strata and the thresholds of the protocol. The full run on the pinned release is outside the CI."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
REGISTRIES = ROOT / "tests" / "data" / "pubblici_gemini"
pytestmark = pytest.mark.skipif(not shutil.which("cargo"), reason="cargo not available")


@pytest.fixture(scope="module")
def sommario(tmp_path_factory):
    out = tmp_path_factory.mktemp("convalida") / "somma.json"
    r = subprocess.run([sys.executable, str(ROOT / "tools" / "validate_rules.py"), "--registri", str(REGISTRIES), "--out", str(out)], cwd=ROOT, capture_output=True, text=True)
    assert r.returncode == 0, r.stderr[-2000:]
    return json.loads(out.read_text())


def test_the_strata_are_kept_apart_and_the_thresholds_are_declared(sommario):
    assert sommario["thresholds"] == {"min_count": 20, "max_rate": 0.3, "min_difference": 0.5}
    names = {(s["layer"], s["pol"]) for s in sommario["layers"]}
    assert ("hcd", 1) in names and ("cid_trappola", 1) in names and ("qtof", 1) in names
    assert sum(s["records"] for s in sommario["layers"]) >= 20


def test_with_so_few_records_no_rule_is_measured(sommario):
    esiti = {r["outcome"] for s in sommario["layers"] for r in s["rules"]}
    assert esiti <= {"not_measured"}
    for s in sommario["layers"]:
        assert 0.0 <= s["coverage"]["c"] <= 1.0


def test_the_script_computes_nothing_itself():
    testo = (ROOT / "tools" / "validate_rules.py").read_text()
    assert '"msn", "rules"' in testo and "sum" in testo
