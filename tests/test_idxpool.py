"""Pool of Rust workers that build the .mzidx in the browser (mzlab/web/idxpool.js, run by node with a scripted fake Worker)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "idxpool.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

PRELUDE = f"""
import {{ createPool, poolSize }} from {JS.as_uri()!r};
const log = [];
let live = 0, maxLive = 0;
// script(job name, attempt) -> "done" | "die" | "mem" | "unavail" | "fail"
function fake(script) {{
  const attempts = {{}};
  return class {{
    constructor() {{ this.dead = false; live++; maxLive = Math.max(maxLive, live); setTimeout(() => {{ if (!this.dead) this.onmessage({{ data: {{ op: "ready" }} }}); }}, 1); }}
    postMessage(m) {{
      const n = m.blob; attempts[n] = (attempts[n] || 0) + 1; log.push("start " + n + "#" + attempts[n] + " b" + m.budget);
      const act = script(n, attempts[n]);
      setTimeout(() => {{
        if (this.dead) return;
        const msg = {{ done: {{ op: "done", stats: {{ passes: 1 }} }}, mem: {{ op: "error", text: '{{"error_key":"error.memoryBudget","params":{{}}}}', fatal: false }},
                      die: {{ op: "error", text: "unreachable", fatal: true }}, fail: {{ op: "error", text: '{{"error_key":"err.file.xml","params":{{}}}}', fatal: false }},
                      unavail: {{ op: "unavailable", text: "no wasm" }} }}[act];
        this.onmessage({{ data: msg }});
      }}, 3);
    }}
    terminate() {{ if (!this.dead) {{ this.dead = true; live--; }} }}
  }};
}}
const job = n => ({{ name: n, blob: n, inputs: [], part: n + ".part" }});
const out = async f => console.log(JSON.stringify(await f()));
"""


def run(body):
    code = PRELUDE + "\n" + body
    r = subprocess.run(["node", "--input-type=module", "-e", code], capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout.strip().splitlines()[-1])


def test_pool_size_rule():
    assert run("console.log(JSON.stringify([poolSize(false, 8), poolSize(false, 3), poolSize(false, 1), poolSize(false, undefined), poolSize(true, 8)]))") == [4, 2, 1, 1, 2]


def test_never_more_workers_than_the_limit_and_queue_order():
    got = run("""
await out(async () => {
  const p = createPool({ workerUrl: "x", size: 2, WorkerClass: fake(() => "done") });
  const rs = await Promise.all("abcde".split("").map(n => p.build(job(n))));
  return { ok: rs.every(r => r.ok), maxLive, starts: log.map(l => l.split("#")[0].slice(6)) };
});""")
    assert got["ok"] and got["maxLive"] <= 2 and got["starts"] == list("abcde")


def test_prioritize_puts_the_list_order_first():
    got = run("""
await out(async () => {
  const p = createPool({ workerUrl: "x", size: 1, WorkerClass: fake(() => "done") });
  const rs = ["a", "b", "c", "d"].map(n => p.build(job(n)));
  p.prioritize(["d", "c"]);
  await Promise.all(rs);
  return log.map(l => l.split("#")[0].slice(6));
});""")
    assert got == ["a", "d", "c", "b"]          # "a" had already started


def test_a_dead_worker_is_replaced_and_its_file_goes_back_to_the_queue_once():
    got = run("""
await out(async () => {
  const p = createPool({ workerUrl: "x", size: 1, WorkerClass: fake((n, k) => n === "a" && k === 1 ? "die" : n === "b" ? "die" : "done") });
  const rs = await Promise.all([p.build(job("a")), p.build(job("b")), p.build(job("c"))]);
  return { rs: rs.map(r => r.ok ? "ok" : r.reason), log, live };
});""")
    assert got["rs"] == ["ok", "crash", "ok"] and got["live"] == 0
    assert got["log"][:2] == ["start a#1 b64", "start a#2 b64"]       # retried at once, at the front of the queue


def test_memory_budget_halves_the_pool_and_the_budget_then_gives_up_once():
    got = run("""
await out(async () => {
  const notes = [];
  const p = createPool({ workerUrl: "x", size: 4, budget: 64, notify: k => notes.push(k), WorkerClass: fake((n, k) => n === "big" ? "mem" : "done") });
  const r = await p.build(job("big"));
  return { r: r.reason, limit: p.limit, notes, log };
});""")
    assert got["r"] == "memory" and got["limit"] == 1 and got["notes"] == ["error.memoryBudget"] * 2
    assert got["log"] == ["start big#1 b64", "start big#2 b32"]


def test_no_wasm_means_unavailable_for_every_job():
    got = run("""
await out(async () => {
  const p = createPool({ workerUrl: "x", size: 2, WorkerClass: fake(() => "unavail") });
  const rs = await Promise.all([p.build(job("a")), p.build(job("b"))]);
  const later = await p.build(job("c"));
  return { rs: rs.map(r => r.reason), later: later.reason, off: p.unavailable() };
});""")
    assert got == {"rs": ["unavailable", "unavailable"], "later": "unavailable", "off": True}


def test_a_file_error_is_not_retried_and_cancel_stops_a_queued_job():
    got = run("""
await out(async () => {
  const p = createPool({ workerUrl: "x", size: 1, WorkerClass: fake((n) => n === "a" ? "fail" : "done") });
  const ra = p.build(job("a")), rb = p.build(job("b")), rc = p.build(job("c"));
  p.cancel("b");
  const rs = await Promise.all([ra, rb, rc]);
  return { rs: rs.map(r => r.ok ? "ok" : r.reason), started: log.map(l => l.split("#")[0].slice(6)) };
});""")
    assert got == {"rs": ["error", "cancelled", "ok"], "started": ["a", "c"]}
