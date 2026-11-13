"""`replace_section` of tools/bench.py: `--librerie --scrivi` rewrites only its own section of docs/agenti/misure.md and keeps the sections after it."""
import importlib.util
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("bench_script", Path(__file__).resolve().parents[1] / "tools" / "bench.py")
B = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(B)


def test_the_sections_after_it_stay():
    txt = "# M\n\n## A\na\n\n## Librerie spettrali (old)\nold\n\n## C\nc\n"
    out = B.replace_section(txt, "## Librerie spettrali", ["## Librerie spettrali (new)", "new"])
    assert out == "# M\n\n## A\na\n\n## Librerie spettrali (new)\nnew\n\n## C\nc\n"


def test_last_section_and_missing_section():
    head = "## Librerie spettrali"
    assert B.replace_section("# M\n\n## Librerie spettrali\nold\n", head, [head, "n"]) == "# M\n\n## Librerie spettrali\nn\n"
    assert B.replace_section("# M\n", head, [head, "n"]) == "# M\n\n## Librerie spettrali\nn\n"
