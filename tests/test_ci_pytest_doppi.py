"""tools/ci_pytest_doppi.py: the 3.11 job leaves out a test file only when rust.yml (rust-wasm) runs it on the same diff, so no test is lost."""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import ci_pytest_doppi as ci  # noqa: E402

F = ci.filtro_rust()


def test_a_rust_change_skips_the_files_rust_yml_runs():
    assert ci.da_saltare(["crates/mzlab-core/src/lib.rs", "docs/x.md"], F) == ci.DOPPI
    assert ci.da_saltare(["Cargo.lock"], F) == ci.DOPPI


def test_no_rust_change_keeps_every_test():
    assert ci.da_saltare(["mzlab/web/shell.js", "docs/x.md", "tools/verifica.py"], F) == []
    assert ci.da_saltare([], F) == []


def test_every_skipped_file_exists_and_is_run_by_rust_yml():
    rust = (ROOT / ".github/workflows/rust.yml").read_text(encoding="utf-8")
    for f in ci.DOPPI:
        assert (ROOT / f).is_file(), f
        assert re.search(r"pytest[^\n]*" + re.escape(f), rust), f"rust.yml does not run {f}"


def test_the_files_that_trigger_rust_yml_also_trigger_the_skip():
    assert ci.rust_gira(["tests/test_feat_wasm.py"], F) and ci.rust_gira([".github/workflows/rust.yml"], F)
