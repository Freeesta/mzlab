"""quant-targets-plan/1 (`mzlab-cli quant targets --dry-run`) and the `members` of quant-targets/1 (`--json`) respect their schemas, with and without `--sample`, on a synthetic
multi-sample `.wiff` (`tools/wiff_grande.py`); a plan with an extra or a missing member is refused."""
import json
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]


def schema(name):
    return json.loads((ROOT / f"docs/schemi/{name}-1.json").read_text(encoding="utf-8"))


@pytest.fixture(scope="module")
def made(tmp_path_factory):
    d = tmp_path_factory.mktemp("quant_plan_schema")
    out = subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(d), "--samples", "3", "--scans", "40", "--points", "50"], capture_output=True, encoding="utf-8")
    assert out.returncode == 0, out.stderr
    (d / "targets.csv").write_text("id,mz,rt_min,rt_max\nT1,229.1,0.2,1.0\n", encoding="utf-8")
    build("-p", "mzlab-cli")
    return d


def answer(d, *more):
    return json.loads(cli("quant", "targets", "--targets", str(d / "targets.csv"), str(d / "grande.wiff"), *more).stdout)


@pytest.mark.parametrize("name", ["quant-targets-plan", "quant-targets"])
def test_the_schema_is_valid_json_schema(name):
    s = schema(name)
    Draft202012Validator.check_schema(s)
    assert s["$id"] == f"{name}/1"


@pytest.mark.parametrize("sample", [[], ["--sample", "1,3"]], ids=["first sample", "--sample"])
def test_the_plan_and_the_json_answer_respect_their_schema(made, sample):
    for name, more in (("quant-targets-plan", ["--dry-run"]), ("quant-targets", ["--json"])):
        doc = answer(made, *sample, *more)
        errors = sorted(Draft202012Validator(schema(name)).iter_errors(doc), key=lambda e: list(e.path))
        assert not errors, [f"{list(e.path)}: {e.message}" for e in errors]
        assert ("members" in doc) == bool(sample)


def test_a_plan_with_an_extra_key_or_a_member_without_its_identity_is_refused(made):
    v = Draft202012Validator(schema("quant-targets-plan"))
    plan = answer(made, "--sample", "2", "--dry-run")
    assert not list(v.iter_errors(plan))
    assert list(v.iter_errors({**plan, "extra": 1}))
    assert list(v.iter_errors({**plan, "members": [{"name": "x"}]}))
