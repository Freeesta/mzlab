"""The comparator of scandump/1 dumps (tools/rosina/scandump_compare.py) on two small dumps written by hand with known differences (tests/data/scandump/),
and the conversion of a dump into the truth verita/1 (tools/rosina/verita_scandump.py). No data of the laboratory: the dumps are synthetic."""
import gzip
import importlib.util
import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STR = ROOT / "tools" / "rosina"
FIX = ROOT / "tests" / "data" / "scandump"
ORACLE, OURS = FIX / "oracle.jsonl", FIX / "ours.jsonl"


def _carica(nome: str):
    spec = importlib.util.spec_from_file_location("rosina_" + nome, STR / (nome + ".py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


compare = _carica("scandump_compare")
verita_scandump = _carica("verita_scandump")

# field -> (compared, equal, different, absent in A, absent in B), for --tol hr
EXPECTED = {
    "analyzer": (2, 2, 0, 0, 0),
    "bp_int": (2, 2, 0, 0, 0),
    "bp_mz": (2, 2, 0, 0, 0),
    "chromatogram.count": (1, 1, 0, 0, 0),
    "chromatogram.time": (3, 3, 0, 0, 0),
    "chromatogram.values": (3, 3, 0, 0, 0),
    "event": (2, 2, 0, 0, 0),
    "faims_cv": (0, 0, 0, 1, 0),
    "file.controller": (1, 1, 0, 0, 0),
    "file.format_version": (1, 1, 0, 0, 0),
    "file.instrument": (1, 1, 0, 0, 0),
    "file.n_scans": (1, 1, 0, 0, 0),
    "filter": (2, 2, 0, 0, 0),
    "level": (2, 2, 0, 0, 0),
    "master": (1, 1, 0, 0, 0),
    "mode": (2, 2, 0, 0, 0),
    "mz_high": (2, 2, 0, 0, 0),
    "mz_low": (2, 2, 0, 0, 0),
    "n_peaks": (2, 2, 0, 0, 0),
    "peaks.count": (1, 1, 0, 0, 0),
    "peaks.int": (3, 2, 1, 0, 0),
    "peaks.mz": (3, 3, 0, 0, 0),
    "polarity": (2, 2, 0, 0, 0),
    "reactions.activation": (1, 0, 1, 0, 0),
    "reactions.count": (1, 1, 0, 0, 0),
    "reactions.energy": (1, 1, 0, 0, 0),
    "reactions.energy_unit": (0, 0, 0, 0, 1),
    "reactions.iso_width": (1, 1, 0, 0, 0),
    "reactions.precursor": (1, 0, 1, 0, 0),
    "rt": (2, 2, 0, 0, 0),
    "scan_type": (2, 2, 0, 0, 0),
    "segment": (1, 1, 0, 0, 1),
    "tic": (2, 1, 1, 0, 0),
    "trailer.elapsed scan time (sec)": (0, 0, 0, 1, 0),
    "trailer.ion injection time (ms)": (2, 2, 0, 0, 0),
    "trailer.micro scan count": (1, 1, 0, 0, 0),
}


def _counts(rep):
    return {k: (v["compared"], v["equal"], v["different"], v["absent_a"], v["absent_b"]) for k, v in rep["fields"].items()}


def test_every_field_exact():
    rep = compare.compare(ORACLE, OURS, "hr")
    assert _counts(rep) == EXPECTED
    assert rep["scans"] == {"a": 3, "b": 3, "common": 2, "only_a": 1, "only_b": 1}
    assert rep["peaks_rows"] == {"a": 1, "b": 1, "common": 1}
    assert rep["instrument"] == "Synthetic FT"


def test_differences_and_examples():
    f = compare.compare(ORACLE, OURS, "hr")["fields"]
    assert f["reactions.precursor"]["examples"] == [{"n": 2, "a": 500.0, "b": 500.0008}]
    assert round(f["reactions.precursor"]["max_diff"], 3) == 1.6  # ppm
    assert f["reactions.activation"]["examples"] == [{"n": 2, "a": "HCD", "b": "CID"}]
    assert f["tic"]["examples"] == [{"n": 2, "a": 500000.0, "b": 500010.0}]
    assert f["peaks.int"]["examples"] == [{"n": 1, "a": 5000.0, "b": 5001.0}]
    assert f["reactions.energy_unit"]["examples"] == [{"n": 2, "a": "NCE", "b": None}]


def test_extra_is_not_an_error():
    f = compare.compare(ORACLE, OURS, "hr")["fields"]
    assert f["faims_cv"]["extra"] and f["trailer.elapsed scan time (sec)"]["extra"]
    assert not f["segment"]["extra"] and not f["reactions.energy_unit"]["extra"]


def test_tolerance_hr_and_lr():
    hr = compare.compare(ORACLE, OURS, "hr")["fields"]["reactions.precursor"]
    lr = compare.compare(ORACLE, OURS, "lr")["fields"]["reactions.precursor"]
    assert (hr["equal"], hr["different"]) == (0, 1)
    assert (lr["equal"], lr["different"]) == (1, 0)


def test_text_normalization_and_numbers_as_text():
    f = compare.compare(ORACLE, OURS, "hr")["fields"]
    assert f["filter"]["different"] == 0  # case and blanks
    assert f["trailer.ion injection time (ms)"]["different"] == 0  # "10.5" = "10.50", label colon dropped


def test_identical_dumps_have_no_difference():
    rep = compare.compare(ORACLE, ORACLE, "hr")
    assert all(v["different"] == 0 and v["absent_a"] == 0 and v["absent_b"] == 0 for v in rep["fields"].values())


def test_gzip_and_report_and_aggregate(tmp_path):
    for src in (ORACLE, OURS):
        with src.open("rb") as i, gzip.open(tmp_path / (src.name + ".gz"), "wb") as o:
            shutil.copyfileobj(i, o)
    out = tmp_path / "reports"
    out.mkdir()
    for k in range(2):
        assert compare.main([str(tmp_path / "oracle.jsonl.gz"), str(tmp_path / "ours.jsonl.gz"), "--out", str(out / f"r{k}.json")]) == 0
    rep = json.loads((out / "r0.json").read_text())
    assert rep["schema"] == "scandump-compare/1" and _counts(rep) == EXPECTED
    agg = compare.aggregate(out)
    assert agg["instruments"] == ["Synthetic FT"]
    cell = agg["fields"]["tic"]["Synthetic FT"]
    assert {k: cell[k] for k in ("compared", "equal", "different", "absent_a", "absent_b", "files")} == {"compared": 4, "equal": 2, "different": 2, "absent_a": 0, "absent_b": 0, "files": 2}
    assert "| tic | 2/4 |" in compare.table_text(agg)
    assert compare.main(["--aggregate", str(out), "--out", str(tmp_path / "t.json")]) == 0


def test_verita_from_dump():
    d = verita_scandump.da_scandump(ORACLE, ("", "", "scandump/1"))
    assert verita_scandump._verita().valida(d) == []
    assert d["conteggi"] == {"spettri": {"ms1": 2, "ms2": 1}, "cromatogrammi": {"tic": 1}}
    s = d["scansioni"][1]
    assert (s["livello"], s["precursore_mz"], s["ce"], s["attivazione"], s["isolamento"]) == (2, 500.0, 30.0, "HCD", {"mz": 500.0, "inf": 1.0, "sup": 1.0})
    assert d["scansioni"][0]["profilo"] is True and d["scansioni"][0]["bpi_mz"] == 500.0
    assert d["punti"] == {"0": {"mz": [100.0, 500.0, 900.0], "intensita": [1000.0, 250000.0, 5000.0]}}
    assert d["oracolo"]["nome"] == "rfr"
