"""The honest view of the Method window, from the real readers through the CLI (`mzlab-cli file method`, the same bytes as the site's .wasm: tests/test_file_method_wasm.py).
Synthetic .wiff files of tools/wiff_riferimento.py: `dadmetodo` (an LC method: pumps, Time Program with a Stop event after the last pump step, PDA, oven; no eluent names) and
`dispersione` (tandem segments). The document validates against docs/schemi/file_method-1.json; every LC value says where it comes from; the last %B is held to the end of the run and
marked inferred; nothing names an eluent the file does not; every display string has at most two decimals while the full value stays beside it."""
import json
import os
import re
import shutil
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli, run

ROOT = Path(__file__).resolve().parents[1]
CI = bool(os.environ.get("GITHUB_ACTIONS"))
pytestmark = pytest.mark.skipif(not shutil.which("cargo") and not CI, reason="cargo not available")
jsonschema = pytest.importorskip("jsonschema")


@pytest.fixture(scope="module")
def docs(tmp_path_factory):
    w = tmp_path_factory.mktemp("honest")
    run(sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(w))
    build("-p", "mzlab-cli")
    return {n: json.loads(cli("file", "method", str(w / f"{n}.wiff")).stdout) for n in ("dadmetodo", "dispersione")}


def test_both_documents_validate_against_the_schema(docs):
    schema = json.loads((ROOT / "docs" / "schemi" / "file_method-1.json").read_text(encoding="utf-8"))
    for name, d in docs.items():
        jsonschema.validate(d, schema)


def test_the_pump_program_is_held_to_the_end_of_the_run_and_the_hold_is_inferred(docs):
    lc = docs["dadmetodo"]["lc"]
    assert [(p["t_min"], p["B_pct"]) for p in lc["gradient"]] == [(0, 10), (0.5, 60), (1.5, 95)], "the points are only those of the file"
    assert lc["run_time_min"] == 2 and lc["run_time_shown"] == "2.00"
    assert lc["hold"]["from_min"] == 1.5 and lc["hold"]["to_min"] == 2 and lc["hold"]["B_pct"] == 95
    assert lc["sources"]["hold"]["kind"] == "inferred" and lc["sources"]["a_pct"]["kind"] == "inferred"


def test_every_lc_value_names_its_source_and_nothing_names_an_eluent(docs):
    lc = docs["dadmetodo"]["lc"]
    for key in ("gradient", "run_time", "flow", "oven"):
        s = lc["sources"][key]
        assert s["kind"] == "read" and "VendorAppMethod" in s["at"], (key, s)
    assert lc["eluents"] == [] and lc["sources"]["eluents"] is None
    text = json.dumps(docs["dadmetodo"]).lower()
    for invented in ("formic", "0.1%", "acetonitrile", "h2o"):
        assert invented not in text, invented


def test_the_dad_rows_carry_the_method_stream_as_their_source(docs):
    rows = {r["id"]: r for r in docs["dadmetodo"]["dad"]["rows"]}
    assert rows["range"]["shown"] == "210.00–400.00" and rows["range"]["unit"] == "nm" and rows["step"]["shown"] == "1.00"
    assert all(r["source"]["kind"] == "read" and "PDA Detector" in r["source"]["at"] for r in rows.values())


def test_every_display_string_has_at_most_two_decimals_and_the_full_value_stays(docs):
    seg = next(s for s in docs["dispersione"]["segments"] if s["ms_level"] == 2)
    assert seg["precursor_mz"] == 253.1 and seg["shown"]["precursor_mz"] == "253.10" and seg["shown"]["ce_eV"] == "35.00"

    def shown_strings(v, key=""):
        if isinstance(v, dict):
            for k, x in v.items():
                yield from shown_strings(x, k if k != "shown" else "shown")
        elif isinstance(v, list):
            for x in v:
                yield from shown_strings(x, key)
        elif isinstance(v, str) and (key == "shown" or key.endswith("_shown")):
            yield v
    for d in docs.values():
        for s in shown_strings(d):
            assert not re.search(r"\d+\.\d{3,}", s), s
