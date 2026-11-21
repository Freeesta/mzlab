// @ts-strict
"use strict";
// Peak info of the spectrum cards: the extra lines of the box that opens on a peak (relative intensity, shift from the precursor, close peaks, formula and notes),
// the highlighted stick, the arrow keys that walk the peaks (announced to the screen reader) and the click that pins or removes the label of a peak.
// The numbers are the ones of the peak table (tabella-picchi.js): `top`, `dmz` and `nl` below are used by both, so the box and the table never disagree.
// Classic helpers used by name: I18N, EH, spanOf, lowerIdx, COMMANDS, FRAME, draw, uiSave, showHover, hideHover, putHover, M.
(() => {
  /** @param {string} k @param {any} [v] */
  const T = (k, v) => I18N.t(k, v);
  const EPS = 1e-4, CLOSE = 6, KEEP = 200;       // m/z: same peak; px: peaks this close to the one shown are counted; at most this many pinned / removed labels
  /** px that count as «on the peak»: the hover reaches farther than a click (a stray click must not pin a label); a finger needs more room than a mouse */
  const tol = (/** @type {boolean} */ click) => {
    const touch = document.documentElement.classList.contains("ptr-touch");
    return click ? (touch ? 14 : 6) : touch ? 24 : 14;
  };
  /** decimals of the table: the chosen ones on high resolution, else two @param {any} a */
  const dec = a => (a.hrp ? a.dec : 2);
  /** the base peak that is 100 in the table: the window when zoomed, else the whole spectrum. Kept per drawing. @param {any} p */
  function top(p) {
    const a = p._a; if (a._ptop != null) return a._ptop;
    const d = a.data[0].d, y = d.y0 || d.y, [i0, i1] = p.zoom ? spanOf(d.mz, a.x0, a.x1) : [0, y.length];
    let t = 1e-9; for (let j = i0; j < i1; j++) if (y[j] > t) t = y[j];
    return (a._ptop = t);
  }
  /** m/z minus precursor (the Δm/z column) @param {number} m @param {number} prec */
  const dmz = (m, prec) => m - prec;
  /** precursor minus m/z (the neutral loss column) @param {number} m @param {number} prec */
  const nl = (m, prec) => prec - m;
  /** the precursor of a peak of an MS² or MSn spectrum, else 0 @param {any} p @param {any} b */
  const precOf = (p, b) => (p.level >= 2 ? (b.prec > 0 ? b.prec : p.prec > 0 ? p.prec : 0) : 0);
  /** the second intensity of the box, in the other unit than the first line @param {any} p @param {any} b */
  const relOf = (p, b) => ` <span class="sm">(${I18N.num((b.y0 ?? b.y) / top(p) * 100, 1)} %)</span>`;

  /** the lines under the m/z and the intensity: shift from the precursor, close peaks, formula, notes. @param {any} p @param {any} b */
  function extra(p, b) {
    const a = p._a, d = a.data[0].d; let h = "";
    const prec = precOf(p, b);
    if (prec) { const v = dmz(b.m, prec); h += `<div class="sm">${T("pk.loss", { d: (v < 0 ? "−" : "+") + I18N.num(Math.abs(v), dec(a)), prec: I18N.num(prec, dec(a)), nl: I18N.num(nl(b.m, prec), dec(a)) })}</div>`; }
    if (b.f === a.data[0].f) {                                                        // peaks beside it (the box shows the nearest one): a binary search, never a pass over the pixels
      const k = (a.x1 - a.x0) / (a.W - M.l - M.r) * CLOSE, [i0, i1] = spanOf(d.mz, b.m - k, b.m + k); let n = 0;
      for (let j = i0; j < i1; j++) if (d.y[j] > 0 && Math.abs(d.mz[j] - b.m) > EPS) n++;
      if (n) h += `<div class="sm">${T("pk.close", { n })}</div>`;
    }
    const fo = (p.fm || []).find((/** @type {any} */ z) => Math.abs(z.m - b.m) < EPS); if (fo) h += `<div><i>${EH(fo.text)}</i></div>`;
    for (const n of p.anns || []) if (Math.abs(n.x - b.m) < 0.7 && b.f === a.data[0].f) h += `<div>${EH(n.text)}</div>`;
    return h;
  }

  /** the thick coloured stick over the peak the box describes; null hides it @param {any} p @param {any} s */
  function stick(p, s) {
    let el = p._pkhi;
    if (!s) { p._hvQ = null; if (el) el.hidden = true; return; }
    if (!el) { el = p._pkhi = document.createElement("div"); el.className = "pkhi"; el.hidden = true; p.el.appendChild(el); }
    const t = Math.min(s.y0, s.y1);
    el.hidden = false; el.style.left = p.cv.offsetLeft + s.px + "px"; el.style.top = p.cv.offsetTop + t + "px"; el.style.height = Math.max(2, Math.abs(s.y0 - s.y1)) + "px";
  }

  /** the box of a spectrum peak under the pointer: the first move of a burst is shown at once, the rest of the frame is folded into one step (profile spectra stay smooth) @param {any} p @param {number} px @param {number} py @param {() => boolean} dragging */
  function hover(p, px, py, dragging) {
    if (p._hvBusy) { p._hvQ = [px, py]; return; }
    p._hvBusy = true; showHover(p, px, py);
    FRAME.next(() => { p._hvBusy = false; const q = p._hvQ; p._hvQ = null; if (q && !dragging()) showHover(p, q[0], q[1]); });
  }
  /** the labels of a drawing: the peaks `pk` (tallest first, above the threshold) lose the ones switched off; the pinned ones (in the window, between ymin and ymax) are returned and leave `pk` @param {any} p @param {[number, number][]} pk @param {any} d0 @param {number} x0 @param {number} x1 @param {number} ymin @param {number} ymax */
  function pinned(p, pk, d0, x0, x1, ymin, ymax) {
    const same = (/** @type {number} */ u, /** @type {number} */ v) => Math.abs(u - v) < EPS;
    for (let i = pk.length - 1; i >= 0; i--) if ((p.lblOff || []).some((/** @type {number} */ m) => same(m, pk[i][0]))) pk.splice(i, 1);
    /** @type {[number, number][]} */ const out = [];
    for (const m of p.lblOn || []) {
      const j = lowerIdx(d0.mz, m - EPS); if (j >= d0.mz.length || !same(d0.mz[j], m) || d0.mz[j] < x0 || d0.mz[j] > x1 || d0.y[j] > ymax || d0.y[j] < ymin) continue;
      out.push([d0.mz[j], d0.y[j]]); const q = pk.findIndex(v => same(v[0], d0.mz[j])); if (q >= 0) pk.splice(q, 1);
    }
    return out;
  }
  /** the label of a peak: on when it is there, off when it is not. The choice is kept with the card (lblOn: pinned, lblOff: removed) and is one undo step. @param {any} p @param {number} m */
  function toggle(p, m) {
    const shown = (p._a.lbls || []).some((/** @type {any} */ l) => l.m != null && Math.abs(l.m - m) < EPS);
    COMMANDS.run("spec.display", p, () => {
      const on = (p.lblOn || []).filter((/** @type {number} */ v) => Math.abs(v - m) >= EPS), off = (p.lblOff || []).filter((/** @type {number} */ v) => Math.abs(v - m) >= EPS);
      if (shown) off.push(m); else on.push(m);
      p.lblOn = on.slice(-KEEP); p.lblOff = off.slice(-KEEP);
    });
    draw(p); uiSave();
  }
  /** a click at (px, py): true when it pinned or removed the label of the peak under it @param {any} p @param {number} px @param {number} py */
  function click(p, px, py) {
    const a = p._a; if (!a || !a.snap || !a.data[0].d.mz.length) return false;
    const b = a.snap(px, tol(true)); if (!b || b.f !== a.data[0].f) return false;
    toggle(p, b.m); return true;
  }

  // ---- keys: the arrows walk the peaks in view, Shift walks them by intensity
  /** the index of the next peak of the window by m/z (dir 1: higher, -1: lower) from the peak at m (null: from the edge), or -1 @param {any} a @param {number | null} m @param {number} dir */
  function stepMz(a, m, dir) {
    const d = a.data[0].d, [i0, i1] = spanOf(d.mz, a.x0, a.x1);
    let j = m == null ? (dir > 0 ? i0 - 1 : i1) : lowerIdx(d.mz, m - EPS);
    if (m != null && !(j < d.mz.length && Math.abs(d.mz[j] - m) < EPS)) j = dir > 0 ? j - 1 : j;      // the peak is gone (another scan): the neighbours decide
    for (j += dir; j >= i0 && j < i1; j += dir) if (d.y[j] > 0) return j;
    return -1;
  }
  /** the peaks of the window, tallest first (the tallest of each pixel column when the spectrum has many more peaks than pixels); once per drawing @param {any} a */
  function ranked(a) {
    if (a._pkrk) return a._pkrk;
    const x = a.data[0], d = x.d, [i0, i1] = spanOf(d.mz, a.x0, a.x1);
    /** @type {number[]} */ const ix = [];
    if (x.tall) { for (const j of x.tall) if (d.mz[j] >= a.x0 && d.mz[j] <= a.x1 && d.y[j] > 0) ix.push(j); } else for (let j = i0; j < i1; j++) if (d.y[j] > 0) ix.push(j);
    ix.sort((u, v) => d.y[v] - d.y[u] || u - v);
    return (a._pkrk = ix);
  }
  /** the box of the peak j, at the top of its stick, and its text for the screen reader @param {any} p @param {number} j */
  function focusPeak(p, j) {
    const a = p._a, x = a.data[0], d = x.d, m = d.mz[j];
    const h = a.tipOf({ m, y: d.y[j], y0: (d.y0 || d.y)[j], f: x.f, multi: x.multi, prec: x.prec });
    p._pk = { m };
    const py = Math.max(M.t + 8, Math.min(a.H - M.b - 8, h.stick.y1));
    putHover(p, h, h.px, py);
    if (p._pklive) p._pklive.textContent = h.html.replace(/<\/div>/g, ", ").replace(/<[^>]+>/g, "").replace(/\s+/g, " ").replace(/[ ,]+$/, "");
  }
  /** @param {any} p @param {KeyboardEvent} e */
  function onKey(p, e) {
    const a = p._a; if (e.ctrlKey || e.metaKey || e.altKey || e.target !== p.cv || !a || !a.tipOf || !a.data[0].d.mz.length) return;
    if (e.key === "Escape") { if (p._pk) { p._pk = null; hideHover(p); if (p._pklive) p._pklive.textContent = ""; } return; }
    if (e.key === "Enter" || e.key === " ") { if (!p._pk) return; e.preventDefault(); toggle(p, p._pk.m); return; }
    const dir = e.key === "ArrowRight" || e.key === "ArrowUp" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowDown" ? -1 : 0;
    if (!dir) return;
    e.preventDefault();
    const d = a.data[0].d; let j = -1;
    if (e.shiftKey) {                                                             // by intensity: right / up = the next taller peak, left / down = the next shorter one
      const r = ranked(a), at = p._pk ? r.findIndex((/** @type {number} */ q) => Math.abs(d.mz[q] - p._pk.m) < EPS) : -1;
      j = r.length ? r[at < 0 ? 0 : Math.max(0, Math.min(r.length - 1, at - dir))] : -1;
    } else j = stepMz(a, p._pk ? p._pk.m : null, dir);
    if (j >= 0) focusPeak(p, j);
  }
  /** wires the keys and the screen reader text of a spectrum card (once) @param {any} p */
  function attach(p) {
    const cv = p.cv; if (!cv || cv._pkOn) return; cv._pkOn = true;
    cv.setAttribute("aria-keyshortcuts", "ArrowLeft ArrowRight Shift+ArrowLeft Shift+ArrowRight Enter");
    cv.setAttribute("aria-description", T("pk.keys"));
    const live = p._pklive = document.createElement("div"); live.className = "sr-only"; live.setAttribute("role", "status"); live.setAttribute("aria-live", "polite"); p.el.appendChild(live);
    cv.addEventListener("keydown", e => onKey(p, e));
    cv.addEventListener("blur", () => { if (p._pk) { p._pk = null; hideHover(p); live.textContent = ""; } });
  }

  /** @type {any} */ (window).PEAKINFO = { hover, pinned, tol, top, dmz, nl, relOf, extra, stick, toggle, click, attach, stepMz };
})();
