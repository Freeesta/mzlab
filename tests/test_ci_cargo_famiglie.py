"""Feature families chosen by the cargo shim of the CI pytest jobs (tools/ci_cargo/cargo) and the hard links back to target/."""
import importlib.machinery
import importlib.util
import os
from pathlib import Path

SHIM = Path(__file__).resolve().parent.parent / "tools" / "ci_cargo" / "cargo"


def carica():
    loader = importlib.machinery.SourceFileLoader("ci_cargo_shim", str(SHIM))
    spec = importlib.util.spec_from_loader("ci_cargo_shim", loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


def test_famiglie():
    f = carica().famiglia
    assert f(["build", "--release", "-p", "mzlab-cli", "--example", "flow"]) == "cli"
    assert f(["run", "--release", "-q", "-p", "mzlab-cli", "--", "msn-graph"]) == "cli"
    assert f(["build", "--release", "-p", "mzlab-feat-wasm", "--example", "nativo"]) == "feat"
    assert f(["build", "--release", "-p", "mzlab-feat-wasm", "--features", "untargeted"]) == "feat-untargeted"
    assert f(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm"]) == "wasm"
    assert f(["build", "--release", "-p", "mzlab-core", "--example", "mzidx"]) is None
    assert f(["metadata"]) is None
    assert f(["build", "--target-dir", "x", "-p", "mzlab-cli"]) is None


def test_rispecchia(tmp_path):
    fam, radice = tmp_path / "fam", tmp_path / "target"
    (fam / "release" / "examples").mkdir(parents=True)
    (fam / "release" / "examples" / "nativo").write_bytes(b"bin")
    (fam / "release" / "examples" / "nativo.d").write_text("dep")
    (fam / "release" / "mzlab-cli").write_bytes(b"cli")
    carica().rispecchia(str(radice), str(fam))
    assert (radice / "release" / "examples" / "nativo").read_bytes() == b"bin"
    assert (radice / "release" / "mzlab-cli").read_bytes() == b"cli"
    assert not (radice / "release" / "examples" / "nativo.d").exists()
    assert os.path.samefile(fam / "release" / "mzlab-cli", radice / "release" / "mzlab-cli")
