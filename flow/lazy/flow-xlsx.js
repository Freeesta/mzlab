// @ts-check
// Flow (administrator mode): a minimal reader of .xlsx files for the metadata table of the Import data card. An xlsx is a zip of XML parts: the central directory gives the parts,
// DecompressionStream inflates them, and the first sheet is read with its shared strings (text and number cells only). It returns the sheet as tab-separated text.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const dec = new TextDecoder();

  /** the parts of a zip by name, each as a function that returns its bytes (stored or deflated) @param {Uint8Array} u */
  const parts = u => {
    const v = new DataView(u.buffer, u.byteOffset, u.byteLength);
    let end = -1;
    for (let i = u.length - 22; i >= Math.max(0, u.length - 65557); i--) if (v.getUint32(i, true) === 0x06054b50) { end = i; break; }
    if (end < 0) throw new Error("This is not an xlsx file (no zip directory).");
    const n = v.getUint16(end + 10, true), out = /** @type {Map<string, () => Promise<Uint8Array>>} */ (new Map());
    let p = v.getUint32(end + 16, true);
    for (let k = 0; k < n && v.getUint32(p, true) === 0x02014b50; k++) {
      const method = v.getUint16(p + 10, true), size = v.getUint32(p + 20, true), nameLen = v.getUint16(p + 28, true), extra = v.getUint16(p + 30, true), note = v.getUint16(p + 32, true), at = v.getUint32(p + 42, true);
      const name = dec.decode(u.subarray(p + 46, p + 46 + nameLen)), body = at + 30 + v.getUint16(at + 26, true) + v.getUint16(at + 28, true);
      out.set(name, async () => {
        const raw = u.subarray(body, body + size);
        if (method === 0) return raw;
        if (method !== 8) throw new Error("The xlsx uses a compression this reader does not know.");
        return new Uint8Array(await new Response(new Blob([raw]).stream().pipeThrough(new DecompressionStream("deflate-raw"))).arrayBuffer());
      });
      p += 46 + nameLen + extra + note;
    }
    return out;
  };

  /** the text of an XML fragment: entities decoded @param {string} s */
  const unesc = s => s.replace(/&(#x[0-9a-f]+|#\d+|amp|lt|gt|quot|apos);/gi, (_, e) => {
    const t = e.toLowerCase();
    return t === "amp" ? "&" : t === "lt" ? "<" : t === "gt" ? ">" : t === "quot" ? '"' : t === "apos" ? "'" : String.fromCodePoint(t[1] === "x" ? parseInt(t.slice(2), 16) : Number(t.slice(1)));
  });
  /** the column number of a reference like «AB12» (A = 0) @param {string} ref */
  const col = ref => [...ref.replace(/\d+$/, "")].reduce((a, c) => a * 26 + c.charCodeAt(0) - 64, 0) - 1;

  /** the first sheet of an xlsx as rows of text; text and number cells, others empty @param {ArrayBuffer} buf @returns {Promise<string[][]>} */
  F.readXlsx = async buf => {
    const z = parts(new Uint8Array(buf)), get = async (/** @type {string} */ n) => { const f = z.get(n); return f ? dec.decode(await f()) : ""; };
    const names = [...z.keys()].filter(n => /^xl\/worksheets\/sheet\d+\.xml$/.test(n)).sort((a, b) => parseInt(a.replace(/\D/g, ""), 10) - parseInt(b.replace(/\D/g, ""), 10));
    if (!names.length) throw new Error("The xlsx has no sheet.");
    const shared = [...(await get("xl/sharedStrings.xml")).matchAll(/<si\b[^>]*>([\s\S]*?)<\/si>/g)].map(m => unesc([...m[1].matchAll(/<t\b[^>]*>([\s\S]*?)<\/t>/g)].map(t => t[1]).join("")));
    const rows = /** @type {string[][]} */ ([]);
    for (const r of (await get(names[0])).matchAll(/<row\b[^>]*>([\s\S]*?)<\/row>/g)) {
      const row = /** @type {string[]} */ ([]);
      for (const c of r[1].matchAll(/<c\b([^>]*?)(?:\/>|>([\s\S]*?)<\/c>)/g)) {
        const attr = c[1], ref = (/\br="([A-Z]+\d+)"/.exec(attr) || [])[1], type = (/\bt="(\w+)"/.exec(attr) || [])[1], inner = c[2] || "";
        const v = (/<v>([\s\S]*?)<\/v>/.exec(inner) || [])[1];
        const text = type === "s" ? shared[Number(v)] : type === "inlineStr" ? unesc([...inner.matchAll(/<t\b[^>]*>([\s\S]*?)<\/t>/g)].map(t => t[1]).join("")) : type === "str" || type === undefined || type === "n" ? (v === undefined ? "" : unesc(v)) : "";
        row[ref ? col(ref) : row.length] = text === undefined ? "" : text;
      }
      rows.push([...row].map(x => x ?? ""));
    }
    return rows;
  };

  /** the first sheet of an xlsx as tab-separated text, for F.parseMetadata (tabs and line breaks inside a cell become spaces) @param {ArrayBuffer} buf */
  F.xlsxToText = async buf => (await F.readXlsx(buf)).map(r => r.map(x => x.replace(/[\t\r\n]+/g, " ")).join("\t")).join("\n");
})();
