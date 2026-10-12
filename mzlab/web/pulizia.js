// Automatic cleaning of the archive (docs/agenti/memoria.md), at start and when idle. It removes only what can be made again or is left over, never a source file:
//  - the `.part` files older than an hour (a write that was cut);
//  - an index (`idx/`) whose source is no longer in `files/` and that has not been used for a week (an orphan). The files of the pipeline (`feat/`) are not
//    orphaned by this: they are cleaned only of their `.part`.
//  - a derived file (`idx/`, `feat/`) written in a format older than the current one (manifesto.js: it is made again from the source); a newer or unknown format stays.
// The derived files whose manifest row does not know the version yet get it from the file (`annota`).
// io: { list(kind) -> [{ name, size, lastModified }], remove(kind, name), manifest() -> [{ path, usato, creato, versione_formato }], info?(kind, name) -> {versione_formato, ...}, annota?(kind, entry, info) }, now in ms. Returns the paths removed.
import { vecchio } from "./manifesto.js";
export const HOUR = 3600e3, WEEK = 7 * 24 * HOUR;

export async function limpia(io, now = Date.now()) {
  const rows = new Map((await io.manifest()).map(r => [r.path, r])), gone = [], used = new Map([...rows].map(([p, r]) => [p, r.usato || r.creato || 0]));
  const sources = new Set((await io.list("files")).map(e => e.name));
  for (const kind of ["files", "idx", "feat"]) {
    for (const e of await io.list(kind)) {
      const path = kind + "/" + e.name;
      if (e.name.endsWith(".part")) { if (now - e.lastModified > HOUR) { await io.remove(kind, e.name); gone.push(path); } continue; }
      if (kind !== "files" && io.info) {
        const info = await io.info(kind, e.name);
        if (vecchio(kind, info)) { await io.remove(kind, e.name); gone.push(path); continue; }
        const row = rows.get(path);
        if (info && io.annota && (!row || row.versione_formato == null)) await io.annota(kind, e, info);
      }
      if (kind !== "idx") continue;
      const stem = e.name.replace(/\.mzidx$/, "");
      if (!sources.has(stem) && now - Math.max(e.lastModified, used.get(path) || 0) > WEEK) { await io.remove(kind, e.name); gone.push(path); }
    }
  }
  return gone;
}
