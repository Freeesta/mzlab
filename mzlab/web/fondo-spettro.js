// @ts-check
// Automatic background of the SPECTRUM: when «background of a stretch» is chosen and no stretch is typed, the program picks one from the TIC of each file.
// Pure choice (regioni, waste, tested in Node: tests/fondo_node.js) + the glue used by drawSpec (picchi.js) and ctl (pannelli.js). Classic script, loaded after fondo.js.
const FONDOSP = (() => {
  /** @param {number[]} a */
  const sorted = a => a.slice().sort((p, q) => p - q);
  const r2 = v => Math.round(v * 100) / 100;
  /** End (min from the first scan) of the leading waste: the run of points at the floor (< 5 % of the 90th percentile of the run) before the signal arrives. 0 when the trace does not start at the floor.
   *  @param {number[]} x @param {number[]} y */
  function waste(x, y) {
    const n = x.length, q = sorted(y)[Math.floor(0.9 * (n - 1))]; if (!(q > 0)) return 0;
    let i = 0; while (i < n >> 2 && y[i] < 0.05 * q) i++;
    return i > 2 ? r2(x[i] - x[0]) : 0;
  }
  /** Background windows of a TIC: waste and solvent front are left out (the end of the front, the end of the waste or FONDO.ZD, whichever is later, plus a margin), then the quietest windows
   *  (mean + 2 SD, so a peak or a rising gradient scores badly) of ~6 % of the run (0.3-1.5 min) are ranked: up to 3 that do not overlap.
   *  Returns { lead, win, cands: [[t0, t1], ...] } (min, best first) or null when the run is too short. @param {number[]} x @param {number[]} y */
  function regioni(x, y) {
    const n = x.length; if (n < 30 || y.length !== n) return null;
    const T = x[n - 1] - x[0], lead = Math.max(FONDO.front(x, y) || 0, waste(x, y), FONDO.ZD), start = x[0] + lead + Math.max(0.1, 0.02 * T);
    const win = Math.max(0.3, Math.min(1.5, 0.06 * T)), m = Math.max(3, Math.round(win / FONDO.dtOf(x)));
    const s1 = [0], s2 = [0]; for (let i = 0; i < n; i++) { s1.push(s1[i] + y[i]); s2.push(s2[i] + y[i] * y[i]); }
    const all = [];
    for (let i = 0; i + m <= n; i++) {
      if (x[i] < start) continue;
      const mu = (s1[i + m] - s1[i]) / m, sd = Math.sqrt(Math.max(0, (s2[i + m] - s2[i]) / m - mu * mu));
      all.push({ i, sc: mu + 2 * sd });
    }
    all.sort((a, b) => a.sc - b.sc);
    const cands = [], used = [];
    for (const c of all) {
      if (used.some(j => Math.abs(j - c.i) < m)) continue;
      used.push(c.i); cands.push([r2(x[c.i]), r2(x[c.i + m - 1])]);
      if (cands.length === 3) break;
    }
    return cands.length ? { lead: r2(lead), win: r2(win), cands } : null;
  }
  /** Is a stretch typed in the header of the spectrum panel? @param {any} p */
  const typed = p => p.bg === "w" && p.bw0 != null && p.bw1 != null && p.bw1 > p.bw0;
  /** The function drawSpec uses for the background of each file: [k, t0, t1] of the typed stretch, of the chosen automatic one (p.bwi), of the file chosen as reference, or null.
   *  Fetches the TIC of each file when the stretch is automatic; the header is rebuilt once when the candidates change. @param {any} p @param {any[]} files @param {any} sref */
  async function bgOf(p, files, sref) {
    const win = /** @type {Record<number, any>} */ ({});
    if (p.bg === "w" && !typed(p)) {
      await Promise.all(files.map(f => getChrom(f.k, "tic", 1).then(c => { const r = c && regioni(c.rt, c.y); if (r) win[f.k] = r; }).catch(() => {})));
      const a0 = win[files[0].k];
      if (JSON.stringify(a0 ? a0.cands : "") !== JSON.stringify(p._bgauto ? p._bgauto.cands : "")) { p._bgauto = a0 || null; ctl(p); }
    }
    return f => p.bg === "w" ? (typed(p) ? [f.k, p.bw0, p.bw1] : win[f.k] ? [f.k, ...win[f.k].cands[Math.max(0, Math.min(win[f.k].cands.length - 1, +p.bwi || 0))]] : null)
      : (sref && sref.k !== f.k ? [sref.k, p.r0, p.r1, sref.factor, sref.clip ? 1 : 0] : null);
  }
  /** Legend text after «background subtracted»: the automatic stretch, written. @param {any} p @param {number[] | null} b */
  const note = (p, b) => (b ? I18N.t("spec.bgSubtracted") + (p.bg === "w" && !typed(p) ? I18N.t("spec.bgAuto", { a: b[1], b: b[2] }) : "") : "");
  /** Header controls after the two time boxes: the select of the automatic stretches, or (once a stretch is typed) the button back to automatic. @param {any} p */
  function ui(p) {
    if (p.bw0 != null || p.bw1 != null) return `<button type="button" data-o="bwauto" title="${EH(I18N.t("bgauto.back.title"))}" aria-label="${EH(I18N.t("bgauto.back.title"))}">↺</button>`;
    const c = p._bgauto && p._bgauto.cands; if (!c) return "";
    return `<select data-o="bwi" aria-label="${EH(I18N.t("bgauto.pick"))}" title="${EH(I18N.t("bgauto.pick.title", { lead: p._bgauto.lead }))}">${c.map((r, i) => `<option value="${i}"${i === (+p.bwi || 0) ? " selected" : ""}>${I18N.t("bgauto.opt", { n: i + 1, a: r[0], b: r[1] })}</option>`).join("")}</select>`;
  }
  /** The two time boxes of the stretch (the automatic one as placeholder) and the controls of ui(). @param {any} p */
  function box(p) {
    const a = (p._bgauto?.cands[+p.bwi || 0]) || ["", ""], inp = (o, v, ph) => `<input data-o="${o}" type="number" step="0.1" value="${v ?? ""}" style="width:56px"${v == null ? ` placeholder="${ph}"` : ""}>`;
    return `<label class="muted">${I18N.t("ctl.range.from")} ${inp("bw0", p.bw0, a[0])} ${I18N.t("ctl.range.to")} ${inp("bw1", p.bw1, a[1])} min</label>` + ui(p);
  }
  // the button back to automatic (one listener for every panel: the header is rebuilt by ctl)
  if (typeof document !== "undefined") document.addEventListener("click", e => {
    const b = e.target instanceof Element ? e.target.closest('[data-o="bwauto"]') : null, p = b && E.panels.find(q => q.el && q.el.contains(b));
    if (p) { pushCampi(p, "storia.bianco"); p.bw0 = p.bw1 = null; p.bwi = 0; ctl(p); draw(p); uiSave(); }
  });
  return { waste, regioni, bgOf, note, box };
})();
