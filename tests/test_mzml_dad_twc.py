"""The synthetic TWC fixture builder (tools/mzml_dad_twc.py) gives a well-formed mzML with the standard 1D DAD trace."""
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import mzml_dad_twc  # noqa: E402


def test_twc_fixture_is_well_formed():
    root = ET.fromstring(mzml_dad_twc.build())
    chrom = root.find(".//chromatogram")
    assert chrom.get("id") == "TWC"
    assert any(c.get("accession") == "MS:1000813" for c in chrom.iter("cvParam"))
    assert len(root.findall(".//spectrum")) == mzml_dad_twc.NT
