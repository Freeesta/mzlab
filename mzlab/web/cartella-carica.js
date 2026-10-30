// @ts-strict
"use strict";
// Loading a whole folder (docs/agenti/dati.md): a folder chosen with the «Load a folder» button (input webkitdirectory) or dropped on the start box is read
// recursively and every compatible file in it goes through the usual upload(); the others are only counted. Files keep the folder they came from (`dir`): a
// .wiff and its .wiff.scan are paired only inside the same folder, and two files with the same name in different folders are told apart.
// The pure parts (dir, kind, split, pairKey, uniqueNames, readEntries) have no DOM and are tested in node (tests/test_cartella_carica.py).

// members of a .D run (Agilent, Bruker): dropped without their folder they cannot be read as a run
const LOOSE_D = /^(MSScan\.bin|MSPeak\.bin|MSProfile\.bin|analysis\.(tdf|tsf|baf)(_bin)?)$/i;
const CART_CAP = 16;
const cartCall = (o, m, ...a) => new Promise((ok, ko) => o[m](...a, ok, ko));
// The entries are read in parallel (one IPC per file in the desktop app) with at most CART_CAP calls in flight; each walk returns its own array and the
// arrays are flattened in order, so the result is the same as a sequential walk (uniqueNames depends on that order). One unreadable entry rejects the walk.
async function cartWalkEntries(entries, onFile, HIDDEN, setDir) {
  let live = 0, done = 0, failed = false;
  const waiting = [];
  // a freed slot goes straight to the first waiter, so the count never passes CART_CAP
  const slot = async () => { if (live < CART_CAP) live++; else await new Promise(r => waiting.push(r)); };
  const free = () => { const w = waiting.shift(); if (w) w(); else live--; };
  const limited = async (o, m) => { await slot(); try { if (failed) return []; return await cartCall(o, m); } catch (x) { failed = true; throw x; } finally { free(); } };
  async function walk(e, d) {
    if (e.isFile) { if (HIDDEN(e.name)) return []; const f = setDir(await limited(e, "file"), d); if (onFile) onFile(++done); return [f]; }
    if (!e.isDirectory || HIDDEN(e.name)) return [];
    const rd = e.createReader(), parts = [];
    for (;;) {                                  // readEntries gives the folder in batches until an empty one
      const batch = await limited(rd, "readEntries");
      if (!batch.length) break;
      parts.push(...batch.map(x => walk(x, d + e.name + "/")));
    }
    return (await Promise.all(parts)).flat();
  }
  return (await Promise.all(entries.map(e => walk(e, "")))).flat();
}
const CartCarica = (() => {
  const OK = /\.(mzml|mzxml|cdf|nc|dam|raw|wiff|wiff\.scan)$/i;
  const HIDDEN = n => n[0] === "." || n === "__MACOSX";
  // by-products of the tools (msconvert logs, notes, temp files): skipped silently, never counted or reported
  const BYPRODUCT = /\.(log|txt|tmp|bak|ini|lnk)$|^(thumbs\.db|desktop\.ini)$/i;

  // folder of a file: «a/b/» (empty for a file chosen alone); webkitRelativePath starts with the chosen folder, a dropped entry sets `_dir` itself
  function dir(f) {
    if (typeof f._dir === "string") return f._dir;
    const p = f.webkitRelativePath || "";
    return p.slice(0, p.lastIndexOf("/") + 1);
  }
  function setDir(f, d) { Object.defineProperty(f, "_dir", { value: d, configurable: true }); return f; }
  const pairKey = (f, stem) => dir(f) + "\u0000" + stem.toLowerCase(), hiddenIn = f => dir(f).split("/").some(s => s && HIDDEN(s)) || HIDDEN(f.name);

  // compatible files go on, the others are counted
  function split(files) {
    const ok = [], ignored = [];
    for (const f of files) if (!BYPRODUCT.test(f.name)) (OK.test(f.name) && !hiddenIn(f) ? ok : ignored).push(f);
    return { ok, ignored };
  }

  // two mzML/.dam with the same name in different folders: the second one gets the name of its folder («name (folder).mzML»); .wiff files keep their name (pairing)
  function uniqueNames(files, taken = []) {
    const seen = new Set(taken.map(n => n.toLowerCase())), out = [];
    for (const f of files) {
      let name = f.name;
      if (!/\.(wiff|wiff\.scan)$/i.test(name)) {
        const m = /^(.*?)(\.[^.]+)$/.exec(name) || [null, name, ""], last = dir(f).split("/").filter(Boolean).pop() || "";
        for (let i = 1; seen.has(name.toLowerCase()); i++) name = `${m[1]} (${last || i}${last && i > 1 ? " " + i : ""})${m[2]}`;
        seen.add(name.toLowerCase());
      }
      out.push(name === f.name ? f : setDir(new File([f], name, { lastModified: f.lastModified }), dir(f)));
    }
    return out;
  }

  // ---- reading (DataTransferItem.webkitGetAsEntry): the entries must be taken in the drop event itself, before any await
  function entriesOf(dt) {
    const es = (dt?.items ? [...dt.items] : []).map(i => (i.kind === "file" && i.webkitGetAsEntry ? i.webkitGetAsEntry() : null)).filter(Boolean);
    return es.some(e => e.isDirectory) ? es : null;
  }
  const readEntries = (entries, onFile) => cartWalkEntries(entries, onFile, HIDDEN, setDir);
  const rootName = (entries, files) => (entries && entries.length === 1 && entries[0].isDirectory ? entries[0].name : (files[0] && dir(files[0]).split("/")[0]) || "");
  // ---- page part (only where there is a DOM)
  function note(text) { const n = document.getElementById("cartinfo"); if (!n) return; n.textContent = text; n.hidden = !text; }
  async function load(files, name) {
    const wr = WatersRaw.group(files), ag = AgilentD.group(wr.rest), T = (k, p) => I18N.t(k, p), mm = AgilentD.groupMethods(ag.rest), { ok, ignored } = split(mm.rest);     // Agilent .D folders are converted by agilent-d.js, Waters .raw folders by waters-raw.js, .m method folders are held there
    const kept = mm.methods.filter(m => m.files.length), meth = kept.map(m => m.name); AgilentD.held.push(...kept);
    const ig = (ignored.length ? " " + T(ignored.length === 1 ? "cart.ignored.one" : "cart.ignored", { n: ignored.length }) : "") + (meth.length ? " " + T(meth.length === 1 ? "cart.method.one" : "cart.method", { n: meth.length, name: meth.join(", ") }) : "");
    const n = ok.length + ag.runs.length + wr.runs.length; if (!n) { note(meth.length ? ig.trim() : T("cart.none", { name }) + ig); return; }
    const msg = T(n === 1 ? "cart.found.one" : "cart.found", { n, name }) + ig; note(msg);
    if (ag.runs.length && typeof AVVIO !== "undefined" && !(await AVVIO.pronto())) return;
    await upload([...uniqueNames(ok, (ST.files || []).map(x => x.name)), ...await AgilentD.toMzml(ag.runs), ...WatersRaw.toMzml(wr.runs)], null);
    if (!ig && document.getElementById("cartinfo")?.textContent === msg) note("");      // loaded: «n files to load» is done; the summary of the ignored files and later notices stay
  }
  function fromDrop(e, /** @type {(() => void) | undefined} */ done) {                // true when the drop was a folder (read and loaded here); the entries are taken before any await
    const es = entriesOf(e.dataTransfer);
    if (!es) { const n = [...(e.dataTransfer?.files || [])].filter(f => LOOSE_D.test(f.name)).length; if (n) note(I18N.t("cart.loose", { n })); return false; }
    note(I18N.t("cart.reading", { n: 0 }));
    // a failure of the walk or of the load shows its message; done() runs either way
    readEntries(es, n => { if (n % 50 === 0) note(I18N.t("cart.reading", { n })); }).then(files => load(files, rootName(es, files))).catch(x => note(String(x.message || x))).then(() => { if (done) done(); });
    return true;
  }
  async function fromInput(list) { const files = [...list]; await load(files, rootName(null, files)); }
  return { dir, setDir, pairKey, split, byproduct: n => BYPRODUCT.test(n), uniqueNames, entriesOf, readEntries, fromDrop, fromInput, note, load };
})();
if (typeof module !== "undefined") module.exports = CartCarica;
