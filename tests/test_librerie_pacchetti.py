"""tools/librerie_pacchetti.py: which MassBank records go into the public packages, the pieces and the catalog (with a stand-in for the Rust builder)."""
import hashlib
import json
import struct
import sys
import zipfile
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import librerie_pacchetti as lp  # noqa: E402


def record(acc, lic, mode="POSITIVE", ms="MS2", ion="ESI", peaks=True):
    body = ("PK$PEAK: m/z int. rel.int.\n  100.1 100 999\n  150.2 50 500\n//\n" if peaks else "//\n")
    return (f"ACCESSION: {acc}\nRECORD_TITLE: x; LC-ESI-QTOF; {ms}\nAUTHORS: A B\nLICENSE: {lic}\nCH$NAME: n{acc}\n"
            f"AC$MASS_SPECTROMETRY: MS_TYPE {ms}\nAC$MASS_SPECTROMETRY: ION_MODE {mode}\nAC$MASS_SPECTROMETRY: IONIZATION {ion}\n"
            f"MS$FOCUSED_ION: PRECURSOR_M/Z 200.1\nPK$NUM_PEAK: 2\n{body}")


RECORDS = {
    "Eawag/MSBNK-Eawag-1.txt": record("1", "CC BY"),
    "Eawag/MSBNK-Eawag-2.txt": record("2", "CC BY", mode="NEGATIVE"),
    "RIKEN/MSBNK-RIKEN-3.txt": record("3", "CC0"),
    "RIKEN/MSBNK-RIKEN-4.txt": record("4", "CC BY-SA", mode="NEGATIVE"),
    "KEIO/MSBNK-KEIO-5.txt": record("5", "CC BY-NC"),            # NC: never in the site
    "KEIO/MSBNK-KEIO-6.txt": record("6", "CC BY-NC-SA"),
    "BAFG/MSBNK-BAFG-7.txt": record("7", "dl-de/by-2-0"),        # another licence: not in the list
    "Eawag/MSBNK-Eawag-8.txt": record("8", "CC BY", ms="MS3"),    # not MS2
    "Eawag/MSBNK-Eawag-9.txt": record("9", "CC BY", ion="EI"),    # not ESI
    "Eawag/MSBNK-Eawag-10.txt": record("10", "CC BY", peaks=False),
    "Eawag/MSBNK-Eawag-11.txt": record("11", "CC BY", mode="NEGATIVE"),
    "Eawag/readme.txt": "not a record",
}


@pytest.fixture
def zip_path(tmp_path):
    p = tmp_path / "mb.zip"
    with zipfile.ZipFile(p, "w") as z:
        for k, v in RECORDS.items():
            z.writestr("MassBank-MassBank-data-abc/" + k, v)
    return p


def stand_in(tmp_path, size=0):
    """A «builder» that writes a .mzlib-like file: the header of the format and as many bytes as asked (to force several pieces)."""
    s = tmp_path / "fake_builder.py"
    s.write_text(
        "import sys, struct\n"
        "src, out = sys.argv[1], sys.argv[2]\n"
        "n = open(src, encoding='utf-8').read().count('\\n//')\n"
        f"h = b'MZLB' + struct.pack('<HHIIQII', 1, 1, n, 5, 2 * n, 0, 0) + struct.pack('<I', 0)\n"
        f"open(out, 'wb').write(h + b'x' * ({size} + n))\n")
    return [sys.executable, str(s)]


def test_only_esi_ms2_with_open_licences_are_kept(tmp_path, zip_path):
    out = tmp_path / "out"
    cat = lp.run(out, zip_path, stand_in(tmp_path))
    by = {x["id"]: x for x in cat["librerie"]}
    assert by["massbank-pos"]["spettri"] == 2 and by["massbank-neg"]["spettri"] == 3
    assert by["massbank-pos"]["licenze"] == {"CC BY": 1, "CC0": 1}
    assert by["massbank-neg"]["licenze"] == {"CC BY": 2, "CC BY-SA": 1}
    assert by["massbank-pos"]["contributori"] == {"Eawag": 1, "RIKEN": 1}
    for lib in cat["librerie"][:2]:
        assert "NC" not in json.dumps(lib["licenze"]) and "dl-de" not in json.dumps(lib["licenze"])


def test_the_unito_standards_are_a_package_of_their_own(tmp_path, zip_path):
    cat = lp.run(tmp_path / "out", zip_path, stand_in(tmp_path))
    u = {x["id"]: x for x in cat["librerie"]}["unito-standard"]
    assert u["spettri"] == 61 and u["licenze"] == {"CC BY": 61} and "Cristaudo" in u["attribuzione"]


def test_pieces_are_checked_and_rebuild_the_package(tmp_path, zip_path):
    out = tmp_path / "out"
    cat = lp.run(out, zip_path, stand_in(tmp_path, size=lp.PIECE * 2 + 1000))
    lib = {x["id"]: x for x in cat["librerie"]}["massbank-pos"]
    assert len(lib["pezzi"]) == 3 and lib["pezzi"][0]["dimensione"] == lp.PIECE
    whole = hashlib.sha256()
    for p in lib["pezzi"]:
        b = (out / p["file"]).read_bytes()
        assert len(b) == p["dimensione"] and hashlib.sha256(b).hexdigest() == p["sha256"]
        whole.update(b)
    assert whole.hexdigest() == lib["sha256"] and sum(p["dimensione"] for p in lib["pezzi"]) == lib["dimensione"]
    assert lib["lettore_min"] == 1 and lib["picchi"] == 2 * lib["spettri"]


def test_the_catalog_file_and_the_external_sources(tmp_path, zip_path):
    out = tmp_path / "out"
    lp.run(out, zip_path, stand_in(tmp_path))
    cat = json.loads((out / "catalogo.json").read_text(encoding="utf-8"))
    assert cat["formato"] == "librerie/1" and cat["pezzo"] == lp.PIECE
    assert {e["id"] for e in cat["esterne"]} == {"gnps-cc0", "massbank-msp"}
    assert all(e["url"].startswith("https://") for e in cat["esterne"])
    assert len(lp.ZIP_SHA256) == 64 and lp.VERSION in lp.ZIP_URL


def test_a_wrong_download_hash_stops_the_build(tmp_path, monkeypatch):
    def fake(url, dest, want):
        dest.write_bytes(b"not the release")
        got = hashlib.sha256(dest.read_bytes()).hexdigest()
        raise SystemExit(f"{dest.name}: SHA-256 {got} is not the expected {want}")
    monkeypatch.setattr(lp, "download", fake)
    with pytest.raises(SystemExit):
        lp.run(tmp_path / "out", None, stand_in(tmp_path))


def test_an_unreachable_source_publishes_nothing_and_does_not_fail(tmp_path, monkeypatch, capsys):
    def offline(url, dest, want):
        raise ConnectionError("offline")
    monkeypatch.setattr(lp, "download", offline)
    assert lp.run(tmp_path / "out", None, stand_in(tmp_path)) is None
    assert "::warning::" in capsys.readouterr().out
    assert not (tmp_path / "out").exists()
