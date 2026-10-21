"""The catalog strumenti/1 (docs/schemi/strumenti-1.json, made by `mzlab-cli llm tools`): valid JSON Schema, the ids of ui_show exist in the registry of
actions, and no Italian in what the model reads."""
import json
import re
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "mzlab" / "web"
CATALOG = json.loads((ROOT / "docs" / "schemi" / "strumenti-1.json").read_text(encoding="utf-8"))

# Actions the page registers later (the LLM application PRs); the list only gets shorter.
NOT_YET = {"table.showRow", "flow.selectNode"}

ITALIAN = {"il", "della", "delle", "nel", "nella", "che", "sono", "una", "dei", "degli", "anche", "passo", "parametro", "valore", "gli", "dello"}


def ui_actions():
    return [b["x-action"] for b in next(t for t in CATALOG["tools"] if t["name"] == "ui_show")["input_schema"]["oneOf"]]


def test_header_and_tools():
    assert CATALOG["schema"] == "strumenti/1"
    # the file carries no hash of the registry: it would be rewritten by every change of the registry
    assert set(CATALOG) == {"schema", "tools"}
    assert len(CATALOG["tools"]) == 15
    for t in CATALOG["tools"]:
        assert set(t) == {"name", "class", "description", "input_schema"}
        assert t["class"] in {"read", "propose", "edit", "run", "ui"}


@pytest.mark.parametrize("tool", [t["name"] for t in CATALOG["tools"]])
def test_input_schema_is_valid_json_schema(tool):
    schema = next(t for t in CATALOG["tools"] if t["name"] == tool)["input_schema"]
    Draft202012Validator.check_schema(schema)


def test_a_valid_and_an_invalid_call():
    schema = next(t for t in CATALOG["tools"] if t["name"] == "flow_set_param")["input_schema"]
    branch = schema["oneOf"][0]["properties"]
    ref, name = branch["step"]["const"], branch["name"]["enum"][0]
    v = Draft202012Validator(schema)
    assert not v.is_valid({"instance": "x", "step": ref, "name": "no_such_parameter", "value": 1})
    assert not v.is_valid({"step": ref, "name": name, "value": 1})


def test_ui_actions_are_registered():
    registered = set()
    for f in WEB.rglob("*.js"):
        if "vendor" in f.parts:
            continue
        registered |= set(re.findall(r'(?:reg|registra)\(\s*\{\s*id:\s*"([^"]+)"', f.read_text(encoding="utf-8", errors="ignore")))
    for a in ui_actions():
        assert a in registered or a in NOT_YET, f"{a} is not registered in mzlab/web (azioni/1)"
    for a in NOT_YET:
        assert a not in registered, f"{a} is registered now: drop it from NOT_YET"


def test_no_italian_words():
    def texts(x):
        if isinstance(x, dict):
            for k, v in x.items():
                if k in ("description", "title"):
                    yield v
                else:
                    yield from texts(v)
        elif isinstance(x, list):
            for v in x:
                yield from texts(v)

    for t in texts(CATALOG["tools"]):
        words = set(re.findall(r"[a-zà-ù]+", t.lower()))
        assert not (words & ITALIAN), f"Italian in: {t}"
