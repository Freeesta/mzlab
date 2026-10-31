// @ts-check
// Flow (administrator mode): screen.suspects and annot.regulatory in this browser. The alignment is read from OPFS and, with the suspect lists of the plan (a .mzsus or a CSV the user gave; nothing is bundled or fetched) and the optional
// MGF and candidate formulas of steps of this plan, handed to `flow_screen_suspects` of the `table` .wasm: the Rust function is the one `flow run` calls, so the screen/1 text is the same byte for byte.
// annot.regulatory hands the annot/1 text of a step of this plan and the regulatory package (a .mzsus with the listings extension the user gave) to `flow_regulatory` of the `attributio` .wasm, the function
// `flow run` calls. A .mzsus is binary; the page keeps it in S.samplesJson as "b64:" + base64, a CSV as its text. Results are kept in OPFS under the key of the job (flow-live-jobs.js). English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["screen.suspects", "annot.regulatory"];
  /** the file port of each step that takes a suspect list or a regulatory package */
  const PORTS = /** @type {Record<string, string>} */ ({ "screen.suspects": "suspects", "annot.regulatory": "lists" });
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  const enc = (/** @type {string} */ s) => new TextEncoder().encode(s);
  const hex = (/** @type {ArrayBuffer} */ b) => [...new Uint8Array(b)].map(x => x.toString(16).padStart(2, "0")).join("");
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** the bytes of a list kept in the page: "b64:..." is a binary file, anything else a text @param {string} t @returns {Uint8Array} */
  const bytesOf = t => (t.startsWith("b64:") ? Uint8Array.from(atob(t.slice(4)), c => c.charCodeAt(0)) : enc(t));
  /** a binary file as the text the page keeps @param {Uint8Array} b */
  const asText = b => "b64:" + btoa(Array.from(b, c => String.fromCharCode(c)).join(""));

  /** the lists in the order of the port, as `[name, bytes]`; null when the browser does not hold one of them @param {any} x @param {any} inst @param {string} [port] */
  const lists = (x, inst, port = "suspects") => {
    const refs = F.refs(inst, port).map(String);
    const got = refs.map((/** @type {string} */ r) => { const n = r.slice(5), t = r.startsWith("file:") && x.S.samplesJson && x.S.samplesJson[n]; return typeof t === "string" ? { name: n, text: t } : null; });
    return refs.length && got.every(Boolean) ? got : null;
  };

  /** screen.suspects @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const W = F.webIdent, j = J().job(spec), p = await kit().getPool(), files = lists(x, inst);
    const up0 = x.inst(String(F.refs(inst, "table")[0] || "").split(".")[0]), isTable = !!up0 && ["features.align", "features.gapfill"].includes(base(up0));
    const al = isTable ? x.alignOf(up0) : null, alKey = al && x.keys.get(x.jk({ instance: al.id, item: null }));
    const wired = (/** @type {string} */ port) => F.refs(inst, port).length > 0;
    const spectra = wired("spectra") ? await W.textOf(x, p, inst, "spectra") : null, formulas = wired("formulas") ? await W.textOf(x, p, inst, "formulas") : null;
    if (!files || !alKey || (wired("spectra") && !spectra) || (wired("formulas") && !formulas) || !await J().has(p, J().DIR + alKey + ".mzfeat")) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + alKey + "\n" + (await Promise.all(files.map(f => J().sha(f.text)))).join(",") + "\n" + await J().sha(spectra ? spectra.text : "") + "\n" + await J().sha(formulas ? formulas.text : "")), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now();
    return J().detached(x, spec, inst, async () => {
      const params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
      const bytes = files.map(f => bytesOf(f.text)), packed = new Uint8Array(bytes.reduce((n, b) => n + 8 + b.length, 0)), view = new DataView(packed.buffer);
      let at = 0;
      for (const b of bytes) { view.setBigUint64(at, BigInt(b.length), true); packed.set(b, at + 8); at += 8 + b.length; }
      let text;
      try {
        text = await p.ident("table", "flow_screen_suspects", { params, table: await p.fs.read(J().DIR + alKey + ".mzfeat"), names: JSON.stringify(files.map(f => f.name)), lists: packed,
          mgf: spectra ? enc(spectra.text) : null, formulas: formulas ? enc(formulas.text) : null });
      } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".json", text };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: J().job(spec), result: { ms: performance.now() - t0 } });
    });
  };

  /** annot.regulatory (list_membership): the annot/1 text of a step of this plan against the regulatory package @param {any} x @param {any} spec @param {any} inst */
  const regulatory = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), pkg = lists(x, inst, "lists"), annot = await F.webIdent.textOf(x, p, inst, "annot");
    if (!pkg || pkg.length !== 1 || !annot) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha([spec.key, await J().sha(annot.text), await J().sha(pkg[0].text)].join("\n")), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now();
    return J().detached(x, spec, inst, async () => {
      const params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
      let text;
      try { text = await p.ident("attributio_ms2", "flow_regulatory", { params, annot: enc(annot.text), package: bytesOf(pkg[0].text) }); } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".json", text };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: J().job(spec), result: { ms: performance.now() - t0 } });
    });
  };

  // a suspect list per file input of the plan: a .mzsus or a CSV (name, InChIKey or SMILES, formula or neutral mass, optional RT and fragments), kept in S.samplesJson with the SHA-256 of its bytes
  F.extraResults.push((/** @type {any} */ S, /** @type {any} */ node) => {
    const insts = (node.ids || []).map((/** @type {string} */ id) => F.instanceOf(S.plan, id)).filter((/** @type {any} */ i) => i && PORTS[base(i)]);
    const refs = [...new Set(insts.flatMap((/** @type {any} */ i) => F.refs(i, PORTS[base(i)])).filter((/** @type {string} */ r) => r.startsWith("file:")))];
    if (!refs.length) return null;
    const { el } = F, say = el("p", { class: "fl-note", role: "status" });
    const row = refs.map((/** @type {string} */ ref) => {
      const pick = /** @type {HTMLInputElement} */ (el("input", { type: "file", hidden: true, accept: ".mzsus,.csv,.tsv,.txt", "aria-label": "Choose a suspect list file", onchange: async (/** @type {any} */ e) => {
        const f = e.target.files && e.target.files[0], input = S.plan.inputs.find((/** @type {any} */ i) => "file:" + i.name === ref); if (!f || !input) return;
        const raw = new Uint8Array(await f.arrayBuffer()), mzsus = raw.length > 4 && String.fromCharCode(...raw.slice(0, 4)) === "MZSU", text = mzsus ? asText(raw) : new TextDecoder().decode(raw);
        S.samplesJson = S.samplesJson || {}; S.samplesJson[input.name] = text;
        input.sha256 = hex(await crypto.subtle.digest("SHA-256", raw));
        say.textContent = `${f.name}: ${raw.length} bytes kept as ${input.name}.`;
        if (S.redraw) S.redraw();
      } }));
      return [pick, F.btn("import", "Choose a suspect list", () => pick.click(), "Suspect list…")];
    }).flat();
    return el("div", { class: "fl-results" }, ...row, say);
  });

  F.webSuspects = { run, regulatory, STEPS, bytesOf };
})();
