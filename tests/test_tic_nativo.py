"""The provisional TIC of a .raw (mzlab/web/tic-nativo.js), run by node: it goes with the rpc loading the sample at that moment, a dropped or reloaded sample posts nothing,
and a failed read of the index pool leaves no unhandled rejection (the real TIC comes with the index)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

SCRIPT = r"""
import { createTicNativo } from "%s";
const posts = [], unhandled = [];
process.on("unhandledRejection", e => unhandled.push(String(e)));
const raw = name => ({ id: 1, name, nat: { c: 0, members: [{ name: name + ".raw", blob: new Blob([]) }] } });
const stats = { rt: [0, 1], tic: [5, 6], y: [5, 6] };
const run = async (build, act) => { const t = createTicNativo(() => ({ build }), m => posts.push(m)); await act(t); await new Promise(r => setTimeout(r, 0)); };
await run(async () => ({ ok: true, stats }), t => { t.start(raw("a")); t.tell("a", 7, { frac: 0.5, phase: "scan" }); });
await run(async () => ({ ok: true, stats }), t => { t.start(raw("b")); t.drop("b"); });
await run(async () => { throw new Error("pool gone"); }, t => t.start(raw("c")));
await run(async () => ({ ok: true, stats }), t => t.start({ id: 1, name: "d", nat: { c: 0, members: [{ name: "d.wiff", blob: new Blob([]) }] } }));
console.log(JSON.stringify({ posts, unhandled }));
"""


def test_the_early_tic_follows_the_rpc_and_a_failed_read_is_quiet(tmp_path):
    r = subprocess.run(["node", "--input-type=module", "-e", SCRIPT % (ROOT / "mzlab" / "web" / "tic-nativo.js").as_uri()], capture_output=True, text=True, check=False)
    assert r.returncode == 0, r.stderr[-2000:]
    out = json.loads(r.stdout)
    assert out["unhandled"] == []
    assert out["posts"] == [{"frac": 0.5, "phase": "scan", "op": "progress", "id": 7, "early_tic": {"rt": [0, 1], "tic": [5, 6], "y": [5, 6]}}]
