"""Tests for mzflow creation/validation and MGF/MSP export and parsing."""
import json
from mzlab.workflow import (
    create_workflow,
    validate_workflow,
    verify_file_hashes,
    format_mgf,
    parse_mgf,
    format_msp,
    parse_msp,
    FORMAT_VERSION,
    unknown_operations,
)


def test_workflow_roundtrip():
    files = [
        {"name": "test_full.mzML", "sha256": "abcdef1234567890", "size": 1024, "type": "sample", "time": 0.0},
        {"name": "test_ms2.mzML", "sha256": "1234567890abcdef", "size": 2048, "type": "sample", "time": 15.0},
    ]
    params = {
        "ppm": 5,
        "dec": 4,
        "tema": "dark",
        "tavolozza": "rainbow",
        "fusione_isotopi": True,
        "picchi": {
            "win": 1.2,
            "areaNoise": 3.0,
            "edgeFrac": 3.0,
            "snMin": 0.0,
            "noise": "mad",
            "minHalf": 2,
            "multi": 50.0,
            "tail": 1.5,
            "constrain": 0.0,
            "nr0": 0.0,
            "nr1": 0.0,
        },
        "soglie_libreria": {"ent": 0.75, "shared": 6, "ppm": 5.0, "ent2": 0.60},
        "correzioni": {"snip": True, "snipw": 1.5},
        "filtri": {"prec": 317.2},
    }
    ops = [
        {"op": "load_file", "t": "2026-10-10T12:00:00Z", "file": "test_full.mzML"},
        {"op": "integration", "t": "2026-10-10T12:01:00Z", "panel": 0, "a": 2.4, "b": 2.6},
    ]
    panels = [
        {"type": "chrom", "kind": "tic", "title": "TIC"},
        {"type": "spec", "level": 2, "prec": 317.2, "title": "MS2"},
    ]

    wf = create_workflow(
        files=files,
        parameters=params,
        operations=ops,
        panels=panels,
        program_version="0.1.0",
        git_commit_hash="abc1234",
        language="it",
    )

    # Check structure
    assert wf["formato"] == FORMAT_VERSION
    assert wf["programma"]["versione"] == "0.1.0"
    assert wf["programma"]["commit"] == "abc1234"
    assert wf["lingua"] == "it"

    # Serialize to JSON and parse back
    raw_json = json.dumps(wf)
    wf_loaded = json.loads(raw_json)

    valid, msg = validate_workflow(wf_loaded)
    assert valid, msg

    # Verify all parameters match exactly
    assert wf_loaded["parametri"] == params
    assert wf_loaded["file"] == [
        {"nome": "test_full.mzML", "sha256": "abcdef1234567890", "dimensione": 1024, "tipo": "sample", "etichetta": "", "tempo": 0.0},
        {"nome": "test_ms2.mzML", "sha256": "1234567890abcdef", "dimensione": 2048, "tipo": "sample", "etichetta": "", "tempo": 15.0},
    ]
    assert wf_loaded["operazioni"] == ops
    assert wf_loaded["pannelli"] == panels


def test_workflow_file_hashes():
    wf = {
        "formato": FORMAT_VERSION,
        "file": [
            {"nome": "fileA.mzML", "sha256": "hash_a_123"},
            {"nome": "fileB.mzML", "sha256": "hash_b_456"},
        ],
        "parametri": {},
    }
    # Matching hashes
    actual_ok = {"fileA.mzML": "hash_a_123", "fileB.mzML": "hash_b_456"}
    assert verify_file_hashes(wf, actual_ok) == []

    # Mismatched hash
    actual_bad = {"fileA.mzML": "hash_a_123", "fileB.mzML": "hash_different_789"}
    mismatches = verify_file_hashes(wf, actual_bad)
    assert len(mismatches) == 1
    assert mismatches[0]["file"] == "fileB.mzML"
    assert mismatches[0]["expected"] == "hash_b_456"
    assert mismatches[0]["actual"] == "hash_different_789"


def test_mgf_export_and_parser():
    spectra = [
        {
            "sid": 101,
            "rt": 2.45,
            "prec": 255.1234,
            "charge": 1,
            "polarity": 1,
            "file": "test_sample.mzML",
            "peaks": [(100.05, 500.0), (150.10, 12000.5), (255.1234, 45000.0)],
        },
        {
            "sid": 102,
            "rt": 3.10,
            "prec": 317.0891,
            "charge": 1,
            "polarity": -1,
            "file": "test_sample.mzML",
            "peaks": [(85.02, 1200.0), (200.05, 8000.0)],
        },
    ]
    text = format_mgf(spectra)
    parsed = parse_mgf(text)
    assert len(parsed) == 2

    # Spectrum 1
    s1 = parsed[0]
    assert "scan=101" in s1["params"]["TITLE"]
    assert "test_sample.mzML" in s1["params"]["TITLE"]
    assert float(s1["params"]["RTINSECONDS"]) == round(2.45 * 60, 2)
    assert float(s1["params"]["PEPMASS"]) == 255.1234
    assert s1["params"]["CHARGE"] == "1+"
    assert len(s1["peaks"]) == 3
    assert s1["peaks"][0] == (100.05, 500.0)
    assert s1["peaks"][1] == (150.1, 12000.5)

    # Spectrum 2 (negative polarity)
    s2 = parsed[1]
    assert "scan=102" in s2["params"]["TITLE"]
    assert s2["params"]["CHARGE"] == "1-"
    assert len(s2["peaks"]) == 2


def test_msp_export_and_parser():
    spectra = [
        {
            "sid": 205,
            "rt": 4.12,
            "prec": 415.22,
            "polarity": 1,
            "file": "sample_b.mzML",
            "peaks": [(120.08, 1500.0), (240.15, 8500.0), (415.22, 60000.0)],
        }
    ]
    text = format_msp(spectra)
    parsed = parse_msp(text)
    assert len(parsed) == 1
    s = parsed[0]
    assert s["params"]["NAME"] == "sample_b.mzML_scan_205"
    assert float(s["params"]["PRECURSORMZ"]) == 415.22
    assert s["params"]["IONMODE"] == "Positive"
    assert s["params"]["PRECURSORTYPE"] == "[M+H]+"
    assert float(s["params"]["RETENTIONTIME"]) == 4.12
    assert int(s["params"]["NUM PEAKS"]) == 3
    assert len(s["peaks"]) == 3
    assert s["peaks"][0] == (120.08, 1500.0)


def test_lr_export_principle():
    """In LR mode, no compound names or annotations are included (Principle 1)."""
    spectrum = {
        "sid": 50,
        "rt": 1.5,
        "prec": 200.0,
        "file": "sample_lr.mzML",
        "title_extra": "Candidate Molecule XYZ (verdict strong)",
        "name_extra": "Candidate Molecule XYZ",
        "peaks": [(100.0, 500.0)],
    }
    mgf_lr = format_mgf([spectrum], lr=True)
    assert "Candidate Molecule" not in mgf_lr
    assert "XYZ" not in mgf_lr

    msp_lr = format_msp([spectrum], lr=True)
    assert "Candidate Molecule" not in msp_lr
    assert "XYZ" not in msp_lr


def test_workflow_validate_and_export_without_route():
    # the three POST routes (workflow/validate, export/mgf, export/msp) are gone: the functions stay as a reference
    wf = {"formato": FORMAT_VERSION, "file": [], "parametri": {"ppm": 5}}
    assert validate_workflow(wf)[0] is True
    spectra = [{"sid": 1, "rt": 1.0, "prec": 100.0, "peaks": [[50.0, 10.0]]}]
    assert "BEGIN IONS" in format_mgf(spectra, lr=True)
    assert "PRECURSORMZ: 100.0" in format_msp(spectra, lr=True)
    from mzlab.api import dispatch
    from mzlab.app import App
    from io import BytesIO
    code, *_ = dispatch(App(), "POST", "/api/export/mgf", {}, BytesIO(b"{}"), 2)
    assert code == 404


def test_mzflow_format_and_legacy_name():
    assert FORMAT_VERSION == "mzflow/1"
    wf = create_workflow([], {"ppm": 5})
    assert wf["formato"] == "mzflow/1"
    assert validate_workflow(wf)[0] is True
    old = dict(wf, formato="mzworkflow/1")      # old files are still read
    assert validate_workflow(old)[0] is True
    assert validate_workflow(dict(wf, formato="altro/1"))[0] is False


def test_unknown_operations_are_reported_not_fatal():
    wf = create_workflow([], {}, operations=[{"op": "export_report"}, {"op": "futura_op"}, {"op": "futura_op"}])
    assert validate_workflow(wf)[0] is True
    assert unknown_operations(wf) == ["futura_op"]
