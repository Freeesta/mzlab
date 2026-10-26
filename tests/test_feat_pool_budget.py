"""Budget of the pool of feat-worker.js and of the run of feat-run.js (TP_Mine/files, run by node with a scripted fake Worker): the working memory of a step follows the engine
of the page, and a phase that fits only in the high budget asks once."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

FILES = Path(__file__).resolve().parent.parent / "TP_Mine" / "files"
JS = FILES / "feat-worker.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

PRELUDE = f"""
import {{ createFeatPool, budgetOf }} from {JS.as_uri()!r};
import {{ runFeatPipeline }} from {(FILES / 'feat-run.js').as_uri()!r};
const seen = [];
// the fake worker: `align` is refused with error.featMemory when its budget is under `need` MB
const fake = need => class {{
  postMessage(m) {{
    setTimeout(() => {{
      if (m.op === "init" || m.op === "alias") return this.onmessage({{ data: {{ id: m.id, ok: true, crate: "x" }} }});
      if (m.op === "file_key" || m.op === "check_stored" || m.op.startsWith("fs.")) return this.onmessage({{ data: {{ id: m.id, ok: true, text: "" }} }});   // the cache of the detections is not under test here
      if (m.op === "detect") return this.onmessage({{ data: {{ id: m.id, ok: true, bytes: new Uint8Array(1), mem: 1048576 }} }});
      seen.push(m.budgetMb);
      if ((m.budgetMb || 192) < need) return this.onmessage({{ data: {{ id: m.id, error: {{ key: "error.featMemory", params: {{ fase: "align", stima_mb: String(need), budget_mb: String(m.budgetMb || 192), libero_mb: "100" }} }} }} }});
      this.onmessage({{ data: {{ id: m.id, ok: true, bytes: new Uint8Array(1), mem: 1048576 }} }});
    }}, 1);
  }}
  terminate() {{}}
}};
const wasm = {{ feat: {{ jsUrl: "j", wasmUrl: "w" }} }};
const pool = (need, o = {{}}) => createFeatPool({{ workerUrl: "x", size: 1, idleMs: 0, wasm, WorkerClass: fake(need), ...o }});
const out = async f => console.log(JSON.stringify(await f()));
"""


def run(body):
    r = subprocess.run(["node", "--input-type=module", "-e", PRELUDE + "\n" + body], capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout.strip().splitlines()[-1])


def test_the_budget_is_the_option_then_the_engine_of_the_page_then_zero():
    assert run("""
globalThis.MZLAB_RUST = { choice: { mb: 1024, high_mb: 3072 } };
const a = budgetOf({}), b = budgetOf({ budgetMb: () => 512 });
globalThis.MZLAB_RUST.budgetMb = 192;                       // the supervisor lowered it
const c = budgetOf({});
delete globalThis.MZLAB_RUST;
console.log(JSON.stringify([a, b, c, budgetOf({})]));""") == [1024, 512, 192, 0]


RUN = '{ files: [{ name: "a", path: "a" }], recipe: "{}", parent: null }'


def test_budget_of_is_the_option_then_the_engine_then_zero_and_set_budget_gives_back_the_old_one():
    assert run("""
const p = pool(0, { budgetMb: 300 });
console.log(JSON.stringify([budgetOf({ budgetMb: 300 }), p.setBudget(700), budgetOf({ budgetMb: 700 })]));""") == [300, 300, 700]


def test_a_step_runs_with_the_safe_budget_of_the_engine_of_the_page():
    assert run("""
globalThis.MZLAB_RUST = { choice: { mb: 1024, high_mb: 3072 } };
await out(async () => { await pool(0).align([new Uint8Array(1)], ["p"], "{}"); return seen; });""") == [1024]


def test_a_phase_that_fits_only_in_the_high_budget_asks_once_runs_again_and_gives_the_budget_back():
    got = run("""
globalThis.MZLAB_RUST = { choice: { mb: 512, high_mb: 3072 } };
await out(async () => {
  const asked = [], p = pool(1400), r = { ...%s, askMore: async i => { asked.push(i); return true; } };
  await runFeatPipeline(p, r);
  const during = seen.slice();
  await p.align([new Uint8Array(1)], ["p"], "{}").catch(() => {});        // refused again: the pool works at the safe budget
  return { during, after: seen.slice(during.length), asked };
});""" % RUN)
    assert got["during"] == [512, 3072] and got["after"] == [512]       # the budget of the pool is the safe one again when the run ends
    assert got["asked"] == [{"fase": "align", "stima_mb": 1400, "budget_mb": 512, "libero_mb": 100, "high_mb": 3072}]


def test_a_refusal_goes_on_when_the_user_says_no_or_the_estimate_is_over_the_high_budget_or_nobody_is_asked():
    got = run("""
globalThis.MZLAB_RUST = { choice: { mb: 512, high_mb: 3072 } };
await out(async () => {
  const keys = [];
  const t = async (need, askMore) => { try { await runFeatPipeline(pool(need), { ...%s, askMore }); keys.push("ok"); } catch (e) { keys.push(e.key + ":" + e.params.stima_mb + ":" + e.params.files); } };
  await t(1400, async () => false);
  await t(4000, async () => { throw new Error("must not ask over the high budget"); });
  await t(1400, undefined);
  return { keys, seen };
});""" % RUN)
    assert got["keys"] == ["error.featMemory:1400:1", "error.featMemory:4000:1", "error.featMemory:1400:1"]
    assert got["seen"] == [512, 512, 512]                        # never run at the high budget without the yes


def test_progress_from_inside_a_call_reaches_the_step_callback_and_the_call_still_resolves():
    body = """
class W { postMessage(m) { setTimeout(() => {
  if (m.op === "init" || m.op === "alias") return this.onmessage({ data: { id: m.id, ok: true, crate: "x" } });
  this.onmessage({ data: { id: m.id, progress: [1, 4] } });
  this.onmessage({ data: { id: m.id, progress: [4, 4] } });
  this.onmessage({ data: { id: m.id, ok: true, bytes: new Uint8Array(1), mem: 1 } });
}, 1); } terminate() {} }
const steps = [];
const p = createFeatPool({ workerUrl: "x", size: 1, idleMs: 0, wasm, WorkerClass: W, onStep: s => steps.push([s.name, s.done, s.total]) });
await out(async () => { const r = await p.detectAll([{ name: "a", path: "a" }], "{}"); return { n: r.length, steps }; });
"""
    assert run(body) == {"n": 1, "steps": [["a", 1, 4], ["a", 4, 4]]}
