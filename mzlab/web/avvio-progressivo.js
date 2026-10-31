// @ts-strict
"use strict";

/**
 * @typedef {'pending' | 'ready' | 'failed'} AvvioStato
 */

const AVVIO = (function () {
  /** @type {AvvioStato} */
  let stato = "pending";
  /** @type {any} */
  let errore = null;
  /** the catalog key of the failure (for the details to copy) */
  let chiave = "";
  /** @type {Array<(ok: boolean) => void>} */
  let attesa = [];
  /** @type {any} */
  let bootTimer = null;
  let timedOut = false;

  const T = (k, p) => (typeof window !== "undefined" && window.I18N ? window.I18N.t(k, p) : k);

  // Se siamo su smartphone o in ambiente locale senza browser.js, il motore non usa il worker web:
  if (typeof window !== "undefined") {
    if (window.QQQ_PHONE || !window.QQQ_BROWSER) {
      stato = "ready";
    finisciRiga();
    } else {
      bootTimer = setTimeout(() => {
        if (stato !== "pending") return;
        timedOut = true;
        renderFase();
      }, 10000);
    }
  }

  function renderDropStato() {
    if (typeof document === "undefined") return;
    const info = document.getElementById("avvio-drop-info");
    if (!info) return;
    if (stato === "ready" || stato === "failed" || attesa.length === 0) {
      info.hidden = true;
      info.textContent = "";
      return;
    }
    // pending with files waiting
    info.hidden = false;
    const n = attesa.length;
    const txt = n === 1 ? T("load.engine.waitingOne", { n: 1 }) : T("load.engine.waiting", { n });
    info.textContent = "";
    const s = document.createElement("span");
    s.textContent = `${txt} `;
    info.appendChild(s);
    const b = document.createElement("button");
    b.type = "button";
    b.className = "sec sm";
    b.id = "avvio-annulla";
    b.textContent = T("load.engine.cancel");
    b.onclick = annulla;
    info.appendChild(b);
  }

  function renderFase() {
    if (typeof document === "undefined") return;
    const el = document.getElementById("avvio-fase");
    if (!el) return;
    if (stato !== "pending") {
      if (bootTimer) { clearTimeout(bootTimer); bootTimer = null; }
      el.hidden = true;
      el.textContent = "";
      return;
    }
    const ph = typeof window !== "undefined" && window.qqPhaseCur ? window.qqPhaseCur : null;
    // the way out of a stuck start shows only when files are actually waiting for the engine
    const stuck = timedOut && attesa.length > 0;
    if (!ph && !stuck) {
      el.hidden = true;
      el.textContent = "";
      return;
    }
    el.hidden = false;
    el.textContent = "";
    let txt = ph && typeof window !== "undefined" && window.I18N ? window.I18N.t(ph.key, ph.params) : "";
    if (ph && ph.pct != null) {
      txt += ` ${Math.max(0, Math.min(100, Math.round(ph.pct)))}%`;
    }
    const span = document.createElement("span");
    span.textContent = txt;
    el.appendChild(span);

    if (stuck) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "btn sec sm";
      btn.id = "avvio-fase-reset";
      btn.style.marginLeft = "auto";
      btn.textContent = typeof window !== "undefined" && window.I18N ? window.I18N.t("reset.all.btn") : "Full reset";
      btn.onclick = () => {
        if (typeof window !== "undefined") {
          if (typeof window.qqHardReset === "function") window.qqHardReset();
          else if (typeof window.ldFullReset === "function") window.ldFullReset();
          else location.reload();
        }
      };
      el.appendChild(btn);
    }
  }

  const BOTTONI_RIGA = ["np-chrom", "np-spec", "np-map", "np-xic", "np-mrm"];

  function chiaveRiga() {
    return typeof window.NOMI !== "undefined" ? window.NOMI.KEY("sessione.riga") : "mzlab.sessione.riga";
  }

  /**
   * Remembers the second toolbar row of the Dati view, so that the next restore draws it at its final size in the first paint.
   * Partial objects are merged into what is stored; `null` forgets it (high-resolution bench, no session).
   * @param {{a?: number[], f?: string, fw?: number, s?: string} | null} riga a: which "+ panel" buttons, f: file selector label, fw: its width, s: side panel width
   */
  function segnaRiga(riga) {
    if (typeof window === "undefined" || !window.localStorage) return;
    try {
      if (riga) window.localStorage.setItem(chiaveRiga(), JSON.stringify({ ...JSON.parse(window.localStorage.getItem(chiaveRiga()) || "{}"), ...riga }));
      else window.localStorage.removeItem(chiaveRiga());
    } catch (_) {}
  }

  function leggiRiga() {
    try {
      const r = JSON.parse(window.localStorage.getItem(chiaveRiga()) || "null");
      if (r && Array.isArray(r.a) && r.a.length === BOTTONI_RIGA.length && r.a.every(/** @param {any} x */ x => x === 0 || x === 1) && typeof r.f === "string" && typeof r.fw === "number" && /^\d{2,4}px$/.test(r.s || "")) return r;
    } catch (_) {}
    return null;
  }

  // The row of «+ panel» buttons is drawn in the first paint of a restore, as the last session left it; its buttons stay disabled until the session is back
  function mostraRigaRipristino() {
    const g = (/** @type {string} */ id) => document.getElementById(id), v = g("v-data"), riga = leggiRiga();
    if (!v || !g("tools") || !riga) return;
    BOTTONI_RIGA.forEach((id, i) => { const b = g(id); if (b) b.hidden = !riga.a[i]; });
    const sel = /** @type {HTMLSelectElement | null} */ (g("fsel"));
    if (sel && !sel.options.length) { sel.add(new Option(riga.f)); sel.style.width = riga.fw + "px"; }
    document.documentElement.style.setProperty("--sb-w", riga.s);
    v.classList.add("riga");
  }

  // On leaving the page, the row as the last session left it: the next restore draws it in the first paint
  function ricordaRiga() {
    const g = (/** @type {string} */ id) => document.getElementById(id), v = g("v-data"), sel = /** @type {HTMLSelectElement | null} */ (g("fsel"));
    if (!v || v.hidden || v.classList.contains("riga") || !sel || !haSessione()) return;
    const a = BOTTONI_RIGA.map(id => { const b = g(id); return b && !b.hidden ? 1 : 0; });
    if (!a.some(Boolean) || (window.BANCO && window.BANCO.on())) segnaRiga(null);
    else segnaRiga({ a, f: (sel.selectedOptions[0] || { text: "" }).text.slice(0, 120), fw: Math.round(sel.getBoundingClientRect().width), s: document.documentElement.style.getPropertyValue("--sb-w") });
  }

  function finisciRiga() {
    if (typeof document === "undefined") return;
    const v = document.getElementById("v-data"), sel = document.getElementById("fsel");
    if (v) v.classList.remove("riga");
    if (sel) sel.style.width = "";
  }

  function segnaSessione(attiva) {
    if (typeof window === "undefined" || !window.localStorage) return;
    try {
      const k = typeof window.NOMI !== "undefined" ? window.NOMI.KEY("sessione") : "mzlab.sessione";
      if (attiva) window.localStorage.setItem(k, "1");
      else { window.localStorage.removeItem(k); segnaRiga(null); }
    } catch (_) {}
  }

  function haSessione() {
    if (typeof window === "undefined" || !window.localStorage) return false;
    try {
      const k = typeof window.NOMI !== "undefined" ? window.NOMI.KEY("sessione") : "mzlab.sessione";
      return window.localStorage.getItem(k) === "1";
    } catch (_) { return false; }
  }

  // saved session: «restoring», not the landing
  function applicaIndizioSessione() {
    if (typeof document === "undefined" || !haSessione()) return;
    const g = id => document.getElementById(id), o = g("opening"), w = g("wdata"), v = g("v-data"), m = g("opn-msg");
    if (!o || !w) return;
    w.hidden = true;
    o.hidden = false;
    o.classList.add("restoring");
    if (v) v.classList.add("opn");
    if (m && !m.textContent) m.textContent = window.I18N ? I18N.t("load.session.restoring") : "Restoring…";
    mostraRigaRipristino();
  }

  function rimuoviScheletro() {
    if (typeof document === "undefined") return;
    const g = id => document.getElementById(id), o = g("opening"), w = g("wdata"), v = g("v-data"), m = g("opn-msg");
    finisciRiga();
    if (!o || !o.classList.contains("restoring")) return;
    o.classList.remove("restoring"); o.hidden = true;
    if (w) w.hidden = false;
    if (v) v.classList.remove("opn");
    if (m) m.textContent = "";
  }

  /**
   * Attende che il motore sia pronto prima di procedere con l'upload.
   * @returns {Promise<boolean>}
   */
  function pronto() {
    if (stato === "ready") return Promise.resolve(true);
    if (stato === "failed") return Promise.resolve(false);
    return new Promise(resolve => {
      attesa.push(resolve);
      renderDropStato();
      renderFase();
    });
  }

  /**
   * Segna il motore come pronto (chiamato alla fine di init()).
   */
  function segnaPronto() {
    if (stato !== "pending") return;
    stato = "ready";
    if (bootTimer) { clearTimeout(bootTimer); bootTimer = null; }
    timedOut = false;
    const coda = attesa;
    attesa = [];
    renderDropStato();
    renderFase();
    for (let i = 0; i < coda.length; i++) {
      coda[i](true);
    }
  }

  const GENERICO = "Calculation engine startup failed";
  /** a catalog key looks like `load.err.engineStart`: dots, no spaces (a real message with a full stop is not a key) */
  const eChiave = s => /^[A-Za-z][\w-]*(\.[\w-]+)+$/.test(s);
  /** the text of a catalog key; the generic English one when the catalog is not there or does not know the key */
  function testoChiave(key, params) {
    const I = typeof window !== "undefined" ? window.I18N : null;
    const t = I ? I.t(key, params) : "";
    return t && t !== key ? t : GENERICO;
  }
  /** a label of the catalog, or its English fallback */
  function lab(k, en) {
    const I = typeof window !== "undefined" ? window.I18N : null;
    const t = I ? I.t(k) : "";
    return t && t !== k ? t : en;
  }

  /** the text to copy: version, key, step, technical text; no file name, no path */
  function dettagli() {
    const w = typeof window !== "undefined" ? /** @type {any} */ (window) : {};
    const pulisci = s => String(w.DIAG && w.DIAG.pulisci ? w.DIAG.pulisci(s) : String(s).replace(/[\\/][^\s"']*/g, "<path>")).replace(/\w+:\/\/[^\s"'<>]*/g, "<url>");
    const R = [`mzLab ${w.MZLAB_VERSION || "?"}`, `error: ${errore || ""}`, `key: ${chiave}`, `step: ${w.qqStep || ""}`];
    if (w.MZLAB_LR_ERR) R.push(`engine: ${pulisci(w.MZLAB_LR_ERR)}`);
    if (typeof navigator !== "undefined") R.push(`agent: ${navigator.userAgent}`);
    return R.join("\n");
  }

  /** the fixed banner at the top (not modal, role=alert): the message, «Copy details», «Full reset»; the page stays visible */
  function mostraBanner() {
    if (typeof document === "undefined" || !document.body) return;
    let b = document.getElementById("avvio-errore");
    if (!b) {
      b = document.createElement("div");
      b.id = "avvio-errore"; b.setAttribute("role", "alert");
      document.body.insertBefore(b, document.body.firstChild);
    }
    b.textContent = "";
    const m = document.createElement("span"); m.id = "avvio-errore-msg"; m.textContent = errore || GENERICO; b.appendChild(m);
    const c = document.createElement("button");
    c.type = "button"; c.id = "avvio-copia"; c.className = "sec sm"; c.textContent = lab("load.fail.copy", "Copy details");
    c.onclick = async () => {
      try { await navigator.clipboard.writeText(dettagli()); c.textContent = lab("diag.copied", "Copied"); } catch (_) { c.textContent = lab("diag.copyFail", "Copy failed"); }
    };
    b.appendChild(c);
    const r = document.createElement("button");
    r.type = "button"; r.id = "avvio-reset"; r.className = "btn sec sm"; r.textContent = lab("reset.all.btn", "Full reset");
    r.onclick = () => {
      r.disabled = true;
      const w = /** @type {any} */ (window);
      if (typeof w.qqHardReset === "function") w.qqHardReset(); else if (typeof w.ldFullReset === "function") w.ldFullReset(); else location.reload();
    };
    b.appendChild(r);
  }

  /**
   * Segna l'avvio come fallito: UNA sola fonte per il testo (`errore` = testo del banner).
   * @param {any} [err] `{ key, params }`, una chiave di catalogo, un testo vero o un Error
   */
  function segnaFallito(err) {
    if (stato !== "pending") return;
    stato = "failed";
    if (bootTimer) { clearTimeout(bootTimer); bootTimer = null; }
    timedOut = false;
    let msg = "";
    chiave = "";
    if (err && typeof err === "object" && err.key) { chiave = String(err.key); msg = testoChiave(chiave, err.params); }
    else if (typeof err === "string" && eChiave(err)) { chiave = err; msg = testoChiave(err); }
    else if (typeof err === "string" && err) msg = err;
    else if (err && err.message) msg = err.message;
    else msg = GENERICO;
    errore = msg;
    rimuoviScheletro();
    mostraBanner();
    const coda = attesa;
    attesa = [];
    renderDropStato();
    renderFase();
    for (let i = 0; i < coda.length; i++) {
      coda[i](false);
    }
  }

  /**
   * Annulla i caricamenti attualmente in attesa del motore.
   */
  function annulla() {
    const coda = attesa;
    attesa = [];
    renderDropStato();
    for (let i = 0; i < coda.length; i++) {
      coda[i](false);
    }
  }

  /**
   * Utility per i test unitari per reimpostare lo stato.
   * @param {AvvioStato} [st]
   */
  function _resetTest(st = "pending") {
    stato = st;
    errore = null;
    attesa = [];
    if (bootTimer) { clearTimeout(bootTimer); bootTimer = null; }
    timedOut = false;
  }

  if (typeof window !== "undefined" && window.addEventListener) window.addEventListener("pagehide", ricordaRiga);

  if (typeof document !== "undefined") {
    applicaIndizioSessione();
    renderFase();
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", () => {
        applicaIndizioSessione();
        renderFase();
      });
    }
  }

  return {
    get stato() { return stato; },
    get errore() { return errore; },
    get inAttesa() { return attesa.length; },
    pronto,
    segnaPronto,
    segnaFallito,
    annulla,
    renderDropStato,
    renderFase,
    segnaSessione,
    segnaRiga,
    haSessione,
    applicaIndizioSessione,
    rimuoviScheletro,
    _resetTest,
  };
})();

if (typeof window !== "undefined") {
  /** @type {any} */ (window).AVVIO = AVVIO;
  // browser.js can fail before this module is loaded (old browser): it leaves {key, params} in window.qqFailInfo
  const f = /** @type {any} */ (window).qqFailInfo;
  if (f) AVVIO.segnaFallito(f);
}
if (typeof module !== "undefined" && module.exports) {
  module.exports = AVVIO;
}
