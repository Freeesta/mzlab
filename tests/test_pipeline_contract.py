"""Contract of the Feature table of mzFinder with the table engine: every key of the JSON of `qc/1`, of the view and of the rows of the sheet that TP_Mine/js/36_feature_qc.js and
37_feature_table.js and 38_feature_export.js read (`PIPEQC.CONTRACT`) is in the real output of the engine (the native example `tabella`, the same bytes as the .wasm: tests/test_feat_table_wasm.py), so a number the
page shows cannot come from anywhere but the engine. Skipped without cargo, wasm-bindgen or node (the series is the one of test_feat_table_wasm.py)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest
from test_feat_table_wasm import ROOT, SPEC, built, run  # noqa: F401  (the fixture `built` makes the series and the engine)

JS = ROOT / "TP_Mine" / "js"
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def contract():
    code = "globalThis.window = globalThis; require(process.argv[1]); console.log(JSON.stringify(PIPEQC.CONTRACT))"
    return json.loads(run("node", "-e", code, str(JS / "36_feature_qc.js")))


@pytest.fixture(scope="module")
def outs(built, tmp_path_factory):
    root, idx, _pkg, table = built
    out = tmp_path_factory.mktemp("contract")
    run("target/release/examples/tabella", str(out), str(table), str(root / "campioni.json"), str(root / "spec.json"), "1", *idx)
    return {p.name: json.loads(p.read_text()) for p in out.glob("*.json")}


def test_the_keys_the_page_reads_are_in_the_json_of_the_engine(outs):
    c = contract()
    qc, view, rows = outs["qc.json"], outs["view.json"], outs["features.rows.json"]
    assert qc["schema"] == "qc/1"
    assert set(c["qc"]) <= set(qc) and qc["file"] and set(c["qcFile"]) <= set(qc["file"][0])
    assert set(c["blank"]) <= set(qc["bianchi"]) and all(set(c["rejected"]) <= set(x) for x in qc["scartate"]) and qc["scartate"] and (qc["m1_su_m"] is None or set(c["m1"]) <= set(qc["m1_su_m"]))
    assert qc["gruppi"] == [] or set(c["qcGroup"]) <= set(qc["gruppi"][0])
    assert set(c["view"]) <= set(view) and set(c["counts"]) <= set(view["conteggi"])
    assert set(c["rows"]) <= set(rows) and set(c["sheet"]) <= set(rows["hojas"][0])
    rep = outs["report.json"]
    assert rep["schema"] == "report/1" and set(c["report"]) <= set(rep) and set(c["cells"]) <= set(rep["celle"]) and set(c["reportView"]) <= set(rep["vista"] or {"parametri": 0, "conteggi": 0})
    # the grid reads the sheet by position: six columns, then area and state of each file
    n = len(qc["file"])
    assert all(len(r) == 6 + 2 * n for r in rows["hojas"][0]["filas"])


def test_the_keys_of_the_row_are_in_the_row_and_its_traces(outs):
    c, row, tr = contract(), outs["riga.json"], outs["riga_tracce.json"]
    assert row["schema"] == "riga/1" and set(c["row"]) <= set(row) and row["id"] == 3
    assert row["quality"] is None or set(c["rowQuality"]) <= set(row["quality"])
    assert row["ms2"] is None or set(c["rowMs2"]) <= set(row["ms2"])
    assert row["ident"] is None or set(c["rowIdent"]) <= set(row["ident"])
    assert len(row["cells"]) == len(outs["qc.json"]["file"]) and all(set(c["rowCell"]) <= set(x) for x in row["cells"])
    assert tr["schema"] == "riga_tracce/1" and set(c["traces"]) <= set(tr) and all(set(c["traceXic"]) <= set(x) for x in tr["xic"])
    assert len(tr["xic"]) == len(row["cells"]) and all(0 < len(x["rt"]) == len(x["y"]) <= 400 for x in tr["xic"])
    assert tr["ms1"] is None or set(c["traceMs1"]) <= set(tr["ms1"])
    assert tr["ms2"] is None or set(c["traceMs2"]) <= set(tr["ms2"])


def test_the_page_names_no_key_the_contract_does_not_list():
    src = (JS / "36_feature_qc.js").read_text() + (JS / "37_feature_table.js").read_text() + (JS / "38_feature_export.js").read_text()
    for key in [k for ks in contract().values() for k in ks]:
        assert key in src, key


def test_the_spec_of_the_page_is_read_by_the_engine(outs):
    assert SPEC["normalizza"]["metodo"] in ("somma", "mediana", "nessuna")
    assert outs["view.json"]["conteggi"]["tenute"] == len(outs["view.json"]["righe"])
