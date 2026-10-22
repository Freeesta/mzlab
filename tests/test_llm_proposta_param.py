"""`proposta-param/1` of `mzlab-llm`: the schema accepts the shape of 03-parametri.md §2 and rejects a proposal without proof or with an unknown field."""
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
V = Draft202012Validator(json.loads((ROOT / "docs/schemi/proposta-param-1.json").read_text()))
ONE = {
    "step": "features.detect@1", "algorithm": "centwave", "name": "peak_width_s", "value": [4.2, 16.8], "rule": "R-width-1",
    "signals": ["fwhm_median_s=8.4"], "confidence": "medium", "range_ok": True, "proof": "ispezione:f1:peak_width",
}


def doc(*proposals, **extra):
    return {"schema": "proposta-param/1", "series": "s1", "proposals": list(proposals), "missing": [{"signal": "blank_files", "asks": "Q-blank"}],
            "warnings": ["warn.params.mixed_polarity"], **extra}


def test_valid_document():
    assert not list(V.iter_errors(doc(ONE)))


def test_proposal_without_proof_or_with_unknown_field_is_rejected():
    assert list(V.iter_errors(doc({k: v for k, v in ONE.items() if k != "proof"})))
    assert list(V.iter_errors(doc({**ONE, "free_text": "x"})))
    assert list(V.iter_errors(doc({**ONE, "confidence": "certain"})))
