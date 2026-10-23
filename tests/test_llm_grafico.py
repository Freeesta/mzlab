"""The golden `grafico/1` of `mzlab-llm` validates against `docs/schemi/grafico-1.json`; a specification with a style, with rows from the model or without a view does not."""
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads((ROOT / "docs/schemi/grafico-1.json").read_text())
GOLDEN = json.loads((ROOT / "crates/mzlab-llm/tests/data/grafico_tic.json").read_text())
SPEC = {"schema": "grafico/1", "title": "TIC", "data": {"ref": "tic:*"}, "mark": "line",
        "encoding": {"x": {"field": "rt"}, "y": {"field": "intensity"}, "color": {"field": "file"}}, "why": "Overlaid TIC."}


def errors(doc):
    return list(Draft202012Validator(SCHEMA).iter_errors(doc))


def test_golden_and_specification_validate():
    assert not errors(GOLDEN) and not errors(SPEC)
    assert [f["name"] for f in GOLDEN["data"]["fields"]] == ["rt", "intensity", "file"]
    assert len(GOLDEN["data"]["rows"]) == GOLDEN["data"]["source_rows"] == 120
    assert not errors({"schema": "grafico/1", "error": "no_view", "why": "The concentration is in no view."})


def test_style_rows_and_unknown_marks_are_rejected():
    assert errors({**SPEC, "palette": ["red"]})
    assert errors({**SPEC, "mark": "pie"})
    assert errors({**SPEC, "data": {"ref": "tic:*", "rows": [[1, 2]]}})
    assert errors({**SPEC, "encoding": {"x": {"field": "rt", "scale": "log"}}})
    assert errors({**SPEC, "title": "x" * 61})
