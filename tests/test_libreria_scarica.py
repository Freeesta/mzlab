"""The download of the library packages in the page (mzlab/web/libreria-scarica.js) against an in-memory folder and a fake `fetch`, run by node:
pieces checked one by one, a wrong piece fetched again, resume, the whole file checked, the old file kept until the new one is verified, the catalog checked."""
import json
import subprocess
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"


def run(code):
    full = (f"globalThis.CARTELLA=require({str(WEB / 'libreria-cartella.js')!r});const S=require({str(WEB / 'libreria-scarica.js')!r});"
            "const crypto=require('crypto');const hex=u=>crypto.createHash('sha256').update(u).digest('hex');"
            f"(async()=>{{{code}}})().then(r=>console.log(JSON.stringify(r)))")
    return json.loads(subprocess.run(["node", "-e", full], check=True, capture_output=True, encoding="utf-8").stdout)


# a package of three pieces whose whole is a valid-looking .mzlib (magic, version 1, minimum reader 1)
SETUP = """
const mk = (fill, ver) => { const w = new Uint8Array(2500).fill(fill); w.set([0x4d, 0x5a, 0x4c, 0x42, 1, 0, 1, 0], 0); const ps = [0, 1, 2].map(i => w.slice(i * 1000, Math.min(2500, i * 1000 + 1000)));
  return { w, ps, pkg: { id: "massbank-pos", versione: ver, sha256: hex(w), dimensione: 2500, lettore_min: 1, spettri: 7, picchi: 70, polarita: 1, pezzi: ps.map((p, i) => ({ file: "massbank-pos/00" + i + ".bin", dimensione: p.length, sha256: hex(p) })) } }; };
const dirMem = () => { const files = {}, d = { files,
  async list(sub = "") { return Object.entries(files).filter(([k]) => (sub ? k.startsWith(sub + "/") && !k.slice(sub.length + 1).includes("/") : !k.includes("/"))).map(([k, v]) => ({ name: k.split("/").pop(), size: v.length, mtime: 1 })); },
  async file(rel) { if (!files[rel]) throw new Error("none " + rel); const b = new Blob([files[rel]]); b.lastModified = 5; return b; },
  async write(rel, data) { files[rel] = data instanceof Blob ? new Uint8Array(await data.arrayBuffer()) : typeof data === "string" ? new TextEncoder().encode(data) : data; },
  async remove(rel) { delete files[rel]; }, async replace(a, b) { files[b] = files[a]; delete files[a]; } }; return d; };
const server = (pk, behave) => { const hits = {}; const fetchFn = async (url) => { const k = url.replace("http://x/", ""); hits[k] = (hits[k] || 0) + 1;
  const b = behave ? behave(k, hits[k]) : "ok"; if (b === "drop") throw new TypeError("network"); if (typeof b === "number") return { ok: false, status: b };
  const i = pk.pkg.pezzi.findIndex(p => p.file === k); let body = pk.ps[i]; if (b === "corrupt") { body = body.slice(); body[5] ^= 255; }
  return { ok: true, status: 200, arrayBuffer: async () => body.buffer.slice(body.byteOffset, body.byteOffset + body.length) }; }; return { hits, fetchFn }; };
"""


def test_a_package_is_installed_with_its_sidecar():
    r = run(SETUP + """
const pk = mk(7, "2026.03"), dir = dirMem(), sv = server(pk), ev = [];
await S.downloadPackage({ dir, pkg: pk.pkg, base: "http://x/", onProgress: e => ev.push(e.piece), fetchFn: sv.fetchFn });
const side = JSON.parse(new TextDecoder().decode(dir.files[".mzlab/massbank-pos.mzlib.pkg.json"]));
return { same: hex(dir.files["massbank-pos.mzlib"]) === pk.pkg.sha256, ev, side, keys: Object.keys(dir.files).sort() };""")
    assert r["same"] and r["ev"] == [1, 2, 3] and r["keys"] == [".mzlab/massbank-pos.mzlib.pkg.json", "massbank-pos.mzlib"]
    assert r["side"]["versione"] == "2026.03" and r["side"]["size"] == 2500 and r["side"]["info"] == {"n": 7, "pos": 7, "neg": 0, "dropped": 0, "peaks": 70, "date": 5}


def test_a_corrupt_piece_is_fetched_again_and_one_that_stays_corrupt_fails():
    r = run(SETUP + """
const pk = mk(7, "1"), dir = dirMem(), sv = server(pk, (k, n) => (k.endsWith("001.bin") && n === 1 ? "corrupt" : "ok"));
await S.downloadPackage({ dir, pkg: pk.pkg, base: "http://x/", fetchFn: sv.fetchFn });
const again = sv.hits["massbank-pos/001.bin"];
const dir2 = dirMem(), sv2 = server(pk, k => (k.endsWith("002.bin") ? "corrupt" : "ok")); let e2 = null;
try { await S.downloadPackage({ dir: dir2, pkg: pk.pkg, base: "http://x/", fetchFn: sv2.fetchFn }); } catch (e) { e2 = [e.key, e.params.what]; }
return { again, installed: !!dir.files["massbank-pos.mzlib"], e2, installed2: !!dir2.files["massbank-pos.mzlib"], tries: sv2.hits["massbank-pos/002.bin"] };""")
    assert r["again"] == 2 and r["installed"] and r["e2"] == ["lib.dl.err.hash", "massbank-pos/002.bin"] and not r["installed2"] and r["tries"] == 2


def test_an_interruption_is_resumed_from_the_first_missing_piece():
    r = run(SETUP + """
const pk = mk(7, "1"), dir = dirMem(); let up = false;
const sv = server(pk, k => (k.endsWith("002.bin") && !up ? "drop" : "ok")); let e1 = null;
try { await S.downloadPackage({ dir, pkg: pk.pkg, base: "http://x/", fetchFn: sv.fetchFn }); } catch (e) { e1 = e.key; }
const kept = Object.keys(dir.files).sort(); up = true;
await S.downloadPackage({ dir, pkg: pk.pkg, base: "http://x/", fetchFn: sv.fetchFn });
return { e1, kept, hits: sv.hits, installed: hex(dir.files["massbank-pos.mzlib"]) === pk.pkg.sha256, left: Object.keys(dir.files).sort() };""")
    assert r["e1"] == "lib.dl.err.offline" and r["kept"] == [".mzlab/massbank-pos.000.piece", ".mzlab/massbank-pos.001.piece"]
    assert r["hits"]["massbank-pos/000.bin"] == 1 and r["hits"]["massbank-pos/001.bin"] == 1 and r["hits"]["massbank-pos/002.bin"] == 2
    assert r["installed"] and r["left"] == [".mzlab/massbank-pos.mzlib.pkg.json", "massbank-pos.mzlib"]


def test_a_damaged_piece_already_in_the_folder_is_fetched_again_and_a_status_is_an_error():
    r = run(SETUP + """
const pk = mk(7, "1"), dir = dirMem(), sv = server(pk);
dir.files[".mzlab/massbank-pos.000.piece"] = new Uint8Array(1000);                   // right size, wrong bytes
await S.downloadPackage({ dir, pkg: pk.pkg, base: "http://x/", fetchFn: sv.fetchFn });
const sv3 = server(pk, () => 503); let e3 = null; try { await S.downloadPackage({ dir: dirMem(), pkg: pk.pkg, base: "http://x/", fetchFn: sv3.fetchFn }); } catch (e) { e3 = [e.key, e.params.status]; }
return { hits: sv.hits["massbank-pos/000.bin"], e3 };""")
    assert r["hits"] == 1 and r["e3"] == ["lib.dl.err.http", 503]


def test_the_old_file_stays_until_the_new_one_is_verified():
    r = run(SETUP + """
const dir = dirMem(), old = mk(1, "1"), nw = mk(9, "2");
await S.downloadPackage({ dir, pkg: old.pkg, base: "http://x/", fetchFn: server(old).fetchFn });
const before = hex(dir.files["massbank-pos.mzlib"]);
const wrong = { ...nw.pkg, sha256: "0".repeat(64) }; let e = null;
try { await S.downloadPackage({ dir, pkg: wrong, base: "http://x/", fetchFn: server(nw).fetchFn }); } catch (x) { e = x.key; }
const kept = hex(dir.files["massbank-pos.mzlib"]) === before, leftover = Object.keys(dir.files).filter(k => /\\.(new|piece)$/.test(k));
let replaced = 0; await S.downloadPackage({ dir, pkg: nw.pkg, base: "http://x/", fetchFn: server(nw).fetchFn, beforeReplace: async () => { replaced++; } });
return { e, kept, leftover, replaced, now: hex(dir.files["massbank-pos.mzlib"]) === nw.pkg.sha256, ver: JSON.parse(new TextDecoder().decode(dir.files[".mzlab/massbank-pos.mzlib.pkg.json"])).versione };""")
    assert r["e"] == "lib.dl.err.hash" and r["kept"] and r["leftover"] == [] and r["replaced"] == 1 and r["now"] and r["ver"] == "2"


def test_a_cancel_stops_between_pieces_and_a_newer_reader_is_refused():
    r = run(SETUP + """
const pk = mk(7, "1"), dir = dirMem(), ac = new AbortController(); let e = null;
try { await S.downloadPackage({ dir, pkg: pk.pkg, base: "http://x/", signal: ac.signal, fetchFn: server(pk).fetchFn, onProgress: p => { if (p.piece === 1) ac.abort(); } }); } catch (x) { e = x.key; }
let v = null; try { await S.downloadPackage({ dir: dirMem(), pkg: { ...pk.pkg, lettore_min: 9 }, base: "http://x/", fetchFn: server(pk).fetchFn }); } catch (x) { v = x.key; }
return { e, installed: !!dir.files["massbank-pos.mzlib"], pieces: Object.keys(dir.files).length, v };""")
    assert r["e"] == "lib.dl.err.cancelled" and not r["installed"] and r["pieces"] == 1 and r["v"] == "lib.dl.err.version"


def test_the_catalog_is_checked_and_the_state_of_an_entry():
    r = run(SETUP + """
const pk = mk(7, "2"), good = JSON.stringify({ formato: "librerie/1", librerie: [pk.pkg], esterne: [{ id: "gnps-cc0", url: "https://external.gnps2.org/x.mgf" }] });
const bad = [{ formato: "librerie/9", librerie: [] }, { formato: "librerie/1", librerie: [{ ...pk.pkg, id: "../x" }] }, { formato: "librerie/1", librerie: [{ ...pk.pkg, dimensione: 3 }] },
  { formato: "librerie/1", librerie: [{ ...pk.pkg, pezzi: [{ ...pk.pkg.pezzi[0], file: "b/../../x" }, ...pk.pkg.pezzi.slice(1)] }] }, { formato: "librerie/1", librerie: [], esterne: [{ id: "a", url: "http://insecure/x" }] }].map(c => { try { S.parseCatalog(JSON.stringify(c)); return null; } catch (e) { return e.key; } });
let notJson = null; try { S.parseCatalog("<html>"); } catch (e) { notJson = e.key; }
const v = (stato, versione) => [{ tipo: "pacchetto", nome: "massbank-pos.mzlib", stato, versione }];
return { ok: S.parseCatalog(good).librerie.length, bad, notJson, st: [S.stateOf(pk.pkg, []), S.stateOf(pk.pkg, v("pronta", "2")), S.stateOf(pk.pkg, v("pronta", "1")), S.stateOf(pk.pkg, v("aggiornamento", "1")), S.stateOf(pk.pkg, v("danneggiata", "2"))] };""")
    assert r["ok"] == 1 and set(r["bad"]) == {"lib.dl.err.catalog"} and len(r["bad"]) == 5 and r["notJson"] == "lib.dl.err.catalog"
    assert r["st"] == ["new", "installed", "update", "update", "damaged"]
