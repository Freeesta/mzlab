// @ts-check
// TPMINE-PRIVATE  Table of the files of Tabui with role, condition, time and replicate (campioni/2). Loaded on request (window.TPLAZY.load("gruppi-campioni.js")), never at start.
// window.TPGRUPPI_FILES.mount(el, files, opts) draws it: the proposals (role, time, condition = `serie`, replicate) and the warnings are the Rust of mzlab-core (`campioni`) reached through
// opts.R (the module TP_Mine/files/finder-ruoli.js) and opts.call; this file holds no rule about names. The record lives in the notebook: NB.campioni (canonical campioni/2, one entry per file)
// and NB.gruppi_opts ({reference, min_rep_frac, min_area, factors}; the fields other than `reference` stay as they are). A condition proposed from a name is yellow until «Confirm conditions»
// (page state: the files that the notebook already holds at mount are taken as confirmed). Texts in English (mzFinder convention).
(() => {
  "use strict";
  /** @typedef {import("../files/finder-ruoli.js").Campione} Campione */
  /** @typedef {{R: any, call: any, nb?: any, save?: () => void, onChange?: (r: {campioni: Campione[], reference: string, conditions: string[]}) => void}} Opts */

  const WARN = {
    noT0: "No t0", noBlank: "No blank", noDark: "No dark control", onePoint: "One time point: no kinetics", replicates: "Replicates found",
    noControls: "No t0, blank or dark control", noTreated: "No treated sample", timeUnconfirmed: "Times not confirmed", timeMissing: "Time missing", t0WithTime: "A t0 has a time other than 0",
  };
  const REF = /^(ref|reference|control|no[ _-]?gas)/i;
  const TIMED = ["sample", "t0", "dark"];
  let seq = 0;
  const esc = (/** @type {any} */ s) => String(s).replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const CSS = `.tpg table{border-collapse:collapse;font-size:12px}.tpg th,.tpg td{padding:2px 6px;text-align:left}.tpg input,.tpg select,.tpg button{min-height:28px;min-width:28px}
.tpg input.tpg-time{width:6em}.tpg input.tpg-rep{width:4em}.tpg input.tpg-cond{width:10em}.tpg tr.tpg-prop input.tpg-cond,.tpg tr.tpg-prop input.tpg-time{background:color-mix(in srgb,var(--warn) 30%,transparent)}
.tpg .tpg-bar{display:flex;flex-wrap:wrap;gap:8px;align-items:center;margin:6px 0}.tpg .tpg-w{font-size:12px}.tpg .tpg-w .e{color:var(--bad)}.tpg .tpg-w .w{color:var(--warn)}`;

  /** a file the notebook has no entry for: the Data tab's session (v1) carries no condition and often no time, so the proposal from the names (the Rust, all files together) stands unless the session holds a confirmed time
   * @param {any} R @param {any} call @param {string[]} files @param {Campione[]} cs @param {Set<string>} stored @returns {Promise<Campione[]>} */
  async function withProposals(R, call, files, cs, stored) {
    if (cs.every(c => stored.has(c.file))) return cs;
    const prop = /** @type {Campione[]} */ (JSON.parse(await call("proponi", JSON.stringify({ files: files.map(name => ({ name: R.fileOf(name), sha256: "", kind: "" })) }))).campioni);
    return cs.map(c => {
      const p = prop.find(x => x.file === c.file);
      if (stored.has(c.file) || !p) return c;
      if (c.tempo != null && c.tempo_confermato) return { ...c, serie: c.serie || p.serie, replica: p.replica };
      return { ...p, ordine_iniezione: c.ordine_iniezione };
    });
  }

  /** one row of the table @param {any} R @param {string} id @param {Campione} c @param {number} i @param {string} n name, escaped @param {boolean} checked @param {boolean} seen the user already saw the condition */
  function rowHtml(R, id, c, i, n, checked, seen) {
      const timed = TIMED.includes(c.tipo), tprop = timed && c.tempo != null && !c.tempo_confermato, cprop = !!c.serie && !seen;
      return `<tr${tprop || cprop ? ' class="tpg-prop"' : ""}><td><input type="checkbox" id="${id}-s${i}" class="tpg-sel" data-i="${i}" aria-label="Select ${n}"${checked ? " checked" : ""}></td><td>${n}</td>`
        + `<td><select class="tpg-role" id="${id}-r${i}" data-i="${i}" aria-label="Role of ${n}">${R.ROLES.map((/** @type {string} */ r) => `<option${r === c.tipo ? " selected" : ""}>${r}</option>`).join("")}</select></td>`
        + `<td><input class="tpg-cond" id="${id}-c${i}" data-i="${i}" list="${id}-dl" autocomplete="off" aria-label="Condition of ${n}" placeholder="${TIMED.includes(c.tipo) ? "" : "shared"}" value="${esc(c.serie)}"${cprop ? ' title="Proposed from the name: Confirm conditions keeps it"' : ""}></td>`
        + `<td><input class="tpg-time" id="${id}-t${i}" data-i="${i}" type="text" inputmode="decimal" autocomplete="off" aria-label="Time of ${n}"${timed ? "" : " disabled"} value="${c.tempo == null ? "" : c.tempo}"${tprop ? ' title="Proposed from the name: Enter confirms"' : ""}></td>`
        + `<td><input class="tpg-rep" id="${id}-p${i}" data-i="${i}" type="number" min="1" step="1" aria-label="Replicate of ${n}" value="${c.replica ?? ""}"></td></tr>`;
  }

  /** the warnings of the Rust, one span per code and condition @param {{codice: string, gravita: string, file: string, serie: string}[]} avvisi @param {boolean} several more than one condition: name it */
  function warnHtml(avvisi, several) {
    /** @type {Map<string, {sev: string, n: number, text: string}>} */ const by = new Map();
    for (const a of avvisi) {
      const t = /** @type {any} */ (WARN)[a.codice]; if (!t) continue;
      const key = a.codice + "|" + a.serie, old = by.get(key);
      by.set(key, { sev: a.gravita, n: (old ? old.n : 0) + 1, text: a.file || !a.serie || !several ? t : `${t} in ${a.serie}` });
    }
    return [...by.values()].map(w => `<span class="${w.sev === "error" ? "e" : "w"}">${esc(w.text)}${w.n > 1 ? " (" + w.n + ")" : ""}</span>`).join(" · ");
  }

  /**
   * @param {HTMLElement} el where the table goes @param {string[]} files names of the files, in the order of the Dig @param {Opts} opts
   * @returns {Promise<{campioni: () => Campione[], reference: () => string, conditions: () => string[], unmount: () => void}>}
   */
  async function mount(el, files, opts) {
    const R = opts.R, call = opts.call, nb = opts.nb || null, id = "tpg" + ++seq;
    /** @type {Campione[]} */ let cs = await R.build(call, files, nb, null);
    const stored = new Set(((nb && nb.campioni && nb.campioni.campioni) || []).map((/** @type {Campione} */ c) => c.file));
    // a file the notebook has no entry for: the Data tab's session (v1) carries no condition and often no time, so the proposal from the names (the Rust, all files together) stands unless the session holds a confirmed time
    cs = await withProposals(R, call, files, cs, stored);
    const sure = new Set(cs.filter(c => stored.has(c.file)).map(c => c.file));            // conditions the user already saw
    /** @type {Set<number>} */ const sel = new Set();
    const o = (nb && nb.gruppi_opts) || null;
    let ref = o && typeof o.reference === "string" ? o.reference : null, setOpen = false, dead = false;

    const conds = () => [...new Set(cs.map(c => c.serie).filter(Boolean))];
    const refOk = () => { const k = conds(); if (ref == null || !k.includes(ref)) ref = k.find(x => REF.test(x)) || ""; return ref; };
    /** tells the page what the table holds now: the canonical campioni/2 (the Rust derives `tempo_min`), the reference and the conditions */
    const notify = async () => { refOk(); if (opts.onChange) opts.onChange({ campioni: JSON.parse((await R.canonical(call, cs)).json).campioni, reference: ref || "", conditions: conds() }); };
    const emit = async () => {
      refOk();
      if (nb) {
        R.store(nb, cs); nb.gruppi_opts = { ...(nb.gruppi_opts || {}), reference: ref };
        if (opts.save) opts.save();
      }
      await notify();
    };
    /** one change: apply it, keep it, draw again with the focus where it was */
    const edit = async (/** @type {() => void} */ change, /** @type {string=} */ focus) => { change(); await emit(); await draw(focus); };

    /** @param {string=} focus id of the element that takes the focus again */
    async function draw(focus) {
      if (dead) return;
      const k = conds(), shared = k.length >= 2, u = cs.find(c => c.tempo != null);
      const rows = cs.map((c, i) => rowHtml(R, id, c, i, esc(files[i]), sel.has(i), sure.has(c.file))).join("");
      el.innerHTML = `<div class="tpg"><style>${CSS}</style>`
        + `<div class="tpg-bar"><strong>Files, conditions and times</strong><label>Time unit <select id="${id}-u" aria-label="Unit of the times of the files">${R.UNITS.map((/** @type {string} */ x) => `<option${u && u.unita_tempo === x ? " selected" : ""}>${x}</option>`).join("")}</select></label>`
        + `<button type="button" id="${id}-set"${sel.size ? "" : " disabled"}>Set condition…</button><button type="button" id="${id}-ok" title="Keeps the conditions and the times proposed from the names">Confirm conditions</button>`
        + `<label${shared ? "" : " hidden"}>Reference condition <select id="${id}-ref" aria-label="Reference condition">${["", ...k].map(x => `<option value="${esc(x)}"${x === ref ? " selected" : ""}>${x ? esc(x) : "none"}</option>`).join("")}</select></label></div>`
        + `<div class="tpg-bar" id="${id}-setrow"${setOpen ? "" : " hidden"}><label>Condition for the ${sel.size} selected file${sel.size === 1 ? "" : "s"} <input id="${id}-sv" list="${id}-dl" autocomplete="off" aria-label="Condition for the selected files"></label><button type="button" id="${id}-apply">Apply</button><button type="button" id="${id}-cancel">Cancel</button></div>`
        + `<datalist id="${id}-dl">${k.map(x => `<option value="${esc(x)}">`).join("")}</datalist>`
        + `<table><caption class="sr-only">Role, condition, time and replicate of each file</caption><tr><th scope="col"><input type="checkbox" id="${id}-all" aria-label="Select all files"${sel.size === cs.length && cs.length ? " checked" : ""}></th><th scope="col">File</th><th scope="col">Role</th><th scope="col">Condition</th><th scope="col">Time</th><th scope="col">Replicate</th></tr>${rows}</table>`
        + `<div class="tpg-w" id="${id}-w" role="status"></div></div>`;
      wire(); await warn();
      if (focus) { const f = /** @type {HTMLElement | null} */ (el.querySelector("#" + focus)); if (f) f.focus(); }
    }

    function wire() {
      const q = (/** @type {string} */ s) => /** @type {NodeListOf<HTMLInputElement>} */ (el.querySelectorAll(s)), $ = (/** @type {string} */ s) => /** @type {HTMLInputElement} */ (el.querySelector("#" + id + s));
      const idx = (/** @type {HTMLElement} */ e) => +(e.dataset.i || 0);
      q(".tpg-role").forEach(e => { e.onchange = () => edit(() => R.setRole(cs, idx(e), e.value), e.id); });
      q(".tpg-time").forEach(e => {
        const go = () => edit(() => R.setTime(cs, idx(e), e.value.trim() === "" ? null : parseFloat(e.value.replace(",", ".")), $("-u").value), e.id);
        e.onkeydown = ev => { if (ev.key === "Enter") { ev.preventDefault(); go(); } }; e.onchange = go;
      });
      q(".tpg-cond").forEach(e => { e.onchange = () => edit(() => { cs[idx(e)].serie = e.value.trim(); sure.add(cs[idx(e)].file); }, e.id); });
      q(".tpg-rep").forEach(e => { e.onchange = () => { const v = Math.round(+e.value); edit(() => { if (v >= 1) cs[idx(e)].replica = v; }, e.id); }; });
      q(".tpg-sel").forEach(e => { e.onchange = () => { if (e.checked) sel.add(idx(e)); else sel.delete(idx(e)); $("-set").disabled = !sel.size; $("-all").checked = sel.size === cs.length; }; });
      $("-all").onchange = e => { const on = /** @type {HTMLInputElement} */ (e.target).checked; sel.clear(); if (on) cs.forEach((_, i) => sel.add(i)); q(".tpg-sel").forEach(x => { x.checked = on; }); $("-set").disabled = !sel.size; };
      $("-u").onchange = () => edit(() => R.setUnit(cs, $("-u").value), id + "-u");
      $("-ok").onclick = () => edit(() => { cs.forEach(c => sure.add(c.file)); R.confirmAll(cs); }, id + "-ok");
      $("-ref").onchange = () => { ref = $("-ref").value; emit(); };
      $("-set").onclick = async () => { setOpen = true; await draw(id + "-sv"); };
      const apply = () => edit(() => { const v = $("-sv").value.trim(); for (const i of sel) { cs[i].serie = v; sure.add(cs[i].file); } setOpen = false; }, id + "-set");
      $("-apply").onclick = apply;
      $("-cancel").onclick = async () => { setOpen = false; await draw(id + "-set"); };
      $("-sv").onkeydown = ev => { if (ev.key === "Enter") { ev.preventDefault(); apply(); } else if (ev.key === "Escape") { setOpen = false; draw(id + "-set"); } };
    }

    /** the warnings of the Rust per condition: codes only, the words are here */
    async function warn() {
      const box = el.querySelector("#" + id + "-w"); if (!box) return;
      let v; try { v = await R.check(call, cs); } catch (_) { box.textContent = "The roles of the files could not be checked."; return; }
      if (!dead) box.innerHTML = warnHtml(v.avvisi, conds().length >= 2);
    }

    refOk(); await draw(); await notify();
    return { campioni: () => cs, reference: () => ref || "", conditions: conds, unmount: () => { dead = true; el.textContent = ""; } };
  }

  /** @type {any} */ (window).TPGRUPPI_FILES = { mount };
})();
