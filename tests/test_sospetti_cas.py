"""tools/genera_sospetti.py: the CAS sidecar (`mzlab-cas/1`) from an invented SusDat mini-file, its pieces and catalogue (no real file)."""
import hashlib
import json
import sys
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import genera_sospetti as gs  # noqa: E402

HEAD = "StdInChIKey,MS_Ready_StdInChIKey,CAS_RN,CAS_RN_Dashboard,CAS_RN_PubChem,Molecular_Formula,Monoiso_Mass,SMILES,MS_Ready_SMILES\n"
ROWS = [
    # a parent compound, CAS in two columns (one record, one line)
    "QQQQQQQQQQQQQA-UHFFFAOYSA-N,QQQQQQQQQQQQQA-UHFFFAOYSA-N,7000-00-1,7000-00-1,,C10H12O2,164.083730,CCCCCCCCCC,CCCCCCCCCC",
    # a sodium salt whose MS-ready form is another skeleton: the MS-ready key and SMILES travel with the line
    "QQQQQQQQQQQQQB-UHFFFAOYSA-M,QQQQQQQQQQQQQC-UHFFFAOYSA-N,7000-01-1,,,C2H5NaO,68.023,[Na+].CC[O-],CCO",
    # no valid key: left out; a text that is not a CAS number: left out
    "short,,7000-02-9,,,CH4,16.03,C,C",
    "QQQQQQQQQQQQQD-UHFFFAOYSA-N,,not a number,,,CH4,16.03,C,C",
]


def sidecar(tmp_path):
    p = tmp_path / "su.csv"
    p.write_text(HEAD + "\n".join(ROWS) + "\n", encoding="utf-8")
    return gs.cas_sidecar(p)


def test_lines_and_form(tmp_path):
    lines = sidecar(tmp_path).decode().splitlines()
    assert lines[0] == "# mzlab-cas/1"
    assert lines[1:] == [
        "7000-00-1\tQQQQQQQQQQQQQA-UHFFFAOYSA-N\t\tC10H12O2\t164.083730\tparent\t",
        "7000-01-1\tQQQQQQQQQQQQQB-UHFFFAOYSA-M\tQQQQQQQQQQQQQC-UHFFFAOYSA-N\tC2H5NaO\t68.023\tsalt\tCCO",
    ]


def test_deterministic_and_pieces(tmp_path):
    raw = sidecar(tmp_path)
    assert raw == sidecar(tmp_path)
    cat = gs.pacchetto_cas(raw, tmp_path / "cas")
    on_disk = json.loads((tmp_path / "cas" / "catalogo.json").read_text(encoding="utf-8"))
    assert on_disk == cat and cat["format"] == "mzlab-cas/1" and cat["rows"] == 2 and cat["raw"] == "cas.tsv"
    z = b"".join((tmp_path / "cas" / p["file"]).read_bytes() for p in cat["pieces"])
    assert all(hashlib.sha256((tmp_path / "cas" / p["file"]).read_bytes()).hexdigest() == p["sha256"] for p in cat["pieces"])
    assert zlib.decompress(z) == raw and cat["sha256"] == hashlib.sha256(raw).hexdigest() and cat["size_compressed"] == len(z)
