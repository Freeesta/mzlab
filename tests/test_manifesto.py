"""Version, crate and source of the derived files (mzlab/web/manifesto.js, pulizia.js) read by node from synthetic `.mzidx` and `.mzfeatf` headers."""
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"

PRELUDE = f"""
import {{ infoDi, vecchio, CORRENTI }} from {(WEB / "manifesto.js").as_uri()!r};
import {{ limpia }} from {(WEB / "pulizia.js").as_uri()!r};
const enc = new TextEncoder();
const idx = (ver, meta) => {{ const m = enc.encode(JSON.stringify(meta)), h = new DataView(new ArrayBuffer(64));
  enc.encode("MZIDX001").forEach((b, i) => h.setUint8(i, b)); h.setUint32(8, ver, true); h.setBigUint64(16, 64n, true); h.setBigUint64(24, BigInt(m.length), true);
  return new Blob([h.buffer, m]); }};
const feat = meta => {{ const m = enc.encode(JSON.stringify(meta)), h = new DataView(new ArrayBuffer(16));
  enc.encode("MZFEAT01").forEach((b, i) => h.setUint8(i, b)); h.setBigUint64(8, BigInt(m.length), true); return new Blob([h.buffer, m, new Uint8Array(64)]); }};
const out = async f => console.log(JSON.stringify(await f()));
"""


def run(body):
    r = subprocess.run(["node", "--input-type=module", "-e", PRELUDE + body], capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout.strip().splitlines()[-1])


def test_the_current_versions_follow_the_rust_constants():
    feat = int(re.search(r"pub const FEAT_VERSION: u32 = (\d+);", (ROOT / "crates/mzlab-feat/src/lib.rs").read_text()).group(1))
    got = run("""await out(async () => CORRENTI);""")
    assert got["feat"] == feat and got["idx"] == 1


def test_the_information_is_read_from_the_meta_of_the_file():
    got = run("""await out(async () => [
      await infoDi("idx", idx(1, { crate: "0.2.0", source: { sha256: "ab" } })),
      await infoDi("feat", feat({ crate: "0.2.0", feat_version: 3, inputs: [{ name: "a", sha256: "cd" }] })),
      await infoDi("feat", feat({ crate: "0.2.0", feat_version: 3, inputs: [{ sha256: "1" }, { sha256: "2" }] })),
      await infoDi("idx", new Blob([new Uint8Array(80)])), await infoDi("idx", null)]);""")
    assert got == [{"versione_formato": 1, "crate": "0.2.0", "sorgente_sha256": "ab"},
                   {"versione_formato": 3, "crate": "0.2.0", "sorgente_sha256": "cd"},
                   {"versione_formato": 3, "crate": "0.2.0", "sorgente_sha256": None}, None, None]


def test_cleaning_removes_only_derived_files_of_an_older_format_and_fills_the_manifest():
    got = run("""
  const blobs = { "idx/old.mzidx": idx(0, {}), "idx/ok.mzidx": idx(1, { crate: "1", source: { sha256: "s" } }), "idx/new.mzidx": idx(9, {}),
    "feat/old.mzfeatf": feat({ feat_version: 2, crate: "1" }), "feat/ok.mzfeatf": feat({ feat_version: 3, crate: "1", inputs: [{ sha256: "q" }] }), "feat/junk": new Blob(["x"]) };
  const lists = { files: [], idx: ["old", "ok", "new"].map(n => ({ name: n + ".mzidx", size: 1, lastModified: Date.now() })), feat: ["old", "ok"].map(n => ({ name: n + ".mzfeatf", size: 1, lastModified: Date.now() })).concat([{ name: "junk", size: 1, lastModified: Date.now() }]) };
  const gone = [], noted = [];
  await out(async () => { await limpia({ list: async k => lists[k], remove: async (k, n) => { gone.push(k + "/" + n); }, manifest: async () => [],
    info: async (k, n) => infoDi(k, blobs[k + "/" + n]), annota: async (k, e, i) => { noted.push(k + "/" + e.name + ":" + i.versione_formato); } });
    return [gone.sort(), noted.sort()]; });""")
    assert got[0] == ["feat/old.mzfeatf", "idx/old.mzidx"]
    assert got[1] == ["feat/ok.mzfeatf:3", "idx/new.mzidx:9", "idx/ok.mzidx:1"]
