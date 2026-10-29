"""Which read leases a request of the engine worker wakes (mzlab/web/rust-lease-plan.js, archivio.js wake/parkAll), run by node with a fake OPFS that counts createSyncAccessHandle:
a request that reads no index opens nothing, one that reads a file opens only its handle, an unknown one still wakes every lease; the ops listed as read-free really call no lease."""
import json
import re
import subprocess
from pathlib import Path

from test_archivio import FAKE_OPFS

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"

PRELUDE = f"""
globalThis.self = globalThis;
const nav = o => Object.defineProperty(globalThis, "navigator", {{ value: o, configurable: true }});
import * as A from {(WEB / "archivio.js").as_uri()!r};
import {{ leasePlan }} from {(WEB / "rust-lease-plan.js").as_uri()!r};
const out = async f => {{ console.log(JSON.stringify(await f())); process.exit(0); }};
"""

# the counter wraps the fake handle factory; `serve` is onmessage of rust-worker.js: wake what the plan names, run the op, park when no request is left
COUNTING = """
let opens = 0; const create = FileSystemFileHandle.prototype.createSyncAccessHandle;
FileSystemFileHandle.prototype.createSyncAccessHandle = function () { opens++; return create.call(this); };
const NAMES = Array.from({ length: 14 }, (_, i) => "s" + i + ".mzML");
const slots = new Map(NAMES.map((n, i) => [i, n]));
let active = 0;
const serve = async (m, legacy) => {
  active++;
  try {
    const plan = leasePlan(m, s => slots.get(s));
    await A.wake(legacy || !plan ? undefined : plan.map(n => n + ".mzidx"));
    if (m.op === "lr_items") await A.lease("idx", m.name + ".mzidx");
  } finally { if (--active === 0) await A.parkAll(); }
};
const load = async legacy => {
  for (const n of NAMES) files.set(n + ".mzidx", new Uint8Array([1, 2, 3]));
  opens = 0;
  for (const n of NAMES) await A.lease("idx", n + ".mzidx");
  await A.parkAll();
  const first = opens; opens = 0;
  for (let i = 0; i < 30; i++) await serve({ op: "lr_campioni", cop: "proponi" }, legacy);
  for (const n of NAMES) await serve({ op: "lr_items", name: n }, legacy);
  return [first, opens];
};
"""


def run(body):
    r = subprocess.run(["node", "--input-type=module", "-e", PRELUDE + FAKE_OPFS + COUNTING + body], capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout.strip().splitlines()[-1])


def test_a_14_file_load_opens_one_handle_per_item_request_and_none_for_the_campioni():
    got = run("await out(async () => ({ before: await load(true), after: await load(false) }));")
    first, before = got["before"]
    assert first == 14
    assert before == 14 * 44          # every one of the 44 requests woke the 14 leases
    assert got["after"][1] == 14      # lr_campioni wakes nothing; each lr_items opens only its own file


def test_a_slot_op_wakes_only_the_lease_of_its_file_and_an_unknown_op_wakes_all():
    got = run("""
  await out(async () => {
    for (const n of NAMES) { files.set(n + ".mzidx", new Uint8Array([1])); await A.lease("idx", n + ".mzidx"); }
    const reads = async () => NAMES.map(n => A.leaseRead("idx", n + ".mzidx", new Uint8Array(1), 0));
    await A.parkAll(); const closed = await reads();
    await serve({ op: "tic", slot: 3 }); await A.wake(leasePlan({ op: "tic", slot: 3 }, s => slots.get(s)).map(n => n + ".mzidx"));
    const one = (await reads()).map((r, i) => i).filter(i => A.leaseRead("idx", NAMES[i] + ".mzidx", new Uint8Array(1), 0) === 1);
    await A.parkAll(); await A.wake(leasePlan({ op: "origin" }, () => undefined) ?? undefined);
    const all = (await reads()).filter(r => r === 1).length;
    return { closed: closed.every(r => r === -1), one, all, unknownSlot: leasePlan({ op: "xic", slot: 99 }, () => undefined), bob: leasePlan({ op: "tic", slot: 5 }, () => "") };
  });""")
    assert got == {"closed": True, "one": [3], "all": 14, "unknownSlot": None, "bob": []}


def test_the_ops_listed_as_read_free_call_no_lease():
    plan = (WEB / "rust-lease-plan.js").read_text(encoding="utf-8")
    listed = set(re.findall(r'"(\w+)"', plan[plan.index("READ_FREE") : plan.index("]);")]))
    src = (WEB / "rust-worker.js").read_text(encoding="utf-8")
    cases = re.split(r'(?=\bcase ")', src)
    seen = set()
    for seg in cases:
        m = re.match(r'case "(\w+)"', seg)
        if not m or m.group(1) not in listed:
            continue
        seen.add(m.group(1))
        head = re.split(r'\bcase "', seg[5:])[0]
        body = head
        assert not re.search(r'lrOpen|ARCH\.lease|nativo\.attach|readerFor\([^)]*,|engine\.(?!close|close_all|set_budget_mb|open\(|crash|mzml_to_centroids)\w+', body), (m.group(1), body)
    assert listed <= seen, listed - seen
