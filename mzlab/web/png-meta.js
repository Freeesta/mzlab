// @ts-strict
// Metadata for exported PNGs: tEXt chunks (Latin-1) written right after the IHDR. Loaded on the first PNG export only,
// by the Dati tab plots (pannelli.js) and by Disegno (draw.js), so the startup scripts stay small.
const CRCT = (() => { const t = new Uint32Array(256); for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } return t; })();
const crc32 = (/** @type {Uint8Array} */ u) => { let c = 0xffffffff; for (const b of u) c = CRCT[(c ^ b) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
export async function pngWithMeta(/** @type {Blob} */ blob, /** @type {string[][]} */ fields) {
  const src = new Uint8Array(await blob.arrayBuffer()), l1 = (/** @type {unknown} */ t) => Uint8Array.from(String(t).replace(/[\u2013\u2014]/g, "-").replace(/[^\x20-\x7e\xa0-\xff]/g, "?"), c => c.charCodeAt(0));
  const parts = [src.slice(0, 33)];
  fields.forEach(([k, v]) => {
    const kb = l1(k), vb = l1(v), body = new Uint8Array(4 + kb.length + 1 + vb.length);
    body.set([116, 69, 88, 116], 0); body.set(kb, 4); body.set(vb, 5 + kb.length);  // "tEXt" + keyword + 0 + text
    const ch = new Uint8Array(12 + body.length - 4), dv = new DataView(ch.buffer);
    dv.setUint32(0, body.length - 4); ch.set(body, 4); dv.setUint32(ch.length - 4, crc32(body)); parts.push(ch);
  });
  parts.push(src.subarray(33)); return new Blob(parts, { type: "image/png" });
}
