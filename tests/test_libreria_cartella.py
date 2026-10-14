"""The folder of the libraries (mzlab/web/libreria-cartella.js): SHA-256, the four states and the inventory of a folder, run by node on an in-memory directory."""
import json
import subprocess
from pathlib import Path

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "libreria-cartella.js"


def run(code):
    full = f"const C=require({str(JS)!r});(async()=>{{{code}}})().then(r=>console.log(JSON.stringify(r)))"
    return json.loads(subprocess.run(["node", "-e", full], check=True, capture_output=True, encoding="utf-8").stdout)


# an in-memory folder with the interface of handleDir/nativeDir (list, file, write, remove)
FAKE = """
const mk = files => { const d = { files, async list(sub = "") { return Object.entries(d.files).filter(([k]) => (sub ? k.startsWith(sub + "/") && !k.slice(sub.length + 1).includes("/") : !k.includes("/"))).map(([k, v]) => ({ name: k.split("/").pop(), size: v.data.length, mtime: v.mtime })); },
  async file(rel) { const v = d.files[rel]; if (!v) throw new Error("none"); return { size: v.data.length, lastModified: v.mtime, slice: (a, b) => ({ arrayBuffer: async () => v.data.slice(a, b).buffer.slice(0) , size: b - a }), text: async () => new TextDecoder().decode(v.data) }; },
  async write(rel, data) { d.files[rel] = { data: typeof data === "string" ? new TextEncoder().encode(data) : data, mtime: 1 }; }, async remove(rel) { delete d.files[rel]; } }; return d; };
const F = (s, mtime = 5) => ({ data: new TextEncoder().encode(s), mtime });
"""


def test_sha256_matches_node_crypto_for_any_split():
    r = run("""
const crypto = require("crypto"); const out = [];
for (const n of [0, 1, 55, 56, 63, 64, 65, 119, 120, 1000, 100000]) {
  const b = Buffer.alloc(n); for (let i = 0; i < n; i++) b[i] = (i * 7 + 3) & 255;
  const want = crypto.createHash("sha256").update(b).digest("hex"), h = new C.Sha256();
  for (let o = 0; o < n; o += 37) h.update(new Uint8Array(b.subarray(o, o + 37)));
  out.push(h.hex() === want);
}
const big = Buffer.alloc((5 << 20) + 17, 9), f = { size: big.length, slice: (a, b) => ({ arrayBuffer: async () => big.subarray(a, b).buffer.slice(big.subarray(a, b).byteOffset, big.subarray(a, b).byteOffset + (b - a)) }) };
out.push(await C.sha256Of(f) === crypto.createHash("sha256").update(big).digest("hex"));
return out;""")
    assert all(r), r


def test_states_of_a_source():
    r = run("""
const sha = async () => "aa", meta = { origine: { size: 10, mtime: 5, sha256: "aa" }, bytes: 99 }, f = { size: 10, mtime: 5 };
return [
  (await C.statoSorgente(f, null, null, sha)).stato,
  (await C.statoSorgente(f, meta, 99, sha)).stato,
  (await C.statoSorgente(f, meta, null, sha)).stato,                 // index missing
  (await C.statoSorgente(f, meta, 98, sha)).stato,                   // index of another size
  (await C.statoSorgente({ size: 10, mtime: 6 }, meta, 99, sha)),    // touched, same content
  (await C.statoSorgente({ size: 10, mtime: 6 }, meta, 99, async () => "bb")).stato,   // changed
  (await C.statoSorgente({ size: 11, mtime: 6 }, meta, 99, sha)).stato,
];""")
    assert r[0] == "da_indicizzare" and r[1] == "pronta" and r[2] == "danneggiata" and r[3] == "danneggiata"
    assert r[4] == {"stato": "pronta", "mtime": 6} and r[5] == "da_indicizzare" and r[6] == "da_indicizzare"


def test_the_hash_is_computed_only_when_size_or_date_changed():
    r = run("""
let n = 0; const sha = async () => { n++; return "aa"; }, meta = { origine: { size: 10, mtime: 5, sha256: "aa" }, bytes: 1 };
await C.statoSorgente({ size: 10, mtime: 5 }, meta, 1, sha); await C.statoSorgente({ size: 11, mtime: 9 }, meta, 1, sha);
const side = { id: "x", versione: "1", sha256: "aa", size: 10, mtime: 5 };
await C.statoPacchetto({ size: 10, mtime: 5 }, side, null, true, sha);
return n;""")
    assert r == 0


def test_states_of_a_package():
    r = run("""
const sha = async () => "aa", side = { id: "x", versione: "1", sha256: "aa", size: 10, mtime: 5 }, f = { size: 10, mtime: 5 }, cat = { versione: "2", sha256: "bb" };
return [
  (await C.statoPacchetto(f, null, null, true, sha)).stato,           // a file of the user that opens
  (await C.statoPacchetto(f, null, null, false, sha)).stato,
  (await C.statoPacchetto(f, side, null, true, sha)).stato,
  (await C.statoPacchetto(f, side, cat, true, sha)),                  // the catalog has a newer one
  (await C.statoPacchetto(f, side, { versione: "1", sha256: "aa" }, true, sha)).stato,
  (await C.statoPacchetto({ size: 9, mtime: 5 }, side, null, true, sha)).stato,          // truncated
  (await C.statoPacchetto({ size: 10, mtime: 7 }, side, null, true, async () => "cc")).stato,   // same size, other bytes
];""")
    assert r[0] == "pronta" and r[1] == "danneggiata" and r[2] == "pronta"
    assert r[3]["stato"] == "aggiornamento" and r[3]["nuova"] == "2" and r[4] == "pronta" and r[5] == "danneggiata" and r[6] == "danneggiata"


def test_scan_of_a_folder():
    r = run(FAKE + """
const meta = (name, size, mtime, bytes) => JSON.stringify({ v: 1, origine: { nome: name, size, mtime, sha256: "x" }, info: { n: 3, pos: 3, neg: 0 }, bytes });
const d = mk({
  "a.msp": F("0123456789"), ".mzlab/a.msp.idx.json": F(meta("a.msp", 10, 5, 4)), ".mzlab/a.msp.mzlib": F("MZLB"),            // ready
  "b.mgf": F("0123456789"),                                                                                                    // to index
  "c.msp": F("0123456789", 9), ".mzlab/c.msp.idx.json": F(meta("c.msp", 10, 5, 4)), ".mzlab/c.msp.mzlib": F("MZLB"),           // changed after indexing (other date, other bytes below)
  "d.msp": F("0123456789"), ".mzlab/d.msp.idx.json": F(meta("d.msp", 10, 5, 4)),                                                 // index file lost
  "e.mzlib": F("not a library at all, no magic and a long text"),
  "notes.pdf": F("zzz"), ".hidden.msp": F("zzz"),
  ".mzlab/z.msp.idx.json": F(meta("z.msp", 100, 5, 4)), ".mzlab/z.msp.mzlib": F("MZLB"),                                        // imported by the button
});
d.files["c.msp"].data = new TextEncoder().encode("9876543210");
const s = await C.scan(d, null); return s.voci.map(v => [v.nome, v.tipo, v.stato, v.info && v.info.n]);""")
    assert r == [["a.msp", "sorgente", "pronta", 3], ["b.mgf", "sorgente", "da_indicizzare", None], ["c.msp", "sorgente", "da_indicizzare", None],
                 ["d.msp", "sorgente", "danneggiata", 3], ["e.mzlib", "pacchetto", "danneggiata", None], ["z.msp", "importata", "pronta", 3]]


def test_paths_stay_inside_the_folder():
    r = run("""
const bad = ["../x", "/etc/passwd", "a/b/c", "..", ".mzlab/../x", "x/y", "a\\\\b", "", "c:x"], out = [];
for (const b of bad) { try { C.safeRel(b); out.push(b); } catch (e) { /* refused */ } }
return [out, C.safeRel(".mzlab/a.mzlib"), C.safeRel("a.msp"), C.idOf("a.msp"), C.nameOfId("f:a.msp"), C.isFolderId("f:x"), C.isFolderId("lib1")];""")
    assert r[0] == [] and r[1:4] == [".mzlab/a.mzlib", "a.msp", "f:a.msp"] and r[4] == "a.msp" and r[5] is True and r[6] is False


def test_scan_survives_hostile_sidecars():
    r = run(FAKE + """
const d = mk({
  "a.msp": F("0123456789"), ".mzlab/a.msp.idx.json": F('{"v":1,"bytes":4}'), ".mzlab/a.msp.mzlib": F("MZLB"),                    // no origine
  "b.mgf": F("0123456789"), ".mzlab/b.mgf.idx.json": F('[1,2,3]'),                                                                 // not an object
  "c.mzlib": F("MZLB"), ".mzlab/c.mzlib.pkg.json": F('{"__proto__":{"x":1},"id":5}'),                                             // id of the wrong type
  ".mzlab/z.msp.idx.json": F('{"origine":{"size":"big"},"bytes":4}'), ".mzlab/z.msp.mzlib": F("MZLB"),                           // origine of the wrong type
});
const s = await C.scan(d, null); return { v: s.voci.map(v => [v.nome, v.stato]), proto: ({}).x === undefined };""")
    assert r["proto"] and [v[0] for v in r["v"]] == ["a.msp", "b.mgf", "c.mzlib", "z.msp"]
    assert all(v[1] != "pronta" for v in r["v"]), r
