// @ts-check
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
    if (!ph && !timedOut) {
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

    if (timedOut) {
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

  function segnaSessione(attiva) {
    if (typeof window === "undefined" || !window.localStorage) return;
    try {
      const k = typeof window.NOMI !== "undefined" ? window.NOMI.KEY("sessione") : "mzlab.sessione";
      if (attiva) window.localStorage.setItem(k, "1");
      else window.localStorage.removeItem(k);
    } catch (_) {}
  }

  function haSessione() {
    if (typeof window === "undefined" || !window.localStorage) return false;
    try {
      const k = typeof window.NOMI !== "undefined" ? window.NOMI.KEY("sessione") : "mzlab.sessione";
      return window.localStorage.getItem(k) === "1";
    } catch (_) { return false; }
  }

  function applicaIndizioSessione() {
    if (typeof document === "undefined") return;
    const d = document.getElementById("drop");
    const s = document.getElementById("drop-skeleton");
    const ha = haSessione();
    // the skeleton takes the height of the drop box it replaces: when the session turns out not to exist, nothing moves
    if (d && ha && !d.hidden && d.getBoundingClientRect().height > 0 && s) s.style.minHeight = d.getBoundingClientRect().height + "px";
    if (d) d.hidden = ha;
    const nr = document.getElementById("newrun");
    if (nr && ha) nr.hidden = false;   // the session is known from localStorage: its button is there at the first paint (shell.js decides the same later)
    if (s) {
      s.hidden = !ha;
      if (ha && !s.textContent) s.textContent = typeof window !== "undefined" && window.I18N ? window.I18N.t("load.session.restoring") : "Restoring session…";
    }
  }

  function rimuoviScheletro() {
    if (typeof document === "undefined") return;
    const d = document.getElementById("drop");
    const s = document.getElementById("drop-skeleton");
    if (d) d.hidden = false;
    if (s) s.hidden = true;
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
