"""The library written by `mzlab-cli msn-lib build` (two standards from the public synthetic infusion, never lab data) validates against
docs/schemi/msnlib-1.json, every graph inside against docs/schemi/msngrafo-1.json, and the entries come out sorted by id."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
MZML = ROOT / "tests" / "golden" / "ref" / "msntree_rotta" / "msn.mzML"
SCHEMA = json.loads((ROOT / "docs/schemi/msnlib-1.json").read_text())
GRAFO = json.loads((ROOT / "docs/schemi/msngrafo-1.json").read_text())
pytestmark = pytest.mark.skipif(not shutil.which("cargo"), reason="cargo not available")

MANIFEST = "id\tformula_neutra\taddotto_atteso\tfile\nSTD-2\tC14H14N3O5\t[M+H]+\tmsn.mzML\nSTD-1\tC14H14N3O5\t[M+H]+\tmsn.mzML\n"


@pytest.fixture(scope="module")
def library(tmp_path_factory):
    d = tmp_path_factory.mktemp("msnlib")
    shutil.copy(MZML, d / "msn.mzML")
    (d / "manifest.tsv").write_text(MANIFEST)
    r = subprocess.run(["cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "msn-lib", "build", str(d / "manifest.tsv")], cwd=ROOT, capture_output=True, text=True)
    assert r.returncode == 0, r.stderr[-2000:]
    return json.loads((d / "manifest.json").read_text())


def test_the_built_library_follows_the_schema(library):
    Draft202012Validator(SCHEMA).validate(library)
    assert [v["id"] for v in library["voci"]] == ["STD-1", "STD-2"]


def test_the_graph_of_every_entry_follows_msngrafo(library):
    validator = Draft202012Validator(GRAFO)
    for v in library["voci"]:
        validator.validate(v["grafo"])


def test_the_schema_rejects_a_broken_entry(library):
    bad = json.loads(json.dumps(library))
    del bad["voci"][0]["ione"]["polarita"]
    assert list(Draft202012Validator(SCHEMA).iter_errors(bad))
