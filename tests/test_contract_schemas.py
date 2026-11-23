"""The JSON of the English-contract commands is documented with `schema NAME` and an example: `peak-auto-lr/1` (the real output of `peak auto-lr`,
also through the old `--set lr_*` names) and `hypotheses/3` (the registry of `tp.generate`). The examples respect their schemas (the Italian-word guards
are the Rust tests `english_json` and `lr_english`)."""
import json
import sys

import pytest
from jsonschema import Draft202012Validator

from cargo_util import ROOT, cli

sys.path.insert(0, str(ROOT / "tools"))
from synthetic_data_jagged import CASES, trace, write_csv  # noqa: E402

NAMES = {"peak-auto-lr": 1, "hypotheses": 3}


def schema(name):
    return json.loads((ROOT / f"docs/schemi/{name}-{NAMES[name]}.json").read_text())


@pytest.mark.parametrize("name", NAMES)
def test_the_schema_is_served_and_its_example_is_valid(name):
    s = schema(name)
    Draft202012Validator.check_schema(s)
    assert s["$id"] == f"{name}/{NAMES[name]}"
    assert json.loads(cli("schema", s["$id"]).stdout) == s
    listed = {x["name"] for x in json.loads(cli("schema", "--list").stdout)["schemas"]}
    assert s["$id"] in listed
    assert s["examples"], "a schema of the agent catalog carries an example"
    for ex in s["examples"]:
        errors = [f"{list(e.path)}: {e.message}" for e in Draft202012Validator(s).iter_errors(ex)]
        assert not errors, errors


def test_the_output_of_peak_auto_lr_respects_its_schema_and_the_catalog_names_it(tmp_path):
    name = next(iter(CASES))
    t, y, _ = trace(name, 1)
    f = write_csv(tmp_path / "t.csv", t, y)
    s = schema("peak-auto-lr")
    new = cli("peak", "auto-lr", str(f), "--set", "sn=4", "--set", "valley=0.3")
    doc = json.loads(new.stdout)
    errors = [f"{list(e.path)}: {e.message}" for e in Draft202012Validator(s).iter_errors(doc)]
    assert not errors, errors
    assert doc["peaks"], "the synthetic trace has peaks"
    # the output names are the --set names; the old dialog names are aliases that log a note and give the same bytes
    assert {"half", "valley", "sn", "min_pts", "despike"} <= set(doc["params"])
    old = cli("peak", "auto-lr", str(f), "--set", "lr_sn=4", "--set", "lr_valley=30")
    assert old.stdout == new.stdout
    assert "lr_sn is the old name of sn" in old.stderr
    cmds = {c["path"]: c for c in json.loads(cli("commands", "--json", "peak").stdout)["commands"]}
    assert cmds["peak auto-lr"]["stdout"]["schema"] == "peak-auto-lr/1"
