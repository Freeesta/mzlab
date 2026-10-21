"""The release gate (tools/cancello_rilascio.py) on a small temporary tree: green when aligned, red for a missing cli.md row, an uncited area file or an open box."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import cancello_rilascio as g  # noqa: E402


def tree(tmp: Path) -> Path:
    (tmp / "docs/agenti").mkdir(parents=True)
    (tmp / "docs/schemi").mkdir(parents=True)
    (tmp / "docs/rilascio").mkdir(parents=True)
    (tmp / "crates/mzlab-x").mkdir(parents=True)
    (tmp / "crates/mzlab-x/README.md").write_text("A crate.\n")
    (tmp / "Cargo.toml").write_text('[workspace.package]\nversion = "1.2.3"\n')
    (tmp / "docs/schemi/steps-a-1.json").write_text(json.dumps([{"id": "a.one"}, {"id": "a.two"}]))
    (tmp / "docs/agenti/cli.md").write_text("`a.one` and `a.two`\n")
    (tmp / "docs/agenti/area.md").write_text("x\n")
    (tmp / "AGENTS.md").write_text("see docs/agenti/area.md and cli.md\n")
    (tmp / "docs/rilascio/CHECKLIST-v1.2.3.md").write_text("- [x] done\n")
    return tmp


def run(root):
    out: list[str] = []
    return g.run(root, out=out.append), "\n".join(out)


def test_aligned_tree_passes(tmp_path):
    rc, out = run(tree(tmp_path))
    assert rc == 0, out


def test_missing_cli_row_fails(tmp_path):
    root = tree(tmp_path)
    (root / "docs/agenti/cli.md").write_text("`a.one`\n")
    rc, out = run(root)
    assert rc == 1 and "a.two" in out


def test_uncited_area_file_fails(tmp_path):
    root = tree(tmp_path)
    (root / "docs/agenti/nuovo.md").write_text("x\n")
    rc, out = run(root)
    assert rc == 1 and "nuovo.md" in out


def test_open_box_or_missing_checklist_fails(tmp_path):
    root = tree(tmp_path)
    (root / "docs/rilascio/CHECKLIST-v1.2.3.md").write_text("- [x] a\n- [ ] b\n")
    rc, out = run(root)
    assert rc == 1 and "CHECKLIST-v1.2.3.md:2" in out
    (root / "docs/rilascio/CHECKLIST-v1.2.3.md").unlink()
    assert run(root)[0] == 1


def test_template_has_no_ticked_box_and_missing_readme_fails(tmp_path):
    root = tree(tmp_path)
    (root / "crates/mzlab-x/README.md").write_text("")
    assert run(root)[0] == 1
