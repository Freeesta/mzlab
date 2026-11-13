"""tests/dati/link_golden.json (read by crates/mzlab-feat/tests/link.rs, the desktop route test and the wasm parity test) is what the independent
reference tools/link_reference.py computes, and its SMILES and InChI cover charges, brackets, stereo, isotopes, #, + and /."""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_golden_is_the_reference():
    r = subprocess.run([sys.executable, "tools/link_reference.py"], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr


def test_cases_cover_the_brief():
    cases = json.loads((ROOT / "tests/dati/link_golden.json").read_text(encoding="utf-8"))
    text = "".join(c["request"]["smiles"] + c["request"].get("inchi", "") for c in cases)
    assert all(ch in text for ch in "+-[]@/#=()")
    urls = [l["url"] for c in cases for l in json.loads(c["link"])["links"]]
    assert any("%2B" in u for u in urls) and any("%2F" in u for u in urls) and all(" " not in u for u in urls)
    assert {json.loads(c["link"])["preferred"] for c in cases} == {"pubchem-exact", "pubchem-sub"}
