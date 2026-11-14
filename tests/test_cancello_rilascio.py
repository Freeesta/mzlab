"""The release gate (tools/gate_release.py) on a small temporary tree: green when aligned, red for a missing cli.md row, an uncited area file or an open box."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import gate_release as g  # noqa: E402


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
    (tmp / "docs/agenti/INDICE.md").write_text("see docs/agenti/area.md and cli.md\n")
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


def test_step_row_may_live_in_cli_passi(tmp_path):
    root = tree(tmp_path)
    (root / "docs/agenti/cli.md").write_text("`a.one`\n")
    (root / "docs/agenti/cli-passi.md").write_text("`a.two`\n")
    (root / "docs/agenti/INDICE.md").write_text("see docs/agenti/area.md, cli.md and cli-passi.md\n")
    rc, out = run(root)
    assert rc == 0, out


def test_plain_commands_of_the_cli_are_known(tmp_path):
    root = tree(tmp_path)
    (root / "crates/mzlab-cli/src").mkdir(parents=True)
    (root / "crates/mzlab-cli/src/lib.rs").write_text('const PLAIN: [&str; 2] = ["llm", "steps"];\nmatch a { Some("flow") => {} }\n')
    (root / "docs/agenti/per-agenti.md").write_text("run `mzlab-cli steps --json`\n")
    (root / "docs/agenti/INDICE.md").write_text("see docs/agenti/area.md and per-agenti.md\n")
    assert g.known_commands(root) == {"flow", "llm", "steps"}
    assert g.check_guides(root)[1] == []


def _index(root, *rows):
    (root / "docs/agenti/INDICE.md").write_text("# x\n| Se tocchi… | Leggi |\n|---|---|\n" + "\n".join(rows) + "\n")
    return g.check_index_rows(root)[1]


def test_index_rows_one_sorted_short_row_per_file(tmp_path):
    root = tree(tmp_path)
    assert _index(root, "| a | `docs/agenti/area.md` |", "| c | `docs/agenti/cli.md` |", "| ext | `tools/lab/README.md` (x) |") == []
    assert "exactly one" in _index(root, "| a | `docs/agenti/area.md`, `cli.md` |")[0]
    assert "exactly one" in _index(root, "| a | `docs/agenti/area.md` (see `cli.md`) |")[0]
    assert "not after" in _index(root, "| c | `docs/agenti/cli.md` |", "| a | `docs/agenti/area.md` |")[0]
    assert "not after" in _index(root, "| a | `docs/agenti/area.md` |", "| b | `docs/agenti/area.md` |")[0]
    assert "300" in _index(root, "| " + "x" * 300 + " | `docs/agenti/area.md` |")[0]


def test_the_real_index_has_one_row_per_area_file():
    assert g.check_index_rows(g.ROOT)[1] == []
    rows = [line for line in (g.ROOT / "docs/agenti/INDICE.md").read_text(encoding="utf-8").splitlines() if line.startswith("|") and "`docs/agenti/" in line.rsplit("|", 2)[-2]]
    assert sorted(line.rsplit("`", 2)[-2].rsplit("/", 1)[-1] for line in rows) == sorted(p.name for p in (g.ROOT / "docs/agenti").glob("*.md") if p.name != "INDICE.md")
