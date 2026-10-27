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


def _dump_msn(path, rows):
    head = {"type": "file", "schema": "scandump/1", "name": "x", "format_version": 66, "instrument": "Synthetic FT", "controller": 0, "n_scans": len(rows)}
    path.write_text("\n".join(json.dumps(r) for r in [head, *rows]) + "\n", encoding="utf-8")


def test_msn_chains_are_compared_per_level(tmp_path):
    """MS3 CID then HCD and an MS8 chain: per-level fields, and raw_double of the early steps against RFR's isolation width."""
    ms3 = lambda w, raw: {"type": "scan", "n": 3, "level": 3, "reactions": [
        {"precursor": 500.0, "activation": "CID", "energy": 35.0, **({"iso_width": w} if w is not None else {}), **({"raw_double": raw} if raw is not None else {})},
        {"precursor": 300.0, "activation": "HCD", "energy": 30.0, "iso_width": 1.2}]}
    ms8 = lambda: {"type": "scan", "n": 8, "level": 8, "reactions": [{"precursor": 100.0 + i, "activation": "CID", "energy": 25.0} for i in range(7)]}
    _dump_msn(tmp_path / "a.jsonl", [{"type": "scan", "n": 1, "level": 1, "reactions": []}, ms3(2.0, None), ms8()])
    _dump_msn(tmp_path / "b.jsonl", [{"type": "scan", "n": 1, "level": 1, "reactions": []}, ms3(None, 2.0), ms8()])
    f = compare.compare(tmp_path / "a.jsonl", tmp_path / "b.jsonl", "hr")["fields"]
    assert (f["reactions.iso_width~raw_double@ms3"]["compared"], f["reactions.iso_width~raw_double@ms3"]["equal"]) == (1, 1)
    assert f["reactions.activation@ms3"]["equal"] == 2 and f["reactions.activation@ms8"]["compared"] == 7
    assert f["reactions.iso_width@ms3"]["absent_b"] == 1 and f["reactions.iso_width@ms3"]["equal"] == 1
    assert "reactions.activation@ms2" not in f and "reactions.iso_width~raw_double@ms8" not in f
    _dump_msn(tmp_path / "c.jsonl", [{"type": "scan", "n": 1, "level": 1, "reactions": []}, ms3(None, 3.0), ms8()])
    g = compare.compare(tmp_path / "a.jsonl", tmp_path / "c.jsonl", "hr")["fields"]["reactions.iso_width~raw_double@ms3"]
    assert (g["equal"], g["different"]) == (0, 1)


def _dump(path: Path, rows: list) -> Path:
    head = {"type": "file", "schema": "scandump/1", "name": "x", "producer": "t", "instrument": "Q Exactive", "controller": 0, "n_scans": 0}
    path.write_text("\n".join(json.dumps(r) for r in [head, *rows]), encoding="utf-8")
    return path


def test_rows_that_are_not_scans_are_compared_field_by_field(tmp_path):
    """Sequence, autosampler text, error log, status log and tune: equal / different / only theirs / only ours."""
    oracle = _dump(tmp_path / "a.jsonl", [
        {"type": "sequence", "row_number": 4, "sample_type": "Unknown", "id": "S1", "user_text": ["a", ""]},
        {"type": "asinfo", "text": "Autosampler"},
        {"type": "error", "rt": 1.5, "message": "Warning: spray"},
        {"type": "status", "scan": 0, "rt": 0.1, "values": {"Vacuum:": "1e-5", "Temp": "30"}},
        {"type": "tune", "values": {"Spray Voltage": "2000"}},
    ])
    ours = _dump(tmp_path / "b.jsonl", [
        {"type": "sequence", "row_number": 4, "id": "S2", "user_text": ["a", ""], "unknown_numbers": {"unk1": 1}},
        {"type": "asinfo", "text": "autosampler", "preamble": "00"},
        {"type": "error", "rt": 1.5, "message": "Warning: spray"},
        {"type": "status", "scan": 0, "rt": 0.1, "values": {"Vacuum": "1.0e-5", "Temp": "31"}},
    ])
    f = compare.compare(oracle, ours)["fields"]
    cells = lambda k: (f[k]["compared"], f[k]["equal"], f[k]["different"], f[k]["absent_a"], f[k]["absent_b"])
    assert cells("sequence.row_number") == (1, 1, 0, 0, 0)
    assert cells("sequence.id") == (1, 0, 1, 0, 0)
    assert cells("sequence.user_text") == (1, 1, 0, 0, 0)
    assert cells("sequence.sample_type") == (0, 0, 0, 0, 1)  # only RFR has it
    assert cells("sequence.unknown_numbers") == (0, 0, 0, 1, 0)  # only ours (extra)
    assert cells("asinfo.text") == (1, 1, 0, 0, 0)
    assert cells("error.message") == (1, 1, 0, 0, 0)
    assert cells("status.vacuum") == (1, 1, 0, 0, 0)  # 1e-5 against 1.0e-5
    assert cells("status.temp") == (1, 0, 1, 0, 0)
    assert cells("tune.spray voltage") == (0, 0, 0, 0, 1)


def test_ms1_range_reaction_of_rfr_is_not_a_reaction():
    ms1 = {"level": 1, "mz_low": 350.0, "mz_high": 1050.0, "reactions": [{"precursor": 700, "iso_width": 700, "activation": "HCD"}]}
    assert compare.reactions_of(ms1) == []
    ms2 = dict(ms1, level=2)
    assert len(compare.reactions_of(ms2)) == 1
    other = dict(ms1, reactions=[{"precursor": 500, "iso_width": 2}])
    assert len(compare.reactions_of(other)) == 1


def test_ms1_reaction_matching_a_filter_range_is_not_a_reaction():
    react = {"precursor": 442.036, "iso_width": 617.272, "activation": "HCD"}
    ms1 = {"level": 1, "filter": "FTMS + p ESI Full ms [133.4000-750.6720]", "mz_low": 133.0, "mz_high": 751.0, "reactions": [react]}
    assert compare.reactions_of(ms1) == []
    two = dict(ms1, filter="FTMS + p ESI Full ms [100.0000-200.0000, 133.4000-750.6720]", mz_low=100.0, mz_high=751.0)
    assert compare.reactions_of(two) == []
    assert len(compare.reactions_of(dict(ms1, level=2))) == 1
    assert len(compare.reactions_of(dict(ms1, reactions=[dict(react, precursor=500)]))) == 1
