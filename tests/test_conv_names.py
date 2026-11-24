"""«Name files by: sample name» (mzlab/web/conv-names.js, run by node with the engine stubbed): what the page tells the route `mzml_names` for each file (the file it came from,
the reader's sample name, the sample number only for a dataset with several samples; the default name is the engine's, so the page sends none for a maker's file) and what it keeps of the answer."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "conv-names.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

HARNESS = """
global.window = global; global.I18N = { t: k => k };
const members = (n, extra) => [{ name: n, blob: { size: 10 } }, ...(extra ? [{ name: extra, blob: { size: 5 } }] : [])];
const SETS = { "nomi.wiff": [{ id: 1, name: "Sample A" }, { id: 2, name: "Sample B" }, { id: 3, name: "nomi.wiff #3" }], "a.raw": [{ id: 1, name: "S one" }], "a.D/AcqData": [{ id: 1, name: "S two" }] };
window.MZLAB_RUST = { nativeSamples: async m => ({ samples: SETS[m[0].name] }) };
let posted = null, answer = null;
global.fetch = async (_u, o) => { posted = JSON.parse(o.body); if (answer === "boom") throw new Error("down"); return { json: async () => answer || { files: posted.files.map(f => ({ out: f.plain || f.file, note: null })) } }; };
require(%s);
const conv = { meta: new Map(), renames: new Map(), notes: [] };
%s
"""


def run(setup, check):
    code = HARNESS % (json.dumps(str(JS)), setup + check)
    return json.loads(subprocess.run(["node", "-e", code], check=True, capture_output=True, encoding="utf-8").stdout)


def test_the_route_is_told_file_sample_name_and_number():
    got = run("""
conv.meta.set("Sample A.mzML", { members: members("nomi.wiff", "nomi.wiff.scan"), c: 0, id: 1 });
conv.meta.set("nomi_s3.mzML", { members: members("nomi.wiff", "nomi.wiff.scan"), c: 2, id: 3 });
conv.meta.set("a.mzML", { members: members("a.raw"), c: 0, id: 1 });
""", """(async () => { await CONVNAMES.ask(conv, ["Sample A.mzML", "nomi_s3.mzML", "a.mzML", "plain.mzML"]); console.log(JSON.stringify(posted.files)); })();""")
    assert got == [
        {"who": "nomi.wiff@1", "file": "nomi.wiff", "sample": "Sample A", "id": 1},
        {"who": "nomi.wiff@3", "file": "nomi.wiff", "sample": "nomi.wiff #3", "id": 3},
        {"who": "a.raw", "file": "a.raw", "sample": "S one", "id": None},
        {"who": "plain.mzML", "plain": "plain.mzML", "sample": None, "id": None},
    ]
    assert all("plain" not in f for f in got[:3]), "the default name of a maker's file is the engine's, not the page's"


def test_the_names_and_notes_of_the_route_are_kept_as_given():
    got = run("""
conv.meta.set("a.mzML", { members: members("a.raw"), c: 0, id: 1 });
conv.meta.set("b.mzML", { members: members("a.raw"), c: 0, id: 1 });
answer = { files: [{ out: "S_one.mzML", note: null }, { out: "S_one-2.mzML", note: "note: b: the name S_one is already taken: written as S_one-2.mzML" }] };
""", """(async () => { await CONVNAMES.ask(conv, ["a.mzML", "b.mzML"]); console.log(JSON.stringify([[...conv.renames], conv.notes])); })();""")
    note = "note: b: the name S_one is already taken: written as S_one-2.mzML"
    assert got[0][1] == ["b.mzML", {"out": "S_one-2.mzML", "note": note}]
    assert got[1] == [note]


def test_an_engine_that_does_not_answer_leaves_the_names_and_says_so():
    got = run("answer = 'boom';", """(async () => { const ok = await CONVNAMES.ask(conv, ["a.mzML"]); console.log(JSON.stringify([[...conv.renames], conv.notes, ok])); })();""")
    assert got == [[], ["conv.nameby.fail"], False]      # false: the caller forgets its signature and asks again at the next save
