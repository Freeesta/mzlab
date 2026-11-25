"""`quant.targets@1` in the `quanttargets` .wasm (mzlab-feat-wasm, feature `quant_targets`, through Node: `flow_quant_targets`, the function the browser
calls over the .mzidx of OPFS) gives the same table and log bytes as `mzlab-cli quant targets` on the mzML files and as the native step of a `flow run`
on the synthetic unit-resolution files of `tools/lr_synthetic.py`, with the shape gates off (the defaults) and on (`min_half`); also with the internal RT
reference on (`rt_ref`), on batches whose retention times drifted (a shift, a rival peak, a clipped peak, a blank): the wasm, the CLI and the `flow run` step
give the same bytes. Skipped on a machine without cargo, wasm-bindgen, node or numpy; in CI (GITHUB_ACTIONS) those are required and a missing one fails
the run, never skips it. Run by the `rest` shards of test.yml."""
import base64
import json
import math
import os
import random
import shutil
import struct
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
# skipped on a machine without the tools, but never silently in CI: there it must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")

NAMES = ["LR_bench-noise-s1.mzML", "LR_bench-blank-s1.mzML", "LR_bench-s1.mzML"]
BLANK = "LR_bench-blank*"
# the parameters as the page sends them: the defaults of the step merged with the ones the plan writes
PARAMS = {"da": 0.5, "min_sn": 3, "min_points": 5, "min_shape": 0.5, "min_half": 0, "max_fwhm": 0, "max_asym": 0, "offset": "0", "blank_pattern": BLANK, "ms_level": 1, "smooth_half": 0, "valley": 10}
NODE = """const [dir, params, names, targets, ...paths] = process.argv.slice(1);
const w = require(dir + '/mzlab_feat_wasm.js'), fs = require('fs');
const fds = paths.map(p => fs.openSync(p, 'r')), sizes = paths.map(p => fs.statSync(p).size);
const read = (k, off, len) => { const b = new Uint8Array(len); fs.readSync(fds[k], b, 0, len, off); return b; };
try { const r = w.flow_quant_targets(read, sizes, names, fs.readFileSync(targets), params); process.stdout.write(JSON.stringify({ csv: r.csv(), log: r.log(), warnings: JSON.parse(r.warnings()) })); }
catch (e) { process.stdout.write('ERROR ' + String(e)); }"""


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"
    if CI:
        import numpy  # noqa: F401  (required in CI: a missing numpy fails, never skips)
    else:
        pytest.importorskip("numpy")
    d = tmp_path_factory.mktemp("quant_targets")
    import lr_synthetic
    lr_synthetic.make(d, 1)
    truth = json.loads((d / "LR_bench-s1_truth.json").read_text(encoding="utf-8"))
    rows = ["id,mz,rt_min,rt_max"] + [f"c{c['id']},{c['mz']},{c['rt_s'] / 60 - 0.3:.3f},{c['rt_s'] / 60 + 0.3:.3f}" for c in truth["compounds"][:15]]
    rows.append("away,333.3,0.1,0.4")   # no peak there: an absent row
    (d / "targets.csv").write_text("\n".join(rows) + "\n", encoding="utf-8")
    # the same list with its first row as the internal reference (role standard) for rt_ref
    ref = ["id,mz,rt_min,rt_max,polarity,role"] + [r + (",,standard" if k == 0 else ",,") for k, r in enumerate(rows[1:])]
    (d / "targets_ref.csv").write_text("\n".join(ref) + "\n", encoding="utf-8")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", "quant_targets")))
    build("-p", "mzlab-cli")
    plan = {"format": "mzflow/2", "inputs": [{"name": n, "type": "raw"} for n in NAMES] + [{"name": "targets.csv", "type": "report"}],
            "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n for n in NAMES]}},
                      {"id": "qt", "step": "quant.targets@1", "in": {"raw": ["idx.index"], "targets": "file:targets.csv"}, "params": {"blank_pattern": BLANK}}]}
    (d / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    cli("flow", "run", str(d / "plan.json"), "--in", str(d), "--out", str(d / "run"))
    return d


def wasm(work, targets, params=PARAMS, names=NAMES, runs="run"):
    paths = [str(work / runs / f"idx-{Path(n).stem}.mzidx") for n in names]
    out = run("node", "-e", NODE, str(work / "pkg"), json.dumps(params), json.dumps(names), str(targets), *paths).stdout
    return out if out.startswith("ERROR") else json.loads(out)


def test_wasm_cli_and_flow_run_give_the_same_table_and_log(work):
    got = wasm(work, work / "targets.csv")
    assert not isinstance(got, str), got
    log = work / "cli.log.json"
    table = cli("quant", "targets", "--targets", str(work / "targets.csv"), *[str(work / n) for n in NAMES], "--blank-pattern", BLANK, "--log", str(log)).stdout
    assert got["csv"] == table == (work / "run" / "qt.csv").read_text(encoding="utf-8")
    assert got["log"] == log.read_text(encoding="utf-8") == (work / "run" / "qt.log.json").read_text(encoding="utf-8")
    states = [r.split(",")[7] for r in got["csv"].splitlines()[1:]]
    assert len(states) == 3 * 16 and "detected" in states and "absent" in states, states
    # a peak under a threshold is a forced row: the hint (a warning key, one text for every surface) is the same list in the three
    assert got["warnings"] == (["warn.quant.targets.forced"] if "forced" in states else []), got["warnings"]


def test_the_shape_gates_give_the_same_table_log_and_decisions_in_the_wasm_and_the_cli(work):
    params = {**PARAMS, "min_half": 40, "max_fwhm": 1, "max_asym": 1.001}
    got = wasm(work, work / "targets.csv", params)
    assert not isinstance(got, str), got
    log = work / "gates.log.json"
    table = cli("quant", "targets", "--targets", str(work / "targets.csv"), *[str(work / n) for n in NAMES], "--blank-pattern", BLANK,
                "--set", "min_half=40", "--set", "max_fwhm=1", "--set", "max_asym=1.001", "--log", str(log)).stdout
    assert got["csv"] == table
    assert got["log"] == log.read_text(encoding="utf-8")
    d = json.loads(got["log"])
    assert d["params"]["min_half"] == 40 and d["decisions"], d["decisions"]
    assert {c["name"] for x in d["decisions"] for c in x["checks"]} >= {"half_pts", "fwhm_s", "asym"}


def test_an_error_has_the_same_key_in_the_wasm(work):
    bad = work / "bad.csv"
    bad.write_text("id,mz\nx,1\n", encoding="utf-8")
    got = wasm(work, bad)
    assert isinstance(got, str) and got.startswith("ERROR") and "err.quant.targets.table" in got, got


def test_rt_ref_gives_the_same_bytes_in_the_wasm_and_the_cli(work):
    got = wasm(work, work / "targets_ref.csv", {**PARAMS, "rt_ref": "auto", "rt_span": 0.6})
    assert not isinstance(got, str), got
    log = work / "cli_rt.log.json"
    table = cli("quant", "targets", "--targets", str(work / "targets_ref.csv"), *[str(work / n) for n in NAMES], "--blank-pattern", BLANK,
                "--rt-ref", "auto", "--log", str(log)).stdout
    assert got["csv"] == table
    assert got["log"] == log.read_text(encoding="utf-8")
    assert got["csv"].splitlines()[0].endswith(",mz_offset_da,rt_shift_min")
    assert '"rt_ref":{"reference":"c' in got["log"] and '"key":"rt_reference"' in got["log"], got["log"][:2000]


# --- retention times that drifted between batches: synthetic MS1 centroid files (an ion = m/z, apex RT in min, height, sigma in min) ---
DRIFT_TARGETS = "id,mz,rt_min,rt_max,polarity,role\nclean,200.0,4.8,5.2,+,\nstd,400.0,1.5,2.5,+,standard\n"
ION_CLEAN, ION_STD = (200.0, 5.0, 20000.0, 0.1), (400.3, 2.0, 50000.0, 0.1)
DRIFT = {   # file -> its ions: a reference as it is, drifted 0.3 min, with a rival peak of 80 %, clipped by the search window, and a drifted blank
    "RT_a.mzML": [ION_CLEAN, ION_STD],
    "RT_b.mzML": [(ION_CLEAN[0], ION_CLEAN[1] + 0.3, *ION_CLEAN[2:]), (ION_STD[0], ION_STD[1] + 0.3, *ION_STD[2:])],
    "RT_rival.mzML": [ION_CLEAN, ION_STD, (400.3, 2.5, 40000.0, 0.1)],
    "RT_edge.mzML": [ION_CLEAN, (400.3, 3.1, 50000.0, 0.1)],
    "RT_blank.mzML": [(ION_CLEAN[0], ION_CLEAN[1] + 0.3, *ION_CLEAN[2:]), (ION_STD[0], ION_STD[1] + 0.3, *ION_STD[2:])],
}


def mzml_text(ions, seed):
    """200 centroid MS1 scans every 0.05 min on a 100 +- 20 baseline: the same bytes for the same seed."""
    rng, ions = random.Random(seed), sorted(ions)
    arr = lambda acc, v: ('<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/><cvParam cvRef="MS" accession="MS:1000576" name="no compression"/>'
                          f'<cvParam cvRef="MS" accession="{acc}" name=""/><binary>{base64.b64encode(struct.pack("<%dd" % len(v), *v)).decode()}</binary></binaryDataArray>')
    out = ['<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="200">']
    for i in range(200):
        rt = i * 0.05
        y = [max(1.0, 100 + 20 * rng.uniform(-1, 1) + o[2] * math.exp(-((rt - o[1]) ** 2) / (2 * o[3] ** 2))) for o in ions]
        out.append(f'<spectrum index="{i}" id="scan={i + 1}" defaultArrayLength="{len(ions)}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/>'
                   '<cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="MS:1000130" name="positive scan"/>'
                   f'<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{rt}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList>'
                   f'<binaryDataArrayList count="2">{arr("MS:1000514", [o[0] for o in ions])}{arr("MS:1000515", y)}</binaryDataArrayList></spectrum>')
    out.append("</spectrumList></run></mzML>")
    return "".join(out)


@pytest.fixture(scope="module")
def drift(work):
    names = list(DRIFT)
    for k, n in enumerate(names):
        (work / n).write_text(mzml_text(DRIFT[n], 7 + k), encoding="utf-8")
    (work / "drift_targets.csv").write_text(DRIFT_TARGETS, encoding="utf-8")
    plan = {"format": "mzflow/2", "inputs": [{"name": n, "type": "raw"} for n in names] + [{"name": "drift_targets.csv", "type": "report"}],
            "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n for n in names]}},
                      {"id": "qt", "step": "quant.targets@1", "in": {"raw": ["idx.index"], "targets": "file:drift_targets.csv"}, "params": {"blank_pattern": "RT_blank*", "rt_ref": "auto"}}]}
    (work / "drift_plan.json").write_text(json.dumps(plan), encoding="utf-8")
    cli("flow", "run", str(work / "drift_plan.json"), "--in", str(work), "--out", str(work / "run_drift"))
    return names


def test_a_drifted_batch_a_rival_a_clipped_peak_and_a_blank_give_the_same_bytes_in_the_wasm_the_cli_and_flow_run(work, drift):
    params = {**PARAMS, "blank_pattern": "RT_blank*", "rt_ref": "auto", "rt_span": 0.6}
    got = wasm(work, work / "drift_targets.csv", params, drift, "run_drift")
    assert not isinstance(got, str), got
    log = work / "cli_drift.log.json"
    files = [str(work / n) for n in drift]
    args = ("quant", "targets", "--targets", str(work / "drift_targets.csv"), *files, "--blank-pattern", "RT_blank*", "--rt-ref", "auto")
    table = cli(*args, "--log", str(log)).stdout
    assert got["csv"] == table == (work / "run_drift" / "qt.csv").read_text(encoding="utf-8")
    assert got["log"] == log.read_text(encoding="utf-8") == (work / "run_drift" / "qt.log.json").read_text(encoding="utf-8")
    # what the table and the log say: a shift on the drifted batch, none where the reference is ambiguous, clipped or a blank
    ev = {f["file"]: f["rt_ref"] for f in json.loads(got["log"])["files"]}
    assert [ev[n]["status"] for n in drift] == ["ok", "ok", "ambiguous", "edge", "blank"], ev
    assert abs(ev["RT_a.mzML"]["shift_min"]) < 0.03 and abs(ev["RT_b.mzML"]["shift_min"] - 0.3) < 0.03, ev
    assert [ev[n]["shift_min"] for n in drift[2:]] == [0, 0, 0] and ev["RT_rival.mzML"]["peaks_in_search"] == 2, ev
    assert ev["RT_rival.mzML"]["runner_up"]["height_ratio"] > 0.5 and ev["RT_a.mzML"]["runner_up"] is None, ev
    assert sorted(got["warnings"]) == ["warn.quant.targets.rtRefAmbiguous", "warn.quant.targets.rtRefEdge"], got["warnings"]
    # --json: the site has no json export (its page reads the table and the log), so the CLI answer is compared member by member
    js = json.loads(cli(*args, "--json").stdout)
    assert js["log"] == json.loads(got["log"])
    # the one hint of this data (the clipped file leaves its reference outside the window) is in the CLI answer only: the site has no hints
    assert [h["key"] for h in js["hints"]] == ["note.quant.targets.offsetAuto"], js.get("hints")
    shifts = [r["rt_shift_min"] for r in js["rows"]]
    assert shifts == [float(l.split(",")[-1]) for l in got["csv"].splitlines()[1:]], shifts
