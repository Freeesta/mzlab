// @ts-check
// Flow (administrator mode): the sample list of a plan (input type `samples`, a campioni/2 document) filled in place: «From Data» takes the roles and times of the files open in the Data tab, «Edit» is an inline table
// (file, role, time, series, replicate). The document lives in memory (S.samples[input name]) and the input keeps its SHA-256 (the canonical form of the Rust of mzlab-core), which the plan records.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const TIMED = ["sample", "dark"];

  /** the document of the files open in the Data tab, or null when none is open */
  F.samplesFromData = () => {
    const files = typeof E !== "undefined" && Array.isArray(E.files) ? E.files : [];
    return files.length && G.CDOC ? G.CDOC.doc(files) : null;
  };

  /** keeps `doc` as the content of the sample list `input` and its SHA-256 on the input; resolves when the hash is set @param {any} S @param {any} input @param {any} doc */
  F.samplesSet = async (S, input, doc) => {
    S.samples = S.samples || {}; S.samples[input.name] = doc;
    const c = G.CDOC ? await G.CDOC.canonical(doc) : null;
    if (c && /^[0-9a-f]{64}$/.test(c.sha256)) { input.sha256 = c.sha256; S.samplesJson = S.samplesJson || {}; S.samplesJson[input.name] = c.json; }
    else input.sha256 = "";
  };

  /** an empty entry of the table @param {number} i */
  const blank = i => ({ composto: "", concentrazione: null, file: "", istd: false, note: "", ordine_iniezione: i + 1, replica: 1, serie: "", sha256: "", tempo: null, tempo_confermato: false, tempo_min: null, tipo: "sample", tipo_bianco: null, unita: null, unita_tempo: "min" });

  /** the inline editor of one sample list @param {any} S @param {any} input the input of the plan */
  F.samplesEditor = (S, input) => {
    const { el } = F, box = el("div", { class: "flx-smp" }), say = el("p", { class: "fl-note", role: "status" });
    const doc = () => (S.samples && S.samples[input.name]) || { schema: "campioni/2", campioni: [] };
    /** @param {any} d */
    const save = async d => { try { await F.samplesSet(S, input, d); say.textContent = ""; } catch (e) { say.textContent = "The list could not be hashed: " + ((/** @type {any} */ (e)).message || e); } draw(); };
    const draw = () => {
      box.textContent = "";
      const d = doc(), rows = /** @type {any[]} */ (d.campioni);
      const data = F.samplesFromData();
      const sha = el("code", { class: "flx-smp-sha", title: input.sha256 || "", text: input.sha256 ? input.sha256.slice(0, 12) + "…" : "not saved yet" });
      box.append(el("div", { class: "fl-meta" },
        F.btn("import", "Take the roles and times of the files open in Data", async () => { const x = F.samplesFromData(); if (x) await save(x); }, "From Data"),
        F.btn("plus", "Add a file to the list", () => { const n = { ...d, campioni: [...rows, blank(rows.length)] }; void save(n); }, "Add a file"),
        F.btn("export", "Download the sample list as JSON", () => { const j = S.samplesJson && S.samplesJson[input.name]; if (j) G.ATTRIBUTIO_CORE.download(input.name, j, "application/json"); }, "Download"),
        el("span", { class: "fl-note", text: "SHA-256 " }), sha), say);
      const fd = /** @type {HTMLButtonElement | null} */ (box.querySelector("button[aria-label^='Take']"));
      if (!data && fd) { fd.disabled = true; fd.title = "No file is open in the Data tab"; }
      if (!rows.length) { box.append(el("p", { class: "fl-note", text: "The list is empty. Take it from Data or add the files one by one." })); return; }
      const upd = (/** @type {number} */ i, /** @type {any} */ patch) => { const next = rows.map((r, k) => (k === i ? { ...r, ...patch } : r)); void save({ ...d, campioni: next }); };
      const head = ["File", "Role", "Time (min)", "Series", "Replicate", ""];
      box.append(el("div", { class: "flx-tablewrap", tabindex: 0, role: "region", "aria-label": "Sample list" }, el("table", { class: "flx-table" }, el("caption", { text: `${rows.length} files` }),
        el("thead", {}, el("tr", {}, ...head.map(h => el("th", { scope: "col", text: h })))),
        el("tbody", {}, ...rows.map((r, i) => {
          const timed = TIMED.includes(r.tipo);
          return el("tr", {},
            el("td", {}, el("input", { type: "text", value: r.file, "aria-label": `File ${i + 1}`, onchange: (/** @type {any} */ e) => upd(i, { file: e.target.value.trim() }) })),
            el("td", {}, el("select", { "aria-label": `Role of ${r.file || "file " + (i + 1)}`, onchange: (/** @type {any} */ e) => upd(i, { tipo: e.target.value, ...(TIMED.includes(e.target.value) ? {} : { tempo: null, tempo_min: null, tempo_confermato: false }) }) },
              ...(G.CDOC ? G.CDOC.KINDS : ["sample", "blank", "dark", "infusion", "std", "calibration", "qc"]).map((/** @type {string} */ k) => el("option", { value: k, text: k, selected: k === r.tipo })))),
            el("td", {}, el("input", { type: "number", step: "any", value: r.tempo_min ?? "", disabled: !timed, "aria-label": `Time of ${r.file || "file " + (i + 1)} in minutes`, onchange: (/** @type {any} */ e) => { const v = e.target.value === "" ? null : Number(e.target.value); upd(i, { tempo: v, tempo_min: v, tempo_confermato: v != null }); } })),
            el("td", {}, el("input", { type: "text", value: r.serie, "aria-label": `Series of ${r.file || "file " + (i + 1)}`, onchange: (/** @type {any} */ e) => upd(i, { serie: e.target.value.trim() }) })),
            el("td", {}, el("input", { type: "number", min: "1", step: "1", value: r.replica, "aria-label": `Replicate of ${r.file || "file " + (i + 1)}`, onchange: (/** @type {any} */ e) => upd(i, { replica: Math.max(1, Math.round(Number(e.target.value)) || 1) }) })),
            el("td", {}, F.btn("x", `Remove ${r.file || "file " + (i + 1)} from the list`, () => void save({ ...d, campioni: rows.filter((_, k) => k !== i) }), "Remove")));
        })))));
    };
    draw();
    return box;
  };
})();
