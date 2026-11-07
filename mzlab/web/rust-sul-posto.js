// @ts-check
// A plain mzML of 50 MB or more with no room for its copy (ensureRoom refuses, or the browser stops the copy halfway) is read in place: the user's File is
// the source (lettura-sync.js reads it through FileReaderSync), only its index goes to OPFS, and the marker idx/<name>.sul tells the next start that the
// File went with the tab (Chromium keeps no File over a reload): the boot removes its index and says calmly to load it again. Used by rust-worker.js lrPut.

/** @param {{ ARCH: any, tilesDrop: (name: string, keep: null) => Promise<unknown>, post: (m: any) => void, safe: (f: () => any) => Promise<any> }} h */
export function createSulPosto({ ARCH, tilesDrop, post, safe }) {
  const SUL = new Set();
  const quotaErr = (/** @type {any} */ e) => /quota/i.test(typeof e === "string" ? e : String(e && (e.name || e.message)));
  // a small file is copied (it survives a reload); a .gz, a maker's file or a file warmed ahead has its own path
  const can = (/** @type {any} */ m, /** @type {number} */ size) => !m.nat && !m.gz && !m.warm && /\.mzml$/i.test(m.name) && m.blob instanceof File && size >= ARCH.BIG;
  async function take(/** @type {any} */ m, /** @type {number} */ size) {
    await ARCH.ensureRoom(size * (ARCH.ROOM - 1) / ARCH.ROOM, m.name);      // the room of the index alone (ensureRoom asks bytes x ROOM)
    await ARCH.remove("files", m.name);                                     // an older copy of the same name would be read in place of the File
    await ARCH.put("idx", m.name + ".sul", new Uint8Array(0));
    SUL.add(m.name);
    post({ op: "notice", key: "load.sulPosto", params: { name: m.name } });
    return m.blob;
  }
  return {
    has: (/** @type {string} */ n) => SUL.has(n),
    names: () => [...SUL],
    forget: (/** @type {string} */ n) => { SUL.delete(n); },
    clear: () => SUL.clear(),
    /** the room for the copy; true when there is none and the file can be read in place */
    async room(/** @type {any} */ m, /** @type {number} */ size) {
      try { await ARCH.ensureRoom(size, m.name); return false; } catch (e) { if (!(can(m, size) && quotaErr(e))) throw e; return true; }
    },
    /** the File of the program: the copy made by `put`, or the user's File when there is no room or the copy is refused halfway (its part is removed) */
    async copy(/** @type {any} */ m, /** @type {number} */ size, /** @type {boolean} */ inPlace, /** @type {() => Promise<File>} */ put) {
      if (!inPlace && SUL.delete(m.name)) await ARCH.remove("idx", m.name + ".sul");      // a copy now replaces the file read in place
      if (inPlace) return take(m, size);
      try { return await put(); } catch (e) { if (!(can(m, size) && quotaErr(e))) throw e; return take(m, size); }
    },
    /** at start: a file read in place in a closed tab has no File any more; its index and marker go, and a file that had loaded is named */
    async boot() {
      for (const e of await safe(() => ARCH.list("idx")) || []) {
        const n = e.name.replace(/\.sul$/, ""); if (n === e.name || SUL.has(n)) continue;
        if (!(await safe(() => ARCH.get("files", n)))) {
          const was = await safe(() => ARCH.get("idx", n + ".mzidx"));
          for (const x of [".mzidx", ".cen.mzidx", ".cen.key"]) await safe(() => ARCH.remove("idx", n + x));
          await safe(() => tilesDrop(n, null));
          if (was) post({ op: "notice", key: "load.sulRiapri", params: { name: n } });
        }
        await safe(() => ARCH.remove("idx", e.name));
      }
    },
  };
}
