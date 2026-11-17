"""The `# covers:` header lines of tests_e2e/e2e*.py (read by tools/verifica.py for --cambiati): the reader finds them, every prefix matches a tracked file."""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import verifica  # noqa: E402
import verifica_aree  # noqa: E402

TRACKED = [f for f in subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True).stdout.split("\n") if f]


def test_the_reader_finds_the_declared_lines(tmp_path, monkeypatch):
    (tmp_path / "e2e_a.py").write_text('"""doc"""\n# covers: mzlab/web/a.js, tools/b\n# covers: x/\nimport os\n', encoding="utf-8")
    (tmp_path / "e2e_b.py").write_text("# nothing here\n", encoding="utf-8")
    monkeypatch.setattr(verifica_aree, "E2E", tmp_path)
    assert verifica.covers() == [("mzlab/web/a.js", "e2e_a"), ("tools/b", "e2e_a"), ("x/", "e2e_a")]


def test_a_changed_file_selects_the_tests_that_declare_it(monkeypatch):
    monkeypatch.setattr(verifica_aree, "covers", lambda: [("mzlab/web/a", "e2e_a"), ("mzlab/web/a", "e2e_b")])
    assert verifica.changed_tests({"mzlab/web/a.js"})[0] == {"e2e_a", "e2e_b", "e2e3"}
    assert verifica.changed_tests({"mzlab/web/other.js"})[0] is None            # core: all of them
    assert verifica.changed_tests({"tools/build_site.py"})[0] == {"-"}           # NO_E2E: none


def test_every_declared_prefix_matches_a_tracked_file():
    table = verifica.covers()
    assert len(table) > 100
    dead = sorted({f"{n}: {p}" for p, n in table if not any(f.startswith(p) for f in TRACKED)})
    assert not dead, "prefixes in `# covers:` that match no tracked file: " + "; ".join(dead)
    dead_no = [p for p in verifica.NO_E2E if not any(f.startswith(p) for f in TRACKED)]
    assert not dead_no, f"NO_E2E prefixes that match no tracked file: {dead_no}"
