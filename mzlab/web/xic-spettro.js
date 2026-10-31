// @ts-strict
"use strict";
// Button «XIC» of the mass spectrum (Full Scan, MS2, DDA, map, several files): extracts at once the XIC of the ion chosen in the spectrum and shows it in the
// panel right under it. State per spectrum panel: the chosen peak (ring on the spectrum), its tolerance, the XIC panel it owns. The XIC follows the scan of the
// spectrum (cursor line, band for an averaged spectrum) and says the apex of the peak that holds that scan. Every number comes from the engine through the
// existing XIC panel and the existing peak detectors (autoEdges / autoInt); nothing is computed here.
// Classic helpers used by name: getXic, scanFiles, shownFor, TOL0, xicDirect, xicTraceFor, xicWin, rh, mzd, fmz, numIt, ctl, draw, cursorLine, autoEdges, autoInt, PK, HR, E, ESC, AZIONI.
(() => {
  /** @param {string} k @param {any} [v] */
  const T = (k, v) => I18N.t(k, v);
  /** @typedef {{ m: number, obs: boolean, tol: number | null }} Sel  tol: null = the tolerance of the files; ppm (high resolution) or Da (unit resolution) */
  /** @type {WeakMap<any, { sel: Sel | null, pend: boolean, xp: any, pop: HTMLElement | null, tok: number, xtok: number, raf: number }>} */
  const ST = new WeakMap();
  /** @param {any} p */
  const st = p => { let s = ST.get(p); if (!s) { s = { sel: null, pend: false, xp: null, pop: null, tok: 0, xtok: 0, raf: 0 }; ST.set(p, s); } return s; };
  /** @param {any} p */
  const live = p => { const s = ST.get(p); return s && s.xp && E.panels.includes(s.xp) ? s.xp : null; };
  /** the file of the spectrum @param {any} p */
  const specFile = p => p._a && p._a.data && p._a.data[0] ? p._a.data[0].f : null;
  /** an infusion has no real chromatogram: the XIC button and the lens of its spectrum are off (the kind of the engine, infusione.rs, or the same words in the file name) @param {any} p */
  const infusion = p => { const f = specFile(p); return !!f && (f.kind === "infusion" || !f.manual && /infus|fia|direct|syringe/i.test(String(f.label || f.name || "").split(/[\\/]/).pop() || "") || (!!window.CANALI && ESP.predefinito(CANALI.tipo(f)).lente === false)); };      // the table of the types of experiment (esperimenti.js) says whether the lens is proposed
  /** the XIC lens (the preview on peak hover and its switch) exists only on the MS1 spectrum of a high-resolution file: not on MS2, not at low resolution, not on infusion @param {any} p */
  const lens = { ok: (/** @type {any} */ p) => { const f = specFile(p); return !!f && !infusion(p) && p.level === 1 && !!window.HR && HR.isHr(f, 1); } };       // an object so that a test can lift the condition
  /** the high-resolution file among those the XIC reads, if any @param {any} p */
  const hrFile = p => (window.HR ? HR.xicFiles(p.tab).find((/** @type {any} */ f) => HR.isHr(f, f.lv)) : null) || null;
  const num = (/** @type {number} */ v) => String(+v.toFixed(5));
  const ref = (/** @type {any} */ p) => ({ tab: p.tab });

  // ---------------------------------------------------------------- ion, window and trace
  /** the tolerance of the files: ppm of the profile (high resolution) or half of the unit window (Da) @param {any} p */
  function defTol(p) {
    const hf = hrFile(p);
    return hf ? { hr: true, v: HR.prof(hf, hf.lv).tol } : { hr: false, v: 0.5 };
  }
  /** the window [lo, hi] in Da the low-resolution files read for this ion @param {any} p @param {Sel} sel */
  function lowWin(p, sel) { return sel.tol != null ? [sel.m - sel.tol, sel.m + sel.tol] : xicWin(sel.m, sel.obs); }
  /** the trace of the XIC for the chosen ion: the default one of xicDirect, or with the tolerance written by the student @param {any} p @param {Sel} sel */
  function traceFor(p, sel) {
    const t = traceFor0(p, sel), o = window.CANALI ? CANALI.xicOmo(p, specFile(p)) : null;      // the channel of the chromatogram the spectrum follows (canali.js)
    if (o) t.canale = o;
    return t;
  }
  /** @param {any} p @param {Sel} sel */
  function traceFor0(p, sel) {
    const hf = hrFile(p);
    if (hf) {
      const t = HR.ionTrace(sel.m, { obs: sel.obs, tab: p.tab });
      if (sel.tol != null) { t.ppm = sel.tol; t.label = HR.ionText(sel.m, hf, t); }
      return t;
    }
    if (sel.tol == null) return xicTraceFor(sel.m, ref(p));
    const c = rh(sel.m, 3), w = rh(sel.tol, 3);
    return { id: E.seq++, mz: c, w, label: `m/z ${c.toFixed(mzd(p))} ± ${num(w)} Da` };
  }
  /** «m/z 301.1410 ± 5 ppm» (high resolution) or «m/z 300.8–301.8» (unit window) @param {any} p @param {Sel} sel */
  function ionText(p, sel) {
    const hf = hrFile(p);
    if (hf) return HR.ionText(sel.m, hf, sel.tol != null ? { ppm: sel.tol } : null);
    const [a, b] = lowWin(p, sel);
    return `m/z ${num(rh(a, 3))}–${num(rh(b, 3))}`;
  }

  // ---------------------------------------------------------------- toolbar
  /** @param {any} p */
  /** tooltip of the XIC button: the infusion note, «XIC of N files» in overlay, else the how-to @param {any} p @param {boolean} off */
  const tipOf = (p, off) => { const n = window.SPECBAR ? SPECBAR.nFiles(p) : 1; return off ? T("xicbtn.infusion") : n > 1 ? T("xicbtn.n", { n }) : T("xicbtn.title"); };
  function refresh(p) {
    const s = st(p), b = p.el.querySelector('[data-a="xicb"]'), l = p.el.querySelector('[data-a="xicl"]'), pv = /** @type {HTMLButtonElement | null} */ (p.el.querySelector('[data-a="xicp"]'));
    if (p.uv) { for (const x of [b, l, pv]) if (x) x.hidden = true; return; }          // a UV spectrum has no ion to extract
    const off = infusion(p);
    if (off && (s.pend || s.sel)) { s.pend = false; s.sel = null; }
    if (b) {
      b.classList.toggle("on", s.pend); b.setAttribute("aria-pressed", String(s.pend));
      const bt = /** @type {HTMLButtonElement} */ (b), tip = tipOf(p, off);
      if (bt.disabled !== off) bt.disabled = off;
      if (bt.title !== tip) bt.title = tip;
    }
    if (pv) {
      const ok = lens.ok(p), on = !!UIP.xicLens && ok;
      pv.classList.toggle("on", on); pv.setAttribute("aria-pressed", String(on));
      if (pv.hidden === ok) pv.hidden = !ok;
    }
    if (l) {
      l.hidden = !s.sel;
      if (s.sel) { const t = ionText(p, s.sel); l.textContent = t; l.setAttribute("aria-label", T("xicbtn.ion.aria", { ion: t })); }
    }
  }
  /** @param {any} p @param {string} msg */
  function say(p, msg) {
    let r = p.el.querySelector(".xb-live");
    if (!r) { r = document.createElement("div"); r.className = "xb-live sr-only"; r.setAttribute("role", "status"); r.setAttribute("aria-live", "polite"); p.el.appendChild(r); }
    r.textContent = msg;
  }
  /** @param {any} p @param {number} m @param {boolean} obs */
  function choose(p, m, obs) {
    const s = st(p);
    s.sel = { m, obs, tol: null };
    refresh(p); draw(p);
    say(p, T("xicbtn.picked", { ion: ionText(p, s.sel) }));
    if (window.XICLENS) XICLENS.show(p, m);
  }
  /** the labelled peaks of the spectrum in order of m/z @param {any} p */
  const labelled = p => [...new Set((p._a && p._a.lbls || []).filter((/** @type {any} */ l) => l.m != null).map((/** @type {any} */ l) => l.m))].sort((u, v) => u - v);
  /** the peak chosen with the keys: d = -1 / +1, the most intense one when nothing is chosen yet @param {any} p @param {number} d */
  function step(p, d) {
    const ls = labelled(p); if (!ls.length) return;
    const s = st(p);
    let m;
    if (!s.sel) {
      const d0 = p._a.data[0].d; let b = -1;
      for (let i = 0; i < d0.mz.length; i++) if (ls.includes(d0.mz[i]) && (b < 0 || d0.y[i] > d0.y[b])) b = i;
      m = b >= 0 ? d0.mz[b] : ls[0];
    } else {
      const cur = s.sel.m;
      m = d > 0 ? ls.find(v => v > cur + 1e-9) : [...ls].reverse().find(v => v < cur - 1e-9);
      if (m == null) m = d > 0 ? ls[0] : ls[ls.length - 1];
    }
    choose(p, m, true);
  }

  // ---------------------------------------------------------------- extraction
  /** the RT of the scan the spectrum shows (centre of its window) @param {any} p */
  const scanRt = p => p.r0 != null && p.r1 != null ? (p.r0 + p.r1) / 2 : null;
  /** number of scans the spectrum averages @param {any} p */
  const specScans = p => (p._a && p._a.data && p._a.data[0] && p._a.data[0].d.scans) || 1;
  /** @param {any} p @param {boolean} add true: one more trace in the panel (Shift), false: the trace of the panel is replaced */
  async function extract(p, add) {
    const s = st(p), sel = s.sel; if (!sel) return;
    s.pend = false; refresh(p);
    if (window.XICLENS) XICLENS.close(p);
    const tok = ++s.xtok, tr = traceFor(p, sel), old = live(p);
    let xp = old;
    if (!xp) { xp = s.xp = xicDirect(sel.m, p, null, tr); await xp.ready; }
    else {
      COMANDI.fai("xic.traces", xp, () => { if (!add) { xp.ints = []; xp.traces = []; } xp.traces.push(tr); });
      ctl(xp); reveal(xp); await draw(xp);
    }
    if (tok !== s.xtok) return;
    if (!add && hrFile(p)) await integrate(p, xp);
    follow(p);
  }
  /** high resolution: the area comes with the XIC, from the same automatic integration the student has (same table); unit resolution: the student integrates @param {any} p @param {any} xp */
  async function integrate(p, xp) {
    const rt = scanRt(p), f = specFile(p), a = xp._a;
    if (rt == null || !f || !a || !a.sr) return;
    const sr = a.sr.find((/** @type {any} */ q) => q.k === f.k);
    if (sr && typeof autoInt === "function") await autoInt(xp, sr, rt);
  }

  // ---------------------------------------------------------------- the XIC follows the scan of the spectrum
  /** called after every drawing of the spectrum @param {any} p */
  function sync(p) {
    if (window.XICLENS) XICLENS.sync(p);
    const s = ST.get(p), xb = /** @type {HTMLButtonElement | null} */ (p.el.querySelector('[data-a="xicb"]')); if (s && (infusion(p) !== xb?.disabled || (xb && xb.title !== tipOf(p, infusion(p))) || (p.el.querySelector('[data-a="xicp"]')?.getAttribute("aria-pressed") === "true") !== (!!UIP.xicLens && lens.ok(p)))) refresh(p);
    if (!s || !live(p)) return;
    FRAME.cancel(s.raf); s.raf = FRAME.next(() => follow(p));
  }
  /** @param {any} p */
  async function follow(p) {
    const s = ST.get(p), xp = live(p), rt = scanRt(p); if (!s || !xp || rt == null) return;
    xp.cur = rt;
    xp.band = specScans(p) > 1 ? [p.r0, p.r1] : null;
    if (typeof cursorLine === "function") cursorLine(xp);
    if (xp.band) draw(xp);
    const tok = ++s.tok;
    const lines = await apexLines(p, xp, rt);
    if (tok !== s.tok) return;
    info(xp).innerHTML = lines.join("<br>");
  }
  /** @param {any} xp */
  function info(xp) {
    let d = xp.el.querySelector(".xb-info");
    if (!d) { d = document.createElement("div"); d.className = "xb-info"; d.setAttribute("aria-live", "off"); xp.el.appendChild(d); }
    return d;
  }
  /** the limits of the peak that holds RT x on a series, with the detector of the student (Settings «Rilevamento dei picchi») @param {any} s @param {number} x */
  async function edges(s, x) { return autoEdges(s, x, 0); }
  /** «apex RT … min · intensity …» for each trace of the file of the spectrum @param {any} p @param {any} xp @param {number} rt */
  async function apexLines(p, xp, rt) {
    const f = specFile(p), a = xp._a; if (!f || !a || !a.sr) return [];
    const sr = a.sr.filter((/** @type {any} */ q) => q.k === f.k && !(typeof isHid === "function" && isHid(xp, q))).slice(0, 4), out = [];
    for (const q of sr) {
      const ed = await edges(q, rt), nm = sr.length > 1 ? `${EH(q.name)} · ` : "";
      if (!ed) { out.push(nm + T("xicbtn.nopeak")); continue; }
      let b = -1; q.x.forEach((/** @type {number} */ r, /** @type {number} */ i) => { if (r >= ed[0] - 1e-9 && r <= ed[1] + 1e-9 && (b < 0 || q.y[i] > q.y[b])) b = i; });
      out.push(b < 0 ? nm + T("xicbtn.nopeak") : nm + T("xicbtn.apex", { rt: q.x[b].toFixed(2), y: fmt(q.y[b]) }));
    }
    return out;
  }

  // ---------------------------------------------------------------- the button, the label, the pop-up
  /** the button was pressed @param {any} p @param {boolean} add */
  function press(p, add) {
    if (infusion(p)) return;
    const s = st(p);
    if (s.sel) { extract(p, add); return; }
    s.pend = !s.pend; refresh(p);
    if (s.pend) say(p, T("xicbtn.pend"));
  }
  /** @param {any} p */
  function closePop(p) {
    const s = st(p); if (!s.pop) return;
    s.pop.remove(); s.pop = null;
    const l = p.el.querySelector('[data-a="xicl"]'); if (l) { l.setAttribute("aria-expanded", "false"); /** @type {HTMLElement} */ (l).focus({ preventScroll: true }); }
  }
  /** the ion and its tolerance as two fields; Enter extracts (Shift+Enter: as one more trace), Esc closes @param {any} p */
  function openPop(p) {
    const s = st(p); if (!s.sel) return;
    if (s.pop) { closePop(p); return; }
    const d = defTol(p), unit = d.hr ? "ppm" : "Da", sel = s.sel;
    const pop = document.createElement("div"); pop.className = "sp-pop xb-pop"; pop.setAttribute("role", "dialog"); pop.setAttribute("aria-label", T("xicbtn.pop.aria"));
    /** @param {string} f @param {string} v @param {string} txt */
    const field = (f, v, txt) => {
      const l = document.createElement("label"), i = document.createElement("input");
      i.dataset.f = f; i.setAttribute("inputmode", "decimal"); i.autocomplete = "off"; i.value = v;
      l.append(txt + " ", i); return l;
    };
    /** @param {string} f @param {string} txt */
    const btn = (f, txt) => { const b = document.createElement("button"); b.type = "button"; b.className = "bt"; b.dataset.f = f; b.textContent = txt; return b; };
    const msg = document.createElement("div"), row = document.createElement("div");
    msg.className = "xb-err"; msg.setAttribute("role", "alert"); row.className = "xb-row";
    row.append(btn("go", T("xicbtn.pop.go")), btn("no", T("xicbtn.pop.close")));
    pop.append(field("mz", num(sel.m), T("xicbtn.pop.mz")), field("tol", num(sel.tol != null ? sel.tol : d.v), T("xicbtn.pop.tol", { unit })), msg, row);
    const lb = /** @type {HTMLElement} */ (p.el.querySelector('[data-a="xicl"]')), pr = p.el.getBoundingClientRect(), br = lb.getBoundingClientRect();
    p.el.appendChild(pop); s.pop = pop; lb.setAttribute("aria-expanded", "true");
    pop.style.left = Math.max(4, Math.min(br.left - pr.left, p.el.clientWidth - pop.offsetWidth - 6)) + "px"; pop.style.top = br.bottom - pr.top + 4 + "px";
    const fm = /** @type {HTMLInputElement} */ (pop.querySelector('[data-f="mz"]')), ft = /** @type {HTMLInputElement} */ (pop.querySelector('[data-f="tol"]')), err = /** @type {HTMLElement} */ (pop.querySelector(".xb-err"));
    /** @param {boolean} add */
    const apply = add => {
      const m = numIt(fm.value), t = numIt(ft.value), okM = isFinite(m) && m > 0, okT = isFinite(t) && t > 0;
      fm.setAttribute("aria-invalid", String(!okM)); ft.setAttribute("aria-invalid", String(!okT));
      err.textContent = okM && okT ? "" : T("xicbtn.pop.bad");
      if (!okM || !okT) { (okM ? ft : fm).focus(); return; }
      const same = Math.abs(m - sel.m) < 1e-9;
      s.sel = { m, obs: same ? sel.obs : false, tol: Math.abs(t - d.v) < 1e-9 ? null : t };
      closePop(p); refresh(p); draw(p); extract(p, add);
    };
    pop.addEventListener("keydown", e => { if (e.key === "Enter" && e.target instanceof HTMLInputElement) { e.preventDefault(); apply(e.shiftKey); } });
    /** @type {HTMLElement} */ (pop.querySelector('[data-f="go"]')).onclick = e => apply(e.shiftKey);
    /** @type {HTMLElement} */ (pop.querySelector('[data-f="no"]')).onclick = () => closePop(p);
    fm.focus(); fm.select();
  }

  // ---------------------------------------------------------------- hooks used by the classic scripts
  /** the toolbar of one spectrum panel @param {any} p */
  function wire(p) {
    const b = /** @type {HTMLElement | null} */ (p.el.querySelector('[data-a="xicb"]')), l = /** @type {HTMLElement | null} */ (p.el.querySelector('[data-a="xicl"]'));
    if (!b || !l || /** @type {any} */ (b)._xb) return;
    if (p.uv) { b.hidden = true; return; }
    /** @type {any} */ (b)._xb = true;
    b.onclick = e => press(p, e.shiftKey);
    // Space on the button presses it: the walk over the scans (scroll.js) must not take it
    for (const t of ["keydown", "keyup"]) b.addEventListener(t, e => { if (/** @type {KeyboardEvent} */ (e).key === " ") e.stopPropagation(); });
    l.onclick = () => openPop(p);
    const pv = document.createElement("button");           // the lens switch is built here (pannelli.js is at its start-up size cap)
    pv.className = "bt xbp"; pv.dataset.a = "xicp"; pv.title = T("xiclens.toggle"); pv.setAttribute("aria-label", T("xiclens.toggle"));
    pv.innerHTML = '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true" focusable="false"><circle cx="8" cy="8" r="4.5"/><path d="M8 1v4M8 11v4M1 8h4M11 8h4"/></svg>';
    pv.onclick = () => setLens(!UIP.xicLens);
    const mb = p.el.querySelector('.tbs [data-a="more"]'), g = document.createElement("span");       // its own group, the rightmost: the first one the header moves into the «⋯» menu when it is tight (fitHd)
    g.className = "tbg"; g.append(pv); if (mb) mb.before(g); else b.after(pv);
    refresh(p);
  }
  /** the lens preference from the button of any spectrum panel (the Settings switch is the other place) @param {boolean} on */
  function setLens(on) {
    UIP.xicLens = on; uipSave();
    if (!on && window.XICLENS) XICLENS.closeAll();
    const sw = /** @type {HTMLInputElement | null} */ (document.getElementById("uip-xiclens")); if (sw) sw.checked = on;
    E.panels.forEach((/** @type {any} */ q) => { if (q.type === "spec" && q.el) refresh(q); });
  }
  /** the m/z of the peak a pixel means: the nearest peak label within 34 px, otherwise the peak the mouse box highlights (null: none) @param {any} p @param {number} px @returns {number | null} */
  function target(p, px) {
    if (!p._a || !p._a.snap) return null;
    /** @type {any} */ let lb = null, bd = 34;
    (p._a.lbls || []).forEach((/** @type {any} */ l) => { if (l.m == null) return; const dd = Math.abs(l.x + l.w / 2 - px); if (dd < bd) { bd = dd; lb = l; } });
    const m = lb ? lb.m : (p._a.snap(px) || {}).m;
    return m == null ? null : m;
  }
  /** what the lens asks the engine for the ion m of this spectrum: the file of the spectrum, the (m/z, tolerance) of the XIC button, the label @param {any} p @param {number} m */
  function lensArgs(p, m) {
    const f = specFile(p); if (!f || !lens.ok(p)) return null;
    const sel = { m, obs: true, tol: null }, tr = traceFor(p, sel);
    return { f, args: tr.ion && window.HR ? HR.xicArgs(f, tr, { tab: p.tab, tol: TOL0 }) : [tr.mz, tr.w ?? TOL0], text: ionText(p, sel), canale: window.CANALI ? CANALI.idOf(f, tr.canale) : null };
  }
  /** the lens was committed: the same path as a click on the peak with the button waiting @param {any} p @param {number} m */
  function commit(p, m) { choose(p, m, true); extract(p, false); }
  /** a click on the spectrum, no drag: with the ruler on (or a measure open) it stays the ruler's; otherwise the nearest peak is chosen, and extracted when the button is waiting @param {any} p @param {number} px @param {number} py */
  function click(p, px, py) {
    if (infusion(p) || p.rul || (p.meas && p.meas.ref != null) || !p._a || !p._a.snap) return false;
    const m = target(p, px);
    if (m == null) return false;
    const s = st(p);
    choose(p, m, true);
    if (s.pend) extract(p, false);
    return false;                                           // the ruler code of specClick follows and does nothing: the ruler is off here
  }
  /** small dot on the apex of the chosen peak (a ring would cover the m/z label above it); the halo keeps it readable on any background @param {any} p @param {CanvasRenderingContext2D} g @param {(v: number) => number} X @param {(v: number) => number} Y @param {number} W @param {any} d0 */
  function mark(p, g, X, Y, W, d0) {
    const s = ST.get(p); if (!s || !s.sel || p._exp) return;
    const sel = s.sel;
    const px = X(sel.m); if (px < M.l || px > W - M.r) return;
    let j = -1; d0.mz.forEach((/** @type {number} */ v, /** @type {number} */ i) => { if (Math.abs(v - sel.m) < 0.3 && (j < 0 || Math.abs(v - sel.m) < Math.abs(d0.mz[j] - sel.m))) j = i; });
    g.save(); g.strokeStyle = ecss("--accent"); g.lineWidth = 2;
    if (j < 0) { g.setLineDash([4, 3]); g.beginPath(); g.moveTo(px, Y(0)); g.lineTo(px, M.t + 2); g.stroke(); }
    else {
      const py = Math.max(M.t + 6, Y(d0.y[j]));
      g.beginPath(); g.arc(px, py, 3, 0, 7); g.lineWidth = 2; g.strokeStyle = ecss("--panel"); g.stroke(); g.fillStyle = ecss("--accent"); g.fill();
    }
    g.restore();
  }
  /** the Esc of the choice: the pop-up first, then the waiting button @returns {any} */
  const open = () => E.panels.find((/** @type {any} */ q) => q.type === "spec" && ST.get(q) && (ST.get(q)?.pend || ST.get(q)?.pop));
  ESC.registra({ id: "xic-scelta", prio: 29, aperto: () => !!open(), chiudi: () => {
    const p = open(); if (!p) return;
    const s = st(p);
    if (s.pop) closePop(p); else { s.pend = false; refresh(p); }
  } });
  // keys on the focused spectrum (not in a text field): the labelled peaks in order of m/z
  /** @param {string} id @param {string} tasto @param {string} et @param {() => string} nome @param {number} d */
  const key = (id, tasto, et, nome, d) => AZIONI.registra({ id, viste: ["spec"], gruppo: "xic", menu: false, scorciatoia: { tasto, dove: "chart", et },
    nome, disponibile: (/** @type {any} */ c) => !!c.p._a && labelled(c.p).length > 0, esegui: (/** @type {any} */ c) => step(c.p, d) });
  key("spec.xicPrev", ",", "scorc.xicPrev", () => T("scorc.xicPrev"), -1); key("spec.xicNext", ".", "scorc.xicNext", () => T("scorc.xicNext"), 1);

  /** Starts the requests of the XIC panel about to be created in the current tab (getXic memoises them): they run while addPanel builds the DOM, and draw() finds them done or under way. @param {any} tr */
  function prefetch(tr) {
    try {
      const p = { tab: E.tab, tol: TOL0 };
      scanFiles(shownFor(p)).forEach((/** @type {any} */ f) => { getXic(f.k, ...(tr.ion && window.HR ? HR.xicArgs(f, tr, p) : [tr.mz, tr.w ?? p.tol]), f.lv).catch(() => {}); });
    } catch (e) { /* draw() asks again */ }
  }

  /** @type {any} */ (window).XICBTN = { wire, prefetch, setLens, lens, click, target, lensArgs, commit, mark, sync, chosen: (/** @type {any} */ p) => { const s = ST.get(p); return s && s.sel ? s.sel.m : null; }, pending: (/** @type {any} */ p) => { const s = ST.get(p); return !!(s && s.pend); } };
  E.panels.forEach((/** @type {any} */ p) => { if (p.type === "spec" && p.el) wire(p); });
})();
