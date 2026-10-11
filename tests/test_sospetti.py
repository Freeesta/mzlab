"""tools/genera_sospetti.py: deterministic .mzsus from a synthetic mini-set (50 invented rows, no real file), reader, ppm search, SHA-256 check, licences."""
import csv
import hashlib
import json
import random
import sys
import zlib
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import genera_sospetti as gs  # noqa: E402

A = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"


def make_sources(d: Path):
    """25 keys in SusDat, 25 in PubChemLite (5 shared), 10 compounds in two reactions files; masses and keys invented."""
    rnd = random.Random(7)
    keys = ["".join(rnd.choice(A) for _ in range(14)) for _ in range(45)]
    mass = {k: round(rnd.uniform(100, 600), 6) for k in keys}
    form = {k: f"C{rnd.randint(1, 30)}H{rnd.randint(1, 40)}O{rnd.randint(0, 6)}" for k in keys}
    su = keys[:25] + ["short", "X" * 14]
    pl = keys[20:45]

    def w(name, header, rows):
        with open(d / name, "w", newline="", encoding="utf-8") as f:
            c = csv.writer(f)
            c.writerow(header)
            c.writerows(rows)
        return name, hashlib.sha256((d / name).read_bytes()).hexdigest(), "http://invalid/" + name

    f1 = w("su.csv", ["Norman_SusDat_ID", "StdInChIKey", "Molecular_Formula", "Monoiso_Mass"],
           [[i, k + "-UHFFFAOYSA-N", form.get(k, "CH4"), mass.get(k, "") if k != "short" else ""] for i, k in enumerate(su)])
    f2 = w("pl.csv", ["FirstBlock", "PubMed_Count", "Patent_Count", "MolecularFormula", "MonoisotopicMass"],
           [[k, 100000 if i == 0 else i, 3 * i, form[k], mass[k]] for i, k in enumerate(pl)])
    f3 = w("re.csv", ["predecessor", "predecessorcid", "successor", "successorcid"],
           [["a", 1, "b", 2], ["b", 2, "c", 3], ["c", 3, "d", 99]])
    f4 = w("ci.csv", ["CID", "MolecularFormula", "ExactMass", "IKFB"],
           [[1, form[keys[40]], mass[keys[40]], keys[40]], [2, form[keys[41]], mass[keys[41]], keys[41]], [3, form[keys[42]], mass[keys[42]], keys[42]]])
    base = {"licenza": "CC-BY-4.0", "doi": "10.0000/x", "url": "http://x", "istituzione": "Inventata"}
    fonti = [dict(base, id="susdat", nome="S", versione="1", files=[f1]), dict(base, id="pubchemlite", nome="P", versione="1", files=[f2]),
             dict(base, id="transpubchem", nome="T", versione="1", files=[f3, f4])]
    return fonti, keys, mass, form


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
