"""Which pytest files a pull request needs: the tests that call cargo (the slow ones: they build or install Rust and .wasm) are left out when the diff cannot reach them.
`python3 tools/ci_pytest_aree.py <base sha>` writes `ignore=--ignore=tests/a.py ...` (what to leave out) and `cargo=true|false` (is any cargo test left) to $GITHUB_OUTPUT; `--build <base sha>` writes `solo=true` when no test that calls
cargo is selected, so the `prebuilt` jobs have nothing to build. Only a `pull_request` is reduced: push, merge_group, no base or a git failure keep every test.
The table is derived from the sources, so it cannot go stale: a test calls cargo when it (or a tests/ or tools/ module it imports) names cargo_util, build_example, the prebuilt helpers or
"cargo" as a string (a command); it depends on RUST_INPUTS, on the files the crates embed (include_str!/include_bytes!) and on the area of every path-like word its source mentions (AREAS). A new test therefore
selects itself by default. Any changed file outside KNOWN and RUST_INPUTS (workflows, the Python package, tools/ ...) or any non-test file under tests/ selects everything; a changed test selects itself."""
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
# what a Rust build reads besides the files the crates embed: the crates, lockfile, toolchain, vendored sources and cargo config
RUST_INPUTS = [re.compile(p) for p in (r"crates/", r"Cargo\.(toml|lock)$", r"rust-toolchain\.toml$", r"vendor/", r"\.cargo/")]
# path-like word in a test source -> the changed files that can alter that test (a test that never names an area is independent of it)
AREAS = {
    r"mzlab/web|[\"']web[\"']|\bWEB\b|build_site|prepara_sito|\bsite/|esempi": re.compile(r"mzlab/web/"),
    r"TP_Mine": re.compile(r"TP_Mine/"),
    r"\bdocs/|[\"']docs[\"']": re.compile(r"docs/"),
    r"tests_e2e": re.compile(r"tests_e2e/"),
    r"\.claude": re.compile(r"\.claude/"),
    r"\.md\b|CLAUDE|CHANGELOG": re.compile(r".*\.md$"),
}
# areas that, changed alone, reduce the selection (everything else selects all)
KNOWN = [re.compile(p) for p in (r"mzlab/web/", r"TP_Mine/", r"docs/", r"tests_e2e/", r"\.claude/", r"[^/]+\.md$")]
CARGO = re.compile(r"cargo_util|build_example|ci_prebuilt|MZLAB_PREBUILT|[\"']cargo[\"']|\bcu\.(build|cli|example|installed)\b")
EMBED = re.compile(r"include(?:_str|_bytes)?!\(\s*\"([^\"]+)\"")


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace") if path.is_file() else ""


def calls_cargo(path: Path, seen: frozenset = frozenset()) -> bool:
    """True when the file names cargo, or imports a local module (tests/ or tools/) that does."""
    text = read(path)
    if CARGO.search(text):
        return True
    for mod in re.findall(r"^\s*(?:from|import)\s+(\w+)", text, re.M):
        for d in ("tests", "tools"):
            dep = ROOT / d / f"{mod}.py"
            if dep.is_file() and dep != path and dep not in seen and calls_cargo(dep, seen | {path}):
                return True
    return False


def embedded() -> set[str]:
    """Files outside crates/ that a crate embeds at compile time (the repository-relative paths)."""
    out = set()
    for rs in (ROOT / "crates").rglob("*.rs"):
        for rel in EMBED.findall(read(rs)):
            p = (rs.parent / rel).resolve()
            try:
                r = p.relative_to(ROOT).as_posix()
            except ValueError:
                continue
            if not r.startswith("crates/"):
                out.add(r)
    return out


def selected(test: Path, files: list[str], embed: set[str]) -> bool:
    """Does this diff need `test`? Only the tests that call cargo are ever left out."""
    if not calls_cargo(test):
        return True
    rel = test.relative_to(ROOT).as_posix()
    text = read(test)
    prefixes = [rx for words, rx in AREAS.items() if re.search(words, text)]
    return any(f == rel or f in embed or any(rx.match(f) for rx in RUST_INPUTS) or any(rx.match(f) for rx in prefixes) for f in files)


def tests() -> list[Path]:
    return sorted((ROOT / "tests").glob("test_*.py"))


def reduces(files: list[str]) -> bool:
    """False (select everything) for no file, a file outside the known areas and Rust inputs, or a non-test file of tests/ (conftest, helpers, golden data)."""
    for f in files:
        if f.startswith("tests/") and not re.fullmatch(r"tests/test_[^/]+\.py", f):
            return False
        if not (any(rx.match(f) for rx in KNOWN) or any(rx.match(f) for rx in RUST_INPUTS) or re.fullmatch(r"tests/test_[^/]+\.py", f)):
            return False
    return bool(files)


def left_out(files: list[str]) -> list[str]:
    if not reduces(files):
        return []
    embed = embedded()
    return [t.relative_to(ROOT).as_posix() for t in tests() if not selected(t, files, embed)]


def changed(base: str) -> list[str]:
    out = subprocess.run(["git", "diff", "--name-only", "--no-renames", "-z", base, "HEAD"], check=True, capture_output=True, encoding="utf-8").stdout
    return [f for f in out.split("\0") if f]


def emit(**kv: str) -> None:
    for k, v in kv.items():
        print(f"{k}={v}")
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
            f.writelines(f"{k}={v}\n" for k, v in kv.items())


def main(argv: list[str]) -> int:
    build = argv[:1] == ["--build"]
    base = (argv[1:] if build else argv)[:1]
    out: list[str] = []
    every = True  # no cargo test is selected -> prebuilt has nothing to do
    try:
        if os.environ.get("GITHUB_EVENT_NAME") == "pull_request" and base and base[0] and not re.fullmatch(r"0+", base[0]):
            files = changed(base[0])
            out = left_out(files)
            every = not reduces(files) or any(calls_cargo(t) and t.relative_to(ROOT).as_posix() not in out for t in tests())
    except (subprocess.CalledProcessError, OSError):
        out, every = [], True
    if build:
        if not every:
            emit(solo="true")
        return 0
    emit(ignore=" ".join(f"--ignore={f}" for f in out), cargo="true" if every else "false")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
