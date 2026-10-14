// @ts-check
"use strict";
// Warnings of the engine (`valida`) for the loading table: the document sent, the findings, the files whose time is required.
Object.assign(RUOLI, (() => {
  const { post, val, roleOf, t, REQUIRED } = RUOLI;
  // ---------------------------------------------------------------- warnings (valida)
  /** @type {{sig: string, names: string[], avvisi: {codice: string, file: number|null}[]}} */   // `file` of a finding = position among the files sent (the ones in use)
  let last = { sig: "", names: [], avvisi: [] };
  let seq = 0;
  function doc(/** @type {Voce[]} */ F) {
    return JSON.stringify({ schema: "campioni/2", campioni: F.filter(f => f.use).map((f, i) => {
      const r = roleOf(f);
      return { file: f.name, sha256: "", tipo: r, tipo_bianco: r === "blank" ? f.sub || null : null, tempo: f.time ?? null, unita_tempo: "min", tempo_min: f.time ?? null,
        tempo_confermato: f.time != null && (r === "t0" || !f.gT), serie: "", replica: 1, concentrazione: null, unita: null, ordine_iniezione: i + 1, istd: false, note: "", composto: "" };
    }) });
  }
  async function refresh(/** @type {Voce[]} */ F) {
    const sig = doc(F), n = ++seq;
    if (sig === last.sig) return last;
    let r;
    try { r = await post("valida", JSON.parse(sig)); } catch (_) { return last; }
    if (n === seq) last = { sig, names: F.filter(f => f.use).map(f => f.name), avvisi: (r && r.avvisi) || [] };
    return last;
  }
  const treated = (/** @type {Voce[]} */ F) => F.filter(f => f.use && val(f) === "sample");
  /** the files whose time is missing or unconfirmed, when there are at least two samples */
  function required(/** @type {Voce[]} */ F) {
    const n = treated(F).length;
    return n < 2 ? new Set() : new Set(last.avvisi.filter(a => REQUIRED.has(a.codice) && a.file != null).map(a => last.names[/** @type {number} */ (a.file)]));
  }
  /** warning texts (priority order) */
  function texts(/** @type {Voce[]} */ F) {
    if (treated(F).length < 2) return [];
    const has = (/** @type {string} */ c) => last.avvisi.some(a => a.codice === c);
    const out = [];
    if (last.avvisi.some(a => a.codice === "timeMissing")) out.push(t("load.warn.missing"));
    if (last.avvisi.some(a => a.codice === "timeUnconfirmed")) out.push(t("load.warn.unconfirmed"));
    if (has("noT0") || has("noControls")) out.push(t("load.warn.noT0"));
    if (has("noBlank") || has("noControls")) out.push(t("load.warn.noBlank"));
    if (has("onePoint")) out.push(t("load.warn.onePoint"));
    return out;
  }

  return { doc, refresh, treated, required, texts };
})());
