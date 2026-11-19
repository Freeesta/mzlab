"""The library written by `mzlab-cli msn lib build` (two standards from the public synthetic infusion, never lab data) validates against
docs/schemi/msn-lib-1.json, every graph inside against docs/schemi/msn-graph-1.json, and the entries come out sorted by id."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest
from cargo_util import cli_argv
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
MZML = ROOT / "tests" / "golden" / "ref" / "msntree_rotta" / "msn.mzML"
SCHEMA = json.loads((ROOT / "docs/schemi/msn-lib-1.json").read_text())
GRAPH = json.loads((ROOT / "docs/schemi/msn-graph-1.json").read_text())
pytestmark = pytest.mark.skipif(not shutil.which("cargo"), reason="cargo not available")

MANIFEST = "id\tneutral_formula\texpected_adduct\tfile\nSTD-2\tC14H14N3O5\t[M+H]+\tmsn.mzML\nSTD-1\tC14H14N3O5\t[M+H]+\tmsn.mzML\n"


@pytest.fixture(scope="module")
def library(tmp_path_factory):
    d = tmp_path_factory.mktemp("msnlib")
    shutil.copy(MZML, d / "msn.mzML")
    (d / "manifest.tsv").write_text(MANIFEST)
    r = subprocess.run(cli_argv("msn-lib", "build", str(d / "manifest.tsv")), cwd=ROOT, capture_output=True, text=True)
    assert r.returncode == 0, r.stderr[-2000:]
    return json.loads((d / "manifest.json").read_text())


def test_the_built_library_follows_the_schema(library):
    Draft202012Validator(SCHEMA).validate(library)
    assert [v["id"] for v in library["entries"]] == ["STD-1", "STD-2"]


def test_the_graph_of_every_entry_follows_msn_graph(library):
    validator = Draft202012Validator(GRAPH)
    for v in library["entries"]:
        validator.validate(v["graph"])


def test_the_schema_rejects_a_broken_entry(library):
    bad = json.loads(json.dumps(library))
    del bad["entries"][0]["ion"]["polarity"]
    assert list(Draft202012Validator(SCHEMA).iter_errors(bad))
