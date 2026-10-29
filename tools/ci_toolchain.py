#!/usr/bin/env python3
"""Installs the toolchain pinned in rust-toolchain.toml ONCE and checks that it really works (CI e2e jobs, tools/prepara_sito.py).
Several tests that call cargo at the same time make rustup download and install the same toolchain on top of each other («<hash>.partial -> <hash>: No such
file or directory», «component download failed»); `rustup show` exits 0 even when that install is rolled back. Here one process installs it, checks
rustc, cargo, the components and the targets of the file, and on a failure removes the partial install and tries again. Exit 0 = ready."""
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TRIES = 3


def pinned(root: Path = ROOT) -> dict[str, list[str] | str]:
    """channel, components and targets of rust-toolchain.toml (the file is simple: key = "value" or key = ["a", "b"])."""
    text = (root / "rust-toolchain.toml").read_text(encoding="utf-8")
    out: dict[str, list[str] | str] = {}
    for key in ("channel", "components", "targets"):
        m = re.search(rf"^{key}\s*=\s*(.+)$", text, re.M)
        vals = re.findall(r'"([^"]*)"', m.group(1)) if m else []
        out[key] = (vals[0] if vals else "") if key == "channel" else vals
    return out


def run(cmd: list[str], root: Path) -> tuple[int, str]:
    # the installing call must install even when the caller (tools/verifica.py) forbids the tests to
    env = {k: v for k, v in os.environ.items() if k != "RUSTUP_AUTO_INSTALL"}
    r = subprocess.run(cmd, cwd=root, capture_output=True, text=True, env=env)
    return r.returncode, (r.stdout + r.stderr).strip()


def problem(root: Path = ROOT) -> str:
    """Empty when the pinned toolchain works from `root`, else what is wrong (first line)."""
    want = pinned(root)
    rc, out = run(["rustc", "--version"], root)
    if rc or f"rustc {want['channel']}" not in out:
        return f"rustc is not {want['channel']}: {out.splitlines()[-1] if out else 'no output'}"
    rc, out = run(["cargo", "--version"], root)
    if rc:
        return f"cargo does not run: {out.splitlines()[-1] if out else 'no output'}"
    for comp in want["components"]:
        tool = {"clippy": ["cargo", "clippy", "--version"], "rustfmt": ["rustfmt", "--version"]}.get(comp)
        if tool and run(tool, root)[0]:
            return f"component {comp} is missing"
    rc, out = run(["rustup", "target", "list", "--installed"], root)
    for t in want["targets"]:
        if rc or t not in out.split():
            return f"target {t} is missing"
    return ""


def ensure(root: Path = ROOT) -> str:
    """Installs and checks the pinned toolchain; returns "" when ready, else the reason. Without rustup the cargo in PATH is taken as is."""
    if not shutil.which("rustup"):
        return "" if shutil.which("cargo") else "neither rustup nor cargo in PATH"
    why = ""
    for i in range(1, TRIES + 1):
        run(["rustup", "show"], root)             # installs what rust-toolchain.toml asks for
        why = problem(root)
        if not why:
            return ""
        print(f"toolchain try {i}/{TRIES}: {why}", file=sys.stderr)
        if i < TRIES:
            run(["rustup", "toolchain", "uninstall", str(pinned(root)["channel"])], root)
            time.sleep(10)
    return why


def main() -> int:
    why = ensure()
    if why:
        print(f"Rust toolchain of rust-toolchain.toml is not usable: {why}")
        return 1
    print(f"Rust toolchain {pinned()['channel']} ready")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
