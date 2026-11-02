"""Side file list (explore.js, open-now.js, app.css): the «Added N files» note is for the first load into an empty list only, only the first
group is expanded by default (a choice in E._grpOpen wins), and the file name takes the row's room while the meta text sits compact on the right."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"


def test_added_note_only_for_first_load():
    s = (WEB / "open-now.js").read_text(encoding="utf-8")
    assert "tally.first = true" in s and 'directNote(first ? "files.drop.done" : "", { n, m })' in s
    assert "const { n, m, first } = tally; tally.n = tally.m = 0; tally.first = false;" in s


def test_only_first_group_open_by_default():
    s = (WEB / "explore.js").read_text(encoding="utf-8")
    assert "firstOpen ? t === firstKey" in s
    assert re.search(r"E\._grpOpen\[t\] !== undefined \? E\._grpOpen\[t\]", s), "a manual choice must win"


def test_name_gets_room_and_meta_is_compact():
    css = (WEB / "app.css").read_text(encoding="utf-8")
    assert ".fl .nm{flex:1 1 auto;min-width:3ch;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}" in css
    m = re.search(r"\.fl \.fmeta\{([^}]*)\}", css)
    assert m and all(x in m.group(1) for x in ("flex:none", "margin-left:auto", "white-space:nowrap", "font-size:11px"))
    assert 'class="fmeta"' in (WEB / "explore.js").read_text(encoding="utf-8")
