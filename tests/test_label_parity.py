"""The label of a sample of a multi-sample `.wiff`, one Rust function (`mzlab_core::readers::label`) behind three routes: `mzlab-cli features --sample --dry-run` (the CLI and, with the same
function, the app and the Flow), the engine .wasm `sample_labels` (the site, run by node) and the site's file name of the sample (`wiffName` of `mzlab/web/conversione.js` given that label).
Byte for byte on synthetic multi-sample files (`tools/wiff_grande.py --names`) whose names collide, inside one file and across two files, and hold `/`, `\\`, `@`, `#` and spaces.
Fails, never skips, in CI when a tool is missing: `test_the_tools_are_there_in_ci`; skipped only on a machine without cargo, wasm-bindgen or node."""
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node", "python3"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen, node or python3 not available")

# names with no comma (`wiff_grande.py --names` is comma-separated); each list is one file (all files of a batch hold as many samples: `--sample` is one list for all); names that clean to the same text (`A/B`, `A_B`) collide too
BATCHES = {
    "one file": [("grande", ["Sample_A", "  padded  ", "A/B\\:C*D?E\"F<G>H|I", "x" * 150, "with space 1.5", "at@5", "tag#7", "Ünï"])],
    "collision in one file": [("grande", ["S", "S", "A/B", "A_B", "at@5", "with space", "with space", "s"])],
    "collision across two files": [("run1", ["Shared", "Own 1", "Shared", "a/b"]), ("run2", ["Shared", "Own 2", "x y@2", "a_b"])],
    "one name everywhere": [("one", ["Same", "Same"]), ("two", ["Same", "Same"]), ("three", ["Same", "Other"])],
}
CONVERSIONE = ROOT / "mzlab" / "web" / "conversione.js"
SITE_LABELS = """const w = require(process.argv[1]);
console.log(w.sample_labels(process.argv[2]));"""     # `node -e`: the arguments start at argv[1]
SITE_NAMES = """const c = require(process.argv[1]);
const rows = JSON.parse(process.argv[2]);
console.log(JSON.stringify(rows.map(r => c.wiffName(r.stem, r.i, true, [], r.sample, r.label))));"""


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen, node and python3 must be installed in CI: this test would not run"


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("labels")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")))
    return d


def make(d, stem, names):
    made = subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(d), "--name", stem, "--samples", str(len(names)), "--scans", "40", "--points", "50", "--names", ",".join(names)],
                          capture_output=True, encoding="utf-8")
    assert made.returncode == 0, made.stderr
    return d / f"{stem}.wiff"


@pytest.mark.parametrize("batch", sorted(BATCHES))
def test_the_label_is_the_same_from_the_cli_and_the_site(tmp_path, pkg, batch):
    files = [make(tmp_path, stem, names) for stem, names in BATCHES[batch]]
    listed = [json.loads(cli("sample", "list", str(f), "--json").stdout)["files"][0]["samples"] for f in files]
    members = [{"file": f.name, "id": s["id"], "sample": s["name"]} for f, smp in zip(files, listed) for s in smp]
    assert [m["sample"] for m in members] == [n.strip() for _, names in BATCHES[batch] for n in names], members      # the names the file was made with (the reader trims them)
    args = [str(f) for f in files]
    plan = json.loads(cli("features", *args, "--sample", f"1-{len(listed[0])}", "--dry-run").stdout)
    cli_labels = [m["column"] for m in plan["members"]]
    assert len(cli_labels) == len(members), plan
    wasm = json.loads(subprocess.run(["node", "-e", SITE_LABELS, str(pkg / "pkg" / "mzlab_wasm.js"), json.dumps({"members": members})], cwd=ROOT, capture_output=True, encoding="utf-8", check=True).stdout)
    assert wasm["schema"] == "sample-labels/1"
    assert [x["label"] for x in wasm["labels"]] == cli_labels, (batch, wasm, cli_labels)
    # the site names the mzML of the sample after that label, without the extension of the file and with "_" for "#" (a "#" separates a file from its items in its list)
    rows = [{"stem": Path(m["file"]).stem, "i": k + 1, "sample": m["sample"], "label": lab} for k, (m, lab) in enumerate(zip(members, cli_labels))]
    named = json.loads(subprocess.run(["node", "-e", SITE_NAMES, str(CONVERSIONE), json.dumps(rows)], check=True, capture_output=True, encoding="utf-8").stdout)
    assert [n.removesuffix(".mzML") for n in named] == [c.removesuffix(".wiff").replace("#", "_") for c in cli_labels], (batch, named, cli_labels)
    assert len(set(cli_labels)) == len(cli_labels), cli_labels
    assert len({n.lower() for n in named}) == len(named), named


def test_a_tie_names_the_file_and_the_id_and_the_others_keep_their_name(tmp_path, pkg):
    f = make(tmp_path, "grande", ["S", "T", "S"])
    plan = json.loads(cli("features", str(f), "--sample", "1-3", "--dry-run").stdout)
    assert [m["column"] for m in plan["members"]] == ["S (grande #1).wiff", "T.wiff", "S (grande #3).wiff"]
