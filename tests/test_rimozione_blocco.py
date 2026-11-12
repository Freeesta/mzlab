"""Removing many files at once (mzlab/web/lista-file.js removeAllFiles, run by node with the page globals stubbed): the list, the side bar and the
panels are repainted once for the whole batch, on «Undo» too, and every file still leaves (and comes back to) the session and the loading table.
One repaint per file made «Remove all» quadratic: 200 files held the main thread for 41 s (docs/agenti/misure-velocita.md, LOAD-REMOVE-1)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

STUBS = """
globalThis.window = globalThis; globalThis.MZLAB_BROWSER = true;
globalThis.COMANDI = { gestore() {}, fai: (_a, _b, fn) => fn(), GLOBALE: 0 };
globalThis.I18N = { t: k => k };
globalThis.N = { renderFiles: 0, renderFileList: 0, renderNav: 0, paintFiles: 0, redrawAll: 0, posts: [] };
for (const k of ["renderFiles", "renderFileList", "renderNav", "paintFiles", "redrawAll"]) globalThis[k] = () => { N[k]++; };
globalThis.toolbar = () => {}; globalThis.ctl = () => {}; globalThis.uiSave = () => {}; globalThis.cleanupOrphanPanels = () => {}; globalThis.relayout = () => {}; globalThis.fitHost = () => {};
globalThis.panelRec = q => q; globalThis.riapriPannello = () => {}; globalThis.mergeFiles = () => {}; globalThis.J = async () => ({ session: E.files.map(f => ({ file: f.file })) });
globalThis.STATO = { file(k) { E.cur = k; } }; globalThis.BARRA = null;
globalThis.tabFiles = () => E.files.filter(f => !f.gone);
globalThis.post = async (_u, b) => { N.posts.push(b.name); return { files: [] }; };
globalThis.STORIA = { tolto(o) { globalThis.ENTRY = o; } };
globalThis.CONVROWS = { dropEnded() {}, clearDone() {} };      // the summary of the loading (conversione.js): nothing ended to drop here
globalThis.OPENNOW = { isOpen: n => E.files.some(f => !f.gone && f.file === n), held: () => false, hold() {}, release: () => false, tick() {}, idle: async () => {} };
const n = 200;
globalThis.E = { cur: 0, panels: [], files: Array.from({ length: n }, (_, k) => ({ k, file: `S_${k}.mzML`, label: `S_${k}`, kind: "full", vis: true })) };
globalThis.ST = { files: E.files.map(f => ({ name: f.file, use: true })) };
"""


def run(code):
    lf = (WEB / "lista-file.js").read_text(encoding="utf-8")
    js = STUBS + f"""
const vm = require("vm"); globalThis.carica = async () => {{}};
vm.runInThisContext({json.dumps(lf)});
globalThis.bootSession = async () => {{}};
(async () => {{ {code} }})().catch(e => {{ console.error(e); process.exit(1); }});
"""
    out = subprocess.run(["node", "-"], input=js, check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


SNAP = "({ ...N, posts: N.posts.length, gone: E.files.filter(f => f.gone).length, use: ST.files.filter(r => r.use).length })"


@needs_node
def test_remove_all_repaints_once():
    out = run(f"removeAllFiles(); console.log(JSON.stringify({SNAP}))")
    assert out == {"renderFiles": 1, "renderFileList": 1, "renderNav": 1, "paintFiles": 1, "redrawAll": 1, "posts": 0, "gone": 200, "use": 0}


@needs_node
def test_undo_of_remove_all_repaints_once():
    out = run(f"removeAllFiles(); for (const k in N) if (k !== 'posts') N[k] = 0; ENTRY.undo(); console.log(JSON.stringify({SNAP}))")
    assert out == {"renderFiles": 1, "renderFileList": 1, "renderNav": 1, "paintFiles": 1, "redrawAll": 1, "posts": 0, "gone": 0, "use": 200}


@needs_node
def test_freeing_the_batch_asks_every_file_and_repaints_the_table_once():
    out = run(f"removeAllFiles(); N.renderFiles = 0; await ENTRY.scade(); console.log(JSON.stringify({SNAP}))")
    assert out["posts"] == 200 and out["renderFiles"] == 1 and out["gone"] == 200


@needs_node
def test_rows_read_but_not_open_are_freed_with_the_batch():
    extra = "ST.files.push({ name: 'Q_0.mzML', use: true }, { name: 'Q_1.mzML', use: false });"
    out = run(f"{extra} removeAllFiles(); N.renderFiles = 0; await ENTRY.scade(); console.log(JSON.stringify({{ ...{SNAP}, q: N.posts.filter(x => x.startsWith('Q_')) }}))")
    assert out["posts"] == 202 and sorted(out["q"]) == ["Q_0.mzML", "Q_1.mzML"] and out["renderFiles"] == 1 and out["gone"] == 200


@needs_node
def test_one_file_still_repaints_by_itself():
    out = run(f"removeFile(E.files[3]); console.log(JSON.stringify({SNAP}))")
    assert out["renderFiles"] == 1 and out["renderFileList"] == 1 and out["gone"] == 1 and out["use"] == 199
