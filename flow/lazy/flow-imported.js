// @ts-check
// Flow (administrator mode): the regulatory lists the user imported (Library window), as a checkbox list in the form of screen.suspects and annot.regulatory. Ticking one adds its private
// .mzsus to the plan as an input (type library, name `imported-ID.mzsus`, content in S.samplesJson as "b64:" + base64, its SHA-256 on the input) and wires it to the port `imported_lists`;
// unticking removes both. The package lists stay as they are: the engine opens each file as a source of its own. Bytes: OPFS in the browser, the libraries folder in the app. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  /** the app module `name` (loaded once, the same instance as the app's) @param {string} name */
  const app = name => import(/* webpackIgnore: true */ new URL(name, G.QTOOLS && G.QTOOLS.ctx ? G.QTOOLS.ctx.base : document.baseURI).href);
  const fileOf = (/** @type {string} */ id) => `imported-${id}.mzsus`;
  const hex = (/** @type {ArrayBuffer} */ b) => [...new Uint8Array(b)].map(x => x.toString(16).padStart(2, "0")).join("");

  /** the imported file `id` as an input of the plan wired to `imported_lists` of `inst`, or removed from it @param {any} S @param {any} inst @param {any} step @param {string} id @param {Uint8Array | null} bytes null: remove */
  F.importedSet = async (S, inst, step, id, bytes) => {
    const name = fileOf(id), port = F.ports(step.in).find((/** @type {any} */ p) => p.name === "imported_lists"), refs = F.refs(inst, "imported_lists").filter((/** @type {string} */ r) => r !== "file:" + name);
    if (!port) return;
    if (bytes) {
      let input = S.plan.inputs.find((/** @type {any} */ f) => f.name === name);
      if (!input) { input = { name, sha256: "", type: port.type }; S.plan.inputs.push(input); }
      input.sha256 = hex(await crypto.subtle.digest("SHA-256", /** @type {BufferSource} */ (bytes)));
      S.samplesJson = S.samplesJson || {};
      S.samplesJson[name] = "b64:" + btoa(Array.from(bytes, c => String.fromCharCode(c)).join(""));
      refs.push("file:" + name);
      F.setPort(inst, port, refs);
    } else {
      F.setPort(inst, port, refs);
      if (!S.plan.steps.some((/** @type {any} */ s) => F.refs(s, "imported_lists").includes("file:" + name))) { F.removeInput(S.plan, name); if (S.samplesJson) delete S.samplesJson[name]; }
    }
  };

  /** the checkbox list of the imported lists for `inst` (empty until the page has read them; nothing when there are none) @param {any} S @param {any} inst @param {any} step @param {() => void} done */
  F.importedLists = (S, inst, step, done) => {
    const { el } = F, host = el("fieldset", { class: "fl-note", hidden: true, "data-fk": "fl-imported", style: "grid-column:1/-1" });
    const say = el("p", { class: "fl-note", role: "status" });
    app("libreria-normative.js").then(async lib => {
      const mine = await lib.importedLists();
      if (!mine.length) return;
      const on = new Set(F.refs(inst, "imported_lists"));
      host.append(el("legend", { text: "Lists you imported" }), ...mine.map((/** @type {any} */ l) => el("label", { style: "display:block" },
        el("input", { type: "checkbox", "data-imported": l.id, checked: on.has("file:" + fileOf(l.id)), "aria-label": `Use the imported list ${l.id}`, onchange: async (/** @type {any} */ e) => {
          const box = e.target;
          try {
            const bytes = box.checked ? await lib.importedBytes(l.id) : null;
            if (box.checked && !bytes) { box.checked = false; say.textContent = `The imported file of ${l.id} could not be read.`; return; }
            await F.importedSet(S, inst, step, l.id, bytes); say.textContent = ""; done();
          } catch (_) { box.checked = !box.checked; say.textContent = `The imported list ${l.id} could not be used.`; }
        } }), " ", l.id, el("span", { class: "fl-note", text: l.tags && l.tags.citation ? ` · ${l.tags.citation}` : "" }))), say);
      host.hidden = false;
    }).catch(() => { /* no imported lists in this page */ });
    return host;
  };
})();
