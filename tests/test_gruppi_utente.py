"""User groups of the side file list (mzlab/web/lista-file.js, run by node with the page globals stubbed): «Group files» takes the ticked files
(both parts of a DDA file, never an MRM file in the bench) into «Group N» with the first free N, rename and ungroup move every file of the group,
the header escapes the name and «Remove group» takes only the group's files away."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

STUBS = """
globalThis.window = globalThis;
globalThis.COMANDI = { gestore() {}, fai: (_a, _b, fn) => fn(), GLOBALE: 0 };
globalThis.I18N = { t: (k, p) => k === "files.ugrp.name" ? "Group " + p.n : k };
globalThis.EH = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
globalThis.fileBase = n => String(n).split("#")[0];
globalThis.ESP = { voce: k => ({ spettri: k !== "mrm" }), unici() {} };
globalThis.BENCH = false; globalThis.lrTog = () => !BENCH;
globalThis.renderFileList = () => {}; globalThis.uiSave = () => {}; globalThis.STORIA = null; globalThis.removed = [];
"""


def run(code):
    lf = (WEB / "lista-file.js").read_text(encoding="utf-8")
    gr = (WEB / "gruppi.js").read_text(encoding="utf-8")
    js = STUBS + f"""
const vm = require("vm"); globalThis.carica = async () => {{}}; globalThis.renderFiles = () => {{}}; globalThis.flush = () => new Promise(r => setTimeout(r, 0));
vm.runInThisContext({json.dumps(lf)}); vm.runInThisContext({json.dumps(gr)});
globalThis.fileRemoval = f => ({{ take() {{ f.gone = true; removed.push(f.file); }}, back() {{}}, free() {{}} }});
globalThis.E = {{ files: [
  {{ file: "a.mzML", kind: "full", vis: true }}, {{ file: "b.mzML", kind: "full", vis: true }},
  {{ file: "d.mzML", kind: "full", vis: true, dda: true }}, {{ file: "d.mzML#MS2", kind: "ms2", vis: false, dda: true }},
  {{ file: "c.mzML", kind: "full", vis: false }}, {{ file: "m.wiff", kind: "mrm", vis: true }} ] }};
(async () => {{ {code} }})().catch(e => {{ console.error(e); process.exit(1); }});
"""
    out = subprocess.run(["node", "-"], input=js, check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


G = "E.files.map(f => f.ugrp || null)"


@needs_node
def test_group_takes_the_ticked_files_and_ddas_other_part():
    assert run(f"groupFiles(); await flush(); console.log(JSON.stringify({G}))") == ["Group 1", "Group 1", "Group 1", "Group 1", None, "Group 1"]


@needs_node
def test_bench_never_groups_mrm_and_the_next_name_is_free():
    out = run(f"BENCH = true; E.files[0].ugrp = 'Group 1'; groupFiles(); await flush(); console.log(JSON.stringify({G}))")
    assert out == ["Group 2", "Group 2", "Group 2", "Group 2", None, None]


@needs_node
def test_rename_and_ungroup_move_the_whole_group():
    out = run(f"""groupFiles(); await flush(); E._grpOpen = {{ "u:Group 1": false }}; globalThis.ask = async () => " Blanks ";
await renameGroup("Group 1"); const a = {G}, open = E._grpOpen["u:Blanks"];
GRUPPI.ungroup("Blanks"); console.log(JSON.stringify([a, open, {G}]))""")
    assert out[0] == ["Blanks", "Blanks", "Blanks", "Blanks", None, "Blanks"] and out[1] is False and out[2] == [None] * 6


@needs_node
def test_header_escapes_the_name_and_counts_rows():
    h = run("""console.log(JSON.stringify(ugHead({ g: 'u:<b>"x"', fs: [{ vis: true }, { vis: false }] }, true)))""")
    assert 'data-ug="&lt;b&gt;&quot;x&quot;"' in h and "<b>" not in h and "<em>2</em>" in h and 'aria-expanded="true"' in h and "checked" not in h.split("title")[0]


@needs_node
def test_remove_group_takes_only_its_files():
    assert run("E.files[0].ugrp = E.files[4].ugrp = 'G'; removeAllFiles(ugFiles('G'), 'G'); console.log(JSON.stringify(removed))") == ["a.mzML", "c.mzML"]
