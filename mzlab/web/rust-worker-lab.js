// The operations of rust-worker.js on the methods (.dam) and on how an item was acquired: method, analog traces, PDA, delay, gradient.
// The .dam files live in OPFS with the mzML; the engine holds them parsed, loaded again (oldest first) whenever the set in OPFS changes.
import { gradient_make } from "./wasm/mzlab_wasm.js";
import { createMetodoOps } from "./rust-metodo.js";

export function createLabOps({ engine, ARCH, fileOf, lrOpen, slotOf, readerFor, jerr }) {
  let sig = null, n = 0;
  const seq = new Map();                    // name -> order of upload in this session: the files of the same millisecond keep it
  async function sync() {
    const list = (await ARCH.list("files")).filter(e => /\.dam$/i.test(e.name) && !/\.part$/.test(e.name)).sort((a, b) => a.lastModified - b.lastModified || (seq.get(a.name) || 0) - (seq.get(b.name) || 0) || (a.name < b.name ? -1 : 1));
    const now = list.map(e => `${e.name}:${e.size}:${e.lastModified}`).join("|");
    if (now === sig) return;
    const e = engine();
    e.lab_clear();
    for (const x of list) { const f = await ARCH.get("files", x.name); if (f) e.lab_put(x.name, new Uint8Array(await f.arrayBuffer())); }
    sig = now;
  }
  // the mzML of an item (name or name#tag) with its index open
  async function open(m) {
    const name = String(m.name).split("#")[0], f = await fileOf(name);
    if (!(await lrOpen(name))) jerr("err.engine.index");
    return { slot: slotOf(name), f, rd: readerFor(f) };
  }
  const metodoOp = createMetodoOps({ engine, ARCH, readerFor, open, sync });
  return async function labOp(m) {
    const mo = await metodoOp(m);
    if (mo) return mo;
    if (m.op === "lr_put" && /\.dam$/i.test(m.name)) seq.set(m.name, ++n);
    const e = engine();
    switch (m.op) {
      case "lr_dam_sync": await sync(); return { info: e.lab_list() };
      case "lr_grad_make": return { info: gradient_make(m.fmt, m.text) };
      case "lr_method_warnings": await sync(); return { info: e.lab_warnings(JSON.stringify(m.have || [])) };
      case "lr_method": { await sync(); const o = await open(m); return { info: e.lab_method(o.slot, m.name, o.f.size, o.rd) }; }
      case "lr_gradient": { await sync(); const o = await open(m); return { info: e.lab_gradient(o.slot, m.name, o.f.size, o.rd) }; }
      case "lr_analog": { const o = await open(m); return { info: e.item_analog(o.slot, o.f.size, o.rd, m.i, m.n) }; }
      case "lr_pda": { const o = await open(m); return { info: e.item_pda(o.slot, o.f.size, o.rd) }; }
      case "lr_delay": { const o = await open(m); return { info: e.item_delay(o.slot, m.name, o.f.size, o.rd) }; }
    }
    return undefined;
  };
}
