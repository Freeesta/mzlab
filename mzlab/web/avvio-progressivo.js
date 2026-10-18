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

  // Se siamo su smartphone o in ambiente locale senza browser.js, il motore non usa il worker web:
  if (typeof window !== "undefined") {
    if (window.QQQ_PHONE || !window.QQQ_BROWSER) {
      stato = "ready";
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
    const T = (/** @type {string} */ k, /** @type {any} */ [p]) => (window.I18N ? I18N.t(k, p) : k);
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
    const coda = attesa;
    attesa = [];
    renderDropStato();
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
    errore = err || new Error("Engine startup failed");
    if (typeof window !== "undefined" && typeof window.fail === "function") {
      try { window.fail(errore); } catch (_) {}
    }
    const coda = attesa;
    attesa = [];
    renderDropStato();
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
    _resetTest,
  };
})();

if (typeof window !== "undefined") {
  /** @type {any} */ (window).AVVIO = AVVIO;
}
if (typeof module !== "undefined" && module.exports) {
  module.exports = AVVIO;
}
