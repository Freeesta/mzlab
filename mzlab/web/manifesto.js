// @ts-strict
// What the derived files of the archive say about themselves (docs/agenti/memoria.md): the format version, the crate that wrote them and the SHA-256 of the source.
// The values are read from the file itself (the meta of `.mzidx` and of `.mzfeatf`/`.mzfeat`), so every producer is covered, also the one that writes OPFS without the archive.

/** the format versions that the readers of this page understand: `idx` = version of the `.mzidx` header, `feat` = `feat_version` of the meta (crates/mzlab-feat/src/lib.rs) */
export const CORRENTI = { idx: 1, feat: 3 };

/** the highest versions that the readers of this page understand (the idx header knows 1 = mzML and 2 = vendor file); a file above them was written by a newer program and is left as it is */
export const MASSIMI = { idx: 2, feat: 3 };

const MAX_META = 16 << 20;
const text = (b, o, n) => String.fromCharCode(...new Uint8Array(b, o, n));

async function metaOf(blob, off, len) {
  if (!(len > 0) || len > MAX_META || off + len > blob.size) return null;
  try { return JSON.parse(await blob.slice(off, off + len).text()); } catch (_) { return null; }
}

/** `{ versione_formato, crate, sorgente_sha256 }` of an index (`idx`) or a feature file (`feat`); null when the file is not one of ours or the meta cannot be read */
export async function infoDi(kind, blob) {
  if (!blob || blob.size < 32 || (kind !== "idx" && kind !== "feat")) return null;
  const head = await blob.slice(0, 32).arrayBuffer(), v = new DataView(head);
  if (kind === "idx") {
    const mn = /^MZIDX(\d{3})$/.exec(text(head, 0, 8));
    // a newer magic: the layout is unknown
    if (mn && Number(mn[1]) > 1) return { versione_formato: MASSIMI.idx + Number(mn[1]) - 1, crate: null, sorgente_sha256: null };
    if (!mn) return null;
    const m = await metaOf(blob, Number(v.getBigUint64(16, true)), Number(v.getBigUint64(24, true)));
    return { versione_formato: v.getUint32(8, true), crate: (m && m.crate) || null, sorgente_sha256: (m && m.source && m.source.sha256) || null };
  }
  if (text(head, 0, 8) !== "MZFEAT01") return null;
  const m = await metaOf(blob, 16, Number(v.getBigUint64(8, true)));
  if (!m) return null;
  const one = Array.isArray(m.inputs) && m.inputs.length === 1 ? m.inputs[0].sha256 : null;
  return { versione_formato: Number.isInteger(m.feat_version) ? m.feat_version : null, crate: m.crate || null, sorgente_sha256: one || null };
}

/** true when the file is of a format older than the one this page writes (it is made again from the source); an unknown version is kept */
export const vecchio = (kind, info) => !!info && Number.isInteger(info.versione_formato) && info.versione_formato < CORRENTI[kind];

/** true when the file was written by a newer program than this page (version above `MASSIMI`): it opens read-only with `error.newerFormat` and is never written over */
export const nuovo = (kind, info) => !!info && Number.isInteger(info.versione_formato) && info.versione_formato > MASSIMI[kind];
