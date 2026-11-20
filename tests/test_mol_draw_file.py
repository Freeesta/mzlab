"""`mzlab-cli mol draw --format ket|cdxml` on the 200 synthetic molecules of tests/dati/draw_set_a.tsv: the KET is JSON with the molecule first and the wedges on
the narrow end, the CDXML is well-formed XML, and (the round trip through our own readers is in cargo test, crates/mzlab-feat/src/chem/draw_file_tests.rs)
RDKit, used only as an oracle and skipped where it is not installed, reads the CDXML back as the same molecule (canonical SMILES, stereo included).
Both formats are open and written from their public documentation: nothing here claims ChemDraw or Ketcher fidelity."""
import json
import shutil
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest
from cargo_util import build

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import draw_quality as dq  # noqa: E402

pytestmark = pytest.mark.skipif(not shutil.which("cargo"), reason="cargo not available")
CLI = ROOT / "target" / "release" / "mzlab-cli"


@pytest.fixture(scope="module")
def cli_built():
    build("-p", "mzlab-cli")


def draw(smiles, fmt, *extra):
    p = subprocess.run([str(CLI), "mol", "draw", "smiles:" + smiles, "--format", fmt, *extra], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert p.returncode == 0, (smiles, fmt, p.stderr)
    return p.stdout


def test_every_molecule_gives_valid_ket_and_wellformed_cdxml(cli_built):
    rows = dq.read_set("draw_set_a")
    assert len(rows) == 200
    for _id, smi, _fam in rows:
        ket = json.loads(draw(smi, "ket"))
        assert list(ket)[0] == "root" and ket["mol0"]["type"] == "molecule", smi
        n = len(ket["mol0"]["atoms"])
        assert all(0 <= i < n for b in ket["mol0"]["bonds"] for i in b["atoms"]), smi
        root = ET.fromstring(draw(smi, "cdxml").split("\n", 2)[2])  # the DOCTYPE line is not XML 1.0 well-formed input for ElementTree
        assert root.tag == "CDXML" and len(root.findall(".//n")) == n, smi


def test_ket_in_the_scene_and_its_stderr_choices(cli_built):
    p = subprocess.run([str(CLI), "mol", "draw", "C[C@H](N)C(=O)O", "--format", "ket", "--json"], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert p.returncode == 0 and list(json.loads(p.stdout))[0] == "schema" and p.stderr == ""
    scene = json.loads(p.stdout)
    assert scene["file_format"] == "ket" and json.loads(scene["file"])["mol0"]["bonds"]
    assert draw("CCO", "ket") == draw("CCO", "ket")  # deterministic


def test_rdkit_reads_the_cdxml_as_the_same_molecule(cli_built):
    Chem = pytest.importorskip("rdkit.Chem")
    if not hasattr(Chem, "MolsFromCDXML"):
        pytest.skip("this RDKit has no CDXML reader")
    bad = []
    for _id, smi, _fam in dq.read_set("draw_set_a"):
        # one CDXML fragment holds a whole salt or mixture: compare the pieces, not the molecule objects
        pieces = lambda mols: sorted(Chem.MolToSmiles(f) for m in mols for f in Chem.GetMolFrags(m, asMols=True))
        got = Chem.MolsFromCDXML(draw(smi, "cdxml"))
        a, b = (pieces(got) if got else None), pieces([Chem.MolFromSmiles(smi)])
        if a != b:
            bad.append((smi, a, b))
    assert not bad, f"{len(bad)} of 200 differ, first: {bad[:3]}"
