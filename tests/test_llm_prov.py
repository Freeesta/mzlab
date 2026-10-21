"""`$defs/prov1` of the draft contract accepts a `prov.json` with and without the optional `llm` block (the log of the assistant that launched the run)."""
import copy
import json
from pathlib import Path

from jsonschema import Draft202012Validator

CONTRACT = json.loads((Path(__file__).resolve().parents[1] / "docs/studi/workflow/contract.schema.json").read_text())
VALIDATOR = Draft202012Validator({**CONTRACT, "$ref": "#/$defs/prov1"})
PROV = {"schema": "prov/1", "agent": {"name": "mzLab", "version": "x", "platform": "native"}, "plan_sha256": "cd" * 32, "entity": {}, "activity": {}}
LLM = {"log_sha256": "ab" * 32, "turn": 2, "provider": "anthropic", "model": "m", "level": "L1"}


def test_prov1_with_and_without_llm():
    assert not list(VALIDATOR.iter_errors(PROV))
    with_llm = {**PROV, "llm": LLM}
    assert not list(VALIDATOR.iter_errors(with_llm))


def test_prov1_rejects_a_broken_llm_block():
    for key in LLM:
        broken = {**PROV, "llm": {k: v for k, v in LLM.items() if k != key}}
        assert list(VALIDATOR.iter_errors(broken)), key
    odd = copy.deepcopy({**PROV, "llm": {**LLM, "level": "L9"}})
    assert list(VALIDATOR.iter_errors(odd))
