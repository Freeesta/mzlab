"""The validation of the fragmentation rules on a handful of public CC BY MassBank records (`tests/data/pubblici_gemini/`): the lines of
`mzlab-cli msn-rules`, summed, give the strata and the thresholds of the protocol. The full run on the pinned release is outside the CI."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
REGISTRI = ROOT / "tests" / "data" / "pubblici_gemini"
pytestmark = pytest.mark.skipif(not shutil.which("cargo"), reason="cargo not available")


@pytest.fixture(scope="module")
def sommario(tmp_path_factory):
    out = tmp_path_factory.mktemp("convalida") / "somma.json"
    r = subprocess.run([sys.executable, str(ROOT / "tools" / "convalida_regole.py"), "--registri", str(REGISTRI), "--out", str(out)], cwd=ROOT, capture_output=True, text=True)
    assert r.returncode == 0, r.stderr[-2000:]
    return json.loads(out.read_text())


def test_the_strata_are_kept_apart_and_the_thresholds_are_declared(sommario):
    assert sommario["soglie"] == {"minime": 20, "tasso_max": 0.3, "differenza_min": 0.5}
    nomi = {(s["strato"], s["pol"]) for s in sommario["strati"]}
    assert ("hcd", 1) in nomi and ("cid_trappola", 1) in nomi and ("qtof", 1) in nomi
    assert sum(s["registri"] for s in sommario["strati"]) >= 20


def test_with_so_few_records_no_rule_is_measured(sommario):
    esiti = {r["esito"] for s in sommario["strati"] for r in s["regole"]}
    assert esiti <= {"non_misurata"}
    for s in sommario["strati"]:
        assert 0.0 <= s["copertura"]["c"] <= 1.0


def test_the_script_computes_nothing_itself():
    testo = (ROOT / "tools" / "convalida_regole.py").read_text()
    assert "msn-rules" in testo and "somma" in testo
