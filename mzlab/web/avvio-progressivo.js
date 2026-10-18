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
    if (stato === "ready" || stato === "failed") {
      info.hidden = true;
      info.textContent = "";
      return;
    }
    // pending
    info.hidden = false;
    const n = attesa.length;
    if (n === 0) {
      info.textContent = T("load.engine.preparing");
    } else {
      const txt = n === 1 ? T("load.engine.waitingOne", { n: 1 }) : T("load.engine.waiting", { n });
      info.textContent = "";
      const s = document.createElement("span");
      s.textContent = `${T("load.engine.preparing")} · ${txt} `;
      info.appendChild(s);
      const b = document.createElement("button");
      b.type = "button";
      b.className = "sec sm";
      b.id = "avvio-annulla";
      b.textContent = T("load.engine.cancel");
      b.onclick = annulla;
      info.appendChild(b);
    }
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
    el.hidden = false;
    el.textContent = "";
    const ph = typeof window !== "undefined" && window.qqPhaseCur ? window.qqPhaseCur : null;
    let txt = ph && typeof window !== "undefined" && window.I18N ? window.I18N.t(ph.key, ph.params) : (typeof window !== "undefined" && window.I18N ? window.I18N.t("load.engine.preparing") : "Preparing calculation engine…");
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
    if (d) d.hidden = ha;
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

  /**
   * Segna l'avvio come fallito.
   * @param {any} [err]
   */
  function segnaFallito(err) {
    if (stato !== "pending") return;
    stato = "failed";
    if (bootTimer) { clearTimeout(bootTimer); bootTimer = null; }
    timedOut = false;
    let msg = "";
    if (err && typeof err === "object" && err.key) {
      msg = (typeof window !== "undefined" && window.I18N ? window.I18N.t(err.key, err.params) : "") || "Calculation engine startup failed";
    } else if (typeof err === "string") {
      if (typeof window !== "undefined" && window.I18N) {
        msg = window.I18N.t(err);
      } else if (err.includes(".")) {
        msg = "Calculation engine startup failed";
      } else {
        msg = err;
      }
    } else if (err && err.message) {
      msg = err.message;
    } else {
      msg = "Calculation engine startup failed";
    }
    errore = msg;
    if (typeof window !== "undefined" && typeof window.fail === "function") {
      try { window.fail(msg); } catch (_) {}
    }
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
}
if (typeof module !== "undefined" && module.exports) {
  module.exports = AVVIO;
}
