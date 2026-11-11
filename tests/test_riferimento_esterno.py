"""The vendor-reference harness (tools/riferimento_esterno.py): its manifest is valid and public-only, its mzML reader and
comparison catch the differences they are meant to catch on synthetic files, and without data it says so and exits 0."""
import base64
import importlib.util
import json
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("riferimento_esterno", ROOT / "tools" / "riferimento_esterno.py")
re_ = importlib.util.module_from_spec(spec)
spec.loader.exec_module(re_)
MANIFEST = json.loads((ROOT / "tools" / "riferimento_esterno.json").read_text())
TOL = MANIFEST["tolerances"]


def _arr(acc: str, vals, f64=True, z=True) -> str:
    raw = struct.pack(f"<{len(vals)}{'d' if f64 else 'f'}", *vals)
    raw = zlib.compress(raw) if z else raw
    enc = "MS:1000523" if f64 else "MS:1000521"
    comp = "MS:1000574" if z else "MS:1000576"
    return (f'<binaryDataArray><cvParam accession="{enc}"/><cvParam accession="{comp}"/><cvParam accession="{acc}"/>'
            f'<binary>{base64.b64encode(raw).decode()}</binary></binaryDataArray>')


def _mzml(tmp: Path, name: str, pol="MS:1000130", rt="60", unit="UO:0000010", target="400.1", ce="30",
          mz=(100.0, 200.0), q1="350.2", q3="120.05", trace=(0.0, 5.0, 7.0)) -> Path:
    spec2 = (f'<spectrum id="s2" index="1"><cvParam accession="MS:1000511" value="2"/><cvParam accession="{pol}"/>'
             f'<cvParam accession="MS:1000127"/><scanList><scan><cvParam accession="MS:1000016" value="{rt}" '
             f'unitAccession="{unit}"/></scan></scanList><precursorList><precursor><isolationWindow>'
             f'<cvParam accession="MS:1000827" value="{target}"/><cvParam accession="MS:1000828" value="0.5"/>'
             f'<cvParam accession="MS:1000829" value="0.5"/></isolationWindow><selectedIonList><selectedIon>'
             f'<cvParam accession="MS:1000744" value="{target}"/></selectedIon></selectedIonList><activation>'
             f'<cvParam accession="MS:1000133"/><cvParam accession="MS:1000045" value="{ce}"/></activation></precursor>'
             f'</precursorList><binaryDataArrayList>{_arr("MS:1000514", mz)}{_arr("MS:1000515", (10.0, 20.0), False, False)}'
             f'</binaryDataArrayList></spectrum>')
    spec1 = ('<spectrum id="s1" index="0"><referenceableParamGroupRef ref="g"/><cvParam accession="MS:1000127"/>'
             f'<scanList><scan><cvParam accession="MS:1000016" value="{rt}" unitAccession="{unit}"/></scan></scanList>'
             f'<binaryDataArrayList>{_arr("MS:1000514", mz)}{_arr("MS:1000515", (1.0, 2.0))}</binaryDataArrayList></spectrum>')
    chrom = (f'<chromatogram id="SRM Q1={q1} Q3={q3}"><cvParam accession="MS:1001473"/><cvParam accession="{pol}"/>'
             f'<precursor><isolationWindow><cvParam accession="MS:1000827" value="{q1}"/></isolationWindow></precursor>'
             f'<product><isolationWindow><cvParam accession="MS:1000827" value="{q3}"/></isolationWindow></product>'
             f'<binaryDataArrayList>{_arr("MS:1000595", (0.0, 1.0, 2.0))}{_arr("MS:1000515", trace)}</binaryDataArrayList>'
             '</chromatogram>')
    tic = (f'<chromatogram id="TIC"><cvParam accession="MS:1000235"/><binaryDataArrayList>{_arr("MS:1000595", (0.0,))}'
           f'{_arr("MS:1000515", (9.0,))}</binaryDataArrayList></chromatogram>')
    text = ('<?xml version="1.0"?><indexedmzML xmlns="http://psi.hupo.org/ms/mzml"><mzML><referenceableParamGroupList>'
            f'<referenceableParamGroup id="g"><cvParam accession="MS:1000511" value="1"/><cvParam accession="{pol}"/>'
            f'</referenceableParamGroup></referenceableParamGroupList><run><spectrumList>{spec1}{spec2}</spectrumList>'
            f'<chromatogramList>{tic}{chrom}</chromatogramList></run></mzML></indexedmzML>')
    p = tmp / name
    p.write_text(text)
    return p


def _cmp(a: Path, b: Path) -> re_.Result:
    ma, mb = re_.read_mzml(a), re_.read_mzml(b)
    res = re_.Result()
    re_.compare_spectra(ma["spectra"], mb["spectra"], TOL, res, True)
    re_.compare_chromatograms(ma["chromatograms"], mb["chromatograms"], TOL, res)
    return res


def test_manifest_is_valid_and_public_only():
    assert re_.validate_manifest(MANIFEST) == []
    assert len(MANIFEST["datasets"]) >= 10
    for d in MANIFEST["datasets"]:
        assert d["input"].startswith("dati/formati/") and d["reference"].startswith("dati/formati/"), d["name"]


def test_reader_resolves_groups_units_and_encodings(tmp_path):
    m = re_.read_mzml(_mzml(tmp_path, "a.mzML"))
    s1, s2 = m["spectra"]
    assert (s1["level"], s1["polarity"], s1["representation"]) == (1, "+", "centroid")
    assert abs(s1["rt"] - 1.0) < 1e-12 and s2["mz"] == [100.0, 200.0] and s2["intensity"] == [10.0, 20.0]
    pre = s2["precursors"][0]
    assert (pre["target"], pre["low"], pre["high"], pre["selected"], pre["ce"]) == (400.1, 0.5, 0.5, [400.1], 30.0)
    assert pre["activation"] == ["MS:1000133"]
    srm = [c for c in m["chromatograms"] if c["ms"]][0]
    assert (srm["q1"], srm["q3"], srm["polarity"]) == (350.2, 120.05, "+")


def test_identical_files_pass_every_check(tmp_path):
    res = _cmp(_mzml(tmp_path, "a.mzML"), _mzml(tmp_path, "b.mzML"))
    assert res.failed() == set()
    assert {"level", "polarity", "rt", "isolation", "activation", "mz", "intensity", "chrom_q1q3"} <= res.n.keys()


def test_each_difference_is_named(tmp_path):
    ref = _mzml(tmp_path, "ref.mzML")
    cases = {
        "polarity": dict(pol="MS:1000129"),
        "rt": dict(rt="60.1"),
        "isolation": dict(target="400.2"),
        "collision_energy": dict(ce="35"),
        "mz": dict(mz=(100.0, 200.001)),
        "chrom_q1q3": dict(q1="350.3"),
        "chrom_paired": dict(trace=(0.0, 5.0, 8.0)),
    }
    for check, kw in cases.items():
        res = _cmp(_mzml(tmp_path, f"{check}.mzML", **kw), ref)
        assert check in res.failed(), (check, res.failed())


def test_minutes_and_seconds_agree(tmp_path):
    res = _cmp(_mzml(tmp_path, "s.mzML", rt="60"), _mzml(tmp_path, "m.mzML", rt="1", unit="UO:0000031"))
    assert "rt" not in res.failed()


def test_q_classes():
    assert re_._q_class(100.0, 100.0004, TOL) == "exact"
    assert re_._q_class(100.0, 100.005, TOL) == "rounded"
    assert re_._q_class(100.0, 100.1, TOL) == "wrong"


def test_without_data_it_skips_loudly(monkeypatch, capsys):
    monkeypatch.setattr(re_.dati_percorsi, "radice", lambda: None)
    assert re_.main([]) == 0
    assert "SKIPPED: no data" in capsys.readouterr().out


def test_manifest_refuses_unexplained_entries():
    bad = json.loads(json.dumps(MANIFEST))
    d = bad["datasets"][0]
    d["expected"] = {"level": "looks wrong"}
    d["ignore"] = {"rt": ""}
    d["tolerances"] = {"rt_min": 1.0}
    d.pop("tolerances_why", None)
    errs = re_.validate_manifest(bad)
    assert any("not a review ID" in e for e in errs)
    assert any("ignored check rt" in e for e in errs)
    assert any("tolerance override rt_min" in e for e in errs)
