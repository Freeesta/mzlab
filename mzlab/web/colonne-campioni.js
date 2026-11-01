// @ts-check
// Extra columns of the loading table (renderFiles in shell.js): a «Columns» button opens a dialog with the preset columns (dilution factor, injection volume, batch, vial, matrix, operator, acquisition,
// instrument, polarity, comment, replicate), and the custom ones (name, text or number, optional unit; renamed, moved, removed there). The definitions live in `NB.sampleCols`
// ({on: [ids in order], custom: [{id, name, type, unit}]}), the values in `NB.sampleEx` ({file: {id: value}}): the notebook keeps both, so a reload finds them and the TSV export of the sample table gets them.
// Lazy (carica("colonne-campioni.js")): shell.js asks `window.COLS` for the header, the cells and the wiring of the table.
window.COLS = (() => {
  const t = (/** @type {string} */ k, /** @type {any} */ p) => I18N.t(k, p);
  /** @type {[string, string, string][]} id, type, unit of the preset columns */
  const PRESETS = [["dil", "number", ""], ["vol", "number", "µL"], ["rep", "text", ""], ["batch", "text", ""], ["vial", "text", ""], ["matrix", "text", ""], ["operator", "text", ""], ["acq", "text", ""], ["instr", "text", ""], ["pol", "text", ""], ["note", "text", ""]];
  // literal keys (the i18n check follows them)
  const pname = (/** @type {string} */ id) => ({ dil: t("load.cols.p.dil"), vol: t("load.cols.p.vol"), rep: t("load.cols.p.rep"), batch: t("load.cols.p.batch"), vial: t("load.cols.p.vial"), matrix: t("load.cols.p.matrix"), operator: t("load.cols.p.operator"), acq: t("load.cols.p.acq"), instr: t("load.cols.p.instr"), pol: t("load.cols.p.pol"), note: t("load.cols.p.note") })[id] || id;
  const tname = (/** @type {string} */ ty) => ty === "number" ? t("load.cols.type.number") : t("load.cols.type.text");
  /** @typedef {{ id: string, name: string, type: string, unit: string }} Col */
  const st = () => { const s = NB.sampleCols; return { on: Array.isArray(s && s.on) ? s.on : [], custom: Array.isArray(s && s.custom) ? s.custom : [] }; };
  const save = (/** @type {{ on: string[], custom: Col[] }} */ s) => { NB.sampleCols = s; if (typeof nbSave === "function") nbSave(); };
  const values = () => (NB.sampleEx && typeof NB.sampleEx === "object" ? NB.sampleEx : {});
  /** @returns {Col | null} */
  const def = (/** @type {string} */ id) => {
    const p = PRESETS.find(x => x[0] === id);
    if (p) return { id, name: pname(id), type: p[1], unit: p[2] };
    return st().custom.find((/** @type {Col} */ c) => c.id === id) || null;
  };
  const active = () => /** @type {Col[]} */ (st().on.map(def).filter(Boolean));
  const label = (/** @type {Col} */ c) => c.unit ? `${c.name} (${c.unit})` : c.name;
  const widest = 120;

  const n = () => active().length;
  /** the header cells, put before the last (empty) one @param {string} head */
  const head = head => {
    const cols = active();
    if (!cols.length) return head;
    const at = head.lastIndexOf("<th></th></tr>");
    return at < 0 ? head : head.slice(0, at) + cols.map(c => `<th scope="col"${c.type === "number" ? ' class="num"' : ""}>${EH(label(c))}</th>`).join("") + head.slice(at);
  };
  /** the cells of a row, put before its last (buttons) one @param {{ name: string }} f @param {string} h */
  const row = (f, h) => {
    const cols = active();
    if (!cols.length) return h;
    const at = h.lastIndexOf('<td style="white-space:nowrap">'), mine = values()[f.name] || {};
    if (at < 0) return h;
    return h.slice(0, at) + cols.map(c => `<td${c.type === "number" ? ' class="num"' : ""}><input type="text" ${c.type === "number" ? 'inputmode="decimal" ' : ""}autocomplete="off" size="9" data-xk="${EH(c.id)}" data-xf="${EH(f.name)}" value="${EH(mine[c.id] ?? "")}" aria-label="${EH(`${c.name} · ${f.name}`)}"></td>`).join("") + h.slice(at);
  };
  const setValue = (/** @type {string} */ file, /** @type {string} */ id, /** @type {string | number | null} */ v) => {
    const all = { ...values() }, mine = { ...(all[file] || {}) };
    if (v === null || v === "") delete mine[id]; else mine[id] = v;
    if (Object.keys(mine).length) all[file] = mine; else delete all[file];
    NB.sampleEx = all; if (typeof nbSave === "function") nbSave();
  };

  // ---------------------------------------------------------------- the dialog
  const redraw = () => { if (typeof renderFiles === "function") renderFiles(); };
  /** @param {string} focusId the control that takes the focus after the dialog is drawn again */
  function drawDialog(dlg, focusId = "") {
    const s = st(), box = /** @type {HTMLElement} */ (dlg.querySelector(".colsbody"));
    const on = new Set(s.on);
    const check = (/** @type {string} */ id, /** @type {string} */ name, /** @type {boolean} */ checked, /** @type {boolean} */ off = false) =>
      `<label class="colsrow"><input type="checkbox" data-cc="${EH(id)}"${checked ? " checked" : ""}${off ? " disabled" : ""}> <span>${EH(name)}</span></label>`;
    const custom = s.custom.map((/** @type {Col} */ c) => {
      const i = s.on.indexOf(c.id);
      return `<li class="colsrow" data-cid="${EH(c.id)}"><input type="checkbox" data-cc="${EH(c.id)}"${on.has(c.id) ? " checked" : ""} aria-label="${EH(t("load.cols.shown", { name: c.name }))}">
<input type="text" data-cname="${EH(c.id)}" value="${EH(c.name)}" aria-label="${EH(t("load.cols.rename", { name: c.name }))}" size="14">
<span class="muted sm">${EH(tname(c.type))}${c.unit ? " · " + EH(c.unit) : ""}</span>
<button type="button" class="sec sm" data-cup="${EH(c.id)}" aria-label="${EH(t("load.cols.up", { name: c.name }))}"${i <= 0 ? " disabled" : ""}>↑</button>
<button type="button" class="sec sm" data-cdown="${EH(c.id)}" aria-label="${EH(t("load.cols.down", { name: c.name }))}"${i < 0 || i >= s.on.length - 1 ? " disabled" : ""}>↓</button>
<button type="button" class="sec sm" data-cdel="${EH(c.id)}" aria-label="${EH(t("load.cols.remove", { name: c.name }))}">&times;</button></li>`;
    }).join("");
    const html = `<fieldset class="colsset"><legend>${EH(t("load.cols.preset"))}</legend>
${PRESETS.map(p => check(p[0], pname(p[0]), on.has(p[0]))).join("")}</fieldset>
${s.custom.length ? `<fieldset class="colsset"><legend>${EH(t("load.cols.custom"))}</legend><ul class="colslist">${custom}</ul></fieldset>` : ""}
<fieldset class="colsset"><legend>${EH(t("load.cols.add.h"))}</legend><div class="colsadd">
<label>${EH(t("load.cols.name"))} <input type="text" id="colsnew" size="16"></label>
<label>${EH(t("load.cols.type"))} <select id="colstype"><option value="text">${EH(t("load.cols.type.text"))}</option><option value="number">${EH(t("load.cols.type.number"))}</option></select></label>
<label>${EH(t("load.cols.unit"))} <input type="text" id="colsunit" size="6"></label>
<button type="button" id="colsadd">${EH(t("load.cols.add"))}</button></div></fieldset>`;
    box.innerHTML = h`${raw(html)}`;      // every piece above is escaped with EH
    if (focusId) /** @type {HTMLElement | null} */ (box.querySelector(focusId))?.focus();
  }
  function open() {
    document.getElementById("colsdlg")?.remove();
    const dlg = /** @type {HTMLDialogElement} */ (document.createElement("dialog"));
    dlg.id = "colsdlg"; dlg.className = "colsdlg"; dlg.setAttribute("aria-labelledby", "colshead");
    const shell = `<h2 id="colshead">${EH(t("load.cols.head"))}</h2><div class="colsbody"></div><div class="colsfoot"><button type="button" id="colsclose">${EH(t("load.cols.close"))}</button></div>`;
    dlg.innerHTML = h`${raw(shell)}`;
    document.body.append(dlg); drawDialog(dlg);
    dlg.addEventListener("close", () => { dlg.remove(); document.getElementById("colsbtn")?.focus(); });
    dlg.addEventListener("click", ev => {
      const b = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (ev.target).closest("button"));
      if (!b) return;
      const s = st();
      if (b.id === "colsclose") { dlg.close(); return; }
      if (b.id === "colsadd") {
        const name = /** @type {HTMLInputElement} */ (dlg.querySelector("#colsnew")).value.trim();
        if (!name) { dlg.querySelector("#colsnew")?.setAttribute("aria-invalid", "true"); /** @type {HTMLElement} */ (dlg.querySelector("#colsnew")).focus(); return; }
        const id = "c" + Date.now().toString(36), type = /** @type {HTMLSelectElement} */ (dlg.querySelector("#colstype")).value, unit = /** @type {HTMLInputElement} */ (dlg.querySelector("#colsunit")).value.trim();
        save({ ...s, custom: [...s.custom, { id, name, type, unit }], on: [...s.on, id] }); drawDialog(dlg, "#colsnew"); redraw(); return;
      }
      const move = (/** @type {string} */ id, /** @type {number} */ d) => {
        const on = [...s.on], i = on.indexOf(id), j = i + d;
        if (i < 0 || j < 0 || j >= on.length) return;
        [on[i], on[j]] = [on[j], on[i]]; save({ ...s, on }); drawDialog(dlg, `[data-${d < 0 ? "cup" : "cdown"}="${id}"]`); redraw();
      };
      if (b.dataset.cup) move(b.dataset.cup, -1);
      else if (b.dataset.cdown) move(b.dataset.cdown, 1);
      else if (b.dataset.cdel) {
        const id = b.dataset.cdel, all = { ...values() };
        Object.keys(all).forEach(f => { const m = { ...all[f] }; delete m[id]; if (Object.keys(m).length) all[f] = m; else delete all[f]; });
        NB.sampleEx = all; save({ ...s, custom: s.custom.filter((/** @type {Col} */ c) => c.id !== id), on: s.on.filter((/** @type {string} */ x) => x !== id) }); drawDialog(dlg, "#colsnew"); redraw();
      }
    });
    dlg.addEventListener("change", ev => {
      const el = /** @type {HTMLInputElement} */ (ev.target), s = st();
      if (el.dataset.cc) {
        const id = el.dataset.cc;
        save({ ...s, on: el.checked ? [...s.on.filter((/** @type {string} */ x) => x !== id), id] : s.on.filter((/** @type {string} */ x) => x !== id) });
        redraw();
      } else if (el.dataset.cname) {
        const name = el.value.trim();
        if (!name) { el.value = (def(el.dataset.cname) || { name: "" }).name; return; }
        save({ ...s, custom: s.custom.map((/** @type {Col} */ c) => c.id === el.dataset.cname ? { ...c, name } : c) }); redraw();
      }
    });
    dlg.showModal();
  }

  /** the handlers of the table just drawn @param {HTMLElement} el */
  function wire(el) {
    document.getElementById("colsbtn")?.addEventListener("click", open);
    el.querySelectorAll("[data-xk]").forEach((/** @type {any} */ x) => {
      x.onchange = () => {
        const c = def(x.dataset.xk);
        let v = x.value.trim();
        if (c && c.type === "number" && v !== "") {
          const num = Number(v.replace(",", "."));
          if (!Number.isFinite(num)) { x.setAttribute("aria-invalid", "true"); x.title = t("load.cols.bad"); return; }
          v = num;
        }
        x.removeAttribute("aria-invalid"); x.removeAttribute("title");
        setValue(x.dataset.xf, x.dataset.xk, v);
      };
    });
  }
  /** the button over the table @returns {string} */
  const button = () => `<button type="button" class="sec sm" id="colsbtn" aria-haspopup="dialog">${EH(t("load.cols.btn"))}</button>`;

  /** the TSV of the sample table with the columns at the end (the first column is the file name) @param {string} text */
  function tsv(text) {
    const cols = active();
    if (!cols.length) return text;
    const clean = (/** @type {unknown} */ v) => String(v == null ? "" : v).replace(/[\t\r\n]+/g, " "), all = values();
    const lines = text.split("\n");
    return lines.map((l, i) => {
      if (!l || l.startsWith("#")) return l;
      const name = l.split("\t")[0];
      return l + cols.map(c => "\t" + clean(i === 0 || l.startsWith("filename\t") ? label(c) : (all[name] || {})[c.id])).join("");
    }).join("\n");
  }
  return { n, head, row, wire, button, tsv, widest, open };
})();
