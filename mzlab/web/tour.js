"use strict";
// Guided tour of the Dati view (low-resolution only).
// Global object TOUR, no external libraries. Pure JS, accessible dialog and spotlight.

function computePosition(rect, dialogWidth, dialogHeight, vw, vh) {
  if (vw < 700) {
    return {
      top: Math.max(8, vh - dialogHeight - 12),
      left: Math.max(8, Math.round((vw - dialogWidth) / 2)),
      side: "bottom-dock"
    };
  }
  const MARGIN = 12;
  // 1. Below
  let top = rect.bottom + MARGIN;
  let left = Math.max(8, Math.min(rect.left, vw - dialogWidth - 8));
  if (top + dialogHeight <= vh - 8) {
    return { top, left, side: "bottom" };
  }
  // 2. Above
  top = rect.top - MARGIN - dialogHeight;
  left = Math.max(8, Math.min(rect.left, vw - dialogWidth - 8));
  if (top >= 8) {
    return { top, left, side: "top" };
  }
  // 3. Right
  left = rect.right + MARGIN;
  top = Math.max(8, Math.min(rect.top, vh - dialogHeight - 8));
  if (left + dialogWidth <= vw - 8) {
    return { top, left, side: "right" };
  }
  // 4. Left
  left = rect.left - MARGIN - dialogWidth;
  top = Math.max(8, Math.min(rect.top, vh - dialogHeight - 8));
  if (left >= 8) {
    return { top, left, side: "left" };
  }
  // Fallback: pick the side with the most clearance
  const spaceBottom = vh - rect.bottom;
  const spaceTop = rect.top;
  if (spaceBottom >= spaceTop) {
    return { top: Math.min(vh - dialogHeight - 8, rect.bottom + MARGIN), left: Math.max(8, Math.min(rect.left, vw - dialogWidth - 8)), side: "bottom" };
  }
  return { top: Math.max(8, rect.top - MARGIN - dialogHeight), left: Math.max(8, Math.min(rect.left, vw - dialogWidth - 8)), side: "top" };
}

const TOUR = (() => {
  const STORAGE_KEY = "qqq.tour.lr";

  const STEPS_CAP1 = [
    {
      id: "file",
      prep: () => {
        const tab = document.getElementById("sb-tab-file");
        if (tab && tab.getAttribute("aria-selected") !== "true") tab.click();
        if (typeof foldFiles === "function" && typeof E !== "undefined" && E.fold) foldFiles(false);
      },
      target: () => document.getElementById("sb-panel-file")
    },
    {
      id: "schede",
      target: () => document.getElementById("dtabs")
    },
    {
      id: "tic",
      target: () => document.querySelector(".pnl.chrom") || (typeof E !== "undefined" && E.panels && E.panels.find(p => p.type === "chrom")?.el)
    },
    {
      id: "istante",
      target: () => document.querySelector(".pnl.chrom canvas"),
      prova: true,
      initProva: () => {
        const p = (typeof E !== "undefined" && E.panels) ? E.panels.find(q => q.type === "chrom") : null;
        return { cur: p ? p.cur : null, sel: p ? p.sel : null };
      },
      attesa: state => {
        const p = (typeof E !== "undefined" && E.panels) ? E.panels.find(q => q.type === "chrom") : null;
        if (!p) return false;
        return p.cur !== state.cur || p.sel !== state.sel;
      }
    },
    {
      id: "spettro",
      target: () => document.querySelector(".pnl.spec") || (typeof E !== "undefined" && E.panels && E.panels.find(p => p.type === "spec")?.el)
    },
    {
      id: "xic",
      target: () => document.querySelector(".pnl.spec") || (typeof E !== "undefined" && E.panels && E.panels.find(p => p.type === "spec")?.el),
      prova: true,
      initProva: () => {
        const count = (typeof E !== "undefined" && E.panels) ? E.panels.filter(p => p.type === "xic").length : 0;
        return { count };
      },
      attesa: state => {
        const count = (typeof E !== "undefined" && E.panels) ? E.panels.filter(p => p.type === "xic").length : 0;
        return count > state.count || document.querySelector(".pnl.xic") != null;
      }
    },
    {
      id: "xicpanel",
      target: () => document.querySelector(".pnl.xic") || document.getElementById("np-xic") || document.querySelector('#g-add button[data-a="xic"]')
    }
  ];

  const STEPS_CAP2 = [
    {
      id: "zoom",
      target: () => document.querySelector('.pnl.chrom [data-a="izoom"]')?.closest(".tbg") || document.querySelector('.pnl.chrom [data-a="izoom"]') || document.querySelector(".pnl.chrom .tbg")
    },
    {
      id: "tasti",
      target: () => document.querySelector(".pnl.chrom canvas")
    },
    {
      id: "navfile",
      target: () => document.getElementById("g-file")
    },
    {
      id: "aggiungi",
      target: () => document.getElementById("g-add")
    },
    {
      id: "pannello",
      target: () => document.querySelector(".pnl.chrom .tbs") || document.querySelector(".pnl.chrom .hd")
    },
    {
      id: "barra",
      target: () => document.getElementById("sidebar-tablist")
    },
    {
      id: "metodo",
      target: () => document.getElementById("np-method")
    },
    {
      id: "ingranaggio",
      target: () => document.getElementById("np-set")
    }
  ];

  let active = false;
  let curCap = 1;
  let curStep = 0;
  let prevFocus = null;
  let provaTimer = null;
  let provaState = null;
  let currentTargetEl = null;

  const isReducedMotion = () => {
    return window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  };

  let memState = null;   // fallback when localStorage is unavailable: the invitation must not return at every step
  const readState = () => {
    try { const v = localStorage.getItem(STORAGE_KEY); if (v) return v; } catch (_) {}
    return memState;
  };
  const saveState = status => {
    memState = status;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ stato: status, data: new Date().toISOString() }));
    } catch (_) {}
  };

  const ensureStyles = () => {
    if (document.getElementById("tour-styles")) return;
    const st = document.createElement("style");
    st.id = "tour-styles";
    st.textContent = `
      #tour-spotlight {
        position: fixed;
        pointer-events: none;
        border-radius: 8px;
        outline: 2px solid var(--accent, #2b5c8a);
        box-shadow: 0 0 0 100vmax rgba(0, 0, 0, 0.45);
        z-index: 6990;
        box-sizing: border-box;
      }
      #tour-dialog {
        position: fixed;
        z-index: 7000;
        width: 320px;
        max-width: calc(100vw - 16px);
        background: var(--panel, #fff);
        color: var(--ink, #25282c);
        border: 1px solid var(--line, #e2e5ea);
        border-radius: 8px;
        box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25);
        padding: 12px 14px;
        box-sizing: border-box;
        font: 13px/1.4 system-ui, -apple-system, sans-serif;
      }
      #tour-dialog .tour-arrow {
        position: absolute;
        width: 10px;
        height: 10px;
        background: var(--panel, #fff);
        border: 1px solid var(--line, #e2e5ea);
        transform: rotate(45deg);
      }
      #tour-dialog.tour-side-bottom .tour-arrow {
        top: -6px; left: 24px; border-right: none; border-bottom: none;
      }
      #tour-dialog.tour-side-top .tour-arrow {
        bottom: -6px; left: 24px; border-left: none; border-top: none;
      }
      #tour-dialog.tour-side-right .tour-arrow {
        left: -6px; top: 20px; border-top: none; border-right: none;
      }
      #tour-dialog.tour-side-left .tour-arrow {
        right: -6px; top: 20px; border-bottom: none; border-left: none;
      }
      #tour-dialog.tour-side-bottom-dock .tour-arrow {
        display: none;
      }
      #tour-dialog .tour-hd {
        font-size: 11px;
        font-weight: 600;
        color: var(--muted, #687080);
        margin-bottom: 4px;
      }
      #tour-dialog h3 {
        margin: 0 0 6px;
        font-size: 15px;
        font-weight: 650;
        color: var(--ink, #25282c);
      }
      #tour-dialog p {
        margin: 0 0 8px;
        font-size: 13px;
        line-height: 1.45;
        color: var(--ink, #25282c);
      }
      #tour-dialog .tour-try {
        margin: 0 0 10px;
        padding: 6px 8px;
        background: var(--soft, #f0f3f6);
        border-radius: 6px;
        font-size: 12px;
        color: var(--ink, #25282c);
        border-left: 3px solid var(--accent, #2b5c8a);
      }
      #tour-dialog .tour-ft {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-top: 10px;
        padding-top: 8px;
        border-top: 1px solid var(--line, #e2e5ea);
      }
      #tour-dialog .tour-btns {
        display: flex;
        gap: 6px;
        align-items: center;
      }
      #tour-btn-skip {
        background: none;
        border: none;
        padding: 0;
        color: var(--muted, #687080);
        text-decoration: underline;
        font-size: 12px;
        cursor: pointer;
      }
      #tour-btn-next, #tour-btn-cont, #tour-btn-fine {
        background: var(--accent, #2b5c8a);
        color: #fff;
        border: 1px solid var(--accent, #2b5c8a);
        border-radius: 5px;
        padding: 4px 12px;
        font-weight: 500;
        cursor: pointer;
      }
      #tour-btn-back, #tour-btn-fine-cap1 {
        background: var(--panel, #fff);
        color: var(--ink, #25282c);
        border: 1px solid var(--line, #e2e5ea);
        border-radius: 5px;
        padding: 4px 10px;
        cursor: pointer;
      }
      #tour-btn-back:disabled {
        opacity: 0.4;
        cursor: not-allowed;
      }
      #tour-live {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip: rect(0, 0, 0, 0);
      }
    `;
    document.head.appendChild(st);
  };

  const getSpotlightEl = () => {
    let el = document.getElementById("tour-spotlight");
    if (!el) {
      el = document.createElement("div");
      el.id = "tour-spotlight";
      document.body.appendChild(el);
    }
    return el;
  };

  const getDialogEl = () => {
    let el = document.getElementById("tour-dialog");
    if (!el) {
      el = document.createElement("div");
      el.id = "tour-dialog";
      el.setAttribute("role", "dialog");
      el.setAttribute("aria-modal", "false");
      el.setAttribute("aria-labelledby", "tour-ttl");
      el.setAttribute("aria-describedby", "tour-txt");
      document.body.appendChild(el);
    }
    return el;
  };

  const getLiveEl = () => {
    let el = document.getElementById("tour-live");
    if (!el) {
      el = document.createElement("div");
      el.id = "tour-live";
      el.setAttribute("aria-live", "polite");
      document.body.appendChild(el);
    }
    return el;
  };

  const updatePosition = () => {
    if (!active || !currentTargetEl) return;
    const r = currentTargetEl.getBoundingClientRect();
    const spot = getSpotlightEl();
    const dlg = getDialogEl();

    if (isReducedMotion()) {
      spot.style.transition = "none";
    } else {
      spot.style.transition = "top 150ms ease, left 150ms ease, width 150ms ease, height 150ms ease";
    }

    const pad = 6;
    spot.style.top = Math.max(0, r.top - pad) + "px";
    spot.style.left = Math.max(0, r.left - pad) + "px";
    spot.style.width = (r.width + pad * 2) + "px";
    spot.style.height = (r.height + pad * 2) + "px";

    const dlgW = dlg.offsetWidth || 320;
    const dlgH = dlg.offsetHeight || 160;
    const pos = computePosition(r, dlgW, dlgH, window.innerWidth, window.innerHeight);

    dlg.style.top = pos.top + "px";
    dlg.style.left = pos.left + "px";
    dlg.className = "tour-side-" + pos.side;
  };

  let rafScroll = null;
  const onScrollOrResize = () => {
    if (!active) return;
    if (rafScroll) cancelAnimationFrame(rafScroll);
    rafScroll = requestAnimationFrame(updatePosition);
  };

  const stopProvaTimer = () => {
    if (provaTimer) {
      clearInterval(provaTimer);
      provaTimer = null;
    }
    provaState = null;
  };

  const renderStep = () => {
    stopProvaTimer();
    const list = curCap === 1 ? STEPS_CAP1 : STEPS_CAP2;
    if (curStep >= list.length) {
      if (curCap === 1) {
        TOUR.continua();
        return;
      }
      TOUR.fine();
      return;
    }

    const step = list[curStep];
    if (step.prep) step.prep();

    const targetEl = step.target();
    if (!targetEl || targetEl.offsetParent === null || targetEl.getBoundingClientRect().width === 0) {
      console.debug("Tour: target missing for step", step.id);
      curStep++;
      renderStep();
      return;
    }

    currentTargetEl = targetEl;
    targetEl.scrollIntoView({ block: "center", behavior: isReducedMotion() ? "auto" : "smooth" });

    const totalSteps = list.length;
    const stepNumber = curStep + 1;
    const chapterName = I18N.t(curCap === 1 ? "tour.cap1" : "tour.cap2");
    const headerText = I18N.t("tour.passo", { n: stepNumber, tot: totalSteps, cap: chapterName });
    const titleText = I18N.t(`tour.${step.id}.titolo`);
    const bodyText = I18N.t(`tour.${step.id}.testo`);
    const provaText = step.prova ? I18N.t(`tour.${step.id}.prova`) : "";

    const dlg = getDialogEl();
    const isCap1End = curCap === 1 && curStep === list.length - 1;
    const isCap2End = curCap === 2 && curStep === list.length - 1;

    let tryHtml = "";
    if (step.prova && provaText) {
      tryHtml = `<div class="tour-try"><b>${I18N.t("tour.prova")}</b> ${provaText}</div>`;
    }

    let endCap1Html = "";
    if (isCap1End) {
      endCap1Html = `<p style="font-weight:600;margin-top:6px;border-top:1px solid var(--line);padding-top:6px">${I18N.t("tour.fine_cap1_testo")}</p>`;
    }

    let buttonsHtml = "";
    if (isCap1End) {
      buttonsHtml = `
        <button type="button" id="tour-btn-back">${I18N.t("tour.indietro")}</button>
        <button type="button" id="tour-btn-fine-cap1">${I18N.t("tour.fine")}</button>
        <button type="button" id="tour-btn-cont">${I18N.t("tour.continua")}</button>
      `;
    } else if (isCap2End) {
      buttonsHtml = `
        <button type="button" id="tour-btn-back">${I18N.t("tour.indietro")}</button>
        <button type="button" id="tour-btn-fine">${I18N.t("tour.fine")}</button>
      `;
    } else {
      const backDisabled = (curCap === 1 && curStep === 0) ? "disabled" : "";
      buttonsHtml = `
        <button type="button" id="tour-btn-back" ${backDisabled}>${I18N.t("tour.indietro")}</button>
        <button type="button" id="tour-btn-next">${I18N.t("tour.avanti")}</button>
      `;
    }

    dlg.innerHTML = `
      <div class="tour-arrow"></div>
      <div class="tour-hd">${headerText}</div>
      <h3 id="tour-ttl">${titleText}</h3>
      <p id="tour-txt">${bodyText}</p>
      ${tryHtml}
      ${endCap1Html}
      <div class="tour-ft">
        <button type="button" id="tour-btn-skip">${I18N.t("tour.salta")}</button>
        <div class="tour-btns">${buttonsHtml}</div>
      </div>
    `;

    // Live announcer for screen readers
    const live = getLiveEl();
    live.textContent = I18N.t("tour.annuncio", { n: stepNumber, tot: totalSteps, titolo: titleText });

    // Bind event handlers
    const btnSkip = dlg.querySelector("#tour-btn-skip");
    if (btnSkip) btnSkip.onclick = () => TOUR.salta();

    const btnBack = dlg.querySelector("#tour-btn-back");
    if (btnBack) btnBack.onclick = () => TOUR.indietro();

    const btnNext = dlg.querySelector("#tour-btn-next");
    if (btnNext) btnNext.onclick = () => TOUR.avanti();

    const btnCont = dlg.querySelector("#tour-btn-cont");
    if (btnCont) btnCont.onclick = () => TOUR.continua();

    const btnFineCap1 = dlg.querySelector("#tour-btn-fine-cap1");
    if (btnFineCap1) btnFineCap1.onclick = () => TOUR.fine();

    const btnFine = dlg.querySelector("#tour-btn-fine");
    if (btnFine) btnFine.onclick = () => TOUR.fine();

    updatePosition();
    setTimeout(updatePosition, 160);

    // Focus primary action button
    const priBtn = btnNext || btnCont || btnFine;
    if (priBtn) priBtn.focus();

    // Setup "Prova tu" observation
    if (step.prova && step.attesa && step.initProva) {
      provaState = step.initProva();
      provaTimer = setInterval(() => {
        if (!active) {
          stopProvaTimer();
          return;
        }
        if (step.attesa(provaState)) {
          stopProvaTimer();
          TOUR.avanti();
        }
      }, 300);
    }
  };

  const onKeyDown = e => {
    if (!active) return;
    const dlg = document.getElementById("tour-dialog");
    if (!dlg || !dlg.contains(e.target)) return;

    if (e.key === "Escape") {
      e.preventDefault();
      TOUR.salta();
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      TOUR.avanti();
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      TOUR.indietro();
    } else if (e.key === "Enter") {
      if (e.target.tagName !== "BUTTON") {
        e.preventDefault();
        TOUR.avanti();
      }
    }
  };

  return {
    get active() { return active; },
    get cap() { return curCap; },
    get step() { return curStep; },

    offri: () => {
      if (window.QQQ_PHONE) return;
      if (window.BANCO && BANCO.on()) return;
      if (window.TPMINE && TPMINE.on) return;
      if (typeof E === "undefined" || !E.files || !E.files.length) return;
      if (readState()) return;
      saveState("invitato");   // the invitation appears once per browser, whatever the answer
      TOUR.mostraInvito();
    },

    mostraInvito: () => {
      if (document.getElementById("tour-invito")) return;
      ensureStyles();
      const div = document.createElement("div");
      div.id = "tour-invito";
      div.setAttribute("role", "dialog");
      div.setAttribute("aria-modal", "false");
      div.setAttribute("aria-label", I18N.t("tour.impost.titolo"));
      div.style.cssText = "position:fixed;bottom:12px;left:12px;z-index:7000;width:260px;max-width:calc(100vw - 24px);background:var(--panel,#fff);color:var(--ink,#25282c);border:1px solid var(--line,#e2e5ea);border-radius:8px;box-shadow:0 4px 16px rgba(0,0,0,0.18);padding:12px 14px;box-sizing:border-box;font:13px/1.4 system-ui,-apple-system,sans-serif";
      div.innerHTML = `
        <p style="margin:0 0 10px;line-height:1.45">${I18N.t("tour.invito.testo")}</p>
        <div style="display:flex;gap:8px;justify-content:flex-end">
          <button type="button" id="tour-inv-no" style="background:var(--panel,#fff);color:var(--ink,#25282c);border:1px solid var(--line,#e2e5ea);border-radius:5px;padding:4px 10px;cursor:pointer">${I18N.t("tour.invito.no")}</button>
          <button type="button" id="tour-inv-si" style="background:var(--accent,#2b5c8a);color:#fff;border:1px solid var(--accent,#2b5c8a);border-radius:5px;padding:4px 12px;font-weight:500;cursor:pointer">${I18N.t("tour.invito.si")}</button>
        </div>
      `;
      document.body.appendChild(div);

      const closeInv = () => {
        if (div.parentNode) div.remove();
        document.removeEventListener("keydown", onInvKeyDown);
      };

      const onInvKeyDown = e => {
        if (e.key === "Escape") {
          closeInv();
        }
      };

      document.addEventListener("keydown", onInvKeyDown);
      try { div.querySelector("#tour-inv-si").focus({ preventScroll: true }); } catch (_) {}

      div.querySelector("#tour-inv-si").onclick = () => {
        closeInv();
        TOUR.inizia(1, 0);
      };
      div.querySelector("#tour-inv-no").onclick = () => {
        closeInv();
        try { document.getElementById("np-set")?.focus({ preventScroll: true }); } catch (_) {}
      };
    },

    chiudiInvito: () => {
      const el = document.getElementById("tour-invito");
      if (el) el.remove();
    },

    inizia: (cap = 1, step = 0) => {
      TOUR.chiudiInvito();
      ensureStyles();
      const act = document.activeElement;
      prevFocus = (act && act !== document.body && !act.closest("#uipset")) ? act : document.getElementById("np-set");
      active = true;
      curCap = cap;
      curStep = step;

      window.addEventListener("scroll", onScrollOrResize, { passive: true });
      window.addEventListener("resize", onScrollOrResize, { passive: true });
      document.addEventListener("keydown", onKeyDown);

      renderStep();
    },

    vai: (cap, step) => {
      if (!active) {
        TOUR.inizia(cap, step);
        return;
      }
      curCap = cap;
      curStep = step;
      renderStep();
    },

    avanti: () => {
      if (!active) return;
      const list = curCap === 1 ? STEPS_CAP1 : STEPS_CAP2;
      if (curStep < list.length - 1) {
        curStep++;
        renderStep();
      } else if (curCap === 1) {
        TOUR.continua();
      } else {
        TOUR.fine();
      }
    },

    indietro: () => {
      if (!active) return;
      if (curStep > 0) {
        curStep--;
        renderStep();
      } else if (curCap === 2) {
        curCap = 1;
        curStep = STEPS_CAP1.length - 1;
        renderStep();
      }
    },

    continua: () => {
      curCap = 2;
      curStep = 0;
      renderStep();
    },

    fine: () => {
      saveState("fatto");
      TOUR.chiudi();
    },

    salta: () => {
      saveState("saltato");
      TOUR.chiudi();
    },

    chiudi: () => {
      stopProvaTimer();
      active = false;
      currentTargetEl = null;

      window.removeEventListener("scroll", onScrollOrResize);
      window.removeEventListener("resize", onScrollOrResize);
      document.removeEventListener("keydown", onKeyDown);

      const spot = document.getElementById("tour-spotlight");
      if (spot) spot.remove();
      const dlg = document.getElementById("tour-dialog");
      if (dlg) dlg.remove();

      const targetFocus = (prevFocus && document.body.contains(prevFocus) && prevFocus !== document.body) ? prevFocus : document.getElementById("np-set");
      if (targetFocus) {
        try { targetFocus.focus(); } catch (_) {}
      }
    },

    ridisegna: () => {
      if (!active) return;
      renderStep();
    },

    caricaEsempio: async () => {
      if (typeof loading === "function") loading(true, I18N.t("load.demo.opening"));
      try {
        const res = await fetch("static/esempi/FullScan_t15.mzML");
        if (!res.ok) throw new Error("FullScan_t15.mzML: " + res.status);
        const blob = await res.blob();
        const file = new File([blob], "Esempio_FullScan_t15.mzML");
        if (typeof upload === "function") {
          await upload([file], "data");
        }
        const op = document.getElementById("opbtn");
        if (op && !op.disabled) {
          op.click();
        }
        const poll = setInterval(() => {
          const l = document.getElementById("loading");
          if ((!l || l.hidden) && typeof E !== "undefined" && E.panels && E.panels.length > 0) {
            clearInterval(poll);
            TOUR.inizia(1, 0);
          }
        }, 200);
        setTimeout(() => clearInterval(poll), 30000);
      } catch (err) {
        if (typeof loading === "function") loading(false);
        console.error("Tour example load error:", err);
      }
    }
  };
})();

if (typeof window !== "undefined") {
  window.TOUR = TOUR;
  // «New session» / «Start from scratch»: the invitation is a user datum too, so it comes back with the clean slate
  const hard = window.qqHardReset;
  if (typeof hard === "function") {
    window.qqHardReset = () => { try { localStorage.removeItem("qqq.tour.lr"); } catch (_) {} return hard(); };
  }
}

if (typeof module !== "undefined") {
  module.exports = { computePosition, TOUR };
}
