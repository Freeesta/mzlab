"""Prebuilt cargo outputs for the pytest jobs of test.yml.
Many tests build the same crates (mzlab-cli, the .wasm of the engine and of the libraries, the examples) in every shard. The `prebuilt` jobs build them once with
`python3 tools/ci_prebuilt.py build GROUP OUT`, upload OUT, and each shard downloads it and sets MZLAB_PREBUILT=OUT: tests/cargo_util.py then installs the file of a
build instead of running cargo (a build that is not listed here, or no variable, runs cargo exactly as before). `canon` turns the arguments of a cargo build
into one key per package, so the tests and this list agree on what a build is. `misses` prints the builds a shard ran itself."""
import hashlib
import json
import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WASM = "wasm32-unknown-unknown"
DROP = {"-q", "--quiet", "--locked", "--release"}
BASE = "-p mzlab-lettori-wasm --no-default-features --features"

# GROUP -> the builds, as the tests pass them to cargo (flags in DROP are ignored); three groups run in parallel jobs
GROUPS = {
    "native": [
        "build -p mzlab-cli",
        "build -p mzlab-core --example mzidx",
        "build -p mzlab-feat-wasm --example native",
        "build -p mzlab-feat-table-wasm --example table",
        "build -p mzlab-feat-wasm --example estimate_detect",
        "build -p mzlab-feat-wasm --example read_estimate",
        "build -p mzlab-lettori --features tutte --example synthetic",
        "build -p mzlab-lib --example fdr_parity",
        "build -p mzlab-lib --example validate",
    ],
    "wasm-a": [f"build --target {WASM} -p {p}" for p in (
        "mzlab-wasm", "mzlab-rotte-wasm", "mzlab-feat-wasm", "mzlab-feat-table-wasm", "mzlab-feat-grafo-wasm", "mzlab-feat-import-wasm", "mzlab-lib-wasm", "mzlab-llm-wasm",
        "mzlab-mol-wasm", "mzlab-msngrafo-wasm", "mzlab-msnregole-wasm", "mzlab-normative-wasm", "mzlab-ticfondo-wasm", "mzlab-flowcal-wasm")] + [
        f"build --target {WASM} -p mzlab-feat-wasm --no-default-features --features finder",
        f"build --target {WASM} -p mzlab-feat-wasm --features untargeted",
        f"build --target {WASM} -p mzlab-attributio-wasm --features rev",
        f"build --target {WASM} -p mzlab-attributio-wasm --features all",
    ],
    "wasm-b": [f"build --target {WASM} {BASE} {f}" for f in ("aperti", "agilent", "waters", "shimadzu", "sciex", "thermo", "bruker", "andi", "valida")],
}


def canon(args: list[str]) -> list[str]:
    """One key per package of a `cargo build|run` command line (`--` and what follows are the program's own arguments), or [] for any other command."""
    if not args or args[0] not in ("build", "run"):
        return []
    args = args[1:args.index("--")] if "--" in args else args[1:]
    pk, target, feat, example, nd = [], "", "", "", False
    i = 0
    while i < len(args):
        a = args[i]
        if a in DROP:
            pass
        elif a in ("-p", "--package", "--target", "--features", "--example") and i + 1 < len(args):
            i += 1
            if a in ("-p", "--package"):
                pk.append(args[i])
            elif a == "--target":
                target = args[i]
            elif a == "--features":
                feat = ",".join(sorted(args[i].split(",")))
            else:
                example = args[i]
        elif a == "--no-default-features":
            nd = True
        else:
            return []   # anything else (--bin, --test, --manifest-path, ...): not a build this module knows
        i += 1
    return [f"{target or 'host'}|{p}|{'nd' if nd else ''}|{feat}|{example}" for p in pk]


def outputs(key: str) -> list[str]:
    """Where cargo leaves the file of a key, relative to the repository."""
    target, pkg, _, _, example = key.split("|")
    if target == WASM:
        return [f"target/{WASM}/release/{pkg.replace('-', '_')}.wasm"]
    return [f"target/release/examples/{example}" if example else f"target/release/{pkg}"]


def directory() -> Path | None:
    d = os.environ.get("MZLAB_PREBUILT")
    return Path(d) if d else None


def manifest(d: Path) -> dict[str, str]:
    """key -> path of the stored file, from every manifest-*.json of the directory (one per group)."""
    out: dict[str, str] = {}
    for f in sorted(d.glob("manifest-*.json")):
        out.update(json.loads(f.read_text(encoding="utf-8")))
    return out


def stored(key: str) -> str:
    return f"files/{hashlib.sha1(key.encode()).hexdigest()[:12]}/{Path(outputs(key)[0]).name}"


def build(group: str, out: Path) -> int:
    """Runs every build of the group (cargo is the shim of tools/ci_cargo when it is first in PATH) and stores its file under OUT; non-zero if any build fails."""
    mapa, rc = {}, 0
    for line in GROUPS[group]:
        args = shlex.split(line)
        r = subprocess.run(["cargo", args[0], "--release", "--locked", "-q", *args[1:]], cwd=ROOT)
        if r.returncode:
            print(f"prebuilt: FAILED {line}", file=sys.stderr)
            rc = 1
            continue
        for key in canon(args):
            src = ROOT / outputs(key)[0]
            dst = out / stored(key)
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)
            mapa[key] = stored(key)
            print(f"prebuilt: {key} -> {dst.stat().st_size} B")
    (out / f"manifest-{group}.json").write_text(json.dumps(mapa, indent=1, sort_keys=True), encoding="utf-8")
    return rc


def misses(path: Path) -> int:
    """Prints the keys a shard built itself (tests/cargo_util.py appends them to MZLAB_PREBUILT_MISSES), most frequent first."""
    lines = path.read_text(encoding="utf-8").split() if path.exists() else []
    for k in sorted(set(lines), key=lambda k: -lines.count(k)):
        print(f"{lines.count(k):4d}  {k}")
    print(f"{len(set(lines))} builds not prebuilt")
    return 0


if __name__ == "__main__":
    cmd = sys.argv[1:2]
    if cmd == ["build"] and len(sys.argv) == 4 and sys.argv[2] in GROUPS:
        sys.exit(build(sys.argv[2], Path(sys.argv[3])))
    if cmd == ["misses"] and len(sys.argv) == 3:
        sys.exit(misses(Path(sys.argv[2])))
    sys.exit("usage: ci_prebuilt.py build {%s} OUT | misses FILE" % "|".join(GROUPS))
