"""The noise with a floor of one ion (`mzlab_core::ionstat`, rule R1 of docs/studies/TP_SCREENING_RULES.md) told alike on the site (mzlab-wasm `noise_floor`
through Node, the function of `peaks-worker.js`) and in the app/CLI (`mzlab-cli peak noise`): the same `noise-floor/1` bytes on synthetic ion-count traces
(a quantum of 2222 cps, mostly zero so the MAD is 0; a counting background; an all-zero trace with one_ion set). Skipped without cargo, wasm-bindgen or node."""
import shutil
import subprocess

import pytest
from cargo_util import ROOT, build, run

pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
NODE = """const w = require(process.argv[2]), fs = require("fs");
const rows = fs.readFileSync(process.argv[3], "utf8").trim().split("\\n").slice(1).map(l => l.split(",").map(Number));
const n = v => (v === "" ? NaN : Number(v)), a = process.argv;
process.stdout.write(w.noise_floor(Float64Array.from(rows.map(r => r[0])), Float64Array.from(rows.map(r => r[1])),
  Float64Array.from([n(a[4]), n(a[5]), n(a[6]), n(a[7])]), Float64Array.from([n(a[8])])) + "\\n");"""


def lcg(seed):
    while True:
        seed = (seed * 6364136223846793005 + 1442695040888963407) % 2 ** 64
        yield (seed >> 11) / 2 ** 53


def poisson(n, lam, seed):
    import math
    u, out = lcg(seed), []
    for _ in range(n):
        k, p, lim = 0, 1.0, math.exp(-lam)
        while True:
            p *= next(u)
            if p <= lim:
                break
            k += 1
        out.append(k)
    return out


def traces():
    rt = [round(i * 0.02, 4) for i in range(300)]
    sparse = [2222 * c for c in poisson(300, 0.03, 11)]
    sparse[150:153] = [4444, 6666, 4444]
    busy = [1000 * c for c in poisson(300, 9.0, 5)]
    busy[149:152] = [x + d for x, d in zip(busy[149:152], (20000, 40000, 20000))]
    zero = [0] * 300
    zero[150] = 3000
    return {"sparse": (rt, sparse), "busy": (rt, busy), "zero": (rt, zero)}


CASES = [  # name, from, to, peak a, peak b, one_ion ("" = auto)
    ("sparse", "", "", "2.98", "3.06", ""),
    ("sparse", "0.5", "5.5", "2.98", "3.06", "0"),
    ("busy", "", "", "2.96", "3.04", ""),
    ("zero", "", "", "2.99", "3.01", ""),
    ("zero", "", "", "2.99", "3.01", "1000"),
    ("sparse", "", "", "", "", ""),
]


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("rumore")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    build("-p", "mzlab-cli")
    for name, (rt, y) in traces().items():
        (d / f"{name}.csv").write_text("rt,y\n" + "\n".join(f"{a},{b}" for a, b in zip(rt, y)) + "\n", encoding="utf-8")
    (d / "w.js").write_text(NODE, encoding="utf-8")
    return d


@pytest.mark.parametrize("case", CASES, ids=lambda c: "-".join(x or "_" for x in c))
def test_site_and_cli_give_the_same_noise_floor(work, case):
    name, lo, hi, a, b, ion = case
    csv = str(work / f"{name}.csv")
    site = run("node", str(work / "w.js"), str(work / "pkg" / "mzlab_wasm.js"), csv, lo, hi, a, b, ion).stdout
    args = [str(ROOT / "target/release/mzlab-cli"), "peak", "noise", csv]
    args += ["--set", f"from={lo}", "--set", f"to={hi}"] if lo else []
    args += ["--rt", f"{a},{b}"] if a else []
    args += ["--set", f"one_ion={ion}"] if ion else []
    p = subprocess.run(args, cwd=ROOT, capture_output=True, encoding="utf-8")
    assert p.returncode == 0, p.stderr
    assert p.stdout == site, (p.stdout, site)
    assert p.stderr.startswith("note: noise floor: ")
    if case == CASES[0]:
        assert '"estimator":"one_ion"' in site and '"sn_mad":null' in site, site
