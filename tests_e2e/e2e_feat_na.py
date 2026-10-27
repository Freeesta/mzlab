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
    recipes = {"cut": '{"polarity":1,"mode":"ion"}', "rt_range": '{"polarity":1,"mode":"ion","layers":[{"step":"features.detect","params":{"rt_range":[3,10]}}]}'}
    def array(table, name):
        b = table.read_bytes(); n = struct.unpack("<Q", b[8:16])[0]; m = json.loads(b[16:16 + n]); base = (16 + n + 63) // 64 * 64
        a = next(a for a in m["arrays"] if a["name"] == name)
        return b[base + a["offset"]: base + a["offset"] + a["count"] * (1 if a["dtype"] == "u8" else 8)], a["dtype"]
    counts, rt = {}, {}
    for name, recipe in recipes.items():
        out = nat / name; out.mkdir()
        subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-feat-wasm", "--example", "nativo", "--", str(out), *idx, "--recipe", recipe],
                       check=True, cwd=ROOT, stdout=subprocess.DEVNULL)
        counts[name] = Counter(array(out / "series.mzfeat", "state")[0])
        rts = array(out / "series.mzfeat", "rt")
        rt[name] = struct.unpack(f"<{len(rts[0]) // 8}d", rts[0])
    print("  ", ("native: cells not measured in the cut series", "ok" if counts["cut"][CELL_NA] > 0 else "FAIL none, states " + str(dict(counts["cut"]))))
    print("  ", ("native: rt_range keeps only the rows inside", "ok" if rt["rt_range"] and min(rt["rt_range"]) >= 3 and max(rt["rt_range"]) <= 10 and max(rt["cut"]) > 10 else "FAIL " + str((min(rt["rt_range"] or [0]), max(rt["rt_range"] or [0])))))
    for name, recipe in recipes.items():
        r = subprocess.run([sys.executable, str(ROOT / "tests_e2e/e2e_feat_run.py")], cwd=ROOT / "tests_e2e", capture_output=True, text=True,
                           env={**os.environ, "MZLAB_FEAT_MZIDX": str(data), "MZLAB_FEAT_RECIPE": recipe})
        lines = [l for l in r.stdout.splitlines() if l.strip().startswith("(") or "FAIL" in l]
        bad = [l for l in lines if "FAIL" in l] or ([] if r.returncode == 0 else ["exit code " + str(r.returncode) + r.stderr[-300:]])
        print("  ", (f"wasm = native, recipe {name}", "ok" if not bad else "FAIL " + " | ".join(bad)[:600]))
finally:
    shutil.rmtree(tmp, ignore_errors=True)
