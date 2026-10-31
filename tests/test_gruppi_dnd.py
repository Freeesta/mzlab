"""Dragging files between groups (mzlab/web/gruppi-dnd.js, run by node with the page globals stubbed): what is under the pointer of a list (header, row thirds, the gap after
the last group), the files that travel with the dragged one, and the repaint that waits while a drag holds the list."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

STUBS = """
globalThis.window = globalThis;
globalThis.I18N = { t: (k, p) => k + (p ? JSON.stringify(p) : "") };
globalThis.document = { getElementById: () => null, createElement: () => ({ setAttribute() {} }), body: { appendChild() {} } };
globalThis.renders = 0; globalThis.renderFileList = () => { renders++; };
globalThis.say = [];
// a list: header MQ, two rows, header TW, one row, a row of a folded group (no height)
const box = (kind, key, top, bottom) => ({ kind, key, top, bottom });
globalThis.BOXES = [box("head", "MQ", 0, 24), box("row", "a", 24, 48), box("row", "b", 48, 72), box("head", "TW", 80, 104), box("row", "c", 104, 128), box("row", "folded", 128, 128)];
"""


def run(code):
    src = (WEB / "gruppi-dnd.js").read_text(encoding="utf-8")
    js = STUBS + f"""
require("vm").runInThisContext({json.dumps(src)});
(async () => {{ {code} }})().catch(e => {{ console.error(e); process.exit(1); }});
"""
    return json.loads(subprocess.run(["node", "-e", js], check=True, capture_output=True, encoding="utf-8").stdout)


@needs_node
def test_a_header_a_row_with_its_thirds_and_the_gap_are_told_apart():
    out = run("""
      const r = y => { const t = GDND.resolve(BOXES, y); return t && [t.kind, t.key, t.zone || null]; };
      console.log(JSON.stringify([r(10), r(26), r(36), r(46), r(60), r(90), r(120), r(130), r(400), r(-5)]));""")
    assert out == [
        ["head", "MQ", None],
        ["row", "a", "before"],
        ["row", "a", "onto"],  # the middle third: 03 treats it as «insert», 05 will drop ONTO the file
        ["row", "a", "after"],
        ["row", "b", "onto"],
        ["head", "TW", None],
        ["row", "c", "after"],
        ["gap", "", None],  # below the last row with a height: the folded row does not count
        ["gap", "", None],
        None,
    ]


@needs_node
def test_a_pointer_in_the_hole_between_two_boxes_belongs_to_the_one_above():
    out = run("""const t = GDND.resolve(BOXES, 76); console.log(JSON.stringify([t.kind, t.key]));""")
    assert out == ["row", "b"]


@needs_node
def test_an_empty_list_is_all_gap():
    assert run("""console.log(JSON.stringify(GDND.resolve([], 5)));""") == {"kind": "gap", "key": ""}


@needs_node
def test_ticked_companions_travel_only_with_a_ticked_dragged_file():
    out = run("""console.log(JSON.stringify([GDND.companions("b", ["a", "b", "c"]), GDND.companions("z", ["a", "b"]), GDND.companions("a", ["a", "a"]), GDND.companions("a", [])]));""")
    assert out == [["a", "b", "c"], ["z"], ["a"], ["a"]]


@needs_node
def test_the_keyboard_move_picks_the_neighbour_header_and_stops_at_the_ends():
    out = run("""const h = ["", "MQ", "TW"]; console.log(JSON.stringify([GDND.pick(h, 1, 1), GDND.pick(h, 1, -1), GDND.pick(h, 2, 1), GDND.pick(h, 0, -1), GDND.pick(h, -1, 1)]));""")
    assert out == ["TW", "", None, None, None]


@needs_node
def test_file_arrivals_wait_for_the_end_of_a_drag_and_paint_once():
    out = run("""
      const seen = [GDND.busy()];
      GDND.begin("a"); seen.push(GDND.busy());
      GDND.defer(); GDND.defer();
      GDND.cancel(); seen.push(GDND.busy(), renders);       // one repaint for any number of arrivals
      GDND.begin("a"); GDND.cancel(); seen.push(renders);   // nothing arrived: no repaint
      console.log(JSON.stringify(seen));""")
    assert out == [False, True, False, 1, 1]


@needs_node
def test_a_drag_that_went_silent_no_longer_holds_the_list():
    out = run("""
      const now = Date.now; GDND.begin("a"); const a = GDND.busy();
      Date.now = () => now() + 5000; const b = GDND.busy(); Date.now = now;
      console.log(JSON.stringify([a, b]));""")
    assert out == [True, False]
