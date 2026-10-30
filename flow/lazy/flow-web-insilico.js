// @ts-check
// Flow (administrator mode): spectra.insilico in this browser. The candidate structures are a file of the plan: a CSV list the user gives, a screen.suspects result, or the catalogue of the
// in-silico candidate package (docs/agenti/insilico.md), which the user downloads with one click (insilico-scarica.js) into the Cache Storage. A package is read in two steps: the
// `attributio_ms2` .wasm names the pieces the mass windows of the precursors overlap (`flow_insilico_pieces`), the page inflates and checks exactly those, and `flow_insilico` ranks the
// candidates (the Rust function `flow run` calls, so the text is the same byte for byte). Nothing is fetched while a plan runs. Results are kept in OPFS under the key of the job. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const STEPS = ["spectra.insilico"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  const enc = (/** @type {string} */ s) => new TextEncoder().encode(s);
  const hex = async (/** @type {string} */ s) => [...new Uint8Array(await crypto.subtle.digest("SHA-256", enc(s)))].map(b => b.toString(16).padStart(2, "0")).join("");
  const fail = (/** @type {string} */ key, /** @type {any} */ params = {}) => Object.assign(new Error(key), { key, params });
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? fail(j.error_key, j.params || {}) : e; };
  /** the module that downloads and reads the package, loaded on first use */
  const dl = () => { const where = "../../insilico-scarica.js"; return import(/* webpackIgnore: true */ where); };
  /** the text of a failed download or read @param {any} e */
  const oops = e => (e && e.key ? F.message({ key: e.key, params: e.params }) : String(e && e.message || e));
  /** true when the text is the catalogue of a package @param {string} t */
  const isCatalogue = t => t.trimStart().startsWith("{") && /"format"\s*:\s*"insilico\/1"/.test(t.slice(0, 200));
  /** the jobs this module blocked for want of candidates, by instance: keeping a package or a list releases them (the scheduler holds a block until it is told) @type {Map<string, any>} */
  const waiting = new Map();
  const wait = async (/** @type {any} */ x, /** @type {any} */ j, /** @type {any} */ inst) => { waiting.set(inst.id, { x, j }); await x.call("block", { job: j, by: "input.insilico" }); };
  const release = async () => { for (const [id, w] of [...waiting]) { waiting.delete(id); try { await w.x.call("unblock", { job: w.j }); } catch (_) { /* the job ended meanwhile */ } } };
  const paramsOf = (/** @type {any} */ x, /** @type {any} */ inst) => JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });

  /** spectra.insilico: the MGF of a spectra.ms2 of this plan against the structures of the suspects file @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), ident = F.webIdent;
    const spectra = await ident.textOf(x, p, inst, "spectra"), suspects = await ident.textOf(x, p, inst, "suspects");
    if (!spectra) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    // no list and no package: the user is told to download the package or choose a list (flow-live-view.js shows the chip)
    if (!suspects) { await wait(x, j, inst); return {}; }
    const key = await J().sha(spec.key + "\n" + await J().sha(suspects.text) + "\n" + await J().sha(spectra.text)), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    const pack = isCatalogue(suspects.text);
    /** @type {any} */ let cat = null, read = null;
    if (pack) {
      const m = await dl(), got = await m.installed();
      // the catalogue of the plan must be the one installed (same bytes), or the pieces would not match its checksums
      if (!got || got.text !== suspects.text) { await wait(x, j, inst); return {}; }
      cat = got.catalog; read = m.reader(cat);
    }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = paramsOf(x, inst);
    return J().detached(x, spec, inst, async () => {
      /** @type {Record<string, Uint8Array>} */ const pieces = {};
      try {
        if (pack) {
          const need = JSON.parse(await p.ident("attributio_ms2", "flow_insilico_pieces", { params, name: spectra.name, mgf: enc(spectra.text), catalogue: enc(suspects.text) }));
          for (let k = 0; k < need.length; k++) {
            pieces[need[k]] = await read(need[k]);
            x.call("progress", { job: j, frac: (k + 1) / (need.length + 1) }).catch(() => {});
          }
        }
        const text = await p.ident("attributio_ms2", "flow_insilico", { params, name: spectra.name, mgf: enc(spectra.text), suspects: enc(suspects.text), pieces });
        const r = { name: inst.id + ".json", text };
        await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
        x.files.set(inst.id, { id: inst.id, ...r });
        await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
      } catch (e) { throw asError(e); }
    });
  };

  // the candidates of the plan: the download of the package (two clicks: the size is shown before anything is fetched) or a list of the user's own
  F.extraResults.push((/** @type {any} */ S, /** @type {any} */ node) => {
    const insts = (node.ids || []).map((/** @type {string} */ id) => F.instanceOf(S.plan, id)).filter((/** @type {any} */ i) => i && String(i.step).split("@")[0] === "spectra.insilico");
    const refs = insts.flatMap((/** @type {any} */ i) => F.refs(i, "suspects")).filter((/** @type {string} */ r) => r.startsWith("file:"));
    if (!refs.length) return null;
    const { el } = F, say = el("p", { class: "fl-note", role: "status" });
    const input = () => S.plan.inputs.find((/** @type {any} */ i) => "file:" + i.name === refs[0]);
    /** keeps the text as the content of the input, with its SHA-256, like the other text inputs of a plan @param {string} text @param {string} what */
    const keepText = async (text, what) => {
      const inp = input(); if (!inp) return;
      S.samplesJson = S.samplesJson || {}; S.samplesJson[inp.name] = text; inp.sha256 = await hex(text);
      say.textContent = `${what} kept as ${inp.name}.`;
      if (S.redraw) S.redraw();
      await release();
      if (F.live && F.live.change) F.live.change();
    };
    const pick = /** @type {HTMLInputElement} */ (el("input", { type: "file", hidden: true, accept: ".csv,.tsv,.txt,.json", "aria-label": "Choose a candidate structures file", onchange: async (/** @type {any} */ e) => {
      const f = e.target.files && e.target.files[0]; if (f) await keepText(await f.text(), f.name);
    } }));
    /** @type {AbortController | null} */ let ctl = null;
    const cancel = /** @type {HTMLElement} */ (F.btn("x", "Cancel the download", () => { if (ctl) ctl.abort(); }, "Cancel"));
    cancel.hidden = true;
    const wrap = el("div", { class: "fl-results" });
    /** @type {HTMLElement} */ let get;
    const draw = (/** @type {HTMLElement} */ b) => { if (get) get.replaceWith(b); else wrap.append(b); get = b; };
    // first click: the catalogue only (a few KB) to tell the size; second click: the pieces
    const stage1 = (/** @type {string} */ label) => {
      const b = /** @type {HTMLElement} */ (F.btn("export", "Show the size of the in-silico candidate package", async () => {
        try {
          const m = await dl(); b.setAttribute("aria-busy", "true");
          const fetched = await m.fetchCatalog();
          stage2(fetched, m);
        } catch (e) { b.removeAttribute("aria-busy"); say.textContent = oops(e); }
      }, label));
      draw(b);
    };
    const stage2 = (/** @type {any} */ fetched, /** @type {any} */ m) => {
      const mb = m.megabytes(fetched.catalog);
      const b = /** @type {HTMLElement} */ (F.btn("export", `Download the in-silico candidate package (${mb} MB)`, async () => {
        ctl = new AbortController(); b.setAttribute("aria-busy", "true"); cancel.hidden = false;
        try {
          const done = await m.install({ fetched, signal: ctl.signal, onProgress: (/** @type {any} */ q) => { say.textContent = `Downloading ${q.piece} of ${q.pieces} (${Math.round(q.bytes / 1048576)} of ${mb} MB)…`; } });
          await keepText(done.text, "The in-silico candidate package");
          say.textContent = `The package (${done.catalog.rows} structures, ${mb} MB) is stored in this browser and works offline. ${done.catalog.attribution}`;
          stage1("Update the package");
        } catch (e) { b.removeAttribute("aria-busy"); say.textContent = oops(e); }
        finally { ctl = null; cancel.hidden = true; }
      }, `Download (${mb} MB)`));
      draw(b);
      say.textContent = `The package is ${mb} MB (${fetched.catalog.rows} structures). Choose Download to store it in this browser; nothing is sent anywhere.`;
    };
    stage1("In-silico candidates…");
    dl().then(m => m.installed()).then(got => { if (got) { say.textContent = `The in-silico candidate package is stored in this browser (${got.catalog.rows} structures).`; stage1("Update the package"); wrap.append(F.btn("ok", "Use the package stored in this browser", () => keepText(got.text, "The stored package"), "Use stored package")); } }).catch(() => {});
    wrap.append(F.btn("import", "Choose a candidate structures file", () => pick.click(), "Own list…"), cancel, pick, say);
    return wrap;
  });

  F.webInsilico = { run, STEPS };
})();
