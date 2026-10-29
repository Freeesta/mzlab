"""`mzrun/1` (docs/schemi/mzrun-1.json, docs/agenti/mzrun.md): the synthetic record and preset of tests/data/mzrun validate, the mode rules and the
privacy rule refuse what they must, and `self_sha256` follows the canonical text. No engine involved: the files are format examples, never lab data."""
import copy
import hashlib
import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads((ROOT / "docs/schemi/mzrun-1.json").read_text())
CONTRACT = json.loads((ROOT / "docs/studi/workflow/contract.schema.json").read_text())
EXAMPLES = ROOT / "tests/data/mzrun"


def retrieve(uri):
    # the schema refers to the contract by its $id; any base the reference is joined with ends in that id
    assert uri.endswith("workflow-contract/1-draft"), uri
    return Resource.from_contents(CONTRACT)


VALIDATOR = Draft202012Validator(SCHEMA, registry=Registry(retrieve=retrieve))


def load(name):
    return json.loads((EXAMPLES / f"{name}.mzrun.json").read_text())


def errors(doc):
    return list(VALIDATOR.iter_errors(doc))


def canonical(doc):
    return json.dumps(doc, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def self_sha256(doc):
    return hashlib.sha256(canonical({k: v for k, v in doc.items() if k != "self_sha256"}).encode()).hexdigest()


def test_the_schema_is_valid():
    Draft202012Validator.check_schema(SCHEMA)


@pytest.mark.parametrize("name", ["record", "preset"])
def test_examples_validate(name):
    assert not errors(load(name))


@pytest.mark.parametrize("name", ["record", "preset"])
def test_self_sha256_matches_the_canonical_text(name):
    doc = load(name)
    assert doc["self_sha256"] == self_sha256(doc)
    assert (EXAMPLES / f"{name}.mzrun.json").read_text() == canonical(doc) + "\n"


def test_self_sha256_follows_the_content():
    doc = load("record")
    doc["seeds"]["detect.init"] = 8
    assert doc["self_sha256"] != self_sha256(doc)


def test_results_digest_is_the_sorted_lines():
    r = load("record")["results"]
    lines = "".join(f"{k}\t{v}\n" for k, v in sorted(r["entities"].items()))
    assert r["results_digest"] == hashlib.sha256(lines.encode()).hexdigest()


def test_a_preset_with_inputs_fails():
    doc = load("preset")
    doc["inputs"] = load("record")["inputs"]
    assert errors(doc)


@pytest.mark.parametrize("key", ["prov", "results"])
def test_a_preset_with_record_parts_fails(key):
    doc = load("preset")
    doc[key] = load("record")[key]
    assert errors(doc)


def test_a_preset_without_slots_fails():
    doc = load("preset")
    del doc["slots"]
    assert errors(doc)


@pytest.mark.parametrize("key", ["results", "inputs", "prov", "roles_sha256"])
def test_a_record_without_a_required_part_fails(key):
    doc = load("record")
    del doc[key]
    assert errors(doc)


def test_a_record_with_slots_fails():
    doc = load("record")
    doc["slots"] = load("preset")["slots"]
    assert errors(doc)


@pytest.mark.parametrize("alias", ["/home/someone/S01", "C:\\data\\S01", "~/S01", "a/b", "https://x.org/S01"])
def test_a_path_in_an_alias_fails(alias):
    doc = copy.deepcopy(load("record"))
    doc["inputs"][0]["alias"] = alias
    assert errors(doc)


@pytest.mark.parametrize("name", ["/home/someone/S01.mzML", "C:\\data\\S01.mzML", "~/S01.mzML", "\\\\server\\share\\S01.mzML"])
def test_an_absolute_path_in_a_name_fails(name):
    doc = copy.deepcopy(load("record"))
    doc["inputs"][0]["name"] = name
    assert errors(doc)
    assert not errors({**doc, "inputs": [{**doc["inputs"][0], "name": "S01.mzML"}, *doc["inputs"][1:]]})


def test_an_unknown_limit_code_fails():
    doc = load("record")
    doc["limits"] = [{"code": "made.up"}]
    assert errors(doc)


def test_a_short_commit_fails():
    doc = load("record")
    doc["engine"]["commit"] = "abc"
    assert errors(doc)
