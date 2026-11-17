"""tools/ci_prebuilt.py and the prebuilt path of tests/cargo_util.py: the key of a build, the install of a prebuilt file, and that every build the tests ask for is one the `prebuilt` jobs make."""
import ast
import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tests"))
import cargo_util as cu  # noqa: E402
import ci_prebuilt as cp  # noqa: E402

W = "wasm32-unknown-unknown"


def test_canon_ignores_flag_order_and_noise():
    a = cp.canon(["build", "--release", "--locked", "-q", "--target", W, "-p", "mzlab-wasm"])
    assert a == cp.canon(["build", "-p", "mzlab-wasm", "--target", W]) == [f"{W}|mzlab-wasm|||"]
    assert cp.canon(["build", "-p", "mzlab-wasm", "-p", "mzlab-rotte-wasm", "--target", W]) == [f"{W}|mzlab-wasm|||", f"{W}|mzlab-rotte-wasm|||"]
    assert cp.canon(["run", "-p", "mzlab-cli", "--", "--target", "x"]) == ["host|mzlab-cli|||"]
    assert cp.canon(["build", "-p", "a", "--no-default-features", "--features", "y,x"]) == ["host|a|nd|x,y|"]


@pytest.mark.parametrize("args", [["test", "-p", "mzlab-cli"], ["build", "-p", "mzlab-cli", "--bin", "x"], ["build", "--manifest-path", "x"], []])
def test_canon_refuses_what_it_does_not_know(args):
    assert cp.canon(args) == []


def test_outputs():
    assert cp.outputs(f"{W}|mzlab-lettori-wasm|nd|sciex|") == [f"target/{W}/release/mzlab_lettori_wasm.wasm"]
    assert cp.outputs("host|mzlab-cli|||") == ["target/release/mzlab-cli"]
    assert cp.outputs("host|mzlab-core|||mzidx") == ["target/release/examples/mzidx"]


def keys():
    return {k for g in cp.GROUPS.values() for line in g for k in cp.canon(line.split())}


def test_every_group_line_is_a_build():
    assert all(cp.canon(line.split()) for g in cp.GROUPS.values() for line in g)


def test_the_builds_of_the_tests_are_prebuilt():
    """Every cargo_util call of the tests with literal arguments names a build listed in GROUPS (a new one must be added there, or it runs cargo in every shard)."""
    known, bad = keys(), []
    for f in sorted((ROOT / "tests").glob("test_*.py")) + [ROOT / "tests" / "conftest.py"]:
        for n in ast.walk(ast.parse(f.read_text(encoding="utf-8"))):
            if not (isinstance(n, ast.Call) and isinstance(n.func, ast.Name)) or not all(isinstance(a, ast.Constant) for a in n.args):
                continue
            a = [x.value for x in n.args]
            if n.func.id in ("build", "installed"):
                ks = cp.canon(["build", *a])
            elif n.func.id in ("example", "example_argv") and len(a) >= 2:
                feat = [x for kw in n.keywords if kw.arg == "features" for x in ("--features", kw.value.value)]
                ks = cp.canon(["build", "-p", a[0], *feat, "--example", a[1]])
            else:
                continue
            bad += [f"{f.name}:{n.lineno} {k}" for k in ks if k not in known]
    assert not bad, bad


@pytest.fixture
def prebuilt(tmp_path, monkeypatch):
    d, root = tmp_path / "pre", tmp_path / "repo"
    key = f"{W}|mzlab-wasm|||"
    (d / cp.stored(key)).parent.mkdir(parents=True)
    (d / cp.stored(key)).write_bytes(b"wasm")
    (d / "manifest-wasm-a.json").write_text(json.dumps({key: cp.stored(key)}))
    root.mkdir()
    monkeypatch.setattr(cu, "ROOT", root)
    monkeypatch.setattr(cu, "_manifest", None)
    monkeypatch.setattr(cu, "_native", set())
    monkeypatch.setenv("MZLAB_PREBUILT", str(d))
    monkeypatch.setenv("MZLAB_PREBUILT_MISSES", str(tmp_path / "misses.txt"))
    return root


def test_a_listed_build_is_installed_and_an_unlisted_one_is_recorded(prebuilt, tmp_path):
    assert cu.installed("--target", W, "-p", "mzlab-wasm")
    assert (prebuilt / f"target/{W}/release/mzlab_wasm.wasm").read_bytes() == b"wasm"
    assert not cu.installed("-p", "mzlab-cli")
    assert (tmp_path / "misses.txt").read_text().split() == ["host|mzlab-cli|||"]


def test_without_the_variable_nothing_is_installed(prebuilt, monkeypatch):
    monkeypatch.delenv("MZLAB_PREBUILT")
    assert not cu.installed("--target", W, "-p", "mzlab-wasm")
    assert not (prebuilt / "target").exists()


def test_misses_prints_the_most_frequent_first(tmp_path, capsys):
    f = tmp_path / "m.txt"
    f.write_text("a\nb\nb\n")
    cp.misses(f)
    assert capsys.readouterr().out.splitlines() == ["   2  b", "   1  a", "2 builds not prebuilt"]


def test_no_test_module_redefines_what_it_imports_from_cargo_util():
    """`def build(...)` next to `from cargo_util import build` silently replaces the helper (the call then fails at run time, in a fixture)."""
    bad = []
    for f in sorted((ROOT / "tests").glob("test_*.py")):
        tree = ast.parse(f.read_text(encoding="utf-8"))
        imported = {a.asname or a.name for n in tree.body if isinstance(n, ast.ImportFrom) and n.module == "cargo_util" for a in n.names}
        bad += [f"{f.name}:{n.lineno} {n.name}" for n in tree.body if isinstance(n, ast.FunctionDef) and n.name in imported]
    assert not bad, bad


def test_every_cargo_util_helper_a_test_calls_is_imported_or_defined():
    """A call to build/cli/example/... in a module that neither imports nor defines it fails only when its fixture runs (and only where cargo is installed)."""
    helpers, bad = {"build", "cli", "example", "installed", "cli_argv", "example_argv", "build_example"}, []
    for f in sorted((ROOT / "tests").glob("test_*.py")) + [ROOT / "tests" / "conftest.py"]:
        tree = ast.parse(f.read_text(encoding="utf-8"))
        known = set()
        for n in ast.walk(tree):
            if isinstance(n, ast.ImportFrom):
                known |= {a.asname or a.name for a in n.names}
            elif isinstance(n, (ast.FunctionDef, ast.ClassDef)):
                known.add(n.name)
            elif isinstance(n, ast.Name) and isinstance(n.ctx, ast.Store):
                known.add(n.id)
        used = {n.func.id for n in ast.walk(tree) if isinstance(n, ast.Call) and isinstance(n.func, ast.Name) and n.func.id in helpers}
        bad += [f"{f.name}: {h}" for h in sorted(used - known)]
    assert not bad, bad


def test_prebuilt_restores_the_cache_the_shards_save():
    """The path list is part of the cache version: a `prebuilt` restore with other paths than the shard's save never hits (every group rebuilt from zero)."""
    lines = (ROOT / ".github/workflows/test.yml").read_text(encoding="utf-8").splitlines()

    def blocks(start: str, end: str) -> list[list[str]]:
        a, b = lines.index(f"  {start}:"), lines.index(f"  {end}:")
        out = []
        for k in range(a, b):
            if lines[k].strip() == "path: |":
                ind = len(lines[k]) - len(lines[k].lstrip())
                body = []
                for ln in lines[k + 1:b]:
                    if not ln.strip() or len(ln) - len(ln.lstrip()) <= ind:
                        break
                    body.append(ln.strip())
                if "target" in body:
                    out.append(sorted(body))
        return out
    pre, shard = blocks("prebuilt", "pytest-shard"), blocks("pytest-shard", "pytest")
    assert len(pre) == 1 and len(shard) == 2 and shard[0] == shard[1] == pre[0]
