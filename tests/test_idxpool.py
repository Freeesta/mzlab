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
const log = [], modules = [], seen = [];
let live = 0, maxLive = 0;
// script(job name, attempt) -> "done" | "die" | "mem" | "unavail" | "fail"
function fake(script) {{
  const attempts = {{}};
  return class {{
    constructor() {{ this.dead = false; live++; maxLive = Math.max(maxLive, live); setTimeout(() => {{ if (!this.dead) this.onmessage({{ data: {{ op: "ready" }} }}); }}, 1); }}
    postMessage(m) {{
      if (m.op === "module") {{ modules.push(m.module === null ? "null" : "set"); return; }}
      const n = m.blob; attempts[n] = (attempts[n] || 0) + 1; log.push("start " + n + "#" + attempts[n] + " b" + m.budget);
      seen.push(m.prep);
      const act = script(n, attempts[n]);
      if (act === "hang") return;
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


def test_the_scan_table_request_reaches_the_worker_and_a_prepare_failure_is_a_file_error():
    got = run("""
await out(async () => {
  const p = createPool({ workerUrl: "x", size: 2, WorkerClass: fake((n) => n === "a" ? "fail" : "done") });
  const mk = n => ({ name: n, blob: n, prep: { size: 5, prints: [n, "p"] }, part: n + ".part" });
  const rs = await Promise.all([p.build(mk("a")), p.build(mk("b"))]);
  return { rs: rs.map(r => r.ok ? "ok" : r.reason), seen, tries: log.filter(l => l.startsWith("start a")).length };
});""")
    assert got["rs"] == ["error", "ok"] and got["tries"] == 1
    assert got["seen"] == [{"size": 5, "prints": ["a", "p"]}, {"size": 5, "prints": ["b", "p"]}]


def test_cancel_while_the_worker_prepares_ends_it_and_frees_the_slot():
    got = run("""
await out(async () => {
  const p = createPool({ workerUrl: "x", size: 1, WorkerClass: fake((n) => n === "a" ? "hang" : "done") });
  const ra = p.build({ name: "a", blob: "a", prep: { size: 5, prints: [] }, part: "a.part" });
  await new Promise(r => setTimeout(r, 20));
  const rb = p.build(job("b"));
  p.cancel("a");
  const rs = await Promise.all([ra, rb]);
  return { rs: rs.map(r => r.ok ? "ok" : r.reason), live, active: p.active };
});""")
    assert got == {"rs": ["cancelled", "ok"], "live": 0, "active": 0}


def test_every_worker_is_told_first_which_module_to_use():
    got = run("""
  const p = createPool({ workerUrl: "x", size: 1, WorkerClass: fake(() => "done") });
  await p.build(job("a")); await p.build(job("b"));
  console.log(JSON.stringify([modules, log]));
""")
    assert got == [["null", "null"], ["start a#1 b64", "start b#1 b64"]]


def test_a_late_event_of_an_ended_worker_does_not_end_it_twice():
    got = run("""
await out(async () => {
  const made = [];
  const Cls = class extends fake((n, a) => a === 1 ? "die" : "done") { constructor(...a) { super(...a); made.push(this); } };
  const p = createPool({ workerUrl: "x", size: 1, WorkerClass: Cls });
  const r = p.build(job("a"));
  await new Promise(s => setTimeout(s, 8));          // first worker died, second one is running
  made[0].onmessage({ data: { op: "error", text: "late", fatal: true } });
  const res = await r;
  return { ok: res.ok, active: p.active };
});""")
    assert got == {"ok": True, "active": 0}


def test_an_early_failure_of_the_second_worker_is_unavailable_not_a_crash():
    got = run("""
await out(async () => {
  let n = 0;
  const Cls = class {
    constructor() { this.k = ++n; setTimeout(() => { if (this.k === 1) this.onmessage({ data: { op: "ready" } }); else this.onerror({ message: "boom" }); }, 1); }
    postMessage(m) { if (m.op !== "module" && this.k === 1) setTimeout(() => this.onmessage({ data: { op: "error", text: "x", fatal: true } }), 2); }
    terminate() {}
  };
  const p = createPool({ workerUrl: "x", size: 1, WorkerClass: Cls });
  const res = await p.build(job("a"));
  return { reason: res.reason, off: p.unavailable() };
});""")
    assert got == {"reason": "unavailable", "off": True}


def test_spill_get_reuses_one_buffer_and_returns_exact_lengths():
    code = """
import { makeSpill } from %r;
const data = new Uint8Array(100).map((_, i) => i);
const sh = { read(b, o) { b.set(data.subarray(o.at, o.at + b.length)); return b.length; } };
const s = makeSpill(sh, () => {}, () => 0);
const a = s.get(0, 10), b = s.get(10, 4), c = s.get(0, 50);
console.log(JSON.stringify({ la: a.length, lb: b.length, lc: c.length, b0: b[0], same: a.buffer === b.buffer, grew: c.buffer !== a.buffer }));
""" % (JS.with_name("idx-spill.js").as_uri(),)
    r = subprocess.run(["node", "--input-type=module", "-e", code], capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    assert json.loads(r.stdout) == {"la": 10, "lb": 4, "lc": 50, "b0": 10, "same": True, "grew": True}
