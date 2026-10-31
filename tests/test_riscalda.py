"""Queue of the warm-up of the pointed folder (mzlab/web/riscalda.js, run by node with a fake page): list order, one file at a time, a click takes a file out of the
queue and waits for work already running on it (never the same work twice), a busy or noisy page waits, big files and unfit devices are skipped, errors are silent."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "riscalda.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

PRELUDE = """
const calls = []; let busy = 0, mem, saveData = false, phone = false, desktop = false;
const listeners = {};
globalThis.window = globalThis; globalThis.addEventListener = (t, f) => { (listeners[t] = listeners[t] || []).push(f); };
Object.defineProperty(globalThis, 'navigator', { value: { get deviceMemory() { return mem; }, get connection() { return { saveData }; } }, configurable: true });
globalThis.document = { hidden: false };
Object.defineProperty(globalThis, 'UPBUSY', { get: () => busy, configurable: true });
globalThis.ST = { files: [] };
globalThis.MZLAB_DESKTOP = false;
Object.defineProperty(globalThis, 'QQQ_PHONE', { get: () => phone, configurable: true });
let delay = 20, answer = () => ({ warmed: true }), reject = false;
globalThis.fetch = async (url, o) => { if (reject) throw new Error('boom'); calls.push(decodeURIComponent(String(url).split('name=')[1])); await new Promise(r => setTimeout(r, delay)); return { json: async () => answer() }; };
const fileOf = (n, size = 1000) => ({ name: n, size });
const item = (n, size) => ({ key: 'd/' + n, name: n, size, get: async () => fileOf(n, size ?? 1000) });
const wait = ms => new Promise(r => setTimeout(r, ms));
const bump = () => (listeners.pointerdown || []).forEach(f => f());
"""


def run(body, timeout=30):
    code = PRELUDE + "import('%s').then(async m => { const R = m.RISCALDA; const out = await (async () => {%s})(); console.log(JSON.stringify(out)); process.exit(0); })" % (JS.as_uri(), body)
    r = subprocess.run(["node", "-e", code], capture_output=True, encoding="utf-8", timeout=timeout)
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout.strip().splitlines()[-1])


def test_files_are_prepared_one_at_a_time_in_list_order():
    out = run("R.offer(['c','a','b'].map(n => item(n + '.mzML'))); await wait(2600); return { calls, st: R.state() }")
    assert out["calls"] == ["c.mzML", "a.mzML", "b.mzML"]
    assert list(out["st"]["finished"].items()) == [("d/c.mzML", "warmed"), ("d/a.mzML", "warmed"), ("d/b.mzML", "warmed")]


def test_a_click_takes_the_file_out_of_the_queue_and_waits_for_the_one_running():
    out = run("""delay = 1000; R.offer(['a', 'b', 'c'].map(n => item(n + '.mzML'))); while (!R.state().running) await wait(10);
      const running = R.state().running; const t0 = Date.now(); await R.claim('d/a.mzML'); const waited = Date.now() - t0;
      await R.claim('d/c.mzML'); await wait(1500); return { running, waited, calls, st: R.state() }""")
    assert out["running"] == "d/a.mzML" and out["waited"] >= 500            # the file under way is awaited, not started twice
    assert out["calls"] == ["a.mzML", "b.mzML"]                              # c was clicked while queued: it never ran in the background
    assert "d/c.mzML" not in out["st"]["finished"]


def test_a_loading_or_a_busy_hand_makes_it_wait():
    out = run("""busy = 1; R.offer([item('a.mzML')]); await wait(2200); const during = calls.length; busy = 0; bump(); await wait(1000); const noisy = calls.length;
      await wait(1500); return { during, noisy, after: calls.length }""")
    assert out == {"during": 0, "noisy": 0, "after": 1}


def test_big_files_unfit_devices_and_other_kinds_are_skipped():
    out = run("""R.offer([item('big.mzML', 300 * 1048576), item('w.wiff'), item('ok.mzML', 5)]); await wait(2300); const a = calls.slice();
      mem = 2; calls.length = 0; R.offer([item('x.mzML')]); await wait(2200); const low = calls.slice(); mem = undefined; saveData = true; R.offer([item('y.mzML')]); await wait(2200); return { a, low, save: calls.slice() }""")
    assert out == {"a": ["ok.mzML"], "low": [], "save": []}


def test_the_same_name_in_two_folders_is_not_warmed_and_an_error_is_silent():
    out = run("""R.offer([{ ...item('same.mzML'), key: 'one/same.mzML' }, { ...item('same.mzML'), key: 'two/same.mzML' }, item('bad.mzML')]);
      reject = true; await wait(2300); return { calls, st: R.state() }""")
    assert out["calls"] == [] and out["st"]["finished"] == {"d/bad.mzML": "failed"}


def test_stop_clears_the_queue():
    out = run("R.offer(['a', 'b', 'c'].map(n => item(n + '.mzML'))); R.stop(); await wait(2200); return calls")
    assert out == []
