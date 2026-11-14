"""tools/ci_pytest_aree.py: which pytest files a pull request leaves out. Fake repository for the rules, the real one for the two cases the CI relies on."""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import ci_pytest_aree as aree  # noqa: E402


@pytest.fixture
def repo(tmp_path, monkeypatch):
    def put(rel: str, text: str = "") -> None:
        (tmp_path / rel).parent.mkdir(parents=True, exist_ok=True)
        (tmp_path / rel).write_text(text, encoding="utf-8")
    put("tests/test_rust.py", "import cargo_util\n")
    put("tests/test_rust_web.py", "import cargo_util\nP = 'mzlab/web/index.html'\n")
    put("tests/test_rust_docs.py", "import cargo_util\nP = 'docs/a.json'\n")
    put("tests/test_plain.py", "import json\n")
    put("tests/test_viahelper.py", "import helper_build\n")
    put("tests/helper_build.py", "import subprocess\nsubprocess.run(['cargo', 'build'])\n")
    put("tests/cargo_util.py", "")
    put("crates/x/src/lib.rs", 'const J: &str = include_str!("../../../mzlab/web/embedded.js");\n')
    monkeypatch.setattr(aree, "ROOT", tmp_path)
    return tmp_path


def names(files: list[str]) -> set[str]:
    return set(aree.left_out(files))


def test_a_web_file_leaves_out_the_cargo_tests_that_never_read_the_web(repo):
    assert names(["mzlab/web/app.css"]) == {"tests/test_rust.py", "tests/test_rust_docs.py", "tests/test_viahelper.py"}


def test_a_file_a_crate_embeds_selects_every_cargo_test(repo):
    assert names(["mzlab/web/embedded.js"]) == set()


def test_docs_leave_out_the_tests_that_do_not_name_docs(repo):
    assert names(["docs/agenti/x.json"]) == {"tests/test_rust.py", "tests/test_rust_web.py", "tests/test_viahelper.py"}


@pytest.mark.parametrize("files", [
    ["crates/x/src/lib.rs"], ["Cargo.lock"], ["rust-toolchain.toml"],  # Rust inputs
    ["mzlab/web/app.css", ".github/workflows/test.yml"], ["mzlab/web/app.css", "mzlab/foo.py"], ["tools/x.py"],  # outside the known areas
    ["tests/conftest.py"], ["tests/golden/a.json"], [],  # tests/ infrastructure, no file
])
def test_everything_else_selects_every_test(repo, files):
    assert names(files) == set()


def test_a_changed_test_selects_itself(repo):
    assert names(["mzlab/web/app.css", "tests/test_rust.py"]) == {"tests/test_rust_docs.py", "tests/test_viahelper.py"}


def test_a_test_that_calls_no_cargo_is_never_left_out(repo):
    assert "tests/test_plain.py" not in names(["mzlab/web/app.css"]) | names(["docs/x.json"])


def test_the_import_of_a_module_that_calls_cargo_counts(repo):
    assert aree.calls_cargo(repo / "tests/test_viahelper.py") and not aree.calls_cargo(repo / "tests/test_plain.py")


def run_main(monkeypatch, capsys, event: str, files: list[str], build: bool = False) -> str:
    monkeypatch.setenv("GITHUB_EVENT_NAME", event)
    monkeypatch.delenv("GITHUB_OUTPUT", raising=False)
    monkeypatch.setattr(aree, "changed", lambda base: files)
    assert aree.main((["--build"] if build else []) + ["abc123"]) == 0
    return capsys.readouterr().out


def test_only_a_pull_request_is_reduced(repo, monkeypatch, capsys):
    for event in ("push", "merge_group", "workflow_dispatch"):
        assert "--ignore" not in run_main(monkeypatch, capsys, event, ["mzlab/web/app.css"])
    out = run_main(monkeypatch, capsys, "pull_request", ["mzlab/web/app.css"])
    assert "--ignore=tests/test_rust.py" in out and "--ignore=tests/test_plain.py" not in out and "cargo=true" in out  # test_rust_web still calls cargo


def test_prebuilt_has_nothing_to_build_only_when_no_cargo_test_is_left(repo, monkeypatch, capsys):
    (repo / "tests/test_rust_web.py").unlink()  # the web reader is the only cargo test a web change keeps
    assert "solo=true" in run_main(monkeypatch, capsys, "pull_request", ["mzlab/web/app.css"], build=True)
    assert run_main(monkeypatch, capsys, "pull_request", ["crates/x/src/lib.rs"], build=True) == ""
    assert run_main(monkeypatch, capsys, "push", ["mzlab/web/app.css"], build=True) == ""


def test_in_the_real_repository_a_stylesheet_skips_the_parity_tests_and_a_core_file_skips_nothing():
    out = set(aree.left_out(["mzlab/web/app.css"]))
    assert "tests/test_lettori_parita.py" in out and "tests/test_proxy.py" not in out  # test_proxy reads mzlab/web
    assert aree.left_out(["crates/mzlab-core/src/lib.rs"]) == [] and aree.left_out([".github/workflows/test.yml"]) == []
    assert aree.embedded() and all((ROOT / f).exists() for f in aree.embedded())
