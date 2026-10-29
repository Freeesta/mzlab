"""lr.js (the loading path of the browser engine) driven in node with a stub engine: remove() drops the items
of a `file#tag` item, upload() frees the early TIC. Each case fails on the code before the fix."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

LR = (Path(__file__).resolve().parents[1] / "mzlab" / "web" / "lr.js").as_uri()
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

DRIVER = """
import { createLr } from %(lr)r;
globalThis.window = globalThis; globalThis.CustomEvent = class { constructor(t, o) { this.type = t; this.detail = o && o.detail; } };
globalThis.dispatchEvent = () => true;
const it = (file) => ({ file, type: "sample", hr: false });
const rb = { noteExplore() {}, noteRemove() {}, noteNew() {}, noteUpload() {}, rpc: async (m, onp) => {
  if (m.op === "lr_items") return { info: JSON.stringify([{ info: it(m.name) }, { info: it(m.name + "#ms2") }]) };
  if (m.op === "lr_put") { onp && onp({ frac: 1, early_tic: [1, 2] }); return { files: [] }; }
  if (m.op === "lr_index") { onp && onp({ frac: 0.9, phase: "scan", early_tic: [1, 2] }); return { info: "{}" }; }
  if (m.op === "lr_entry") return { info: JSON.stringify({ name: m.name }) };
  if (m.op === "lr_methods") return { info: "[]" };
  return { info: "{}" };
} };
const lr = createLr({ rb });
const post = async (p, o) => JSON.parse((await lr.handle("POST", p, { body: JSON.stringify(o) })).text);
const out = {};
await lr.handle("POST", "/api/upload?name=a.mzML", { body: new Blob(["x"]) });
out.early_after_upload = lr.earlyTics.has("a.mzML");
await lr.handle("POST", "/api/upload?name=b.mzML", { body: new Blob(["x"]) });
const ex = await post("/api/explore", { samples: [{ file: "a.mzML" }, { file: "b.mzML" }] }); if (ex.error) console.error(JSON.stringify(ex));
await post("/api/remove", { name: "a.mzML" });
out.left = JSON.parse((await lr.handle("GET", "/api/session", {})).text).session.map(x => x.file);
console.log(JSON.stringify(out));
"""


@needs_node
def test_upload_meta_remove():
    r = subprocess.run(["node", "--input-type=module", "-e", DRIVER % {"lr": LR}], capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr[-1500:]
    out = json.loads(r.stdout.strip().splitlines()[-1])
    assert out["early_after_upload"] is False          # E12c
    assert out["left"] == ["b.mzML", "b.mzML#ms2"]     # E4a
