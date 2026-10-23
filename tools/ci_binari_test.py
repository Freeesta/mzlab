#!/usr/bin/env python3
"""Runs the test binaries of `cargo test` in parallel (used by the Rust test jobs of .github/workflows/rust.yml).

Usage: tools/ci_binari_test.py [-j N] [--] <arguments of cargo test, e.g. --locked --release -p mzlab-core>
It builds once (`cargo test --no-run`), then runs every unit/integration test binary as cargo would
(working directory = package root, CARGO_MANIFEST_DIR set), N at a time, and finally the doctests with
`cargo test --doc`. Each binary has its own log group; the total of passed/failed/ignored tests is printed
at the end and the exit status is non-zero if any binary (or the doctests) failed.
"""
import json
import os
import re
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

RESULT = re.compile(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored")
# cargo skips the doctests when a target is selected, so do we
SELECTORS = ("--test", "--lib", "--bin", "--bins", "--tests", "--example", "--examples", "--bench", "--benches")


def parse(argv):
    jobs = os.cpu_count() or 4
    rest = list(argv)
    if rest[:1] == ["-j"]:
        jobs, rest = int(rest[1]), rest[2:]
    if rest[:1] == ["--"]:
        rest = rest[1:]
    return jobs, rest


def build(cargo_args):
    """Compiles every test target and returns [(label, executable, package root)] in a stable order."""
    cmd = ["cargo", "test", *cargo_args, "--no-run", "--message-format=json-render-diagnostics"]
    proc = subprocess.run(cmd, stdout=subprocess.PIPE, text=True)
    if proc.returncode != 0:
        sys.exit(proc.returncode)
    found = {}
    for line in proc.stdout.splitlines():
        try:
            msg = json.loads(line)
        except ValueError:
            continue
        if msg.get("reason") != "compiler-artifact" or not msg.get("profile", {}).get("test") or not msg.get("executable"):
            continue
        target = msg["target"]
        root = os.path.dirname(msg["manifest_path"])
        pkg = msg["package_id"].split("#")[-1].split("@")[0] if "#" in msg["package_id"] else os.path.basename(root)
        found[msg["executable"]] = (f"{os.path.basename(root)}::{target['name']} [{'/'.join(target['kind'])}]", msg["executable"], root, pkg)
    # the biggest binaries first: they are the ones likely to run longest, so the tail is short
    return sorted(found.values(), key=lambda t: (-os.path.getsize(t[1]), t[0]))


def run_binary(item):
    label, exe, root, pkg = item
    env = dict(os.environ, CARGO_MANIFEST_DIR=root, CARGO_PKG_NAME=pkg)
    t0 = time.time()
    proc = subprocess.run([exe], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    return label, proc.returncode, proc.stdout, time.time() - t0


def totals(out):
    passed = failed = ignored = 0
    for m in RESULT.finditer(out):
        passed += int(m.group(1))
        failed += int(m.group(2))
        ignored += int(m.group(3))
    return passed, failed, ignored


def main():
    jobs, cargo_args = parse(sys.argv[1:])
    items = build(cargo_args)
    print(f"{len(items)} test binaries, {jobs} at a time", flush=True)
    t0 = time.time()
    passed = failed = ignored = 0
    bad = []
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        for label, code, out, secs in pool.map(run_binary, items):
            p, f, i = totals(out)
            passed, failed, ignored = passed + p, failed + f, ignored + i
            ok = code == 0
            if ok:
                print(f"::group::{label}: {p} passed, {i} ignored, {secs:.1f} s")
                print(out, end="")
                print("::endgroup::", flush=True)
            else:
                bad.append(label)
                print(f"::error::test binary {label} failed (exit {code})")
                print(out, end="", flush=True)
    print(f"binaries: {passed} passed, {failed} failed, {ignored} ignored, {time.time() - t0:.1f} s", flush=True)
    if not any(a in SELECTORS for a in cargo_args):
        doc = subprocess.run(["cargo", "test", *cargo_args, "--doc"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
        p, f, i = totals(doc.stdout)
        print(f"::group::doctests: {p} passed, {i} ignored")
        print(doc.stdout, end="")
        print("::endgroup::", flush=True)
        passed, failed, ignored = passed + p, failed + f, ignored + i
        if doc.returncode != 0:
            bad.append("doctests")
    print(f"TOTAL: {passed} passed, {failed} failed, {ignored} ignored")
    if bad:
        print("::error::failed: " + ", ".join(bad))
        sys.exit(1)


if __name__ == "__main__":
    main()
