"""tools/genera_sospetti.py: deterministic .mzsus from a synthetic mini-set (50 invented rows, no real file), reader, ppm search, SHA-256 check, licences."""
import hashlib
import json
import sys
import zlib
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import genera_sospetti as gs  # noqa: E402
from sospetti_prova import make_sources  # noqa: E402

@pytest.fixture
def mini(tmp_path):
    fonti, keys, mass, form = make_sources(tmp_path)
    raw = gs.genera(fonti, tmp_path, scarica=False)
    return fonti, keys, mass, form, raw, tmp_path


def test_deterministic(mini):
    fonti, _, _, _, raw, d = mini
    assert hashlib.sha256(gs.genera(fonti, d, scarica=False)).hexdigest() == hashlib.sha256(raw).hexdigest()
    z1, z2 = zlib.compress(raw, 9), zlib.compress(gs.genera(fonti, d, scarica=False), 9)
    assert z1 == z2


def test_read_and_merge(mini):
    _, keys, mass, form, raw, _ = mini
    s = gs.leggi_sospetti(raw)
    assert s.n == 45 and s.manifest["righe"] == 45
    assert list(s.masses) == sorted(s.masses)
    by = {s.hit(i).inchikey14: s.hit(i) for i in range(s.n)}
    h = by[keys[22]]                                           # in SusDat and PubChemLite
    assert h.sources == ("susdat", "pubchemlite") and h.formula == form[keys[22]] and h.mass == mass[keys[22]] and h.reaction == 0
    assert by[keys[0]].sources == ("susdat",) and by[keys[30]].sources == ("pubchemlite",)
    assert by[keys[20]].pubmed == 65535 and by[keys[41]].reaction == 1 and by[keys[42]].reaction == 2 and by[keys[41]].sources == ("pubchemlite", "transpubchem")
    assert [f["usati"] for f in s.manifest["fonti"]] == [25, 25, 3] and s.manifest["fonti"][0]["letti"] == 27


def test_search_ppm_exact(mini):
    _, keys, mass, _, raw, _ = mini
    s = gs.leggi_sospetti(raw)
    for k in keys:
        for ppm in (0.0, 5.0, 5000.0):
            want = {x for x in keys if abs(mass[x] - mass[k]) <= mass[k] * ppm * 1e-6}
            assert {h.inchikey14 for h in s.cerca(mass[k], ppm)} == want and k in want
        assert k not in {h.inchikey14 for h in s.cerca(mass[k] * (1 + 6e-6), 5.0)}      # 6 ppm away is outside +-5 ppm
        assert k in {h.inchikey14 for h in s.cerca(mass[k] * (1 + 4e-6), 5.0)}
    assert s.cerca(50.0, 5.0) == [] and s.cerca(700.0, 5.0) == []


def test_wrong_hash_rejected(mini):
    fonti, _, _, _, _, d = mini
    (d / "su.csv").write_text("changed\n", encoding="utf-8")
    with pytest.raises(SystemExit):
        gs.genera(fonti, d, scarica=False)


def test_missing_file_without_download(mini):
    fonti, _, _, _, _, d = mini
    (d / "pl.csv").unlink()
    with pytest.raises(SystemExit):
        gs.genera(fonti, d, scarica=False)


def test_bad_files_rejected():
    with pytest.raises(ValueError):
        gs.leggi_sospetti(b"nope" * 20)


def test_licenses_and_package(mini, tmp_path):
    _, _, _, _, raw, _ = mini
    s = gs.leggi_sospetti(raw)
    assert [f["licenza"] for f in s.manifest["fonti"]] == ["CC-BY-4.0"] * 3 and all(f["doi"] and f["sha256"] for f in s.manifest["fonti"])
    cat = gs.pacchetto(raw, tmp_path / "pk")
    pieces = [(tmp_path / "pk" / p["file"]).read_bytes() for p in cat["pezzi"]]
    assert all(hashlib.sha256(b).hexdigest() == p["sha256"] for b, p in zip(pieces, cat["pezzi"]))
    assert zlib.decompress(b"".join(pieces)) == raw and cat["sha256"] == hashlib.sha256(raw).hexdigest() and cat["formato"] == "sospetti/1"
    assert json.loads((tmp_path / "pk" / "catalogo.json").read_text(encoding="utf-8"))["fonti"][0]["licenza"] == "CC-BY-4.0"


def test_pinned_sources_have_licence():
    assert [s["id"] for s in gs.SOURCES] == list(gs.NAMES)
    for s in gs.SOURCES:
        assert s["licenza"] == "CC-BY-4.0" and s["doi"].startswith("10.5281/zenodo.")
        assert all(len(h) == 64 for _, h, _ in s["files"])
