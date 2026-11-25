"""tools/controlla_coda.py: the queue in docs/coda stays consistent (rows, prompts, sizes, dependencies)."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import controlla_coda as cc  # noqa: E402

HEAD = "| id | prompt file | needs first | notes |\n|---|---|---|---|\n"


def row(rid, needs="-"):
    return f"| {rid} | `docs/coda/prompt/{rid}.md` | {needs} | note |\n"


def make(tmp_path, rows, prompts=None, brief=True):
    d = tmp_path / "docs" / "coda"
    (d / "prompt").mkdir(parents=True)
    (d / "CODA.md").write_text("# queue\n\n" + HEAD + "".join(rows), encoding="utf-8")
    if brief:
        (d / "BRIEF.md").write_text("brief\n", encoding="utf-8")
    for pid, body in (prompts or {}).items():
        (d / "prompt" / f"{pid}.md").write_text(body, encoding="utf-8")
    return tmp_path


OK = cc.BRIEF_LINE + "\n\n# X\n\n" + "\n\n".join(f"{s} text" for s in cc.SECTIONS) + "\n"


def test_a_consistent_queue_passes(tmp_path):
    root = make(tmp_path, [row("AA-ONE"), row("BB-TWO", "AA-ONE"), row("CC", "needs owner decision: x")],
                {"AA-ONE": OK, "BB-TWO": OK, "CC": OK})
    assert cc.peggiori(root) == []


def test_the_real_queue_passes():
    assert cc.peggiori() == []


def test_a_row_without_its_prompt_fails(tmp_path):
    root = make(tmp_path, [row("AA-ONE")], {})
    assert any("does not exist" in b for b in cc.peggiori(root))


def test_an_orphan_prompt_fails(tmp_path):
    root = make(tmp_path, [row("AA-ONE")], {"AA-ONE": OK, "ZZ": OK})
    assert any("orphan" in b for b in cc.peggiori(root))


def test_a_prompt_without_the_brief_line_or_too_big_fails(tmp_path):
    root = make(tmp_path, [row("AA-ONE"), row("BB")], {"AA-ONE": "Goal.\n", "BB": OK + "x" * cc.MAX_PROMPT})
    bad = cc.peggiori(root)
    assert any("exactly" in b for b in bad) and any("over 3500" in b for b in bad)


def test_the_brief_line_must_be_equal_not_a_prefix(tmp_path):
    root = make(tmp_path, [row("AA")], {"AA": OK.replace("BRIEF.md.", "BRIEF.mdx.", 1)})
    assert any("exactly" in b for b in cc.peggiori(root))
    root = make(tmp_path / "ok", [row("AA")], {"AA": "  " + OK})   # surrounding spaces are not a difference
    assert cc.peggiori(root) == []


def test_a_prompt_without_a_required_section_fails(tmp_path):
    for sec in cc.SECTIONS:
        root = make(tmp_path / sec[2:6], [row("AA")], {"AA": OK.replace(sec, "text")})
        assert any(f"missing the section {sec}" in b for b in cc.peggiori(root)), sec


def test_duplicate_and_long_and_lower_case_ids_fail(tmp_path):
    long_id = "A" * (cc.MAX_ID + 1)
    root = make(tmp_path, [row("AA"), row("AA"), row(long_id), row("low-er")],
                {"AA": OK, long_id: OK, "low-er": OK})
    bad = " ".join(cc.peggiori(root))
    assert "appears twice" in bad and "at most 24" in bad and "`low-er`" in bad


def test_a_dependency_on_a_removed_row_fails(tmp_path):
    root = make(tmp_path, [row("AA", "GONE")], {"AA": OK})
    assert any("not a row" in b for b in cc.peggiori(root))


def test_a_running_chat_dependency_is_allowed_but_must_be_well_formed(tmp_path):
    root = make(tmp_path, [row("AA", "chat:RUNNING-ONE, BB"), row("BB")], {"AA": OK, "BB": OK})
    assert cc.peggiori(root) == []
    root = make(tmp_path / "x", [row("AA", "chat:bad id")], {"AA": OK})
    assert any("bad chat id" in b for b in cc.peggiori(root))


def test_the_size_caps_fail(tmp_path):
    root = make(tmp_path, [row("AA")], {"AA": OK})
    (root / "docs/coda/BRIEF.md").write_text("x" * (cc.MAX_BRIEF + 1), encoding="utf-8")
    with (root / "docs/coda/CODA.md").open("a", encoding="utf-8") as f:
        f.write("y" * cc.MAX_CODA)
    bad = " ".join(cc.peggiori(root))
    assert "BRIEF.md" in bad and "CODA.md: " in bad and "over 25600" in bad


def test_a_missing_brief_or_queue_fails(tmp_path):
    root = make(tmp_path, [row("AA")], {"AA": OK}, brief=False)
    assert any("BRIEF.md: missing" in b for b in cc.peggiori(root))
    assert cc.peggiori(tmp_path / "nowhere") != []
