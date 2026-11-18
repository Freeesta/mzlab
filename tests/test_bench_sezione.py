"""`replace_section` of tools/md_sezione.py: `--librerie --scrivi` rewrites only its own section of docs/agenti/measurements.md and keeps the sections after it."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import md_sezione as B  # noqa: E402


def test_the_sections_after_it_stay():
    txt = "# M\n\n## A\na\n\n## Spectral libraries (old)\nold\n\n## C\nc\n"
    out = B.replace_section(txt, "## Spectral libraries", ["## Spectral libraries (new)", "new"])
    assert out == "# M\n\n## A\na\n\n## Spectral libraries (new)\nnew\n\n## C\nc\n"


def test_last_section_and_missing_section():
    head = "## Spectral libraries"
    assert B.replace_section("# M\n\n## Spectral libraries\nold\n", head, [head, "n"]) == "# M\n\n## Spectral libraries\nn\n"
    assert B.replace_section("# M\n", head, [head, "n"]) == "# M\n\n## Spectral libraries\nn\n"
