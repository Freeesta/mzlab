// @ts-strict
// The routes of lr.js about the methods (.dam) and how an item was acquired, answered by the Rust engine: /api/method, /api/method_warnings, /api/analog,
// /api/delay, /api/chrom?kind=pda, /api/gradient (GET and POST), /api/gradient_parse. `get` and `post` return { status, text }, or undefined when the route is
// not theirs, or null when it is not served (a part of a mixed file with a PDA for the delay, a reference file).
export function createMethodRoutes({ rpc, json, items }) {
  let methods = [];                       // App.methods(): [{name, size, info}], oldest first
  const gradients = new Map();            // item file -> the gradient typed, pasted or imported for it (the model)
  const api = { gradients, list: () => methods, clear: () => { methods = []; } };
  const num = (q, k, d) => (q.get(k) === null || q.get(k) === "" ? d : Number(q.get(k)));
  // an error of the engine with a key: the JSON of Python's _fail (the page words it from the key); 500 for a GET, as Python's
  const fail = (e, status) => {
    let j = null; try { j = JSON.parse(e.raw); } catch (_) { /* not an engine error */ }
    if (!j || !j.error_key) throw e;
    if (j.error_key === "err.route.unavailable") return null;          // not served: the caller counts it as a miss
    return json({ error: j.error_key, error_key: j.error_key, params: j.params || {} }, status);
  };
  const item = q => { const k = num(q, "k", NaN), it = items(); return it && Number.isInteger(k) && it[k] ? it[k] : null; };
  const raw = r => ({ status: 200, text: r.info });

  api.refresh = async () => { methods = JSON.parse((await rpc({ op: "lr_dam_sync" })).info); };

  api.get = async (path, q) => {
    if (path === "/api/method_warnings") {
      const its = items();
      if (!its) return json({ warnings: [] });
      return raw(await rpc({ op: "lr_method_warnings", have: [...new Set(its.map(x => x.kind))].sort() }));
    }
    if (!["/api/method", "/api/metodo", "/api/analog", "/api/registro", "/api/delay", "/api/gradient"].includes(path) && !(path === "/api/chrom" && q.get("kind") === "pda") && !(path === "/api/map" && q.get("kind") === "pda")) return undefined;
    const it = item(q), name = it && it.file;
    if (!it || q.get("ref")) return null;
    try {
      if (path === "/api/gradient") {
        if (!methods.length) return json({ gradient: gradients.get(name) || null });
        const g = JSON.parse((await rpc({ op: "lr_gradient", name })).info);
        return json({ gradient: g || gradients.get(name) || null });
      }
      if (path === "/api/method") return raw(await rpc({ op: "lr_method", name }));
      // metodo/1 of the item (docs/agenti/metodo.md)
      if (path === "/api/metodo") return raw(await rpc({ op: "lr_metodo", name }));
      if (path === "/api/analog") return raw(await rpc({ op: "lr_analog", name, i: num(q, "i", NaN), n: Math.max(10, Math.min(Math.trunc(num(q, "n", 2000)), 20000)) }));
      // a series of the instrument log of a .raw: {id, kind, unit, time, intensity}
      if (path === "/api/registro") return raw(await rpc({ op: "natregistro", name, id: q.get("id") || "" }));
      if (path === "/api/chrom") return raw(await rpc({ op: "lr_pda", name }));
      if (path === "/api/map") return raw(await rpc({ op: "lr_pda_map", name }));
      // a part of a mixed file with a PDA reads its own scans: not served (null)
      return raw(await rpc({ op: "lr_delay", name }));
    } catch (e) { return fail(e, 500); }
  };

  const make = async (fmt, text) => JSON.parse((await rpc({ op: "lr_grad_make", fmt, text })).info);
  api.post = async (path, body) => {
    try {
      if (path === "/api/gradient_parse") return json({ gradient: await make(["course", "chromeleon"].includes(body.fmt) ? body.fmt : "csv", body.text || "") });
      if (path !== "/api/gradient") return undefined;
      const its = items();
      const ks = (Array.isArray(body.all) && body.all.length ? body.all : [body.k]).map(Number);
      if (!its || ks.some(k => !Number.isInteger(k) || !its[k])) return null;
      let model = null;
      if (body.text !== undefined && body.text !== null) model = await make(body.fmt === "chromeleon" ? "chromeleon" : "csv", body.text);
      else if (body.model) model = await make("model", JSON.stringify(body.model));
      for (const k of ks) { if (model) gradients.set(its[k].file, model); else gradients.delete(its[k].file); }
      return json({ gradient: model });
    } catch (e) { return fail(e, 400); }
  };
  return api;
}
