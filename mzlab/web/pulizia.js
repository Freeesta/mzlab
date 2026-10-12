// Automatic cleaning of the archive (docs/agenti/memoria.md), at start and when idle. It removes only what can be made again or is left over, never a source file:
//  - the `.part` files older than an hour (a write that was cut);
//  - an index (`idx/`) whose source is no longer in `files/` and that has not been used for a week (an orphan). The files of the pipeline (`feat/`) are not
//    orphaned by this: they are cleaned only of their `.part`.
// io: { list(kind) -> [{ name, size, lastModified }], remove(kind, name), manifest() -> [{ path, usato, creato }] }, now in ms. Returns the paths removed.
export const HOUR = 3600e3, WEEK = 7 * 24 * HOUR;

export async function limpia(io, now = Date.now()) {
  const gone = [], used = new Map((await io.manifest()).map(r => [r.path, r.usato || r.creato || 0]));
  const sources = new Set((await io.list("files")).map(e => e.name));
  for (const kind of ["files", "idx", "feat"]) {
    for (const e of await io.list(kind)) {
      const path = kind + "/" + e.name;
      if (e.name.endsWith(".part")) { if (now - e.lastModified > HOUR) { await io.remove(kind, e.name); gone.push(path); } continue; }
      if (kind !== "idx") continue;
      const stem = e.name.replace(/\.mzidx$/, "");
      if (!sources.has(stem) && now - Math.max(e.lastModified, used.get(path) || 0) > WEEK) { await io.remove(kind, e.name); gone.push(path); }
    }
  }
  return gone;
}
