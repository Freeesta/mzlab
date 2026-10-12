"use strict";
// Loading a whole folder (docs/agenti/dati.md): a folder chosen with the «Load a folder» button (input webkitdirectory) or dropped on the start box is read
// recursively and every compatible file in it goes through the usual upload(); the others are only counted. Files keep the folder they came from (`dir`): a
// .wiff and its .wiff.scan are paired only inside the same folder, and two files with the same name in different folders are told apart.
// The pure parts (dir, kind, split, pairKey, uniqueNames, readEntries) have no DOM and are tested in node (tests/test_cartella_carica.py).

const CartCarica = (() => {
  const OK = /\.(mzml|dam|raw|wiff|wiff\.scan)$/i;
  const HIDDEN = n => n[0] === "." || n === "__MACOSX";

  // folder of a file: «a/b/» (empty for a file chosen alone); webkitRelativePath starts with the chosen folder, a dropped entry sets `_dir` itself
  function dir(f) {
    if (typeof f._dir === "string") return f._dir;
    const p = f.webkitRelativePath || "";
    return p.slice(0, p.lastIndexOf("/") + 1);
  }
  function setDir(f, d) { Object.defineProperty(f, "_dir", { value: d, configurable: true }); return f; }
  const pairKey = (f, stem) => dir(f) + "\u0000" + stem.toLowerCase();
  const hiddenIn = f => dir(f).split("/").some(s => s && HIDDEN(s)) || HIDDEN(f.name);

  // compatible files go on, the others are counted
  function split(files) {
    const ok = [], ignored = [];
    for (const f of files) (OK.test(f.name) && !hiddenIn(f) ? ok : ignored).push(f);
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
    const items = dt && dt.items ? [...dt.items] : [];
    const es = items.map(i => (i.kind === "file" && i.webkitGetAsEntry ? i.webkitGetAsEntry() : null)).filter(Boolean);
    return es.some(e => e.isDirectory) ? es : null;
  }
  const call = (o, m, ...a) => new Promise((ok, ko) => o[m](...a, ok, ko));
  async function readEntries(entries, onFile) {
    const out = [];
    async function walk(e, d) {
      if (e.isFile) { if (HIDDEN(e.name)) return; const f = setDir(await call(e, "file"), d); out.push(f); if (onFile) onFile(out.length); return; }
      if (!e.isDirectory || HIDDEN(e.name)) return;
      const rd = e.createReader();
      for (;;) {                                  // readEntries gives the folder in batches until an empty one
        const batch = await call(rd, "readEntries");
        if (!batch.length) break;
        for (const x of batch) await walk(x, d + e.name + "/");
      }
    }
    for (const e of entries) await walk(e, "");
    return out;
  }
  const rootName = (entries, files) => (entries && entries.length === 1 && entries[0].isDirectory ? entries[0].name : (files[0] && dir(files[0]).split("/")[0]) || "");
  // ---- page part (only where there is a DOM)
  function note(text) { const n = document.getElementById("cartinfo"); if (!n) return; n.textContent = text; n.hidden = !text; }
  async function load(files, name) {
    const ag = AgilentD.group(files), T = (k, p) => I18N.t(k, p), { ok, ignored } = split(ag.rest);     // Agilent .D folders are converted by agilent-d.js
    const ig = ignored.length ? " " + T(ignored.length === 1 ? "cart.ignored.one" : "cart.ignored", { n: ignored.length }) : "";
    const n = ok.length + ag.runs.length;
    if (!n) { note(T("cart.none", { name }) + ig); return; }
    note(T(n === 1 ? "cart.found.one" : "cart.found", { n, name }) + ig);
    await upload([...uniqueNames(ok, (ST.files || []).map(x => x.name)), ...await AgilentD.toMzml(ag.runs)], null);
  }
  function fromDrop(e) {                // true when the drop was a folder (read and loaded here); the entries are taken before any await
    const es = entriesOf(e.dataTransfer);
    if (!es) return false;
    note(I18N.t("cart.reading", { n: 0 }));
    readEntries(es, n => { if (n % 50 === 0) note(I18N.t("cart.reading", { n })); }).then(files => load(files, rootName(es, files)), x => note(String(x.message || x)));
    return true;
  }
  async function fromInput(list) { const files = [...list]; await load(files, rootName(null, files)); }
  return { dir, setDir, pairKey, split, uniqueNames, entriesOf, readEntries, fromDrop, fromInput, note, load };
})();
if (typeof module !== "undefined") module.exports = CartCarica;
