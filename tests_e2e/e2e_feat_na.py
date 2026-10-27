"""Cells not measured (`CELL_NA`) in the `.wasm` and the native engine: a series cut in RT (`tools/feat_ritagliata.py`, two files kept only between 3 and 10 min) gives
cells with state 4 in the table, and `e2e_feat_run.py` (the same series, the `rt_range` recipe too) finds every file and the table byte for byte the native ones.
Needs cargo, the wasm32 target, wasm-bindgen and numpy."""
import os, shutil, struct, json, subprocess, sys, tempfile
from collections import Counter
from pathlib import Path
from lib import ROOT

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("  ", ("wasm", "SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm")); sys.exit(0)
CELL_NA = 4
tmp = Path(tempfile.mkdtemp(prefix="feat_na_"))
try:
    data = tmp / "data"; nat = tmp / "nat"; nat.mkdir()
    subprocess.run([sys.executable, str(ROOT / "tools/feat_ritagliata.py"), "--out", str(data)], check=True, cwd=ROOT)
    idx = sorted(str(p) for p in data.glob("*.mzidx"))
    recipes = {"cut": '{"polarity":1,"mode":"ion"}', "rt_range": '{"polarity":1,"mode":"ion","rt_range":[3,10]}'}
    def na_cells(table):
        b = table.read_bytes(); n = struct.unpack("<Q", b[8:16])[0]; m = json.loads(b[16:16 + n]); base = (16 + n + 63) // 64 * 64
        a = next(a for a in m["arrays"] if a["name"] == "state")
        return Counter(b[base + a["offset"]: base + a["offset"] + a["count"]])
    counts = {}
    for name, recipe in recipes.items():
        out = nat / name; out.mkdir()
        subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-feat-wasm", "--example", "nativo", "--", str(out), *idx, "--recipe", recipe],
                       check=True, cwd=ROOT, stdout=subprocess.DEVNULL)
        counts[name] = na_cells(out / "series.mzfeat")
    print("  ", ("native: cells not measured in the cut series", "ok" if counts["cut"][CELL_NA] > 0 else "FAIL none, states " + str(dict(counts["cut"]))))
    print("  ", ("native: rt_range drops rows", "ok" if sum(counts["rt_range"].values()) < sum(counts["cut"].values()) else "FAIL same number of cells"))
    for name, recipe in recipes.items():
        r = subprocess.run([sys.executable, str(ROOT / "tests_e2e/e2e_feat_run.py")], cwd=ROOT / "tests_e2e", capture_output=True, text=True,
                           env={**os.environ, "MZLAB_FEAT_MZIDX": str(data), "MZLAB_FEAT_RECIPE": recipe})
        lines = [l for l in r.stdout.splitlines() if l.strip().startswith("(") or "FAIL" in l]
        bad = [l for l in lines if "FAIL" in l] or ([] if r.returncode == 0 else ["exit code " + str(r.returncode) + r.stderr[-300:]])
        print("  ", (f"wasm = native, recipe {name}", "ok" if not bad else "FAIL " + " | ".join(bad)[:600]))
finally:
    shutil.rmtree(tmp, ignore_errors=True)
