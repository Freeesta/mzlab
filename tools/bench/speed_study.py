#!/usr/bin/env python3
"""Speed study harness (docs/studies/SPEED_STUDY.md): times a targeted extraction of N (m/z, RT window) targets x M files the way an agent runs it
today (one `trace xic | peak auto-lr` pipe per target and file) against one in-process pass, plus file open/index, conversion and Dig/align.
Synthetic data only (tools/speed_synthetic.py, tools/synthetic_data_large.py). Not run in CI.

    python3 tools/bench/speed_study.py WORK [--files 245] [--targets 275] [--calls 300] [--threads 4] [--hr-files 12] [--no-build] [--keep]

Every row reports wall time, CPU time (user + system of the process and its children), peak RSS, bytes read through syscalls (`rchar`) and passes
over each file (= rchar / bytes of the inputs). The per-call route is sampled (`--calls` pipes spread over files and targets) and extrapolated
linearly to N x M; the in-process modes run the whole N x M and their outputs are compared byte for byte with the per-call bytes on the files
both covered. Results: WORK/speed_study.json and a Markdown table on stdout.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
import speed_synthetic  # noqa: E402

CLI = ROOT / "target" / "cli" / "mzlab-cli"
EX = ROOT / "target" / "cli" / "examples" / "speed_targets"


def rchar() -> int:
    """Bytes read through syscalls by this process and every child it has reaped (Linux adds a reaped child's counters to its parent)."""
    for line in Path("/proc/self/io").read_text().splitlines():
        if line.startswith("rchar:"):
            return int(line.split()[1])
    return 0


def run(cmd, shell=False, capture=False) -> dict:
    """One measured command: wall, CPU (user + sys of it and the children it reaped), its peak RSS (largest process), rchar; with `capture`
    its stdout as `out`. The output goes to temporary files so `wait4` reaps the child and returns its own rusage."""
    with tempfile.TemporaryFile() as o, tempfile.TemporaryFile() as e:
        r0, t0 = rchar(), time.perf_counter()
        p = subprocess.Popen(cmd, shell=shell, stdout=o if capture else subprocess.DEVNULL, stderr=e)
        _, status, ru = os.wait4(p.pid, 0)
        p.returncode = os.waitstatus_to_exitcode(status)
        wall = time.perf_counter() - t0
        r = {"wall_s": wall, "cpu_s": ru.ru_utime + ru.ru_stime, "peak_rss_mb": ru.ru_maxrss / 1024, "rchar": rchar() - r0}
        if p.returncode != 0:
            e.seek(0)
            raise SystemExit(f"failed ({p.returncode}): {cmd}\n{e.read().decode(errors='replace')[-2000:]}")
        if capture:
            o.seek(0)
            r["out"] = o.read().decode()
    return r


def repeat(cmd, n, **kw) -> dict:
    """The median of `n` runs of `cmd` (per key)."""
    rs = [run(cmd, **kw) for _ in range(n)]
    return {k: statistics.median(r[k] for r in rs) for k in rs[0]}


def targets(path: Path) -> list[tuple[str, str, str]]:
    return [tuple(line.split(",")) for line in path.read_text().splitlines()[1:]]


def pipe(file: Path, t, cache: Path | None) -> str:
    c = f" --cache {cache}" if cache else ""
    return (f"{CLI} trace xic {file} --mz {t[0]} --da 0.5 --rt {float(t[1])},{float(t[2])} --format csv{c} 2>/dev/null"
            f" | {CLI} peak auto-lr - 2>/dev/null || true")


def per_call(files, ts, calls: int, cache: Path | None) -> dict:
    """`calls` pipes spread over files and targets, run one after the other (as the agent did): totals and the per-call mean."""
    picks = [(files[(k * 7919) % len(files)], ts[(k * 104729) % len(ts)]) for k in range(calls)]
    tot = {"wall_s": 0.0, "cpu_s": 0.0, "peak_rss_mb": 0.0, "rchar": 0}
    for f, t in picks:
        r = run(pipe(f, t, cache), shell=True)
        for k in ("wall_s", "cpu_s", "rchar"):
            tot[k] += r[k]
        tot["peak_rss_mb"] = max(tot["peak_rss_mb"], r["peak_rss_mb"])
    size = statistics.mean(f.stat().st_size for f, _ in picks)
    return {"calls": calls, **tot, "per_call_ms": tot["wall_s"] / calls * 1e3, "cpu_per_call_ms": tot["cpu_s"] / calls * 1e3,
            "passes_per_call": tot["rchar"] / calls / size}


def save(w: Path, res: dict) -> None:
    """The results so far (a later step that fails keeps the earlier rows)."""
    (w / "speed_study.json").write_text(json.dumps(res, indent=1))


def sha(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()[:16]


def data(w: Path, a) -> tuple[Path, Path]:
    """The LR series and the HR set under WORK, generated only when missing or of another size."""
    lr, hr = w / "lr", w / "hr"
    if not (lr / "targets.csv").exists() or len(list(lr.glob("*.mzML"))) != a.files:
        shutil.rmtree(lr, ignore_errors=True)
        t0 = time.perf_counter()
        speed_synthetic.make(lr, files=a.files, targets=a.targets)
        print(f"LR data {time.perf_counter() - t0:.1f} s", file=sys.stderr)
    if not hr.exists() or len(list(hr.glob("*.mzML"))) != a.hr_files:
        shutil.rmtree(hr, ignore_errors=True)
        subprocess.run([sys.executable, str(ROOT / "tools" / "synthetic_data_large.py"), str(hr), "--files", str(a.hr_files), "--scans", "2000",
                        "--compounds", "1500", "--noise", "800"], check=True)
    return lr, hr


def inprocess(w: Path, files, tpath: Path, threads: int, res: dict) -> None:
    """In-process rows (speed_targets example) and their byte-for-byte comparison with the per-call bytes."""
    # in-process: the per-call route on a few files (whole target list), then the list and the one pass on every file
    few = files[:4]
    outs = {}
    rows = {}
    for mode, fs, th in [("calls", few, 1), ("list", files, 1), ("list", files, threads), ("onepass", files, 1),
                         ("onepass", files, threads)]:
        out = w / f"out-{mode}-{th}-{len(fs)}.tsv"
        r = run([str(EX), mode, str(tpath), str(out), str(th), *map(str, fs)], capture=True)
        j = json.loads(r["out"].strip().splitlines()[-1])
        j["outside_wall_s"] = r["wall_s"]
        j["outside_cpu_s"] = r["cpu_s"]
        if mode == "calls":
            j["extrapolated_s"] = j["wall_s"] * len(files) / len(fs)
        rows[f"{mode}-{th}"] = j
        outs[f"{mode}-{th}"] = out
    # byte-for-byte: the lines of the few files in every output
    keep = {f.name for f in few}
    def lines(p):
        return [l for l in p.read_text().splitlines() if l.split("\t", 1)[0] in keep]
    ref = lines(outs["calls-1"])
    res["identical_to_calls"] = {k: lines(p) == ref for k, p in outs.items()}
    res["identical_all_files"] = {k: sha(p) for k, p in outs.items() if k != "calls-1"}
    res["inprocess"] = rows


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("work", type=Path)
    ap.add_argument("--files", type=int, default=245)
    ap.add_argument("--targets", type=int, default=275)
    ap.add_argument("--calls", type=int, default=300, help="per-call pipes sampled (extrapolated to N x M)")
    ap.add_argument("--threads", type=int, default=4)
    ap.add_argument("--hr-files", type=int, default=12)
    ap.add_argument("--no-build", action="store_true")
    ap.add_argument("--keep", action="store_true", help="keep WORK/lr and WORK/hr for the next run")
    a = ap.parse_args()
    if not a.no_build:
        subprocess.run(["cargo", "build", "--profile", "cli", "-p", "mzlab-cli", "--bin", "mzlab-cli", "--example", "speed_targets"],
                       cwd=ROOT, check=True)
    w = a.work
    lr, hr = data(w, a)
    files = sorted(lr.glob("QQQ_*.mzML"))
    hrs = sorted(hr.glob("G_*.mzML"))
    tpath = lr / "targets.csv"
    ts = targets(tpath)
    lr_bytes = sum(f.stat().st_size for f in files)
    res = {"machine": {"cpus": os.cpu_count()}, "lr": {"files": len(files), "targets": len(ts), "mb": lr_bytes / 1e6,
                                                         "mb_per_file": lr_bytes / len(files) / 1e6},
           "hr": {"files": len(hrs), "mb": sum(f.stat().st_size for f in hrs) / 1e6}}
    f0, t0 = files[0], ts[0]
    cache = w / "cache"
    shutil.rmtree(cache, ignore_errors=True)
    # warm the page cache: every row reads from memory, as a second run of an agent would
    for f in files + hrs:
        f.read_bytes()
    res["process_start"] = repeat([str(CLI), "--version"], 20)
    res["file_info"] = repeat([str(CLI), "file", "info", str(f0)], 5)
    res["file_index_cache"] = run([str(CLI), "file", "index", "--cache", str(cache), str(f0)])
    res["one_xic"] = repeat([str(CLI), "trace", "xic", str(f0), "--mz", t0[0], "--da", "0.5", "--format", "csv"], 5)
    res["one_pipe"] = repeat(pipe(f0, t0, None), 5, shell=True)
    res["one_xic_list"] = repeat([str(CLI), "trace", "xic", str(f0), "--mz", ",".join(t[0] for t in ts), "--da", "0.5", "--format", "csv"], 3)
    res["phases"] = json.loads(subprocess.run([str(EX), "phases", str(tpath), "-", "1", str(f0)], capture_output=True, text=True,
                                              check=True).stdout)
    nm = len(files) * len(ts)
    pc = per_call(files, ts, a.calls, None)
    pc["extrapolated_s"] = pc["per_call_ms"] * nm / 1e3
    res["per_call"] = pc
    run([str(CLI), "file", "index", "--cache", str(cache), *map(str, files)])
    pcc = per_call(files, ts, a.calls, cache)
    pcc["extrapolated_s"] = pcc["per_call_ms"] * nm / 1e3
    res["per_call_cache"] = pcc
    inprocess(w, files, tpath, a.threads, res)
    save(w, res)
    # conversion (a synthetic Waters .raw folder of 2400 centroid scans: an mzML is not converted) and Dig/align
    import waters_sintetico
    conv = w / "conv"
    shutil.rmtree(conv, ignore_errors=True)
    raw = waters_sintetico.write(w / "vendor", "QQQ_W.raw", 2400)
    res["convert_waters"] = run([str(CLI), "file", "convert", str(raw), "--out", str(conv)])
    res["convert_waters"]["input_mb"] = sum(p.stat().st_size for p in raw.rglob("*") if p.is_file()) / 1e6
    shutil.rmtree(conv, ignore_errors=True)
    shutil.rmtree(w / "vendor", ignore_errors=True)
    save(w, res)
    res["features_hr"] = run([str(CLI), "features", *map(str, hrs), "--out", str(w / "features_hr.csv")])
    save(w, res)
    res["features_lr_all"] = run([str(CLI), "features", *map(str, files), "--out", str(w / "features_lr.csv")])
    shutil.rmtree(cache, ignore_errors=True)
    if not a.keep:
        shutil.rmtree(lr, ignore_errors=True)
        shutil.rmtree(hr, ignore_errors=True)
    save(w, res)
    table(res, lr_bytes)


def table(res: dict, lr_bytes: int) -> None:
    n, m = res["lr"]["files"], res["lr"]["targets"]
    print(f"LR: {n} files x {m} targets ({res['lr']['mb']:.0f} MB, {res['lr']['mb_per_file']:.2f} MB/file); HR {res['hr']['files']} files "
          f"({res['hr']['mb']:.0f} MB); {res['machine']['cpus']} CPU\n")
    print("| row | wall | CPU | peak RSS MB | MB read | passes/file |\n|---|---|---|---|---|---|")
    def row(name, r, wall=None, passes=None):
        wall = r["wall_s"] if wall is None else wall
        mb = r["rchar"] / 1e6
        p = "" if passes is None else f"{passes:.1f}"
        print(f"| {name} | {wall:.3f} s | {r['cpu_s']:.3f} s | {r['peak_rss_mb']:.0f} | {mb:.1f} | {p} |")
    size1 = res["lr"]["mb_per_file"] * 1e6
    row("process start (`--version`)", res["process_start"])
    row("`file info` (open + index, 1 file)", res["file_info"], passes=res["file_info"]["rchar"] / size1)
    row("`file index --cache` (1 file)", res["file_index_cache"], passes=res["file_index_cache"]["rchar"] / size1)
    row("one `trace xic` (CSV)", res["one_xic"], passes=res["one_xic"]["rchar"] / size1)
    row("one pipe `trace xic --rt | peak auto-lr`", res["one_pipe"], passes=res["one_pipe"]["rchar"] / size1)
    row(f"one `trace xic --mz` list of {m}", res["one_xic_list"], passes=res["one_xic_list"]["rchar"] / size1)
    for k in ("per_call", "per_call_cache"):
        r = res[k]
        print(f"| {k}: {r['calls']} pipes sampled | {r['wall_s']:.2f} s ({r['per_call_ms']:.1f} ms/call) | {r['cpu_s']:.2f} s | "
              f"{r['peak_rss_mb']:.0f} | {r['rchar'] / 1e6:.0f} | {r['passes_per_call']:.1f} per call |")
        print(f"| {k}: extrapolated to {n * m} calls | {r['extrapolated_s']:.0f} s | | | | |")
    for k, r in res["inprocess"].items():
        ex = f" (extrapolated to {n} files: {r['extrapolated_s']:.1f} s)" if "extrapolated_s" in r else ""
        print(f"| in-process {k} threads, {r['files']} files{ex} | {r['wall_s']:.3f} s | {r['cpu_s']:.2f} s | {r['peak_rss_mb']:.0f} | "
              f"{r['rchar'] / 1e6:.0f} | {r['passes_per_file']:.2f} |")
    for k in ("convert_waters", "features_hr", "features_lr_all"):
        row(k, res[k])
    print(f"\nphases of one call (ms): {json.dumps(res['phases'])}")
    print(f"identical to the per-call bytes (first 4 files): {res['identical_to_calls']}; sha of whole outputs: {res['identical_all_files']}")


if __name__ == "__main__":
    main()
