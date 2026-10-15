// @ts-strict
"use strict";
// Banco HR (WP-H3a): the cells of the high-resolution bench, as the model of Xcalibur Qual Browser. Classic script, loaded after tabs.js.
// A cell is one panel of the page. In the high-resolution world every chromatogram and spectrum cell gets
//   - a SCAN FILTER (the type of scan it shows: «FTMS + p ESI Full ms», «FTMS + ms2 @hcd», for an infusion MSn file one filter for each path of
//     fragmentation), listed by the server with the number of scans of each type (/api/filters); the filter goes to every request of the cell;
//   - a PIN: a pinned spectrum does not follow the cursor of its chromatogram; a pinned chromatogram is not taken by the MS2 list.
// The active cell (the last one clicked) has an outline. Nothing here exists in the low-resolution world: the functions return at once.
const BANCO = (() => {
  const st = document.createElement("style");
  st.textContent = `.hrw .pnl.act{box-shadow:0 0 0 2px var(--accent)}
.bfw{display:inline-flex;align-items:center;gap:4px;margin-left:6px}.bfw select{max-width:21em;min-width:9em}.bfw .pin{padding:2px 5px}.bfw .pin.on{background:var(--sel);color:var(--accent);border-color:var(--accent)}
.hrw #g-add,.hrw .pnl.spec [data-a="par"]{display:none}
#hrbar{display:flex;flex-wrap:wrap;align-items:center;gap:4px;padding:0 0 6px}
#hrbar[hidden]{display:none}#hrbar .sepv{width:1px;align-self:stretch;background:var(--line);margin:0 4px}
#hrbar button{min-width:32px;height:30px;padding:0 7px;display:inline-flex;align-items:center;justify-content:center;gap:4px}
#hrbar button.on{background:var(--sel);color:var(--accent);border-color:var(--accent)}#hrbar button:disabled{opacity:.35;cursor:default}
#hrbar select{height:30px}#hrbar .act{margin-left:auto;font-size:12px;color:var(--muted)}
#hrinfo{margin-top:10px;display:flex;flex-direction:column;resize:vertical;overflow:hidden;height:340px;min-height:170px;max-height:80vh;padding:0}
#hrinfo[hidden]{display:none}#sb-panels>#hrinfo{height:auto!important;min-height:100%;max-height:none;resize:none;margin:0;border:0;border-radius:0}
#hri-ttl{padding:4px 8px 0;font-weight:600;font-size:12px}#sb-panels>#hrinfo #hri-ttl{display:none}#hri-body{overflow:auto;flex:1;padding:6px 8px;font-size:12px;user-select:text}
#hri-body .row label{display:inline-flex;align-items:center;gap:4px;flex-wrap:wrap;max-width:100%}#hri-body .row input[type=text],#hri-body .row input:not([type]){min-width:0;padding:2px 6px}#hri-body select{padding:2px 4px;max-width:100%;min-width:0}#hri-body button{padding:2px 8px}#hri-body .muted.sm{font-size:11px;line-height:1.35}
#hri-body table{border-collapse:collapse;width:100%}#hri-body td,#hri-body th{padding:1px 6px 1px 0;text-align:left;vertical-align:top;white-space:nowrap}#hri-body td.num,#hri-body th.num{text-align:right}
#hri-body td.v{white-space:normal;word-break:break-all}#hri-body h4{margin:8px 0 2px;font-size:12px}#hri-body .row{display:flex;flex-wrap:wrap;gap:4px;align-items:center;margin:4px 0}
#hri-list{position:relative;overflow:auto;height:100%}#hri-list .hd{position:sticky;top:0;background:var(--panel);z-index:1;display:grid;grid-template-columns:44px 58px 74px 74px 78px 78px;font-weight:600;border-bottom:1px solid var(--line)}
#hri-list .hd span{cursor:pointer;padding:2px 2px}#hri-list .r{display:grid;grid-template-columns:44px 58px 74px 74px 78px 78px;height:20px;line-height:20px;cursor:pointer;white-space:nowrap}
#hri-list .r:hover{background:var(--sel)}#hri-list .r.cur{background:var(--sel);font-weight:600}
.hrw #flst{max-height:34vh;overflow:auto}
.bsp{position:fixed;z-index:10005;width:min(420px,94vw);max-height:70vh;display:flex;flex-direction:column;gap:6px;padding:8px;background:var(--panel);border:1px solid var(--line);border-radius:6px;box-shadow:0 4px 14px rgba(0,0,0,.2);font-size:12px}
.bsp .row{display:flex;flex-wrap:wrap;gap:6px;align-items:center}.bsp select{min-height:24px;max-width:100%}
.bsp .lst{position:relative;overflow:auto;height:264px;border:1px solid var(--line);border-radius:4px}.bsp .lst:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}
.bsp .r{position:absolute;left:0;right:0;height:22px;line-height:22px;padding:0 6px;cursor:pointer;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.bsp .r:hover,.bsp .r.on{background:var(--sel)}.bsp .r.on{font-weight:600}
.mslv{margin-left:8px;padding:0 8px;border:1px solid var(--accent);border-radius:4px;color:var(--accent);font-size:14px;line-height:20px}`;
  document.head.appendChild(st);
  const on = () => !!window.HR && E.files.some(f => f && !f.gone && (HR.isHr(f, 1) || HR.isHr(f, 2)));
  const sync = () => { try { document.body.classList.toggle("hrw", on()); } catch (e) { /* page not ready */ } return on(); };
  // an MSn infusion lists the nodes of its precursor tree instead of the scan types (infusione.js)
  const groupsOf = k => memo(`flt|${k}|${window.INF && INF.isInf(E.files[k]) ? INF.O.tol + INF.O.ce : ""}`, () => (window.INF && INF.isInf(E.files[k]) ? INF.groups(k) : J(`api/filters?k=${k}`).then(j => j.filters)));
  const nfmt = n => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, " ");
  const IC_PIN = '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9.5 1.8l4.7 4.7-2 .6-2.4 2.6.1 3-1 1-2.6-3.1-3.6 3.6-.4-.4 3.6-3.6-3.1-2.6 1-1 3 .1 2.6-2.4z"/></svg>';
  // which cells have a scan filter: not the product-ion spectrum of the DDA trio (its scan is the one the survey scan started) nor the MRM ones
  const eligible = p => (p.type === "chrom" || p.type === "spec") && p.dda == null && p.dock == null && p.tab !== "mrm";
  // the file whose scan types the menu of a cell lists: the one of the cell, or the first visible one of its tab
  function fileOf(p) {
    if (p.type === "spec" && E.files[p.k] && !E.files[p.k].gone) return E.files[p.k];
    return tabFiles(p.tab || E.tab).filter(f => f.vis !== false && f.kind !== "mrm")[0] || null;
  }
  const baseOf = f => (f.file || "").split("#")[0];
  // the menu lists every scan type of the file: choosing one of another experiment (survey / product ions) moves the cell to that experiment
  const tabOf = g => (g.level > 1 ? "ms2" : "full");
  // all the cells that show the scans of the same chromatogram take the filter together (the chromatogram, its spectra)
  const group = p => { const ids = new Set([p.id]); E.panels.forEach(q => { if (q.type === "spec" && (q.link === p.id || q.src === p.id)) ids.add(q.id); if (p.type === "spec" && (p.link === q.id || p.src === q.id)) ids.add(q.id); }); return E.panels.filter(q => ids.has(q.id)); };
  function setFilter(p, key, gs) {
    const g = key ? gs.find(x => x.key === key) : null, nt = g && !g.msn ? tabOf(g) : null;
    group(p).forEach(q => {
      if (q.type === "chrom" && !eligible(q)) return;
      if (nt && q.tab !== nt && q.tab !== "mrm") {                           // another experiment: the files of the cell are the ones of that experiment (same physical file)
        const f0 = q.type === "spec" ? E.files[q.k] : null; q.tab = nt;
        if (f0) { const sib = E.files.find(x => !x.gone && baseOf(x) === baseOf(f0) && x.kind === nt); if (sib) q.k = sib.k; }
        q.title = q.type === "chrom" ? PT.chrom : nt === "ms2" ? PT.product : PT.spec; q.sel = null;
      }
      q.filt = key || null;
      if (g && g.level > 1) { q.prec = null; if (q.type === "chrom") q.title = PT.chrom; }       // the type of scan already says which precursor: not one chosen before
      if (q.type === "chrom") q.flv = g ? g.level : null;
      else { if (g) q.level = g.level; q.zoom = null; q.zoomY = null; q.lock = null; q.si = null; q.meas = null; }
      ctl(q); draw(q);
    });
    uiSave();
  }
  function setPin(p, v) {
    p.pin = !!v;
    if (p.type === "spec") { if (v && p.link) freezeSpec(p); else if (!v && !p.link && p.src) { const s = E.panels.find(q => q.id === p.src); if (s) liveSpec(p, s); } }
    ctl(p); uiSave();
  }
  // ---------------------------------------------------------------- «+ scan»: every scan of the file in time order; the chosen one opens as a TIC with its spectrum under it, at the place the student picks
  /** @type {HTMLElement | null} */ let scanPop = null;
  const scanAway = (/** @type {Event} */ e) => { const t = /** @type {Element} */ (e.target); if (scanPop && !scanPop.contains(t) && !t.closest("[data-bf=scan]")) closeScanPop(); };
  function closeScanPop() { if (scanPop) { scanPop.remove(); scanPop = null; document.removeEventListener("pointerdown", scanAway, true); } }
  /** @returns {Promise<{sid: number, no: number, rt: number, prec: number | null, level: number}[]>} */
  const scansOf = (f, gs, filt) => memo(`scn|${f.k}|${filt || ""}`, async () => {
    const g = filt ? gs.find(x => x.key === filt) : null, levels = g ? [g.level] : [...new Set(gs.map(x => x.level))];
    const parts = await Promise.all(levels.map(lv => J(`api/scanlist?k=${f.k}&level=${lv}${g ? "&filt=" + encodeURIComponent(g.key) : ""}`)));
    const out = [];
    parts.forEach((t, i) => t.sid.forEach((/** @type {number} */ sid, /** @type {number} */ j) => out.push({ sid, no: t.no[j], rt: t.rt[j], prec: t.prec[j], level: levels[i] })));
    return out.sort((a, b) => a.rt - b.rt || a.no - b.no);
  });
  /** Opens the TIC of the experiment of the scan with the scan under it; `where`: top | above | below | end (above / below = the panel the picker was opened from). */
  function openScan(p, f, s, where) {
    const nt = s.level > 1 ? "ms2" : "full", fk = E.files.find(x => !x.gone && baseOf(x) === baseOf(f) && x.kind === nt) || f, here = p.tab === nt;
    if (E.tab !== nt && window.setTab) window.setTab(nt, true, "scan");
    const w = hostWidth(), c = addPanel("chrom", { tab: nt, x: 0, y: 1e5, w, h: 280, full: true }), sp = addPanel("spec", { tab: nt, link: c.id, k: fk.k, level: s.level, prec: s.prec, x: 0, y: 1e5, w, h: 280, full: true });
    const rest = stackOrder().filter(q => q !== c && q !== sp), at = !here || where === "end" ? rest.length : where === "top" ? 0 : Math.max(0, rest.indexOf(p) + (where === "below" ? 1 : 0));
    const base = rest.reduce((m, q) => Math.max(m, q.riga ?? -1), -1) + 1; c.riga = base; sp.riga = base + 1;
    const order = rest.slice(); order.splice(at, 0, c, sp);
    placeRows(order); E.panels.sort(LAYOUT.ordine); relayout(); fitHost(); setActive(c);
    c.ready.then(() => { goScan(c, s.sid, s.rt, fk.k); c.el.scrollIntoView({ block: "center", behavior: "smooth" }); });
    uiSave();
  }
  async function scanPicker(p, btn) {
    if (scanPop) { closeScanPop(); return; }
    const f = fileOf(p); if (!f) return;
    const gs = await groupsOf(f.k).catch(() => null); if (!gs) return;
    const pop = document.createElement("div"); pop.className = "bsp"; pop.setAttribute("role", "dialog"); pop.setAttribute("aria-label", I18N.t("bn.scan.title"));
    const opt = (/** @type {string} */ v, /** @type {string} */ l) => `<option value="${EH(v)}">${EH(l)}</option>`;
    pop.innerHTML = h`<div class="row"><label>${I18N.t("bn.scan.type")} <select data-sp="type">${raw(opt("", I18N.t("bn.filt.all")) + gs.map(g => opt(g.key, `${g.label || g.key} ×${nfmt(g.n)}`)).join(""))}</select></label>
      <label>${I18N.t("bn.scan.where")} <select data-sp="where">${raw([["below", I18N.t("bn.scan.at.below")], ["above", I18N.t("bn.scan.at.above")], ["end", I18N.t("bn.scan.at.end")], ["top", I18N.t("bn.scan.at.top")]].map(([k, l]) => opt(k, l)).join(""))}</select></label></div>
      <div class="lst" role="listbox" tabindex="0" aria-label="${EH(I18N.t("bn.scan.list"))}"></div><div class="muted" data-sp="n"></div>`;
    document.body.appendChild(pop); scanPop = pop;
    const r = btn.getBoundingClientRect(); pop.style.left = Math.max(4, Math.min(r.left, innerWidth - pop.offsetWidth - 4)) + "px"; pop.style.top = Math.max(4, Math.min(r.bottom + 2, innerHeight - pop.offsetHeight - 4)) + "px";
    setTimeout(() => document.addEventListener("pointerdown", scanAway, true), 0);
    const L = /** @type {HTMLElement} */ (pop.querySelector(".lst")), N = /** @type {HTMLElement} */ (pop.querySelector('[data-sp="n"]')), tp = /** @type {HTMLSelectElement} */ (pop.querySelector('[data-sp="type"]')), wh = /** @type {HTMLSelectElement} */ (pop.querySelector('[data-sp="where"]'));
    const H = 22; /** @type {any[]} */ let rows = []; let cur = 0;
    const cur0 = () => { const c = p.link != null ? E.panels.find(q => q.id === p.link) : null; return c && c.cur != null ? c.cur : p.r0 != null && p.r1 != null ? (p.r0 + p.r1) / 2 : null; };
    const txt = (/** @type {any} */ s) => `${s.no} · ${s.rt.toFixed(3)} min · MS${s.level}${s.prec != null ? " → " + s.prec.toFixed(2) : ""}`;
    const paint = () => {
      const a = Math.max(0, Math.floor(L.scrollTop / H) - 2), z = Math.min(rows.length, a + Math.ceil(L.clientHeight / H) + 5);
      L.innerHTML = h`<div style="height:${rows.length * H}px"></div>${raw(rows.slice(a, z).map((s, j) => h`<div class="r${a + j === cur ? " on" : ""}" role="option" id="bsp-${a + j}" aria-selected="${a + j === cur}" data-i="${a + j}" style="top:${(a + j) * H}px">${txt(s)}</div>`).join(""))}`;
      L.setAttribute("aria-activedescendant", "bsp-" + cur);
    };
    const go = (/** @type {number} */ i) => { const s = rows[i]; if (!s) return; closeScanPop(); openScan(p, f, s, wh.value); };
    const show = (/** @type {number} */ i) => { cur = Math.max(0, Math.min(rows.length - 1, i)); if (cur * H < L.scrollTop) L.scrollTop = cur * H; else if ((cur + 1) * H > L.scrollTop + L.clientHeight) L.scrollTop = (cur + 1) * H - L.clientHeight; paint(); };
    L.onscroll = paint;
    L.onclick = e => { const t = /** @type {HTMLElement | null} */ (/** @type {Element} */ (e.target).closest(".r")); if (t) go(+(t.dataset.i || 0)); };
    L.onkeydown = e => {
      if (e.key === "Escape") { e.preventDefault(); closeScanPop(); btn.focus(); }
      else if (e.key === "Enter" || e.key === " ") { e.preventDefault(); go(cur); }
      else if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "PageDown" || e.key === "PageUp") { e.preventDefault(); show(cur + (e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : e.key === "PageDown" ? 10 : -10)); }
      else if (e.key === "Home") { e.preventDefault(); show(0); } else if (e.key === "End") { e.preventDefault(); show(rows.length - 1); }
    };
    pop.onkeydown = e => { if (e.key === "Escape" && e.target !== L) { e.preventDefault(); closeScanPop(); btn.focus(); } };
    const load = async () => {
      rows = await scansOf(f, gs, tp.value).catch(() => /** @type {any[]} */ ([])); if (scanPop !== pop) return;
      N.textContent = I18N.t("bn.scan.count", { n: nfmt(rows.length) });
      const c0 = cur0(); cur = c0 == null ? 0 : Math.max(0, rows.findIndex(s => s.rt >= c0 - 1e-4)); L.scrollTop = Math.max(0, cur * H - L.clientHeight / 2); paint();
    };
    tp.onchange = load; await load(); L.focus();
  }
  // ---------------------------------------------------------------- stacked graphs of a chromatogram cell (the «Ranges» of Xcalibur)
  // p.ranges = [{id, kind: "tic" | "bpc" | "xic" | "sub" (formula, ppm, min), mz, ppm, filt, flv, prec, label}], at most 8. Every range is one row of the stacked chromatogram, with the
  // files of the bench overlaid in it; every row says its NL (the highest intensity) and p.norm100 puts them all on the 0-100 scale.
  const MAXR = 8;
  const filesFor = lv => {
    const seen = new Set(), out = [];
    vis().forEach(f => { const b = baseOf(f); if (f.kind === "mrm" || seen.has(b)) return; seen.add(b); const sib = E.files.find(g => !g.gone && baseOf(g) === b && (lv > 1 ? g.kind === "ms2" : g.kind === "full")); if (sib) out.push({ f: sib, vis: f }); });
    return out;
  };
  const rangeLabel = r => r.label || ((r.kind === "xic" ? `XIC ${r.mz}` : r.kind.toUpperCase()) + (r.filt ? " · " + r.filt : r.flv > 1 ? " · MS" + r.flv : ""));
  async function rangeSeries(p) {
    const out = [], jobs = [];
    p.ranges.forEach((r, ri) => {
      const lv = r.flv || 1;
      filesFor(lv).forEach(({ f, vis: vf }) => {
        if (r.kind === "sub") {         // the sum of the sub-formulas of a formula (chem/subformulas.py): one request per file
          jobs.push(subData(f, r).then(d => { if (d.rt.length) out.push({ x: d.rt, y: d.y, color: vf.color, dash: fdash(vf), name: `${rangeLabel(r)} · ${vf.label}`, k: f.k, key: `r|${r.id}|${f.k}`, ion: rangeLabel(r), time: vf.time, grp: ri, rlabel: rangeLabel(r) }); }).catch(() => null));
          return;
        }
        const q = HR.prof(f, lv), w = r.kind === "xic" ? r.mz * (r.ppm ?? q.tol) * 1e-6 : null;
        jobs.push(getChrom(f.k, r.kind === "xic" ? "tic" : r.kind, lv, w != null ? r.mz - w : null, w != null ? r.mz + w : null, r.prec ?? null, r.filt || null).then(d => {
          if (d.rt.length) out.push({ x: d.rt, y: d.y, color: vf.color, dash: fdash(vf), name: `${rangeLabel(r)} · ${vf.label}`, k: f.k, key: `r|${r.id}|${f.k}`, ion: rangeLabel(r), time: vf.time, grp: ri, rlabel: rangeLabel(r) });
        }));
      });
    });
    await Promise.all(jobs);
    return out.sort((a, b) => a.grp - b.grp || a.k - b.k);
  }
  async function addRange(p) {
    if ((p.ranges || []).length >= MAXR) return info(I18N.t("bn.maxRanges", { n: MAXR }));
    const f0 = fileOf(p) || vis()[0]; if (!f0) return;
    const groups = await groupsOf(f0.k).catch(() => []);
    const opt = g => `<option value="${EH(g.key)}" data-lv="${g.level}">${EH(g.key)} \u00d7${nfmt(g.n)}</option>`;
    big(I18N.t("bn.add.title"), `<div class="muted sm" style="margin-bottom:6px">${I18N.t("bn.add.note")}</div>
      <div class="cmp-grid"><label>${I18N.t("bn.add.kind")} <select id="br-kind"><option value="tic">${I18N.t("bn.add.tic")}</option><option value="bpc">${I18N.t("bn.add.bpc")}</option><option value="xic">${I18N.t("bn.add.xic")}</option></select></label>
      <label id="br-ionw" hidden>${I18N.t("bn.add.ion")} <input id="br-ion" placeholder="${I18N.t("bn.add.ionPh")}" style="width:200px"> <select id="br-ad"><option>[M+H]+</option><option>[M+Na]+</option><option>[M+NH4]+</option><option>[M+K]+</option><option>[M-H]-</option></select> \u00b1 <input id="br-ppm" value="${HR.prof(f0, 1).tol}" style="width:46px"> ppm</label>
      <label>${I18N.t("bn.scanType")} <select id="br-filt"><option value="" data-lv="1">${I18N.t("bn.allMs1")}</option>${groups.map(opt).join("")}</select></label></div>
      <div style="margin-top:8px"><button id="br-ok" type="button" class="go">${I18N.t("lst.lab.add")}</button> <span id="br-err" class="fail"></span></div>`, () => {
      const $ = x => Q(x), sync = () => { $("#br-ionw").hidden = $("#br-kind").value !== "xic"; };
      $("#br-kind").onchange = sync; sync();
      $("#br-ok").onclick = async () => {
        const kind = $("#br-kind").value, fo = $("#br-filt").selectedOptions[0], filt = $("#br-filt").value || null, lv = +(fo.dataset.lv || 1), r = { id: E.seq++, kind, filt, flv: lv, prec: null };
        if (kind === "xic") {
          const t = $("#br-ion").value.trim().replace(",", "."); let mz = parseFloat(t);
          if (!(mz > 0) || /[A-Za-z]/.test(t)) { try { const j = await getFormula(t.split(/\s+/)[0], $("#br-ad").value); mz = j.mz5 ?? j.mz; } catch (e) { $("#br-err").textContent = I18N.t("bn.add.invalid"); return; } }
          r.mz = +(+mz).toFixed(5); r.ppm = parseFloat($("#br-ppm").value) || HR.prof(f0, 1).tol;
          r.label = `XIC ${r.mz.toFixed(HR.prof(f0, 1).dec)} \u00b1${+r.ppm.toFixed(1)} ppm` + (filt ? " · " + filt : "");
        }
        p.ranges = [...(p.ranges || []), r]; if (p.ranges.length > 1) p.mode = "stk";
        Q("#bigdlg").close(); ctl(p); draw(p); uiSave();
      };
    });
  }
  // ---------------------------------------------------------------- formula -> combined XIC (high resolution only)
  // The student writes the formula of an ion; the server sums the peaks at the exact m/z of ALL its sub-formulas (/api/subxic) and measures each one.
  // The table has measures only (no label, no proposal); the sum goes into the cell as one more graph of the stack.
  const subData = (f, r) => memo(`sub|${baseOf(f)}|${f.k}|${r.formula}|${r.ppm}|${r.min}|${r.z}`, () => J(`api/subxic?k=${f.k}&f=${encodeURIComponent(r.formula)}&ppm=${r.ppm}&min=${r.min}&z=${r.z}`));
  const zOf = f => (f && f.polarity === "negative" ? -1 : 1);
  function subXic(p) {
    if ((p.ranges || []).length >= MAXR) return info(I18N.t("bn.maxRanges", { n: MAXR }));
    const f0 = fileOf(p) || vis()[0]; if (!f0) return;
    big(I18N.t("bn.sub.title"), `<div class="muted sm" style="margin-bottom:6px">${I18N.t("bn.sub.note")}</div>
      <div class="cmp-grid"><label>${I18N.t("bn.sub.formula")} <input id="bs-f" placeholder="C34H54O31N5" style="width:200px" autocomplete="off" spellcheck="false"></label>
      <label>${I18N.t("bn.sub.ppm")} <input id="bs-ppm" type="number" min="0.1" max="100" step="0.5" value="5" style="width:70px"></label>
      <label>${I18N.t("bn.sub.min")} <input id="bs-min" type="number" min="0" step="10" value="100" style="width:80px"></label></div>
      <div style="margin-top:8px"><button id="bs-go" type="button" class="go">${I18N.t("bn.sub.calc")}</button> <button id="bs-add" type="button" hidden>${I18N.t("bn.sub.add")}</button> <span id="bs-err" class="fail"></span></div>
      <div id="bs-out" style="margin-top:8px;max-height:46vh;overflow:auto"></div>`, () => {
      const $ = x => Q(x); let cur = null;
      const par = () => ({ formula: $("#bs-f").value.trim(), ppm: parseFloat($("#bs-ppm").value) || 5, min: parseFloat($("#bs-min").value) || 0, z: zOf(f0) });
      const dec = HR.prof(f0, 1).dec;
      $("#bs-go").onclick = async () => {
        const r = par(); $("#bs-err").textContent = ""; $("#bs-out").innerHTML = ""; $("#bs-add").hidden = true; cur = null;
        if (!r.formula) return;
        try {
          const d = await subData(f0, r);
          cur = r;
          const HEAD = { mz: I18N.t("bn.sub.th.mz"), ppm: I18N.t("bn.sub.th.ppm"), int: I18N.t("bn.sub.th.int"), rt: I18N.t("bn.sub.th.rt"), r: I18N.t("bn.sub.th.r") }, th = c => `<th class="num">${HEAD[c]}</th>`, n = (v, k) => (v == null ? "" : v.toFixed(k));
          $("#bs-out").innerHTML = `<div class="muted sm">${I18N.t("bn.sub.count", { hit: d.n_hit, n: d.n_sub })}</div>` + (d.rows.length ? `<table><thead><tr><th>${I18N.t("bn.sub.th.formula")}</th>${["mz", "ppm", "int", "rt", "r"].map(th).join("")}</tr></thead><tbody>`
            + d.rows.map(x => `<tr><td>${EH(x.formula)}</td><td class="num">${x.mz.toFixed(dec)}</td><td class="num">${n(x.ppm, 1)}</td><td class="num">${x.int.toExponential(2).replace("e+", "E")}</td><td class="num">${x.rt.toFixed(2)}</td><td class="num">${n(x.r, 2)}</td></tr>`).join("") + "</tbody></table>" : `<div class="muted">${I18N.t("bn.sub.none")}</div>`);
          $("#bs-add").hidden = false;
        } catch (e) { $("#bs-err").textContent = e && e.message || String(e); }
      };
      $("#bs-f").onkeydown = e => { if (e.key === "Enter") $("#bs-go").click(); };
      $("#bs-add").onclick = () => {
        if (!cur) return;
        const r = { id: E.seq++, kind: "sub", formula: cur.formula, ppm: cur.ppm, min: cur.min, z: cur.z, filt: null, flv: 1, prec: null, label: I18N.t("bn.sub.label", { formula: cur.formula, ppm: +cur.ppm.toFixed(1) }) };
        p.ranges = [...(p.ranges || []), r]; if (p.ranges.length > 1) p.mode = "stk";
        Q("#bigdlg").close(); ctl(p); draw(p); uiSave();
      };
      $("#bs-f").focus();
    });
  }
  const dropRange = (p, id) => { p.ranges = (p.ranges || []).filter(r => r.id !== id); if (!p.ranges.length) p.ranges = null; ctl(p); draw(p); uiSave(); };
  // short header of a spectrum cell (as in Xcalibur): the scan type and the NL (highest intensity) next to the retention time
  function header(p) {
    if (!sync()) return;
    const ac = act();
    if (ac && (p === ac || (p.type === "spec" && p.link === ac.id)) && ist.tab === "hdr") { clearTimeout(ist.t); ist.t = setTimeout(renderInfo, 250); }      // the header of the scan follows the cursor (of the cell or of its chromatogram)
    if (p.type !== "spec") return;
    const rl = p.el.querySelector(".rtl"); if (!rl) return;
    let lv = p.el.querySelector(".mslv"); if (!lv) { lv = document.createElement("b"); lv.className = "mslv"; rl.before(lv); }       // the stage of the spectrum, well visible: MS1 / MS2
    const stage = p.level > 1 ? p.level : 1; if (lv.dataset.n !== String(stage)) { lv.dataset.n = String(stage); lv.innerHTML = `MS<sup>${stage}</sup>`; }
    let h = p.el.querySelector(".bnl"); if (!h) { h = document.createElement("span"); h.className = "bnl muted sm"; h.style.marginLeft = "8px"; rl.after(h); }
    const d = p._a && p._a.data && p._a.data[0] && p._a.data[0].d, nl = d && d.y && d.y.length ? Math.max(...d.y) : null;
    h.textContent = nl == null ? "" : `${p.filt ? p.filt + " · " : ""}NL: ${nl.toExponential(2).replace("e+", "E")}`;
  }
  function decorate(p, c) {
    if (!sync() || !c || !(eligible(p) || p.type === "xic")) return;
    const wrap = document.createElement("span"); wrap.className = "bfw";
    if (p.type === "spec" && eligible(p) && !(window.INF && fileOf(p) && INF.isInf(fileOf(p)))) {
      const sb = document.createElement("button"); sb.type = "button"; sb.className = "bt"; sb.dataset.bf = "scan"; sb.textContent = I18N.t("bn.scan.btn");
      sb.title = I18N.t("bn.scan.title"); sb.setAttribute("aria-haspopup", "dialog"); sb.onclick = e => { e.stopPropagation(); scanPicker(p, sb); }; wrap.appendChild(sb);
    } else if (eligible(p)) {
      const sel = document.createElement("select"); sel.dataset.bf = "filt"; sel.title = I18N.t("bn.filt.title");
      sel.innerHTML = `<option value="">${I18N.t("bn.filt.all")}</option>`; wrap.appendChild(sel);
      const f = fileOf(p);
      if (f) groupsOf(f.k).then(gs => {
        if (!sel.isConnected) return;
        const opt = g => `<option value="${EH(g.key)}"${p.filt === g.key ? " selected" : ""}>${EH(g.label || g.key)} \u00d7${nfmt(g.n)}</option>`;
        if (gs.some(g => g.msn)) {
          sel.innerHTML = h`<option value="">${I18N.t("bn.filt.all")}</option>${raw(gs.map(opt).join(""))}`;
          sel.value = p.filt && gs.some(g => g.key === p.filt) ? p.filt : "";
          sel.onchange = () => setFilter(p, sel.value || null, gs);
          return;
        }
        const lv = gs.filter(g => g.level === 1), l2 = gs.filter(g => g.level === 2), ln = gs.filter(g => g.level > 2);
        sel.innerHTML = `<option value="">${I18N.t("bn.filt.types")}</option>` + [[I18N.t("bn.grp.ms1"), lv], [I18N.t("bn.grp.ms2"), l2], [I18N.t("bn.grp.msn"), ln]].filter(([, l]) => l.length).map(([t, l]) => `<optgroup label="${t}">${l.map(opt).join("")}</optgroup>`).join("");
        sel.value = p.filt && gs.some(g => g.key === p.filt) ? p.filt : "";
        sel.onchange = () => setFilter(p, sel.value || null, gs);
      }).catch(() => { sel.disabled = true; });
    }
    if (p.type === "chrom") {
      const add = document.createElement("button"); add.type = "button"; add.dataset.bf = "range"; add.textContent = "+"; add.title = I18N.t("bn.addBtn.title"); add.setAttribute("aria-label", I18N.t("bn.addBtn.title"));
      add.onclick = e => { e.stopPropagation(); addRange(p); }; wrap.appendChild(add);
      if (p.ranges && p.ranges.length) {
        const ks = c.querySelector('[data-o="kind"]'); if (ks) { const t = document.createElement("b"); t.className = "ttl"; t.textContent = I18N.t("bn.stack"); t.title = I18N.t("bn.stack.title"); ks.replaceWith(t); }
        const n = document.createElement("label"); n.className = "muted"; n.title = I18N.t("bn.norm.title");
        n.innerHTML = `<input type="checkbox" data-bf="norm"${p.norm100 ? " checked" : ""}> 0-100`; n.querySelector("input").onchange = e => { p.norm100 = e.target.checked; draw(p); uiSave(); }; wrap.appendChild(n);
        p.ranges.forEach(r => { const b = document.createElement("button"); b.type = "button"; b.className = "bt"; b.dataset.bf = "drop"; b.dataset.id = r.id; b.textContent = "\u00d7 " + (rangeLabel(r).length > 18 ? rangeLabel(r).slice(0, 17) + "…" : rangeLabel(r)); b.title = I18N.t("bn.drop", { name: rangeLabel(r) }); b.onclick = e => { e.stopPropagation(); dropRange(p, r.id); }; wrap.appendChild(b); });
      }
    }
    const ds = document.createElement("button"); ds.type = "button"; ds.className = "bt"; ds.dataset.bf = "disp"; ds.innerHTML = svg('<path d="M2.5 5h6M11.5 5h2M2.5 11h2M7.5 11h6"/><circle cx="10" cy="5" r="1.5"/><circle cx="6" cy="11" r="1.5"/>');
    ds.title = I18N.t("cmd.disp.name"); ds.setAttribute("aria-label", ds.title); ds.onclick = e => { e.stopPropagation(); if (window.DISP) DISP.open(p); }; wrap.appendChild(ds);
    const pin = document.createElement("button"); pin.type = "button"; pin.className = "bt pin" + (p.pin ? " on" : ""); pin.dataset.bf = "pin"; pin.innerHTML = IC_PIN;
    pin.title = I18N.t(p.pin ? "bn.pin.on" : "bn.pin.off");
    pin.onclick = e => { e.stopPropagation(); setPin(p, !p.pin); };
    wrap.appendChild(pin); c.appendChild(wrap);
  }

  // ---------------------------------------------------------------- the toolbar of the bench (WP-H3b, master-prompt §3.3): in the place of the tabs of the experiments
  const svg = d => `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${d}</svg>`;
  const ICON = {
    prev: svg('<path d="M10 3L5 8l5 5"/>'), next: svg('<path d="M6 3l5 5-5 5"/>'),
    zundo: svg('<path d="M3 7h6a3 3 0 010 6H6"/><path d="M5.5 4.5L3 7l2.5 2.5"/>'), zredo: svg('<path d="M13 7H7a3 3 0 000 6h3"/><path d="M10.5 4.5L13 7l-2.5 2.5"/>'), zall: svg('<path d="M2 6V2h4M14 6V2h-4M2 10v4h4M14 10v4h-4"/>'),
    norm: svg('<path d="M3 13V8M7 13V4M11 13V6"/><path d="M2 13.5h12"/>'), add: svg('<path d="M8 3v10M3 8h10"/>'), del: svg('<path d="M3 4.5h10M6 4.5V3h4v1.5M5 4.5l.5 8.5h5l.5-8.5"/>'),
    type: svg('<path d="M3 5h8M3 11h8"/><path d="M9 3l2 2-2 2M7 9l-2 2 2 2"/>'), peaks: svg('<path d="M2 13c2 0 2-8 4-8s2 8 4 8 1.5-4 4-4"/>'), lib: svg('<circle cx="7" cy="7" r="4"/><path d="M10 10l3.5 3.5"/>'),
    copy: svg('<rect x="5" y="5" width="8" height="8" rx="1"/><path d="M3 10V3h7"/>'), xls: svg('<path d="M3 2.5h6.5l3 3v8H3z"/><path d="M5.5 8l3 3.5M8.5 8l-3 3.5"/>'), tree: svg('<path d="M8 2v4M8 6H4v3M8 6h4v3M4 9v3M12 9v3"/>'),
  };
  const act = () => (E.active && E.panels.includes(E.active) ? E.active : E.panels[0] || null);
  const tabOfAct = () => (act() && act().tab) || E.tab;
  function barBtn(id, tip, ic, fn, label) { return `<button type="button" data-hb="${id}" title="${EH(tip)}" aria-label="${EH(tip)}">${ic}${label ? `<span>${EH(label)}</span>` : ""}</button>`; }
  async function copyPng(p) {
    let cvs; p._exp = true; EXPORTING = true;
    try { await draw(p); cvs = whiteCanvas(p.cv, p.type === "spec" ? specCaption(p) : ""); } finally { p._exp = false; EXPORTING = false; draw(p); }
    const blob = await new Promise(r => cvs.toBlob(r));
    try { await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]); nearMsg(I18N.t("bn.copied.img")); }
    catch (e) { saveBlob(blob, plotName(p) + ".png"); info(I18N.t("bn.copied.dl")); }
  }
  async function copyText(p) {
    const t = plotSheets(p); if (!t || !t[0]) return info(I18N.t("bn.noCopy"));
    const sh = t[0], txt = await PROV.tsv([sh.head.join("\t"), ...sh.rows.map(r => r.map(v => (v == null ? "" : v)).join("\t"))].join("\n"), { p, table: "plot" });
    try { await navigator.clipboard.writeText(txt); nearMsg(I18N.t("bn.copied.txt")); } catch (e) { info(I18N.t("bn.noClip")); }
  }
  function addCell(type) {
    const tab = tabOfAct() === "mrm" ? "full" : tabOfAct(), f = tabFiles(tab)[0]; if (!f) return;
    const q = E.panels.reduce((m, x) => Math.max(m, x.y + x.h + pnlGap()), 0), w = hostWidth();
    const o = { tab, x: 0, y: q, w, h: type === "map" ? 420 : 300, full: true };
    const n = type === "spec" ? addPanel("spec", { ...o, k: f.k, level: f.lv || 1, link: null }) : addPanel(type, o);
    relayout(); fitHost(); setActive(n); n.el.scrollIntoView({ block: "center", behavior: "smooth" }); uiSave();
  }
  function subXicCell() { let a = act(); if (!a || a.type !== "chrom") { addCell("chrom"); a = act(); } if (a && a.type === "chrom") subXic(a); }
  function retype(type) {
    const o = act(); if (!o || o.type === type) return;
    if (!["chrom", "spec", "map"].includes(type) || !["chrom", "spec", "map", "xic"].includes(o.type)) return info(I18N.t("bn.noRetype"));
    const f = tabFiles(o.tab)[0]; if (!f) return;
    const g = { tab: o.tab, x: o.x, y: o.y, w: o.w, h: o.h, full: o.full };
    const n = type === "spec" ? addPanel("spec", { ...g, k: o.k ?? f.k, level: o.level ?? (f.lv || 1) }) : addPanel(type, g);
    o.el.querySelector(".x").click(); setActive(n); relayout(); fitHost(); uiSave();
  }
  const FN = {
    prev: () => { const a = walkTarget(false); if (a) stepScan(a, -1); else info(I18N.t("bn.putCursor")); },
    next: () => { const a = walkTarget(false); if (a) stepScan(a, 1); else info(I18N.t("bn.putCursor")); },
    zundo: () => { if (STORIA) STORIA.annulla(); },
    zredo: () => { if (STORIA) STORIA.ripeti(); },
    zall: () => { E.panels.forEach(q => { pushZh(q); q.zoom = null; q.zoomY = null; if (q.type === "spec") q.lock = null; draw(q); }); uiSave(); },
    norm: () => { const a = act(); if (!a) return; if (a.type === "spec") { a.rel = !specRel(a); draw(a); } else if (a.type === "chrom" || a.type === "xic") { a.norm100 = !a.norm100; draw(a); } uiSave(); sync2(); },
    pin: () => { const a = act(); if (a && (eligible(a) || a.type === "xic")) setPin(a, !a.pin); sync2(); },
    del: () => { const a = act(); if (a) a.el.querySelector(".x").click(); },
    peaks: () => { const a = act(); const b = a && a.el.querySelector('[data-a="iauto"]'); if (b) b.click(); else info(I18N.t("bn.peaksOnly")); sync2(); },
    lib: () => { let a = act(); if (a && a.type !== "spec") a = E.panels.find(q => q.type === "spec" && q.link === a.id); if (a && window.LIB && a.level === 2) LIB.searchFrom(a); else info(I18N.t("bn.pickMs2")); },
    xls: () => { const a = act(), t = a && plotSheets(a); if (t) dlx(plotName(a) + ".xlsx", t, { p: a, table: "plot" }); else info(I18N.t("bn.noExport")); },
    tree: () => { const f = tabFiles("ms2")[0]; if (f && window.COMP) COMP.tree(f.k); else info(I18N.t("bn.treeOnly")); },
  };
  function sync2() {
    const b = Q("#hrbar"); if (!b) return; const a = act();
    const t = (id, on) => { const x = b.querySelector(`[data-hb="${id}"]`); if (x) x.classList.toggle("on", !!on); };
    t("pin", a && a.pin); t("norm", a && (a.type === "spec" ? specRel(a) : a.norm100)); t("peaks", a && a.imode === "auto");
    renderInfo();
    const l = b.querySelector(".act"); if (l) l.textContent = "";
  }
  // the starting layout of the bench: the survey on top (the trio for a DDA file); product ions that are not a DDA get their own chromatogram and spectrum under it
  function afterLayout() {
    if (!sync()) return;
    const f2 = tabFiles("ms2").find(f => !f.dda && f.vis !== false);
    if (!f2 || E.panels.some(q => q.tab === "ms2")) return;
    const y = E.panels.reduce((m, q) => Math.max(m, q.y + q.h + pnlGap()), 0);
    addMs2Pair({ prec: null, k: f2.k }, y); relayout(); fitHost();
  }
    // ---------------------------------------------------------------- the information bar (WP-H3c, master-prompt §3.4): tabs as icons, one at a time, acting on the active cell.
  // header of the scan, list of scans, file and instrument are here; elemental composition, isotope simulation and identification act on the active cell
  // (composizione.js, libreria.js), without a second copy of their code. Peak detection, composite spectrum and the MSn tree are not in the bar.
  const I9 = [["hdr", I18N.t("bn.i9.hdr"), svg('<rect x="2.5" y="2.5" width="11" height="11" rx="1.5"/><path d="M5 6h6M5 8.5h6M5 11h3"/>')],
    ["lst", I18N.t("bn.i9.lst"), svg('<path d="M3 4h10M3 8h10M3 12h10"/><circle cx="1.6" cy="4" r=".6"/><circle cx="1.6" cy="8" r=".6"/><circle cx="1.6" cy="12" r=".6"/>')],
    ["fil", I18N.t("bn.i9.fil"), svg('<path d="M3.5 2.5h6l3 3v8h-9z"/><path d="M9.5 2.5v3h3"/>')],
    ["cmp", I18N.t("bn.i9.cmp"), svg('<path d="M2 12l3-8 3 8M3.2 9h3.6"/><path d="M10 5h4M10 8h4M10 11h4"/>')],
    ["iso", I18N.t("bn.i9.iso"), svg('<path d="M3 13V8M6 13V4M9 13V7M12 13V10"/>')],
    ["idn", I18N.t("bn.i9.idn"), ICON.lib],
    ["disp", I18N.t("bn.i9.disp"), svg('<path d="M2.5 4h6M11.5 4h2M2.5 8h2M7.5 8h6M2.5 12h8"/>')],
    ["alb", I18N.t("bn.i9.alb"), ICON.tree]];
  const IK = "qqq.banco.info";
  const ist = (() => { const o = lsObj(IK); return { tab: I9.some(x => x[0] === o.tab) ? o.tab : "hdr", h: o.h || 340 }; })();
  const isave = () => { try { localStorage.setItem(IK, JSON.stringify(ist)); } catch (e) { /* none */ } };
  const num = (v, d = 4) => (v == null || !Number.isFinite(+v) ? "" : String(+(+v).toFixed(d)));
  const esc = EH;
  // the file, the level and the time of the active cell
  const cFile = p => (p.type === "spec" ? E.files[p.k] : (p._a && p._a.sr && p._a.sr[0] ? E.files[p._a.sr[0].k] : fileOf(p))) || null;
  const cLevel = p => (p.type === "spec" ? p.level || 1 : p.flv ?? (cFile(p) || {}).lv ?? 1);
  const cRt = p => (p.type === "spec" ? (p.r0 != null ? (p.r0 + p.r1) / 2 : null) : p.cur);
  async function cScan(p) {
    const f = cFile(p), rt = cRt(p); if (!f || rt == null) return null;
    return J(`api/nearest_scan?k=${f.k}&rt=${rt}&level=${cLevel(p)}${p.filt ? "&filter=" + encodeURIComponent(p.filt) : ""}`).catch(() => null);
  }
  const kv = rows => `<table>${rows.map(([a, b]) => `<tr><td class="muted">${esc(a)}</td><td class="v">${esc(b)}</td></tr>`).join("")}</table>`;
  async function tHdr(b, p, tok) {
    const n = await cScan(p);
    if (tok !== ist.tok) return;
    if (!n) return void (b.innerHTML = `<div class="muted">${I18N.t("bn.h.click")}</div>`);
    const h = await J(`api/scaninfo?k=${cFile(p).k}&sid=${n.sid}`).catch(e => ({ error: e.message }));
    if (tok !== ist.tok) return;
    if (h.error) return void (b.innerHTML = `<span class="fail">${esc(h.error)}</span>`);
    const top = [[I18N.t("bn.h.scan"), h.no], ["RT (min)", num(h.rt)], [I18N.t("bn.h.level"), "MS" + h.level], [I18N.t("lib.col.polarity"), h.polarity === "positive" || h.polarity === "negative" ? I18N.t(`bn.h.pol.${h.polarity}`) : h.polarity], [I18N.t("bn.h.filter"), h.filter], [I18N.t("bn.add.kind"), h.key], [I18N.t("bn.h.prec"), num(h.prec, 5)],
      [I18N.t("bn.h.isoWin"), h.iso ? `${num(h.iso[0])} – ${num(h.iso[1])}` : ""], [I18N.t("bn.h.act"), [h.act, h.ce != null ? I18N.t("bn.h.energy") + " " + h.ce : ""].filter(Boolean).join(" ")], [I18N.t("bn.h.res"), h.res ? Math.round(h.res) : ""],
      [I18N.t("bn.h.analyzer"), h.analyzer || ""], [I18N.t("bn.h.spectrum"), h.profile ? I18N.t("hr.mode.profile") : I18N.t("hr.mode.centroid")], ["TIC", h.tic != null ? (+h.tic).toExponential(3) : ""]].filter(x => x[1] !== "" && x[1] != null);
    b.innerHTML = `<div class="row"><span class="muted">${I18N.t("bn.h.nParams", { n: h.pairs.length })}</span></div>` + kv(top) + `<h4>${I18N.t("bn.h.all")}</h4>` + kv(h.pairs.map(x => [x.name, x.value + (x.unit ? " " + x.unit : "")]));
  }
  function goScan(p, sid, rt, k) {
    const c = p.type === "spec" ? E.panels.find(q => q.id === p.link) : p;
    if (c && c.type !== "spec") { c.cur = rt; if (c.zoom && (rt < c.zoom[0] || rt > c.zoom[1])) { const w = c.zoom[1] - c.zoom[0]; c.zoom = clampView(rt - w / 2, rt + w / 2, c._a.full[0], c._a.full[1]); draw(c); } else cursorLine(c); pushLinked(c, rt - NEAR, rt + NEAR, k, true, {}); }
    else if (p.type === "spec") { p.r0 = rt - NEAR; p.r1 = rt + NEAR; p.si = null; draw(p); }
    uiSave();
  }
  async function tLst(b, p, tok) {
    const f = cFile(p); if (!f) return void (b.innerHTML = `<div class="muted">${I18N.t("bn.pickCell")}</div>`);
    const t = await J(`api/scanlist?k=${f.k}&level=${cLevel(p)}${p.filt ? "&filt=" + encodeURIComponent(p.filt) : ""}`).catch(e => ({ error: e.message }));
    if (tok !== ist.tok) return;
    if (t.error) return void (b.innerHTML = `<span class="fail">${esc(t.error)}</span>`);
    const cols = [["no", "N"], ["rt", "RT"], ["prec", I18N.t("bn.l.prec")], ["tic", "TIC"], ["bpmz", I18N.t("bn.l.bp")], ["bpint", I18N.t("bn.l.int")]], H = 20;
    let idx = t.sid.map((_, i) => i), sk = null, sd = 1;
    b.innerHTML = `<div class="muted" style="margin-bottom:3px">${I18N.t("bn.l.count", { n: t.n })}${p.filt ? " · " + esc(p.filt) : ` · MS${cLevel(p)}`}. ${I18N.t("bn.l.click")}</div><div id="hri-list"><div class="hd">${cols.map(c => `<span data-c="${c[0]}">${c[1]}</span>`).join("")}</div><div class="rows" style="height:${t.n * H}px;position:relative"></div></div>`;
    const L = b.querySelector("#hri-list"), R = L.querySelector(".rows"), cur = cRt(p);
    const fx = { no: v => v, rt: v => num(v, 3), prec: v => (v == null ? "" : num(v, 3)), tic: v => (+v).toExponential(2), bpmz: v => num(v, 4), bpint: v => (+v).toExponential(2) };
    const paint = () => {
      const a = Math.max(0, Math.floor((L.scrollTop - H) / H)), z = Math.min(idx.length, a + Math.ceil(L.clientHeight / H) + 3);
      R.innerHTML = idx.slice(a, z).map((i, j) => `<div class="r${cur != null && Math.abs(t.rt[i] - cur) < 1e-4 ? " cur" : ""}" data-i="${i}" style="position:absolute;top:${(a + j) * H}px;left:0;right:0">${cols.map(c => `<span>${esc(fx[c[0]](t[c[0]][i]))}</span>`).join("")}</div>`).join("");
      R.querySelectorAll(".r").forEach(r => { r.onclick = () => { const i = +r.dataset.i; goScan(p, t.sid[i], t.rt[i], f.k); R.querySelectorAll(".r").forEach(x => x.classList.toggle("cur", x === r)); }; });
    };
    L.onscroll = paint;
    L.querySelectorAll(".hd span").forEach(h => { h.onclick = () => { const c = h.dataset.c; sd = sk === c ? -sd : 1; sk = c; idx.sort((x, y) => sd * ((t[c][x] ?? -Infinity) - (t[c][y] ?? -Infinity)) || x - y); L.scrollTop = 0; paint(); }; });
    paint();
    if (cur != null) { const i0 = t.rt.findIndex(v => v >= cur - 1e-4); if (i0 >= 0) { L.scrollTop = Math.max(0, i0 * H - 40); paint(); } }
  }
  async function tFil(b, p, tok) {
    const f = cFile(p); if (!f) return void (b.innerHTML = `<div class="muted">${I18N.t("bn.pickCell")}</div>`);
    const j = await J(`api/fileinfo?k=${f.k}`).catch(e => ({ error: e.message }));
    if (tok !== ist.tok) return;
    if (j.error) return void (b.innerHTML = `<span class="fail">${esc(j.error)}</span>`);
    const rg = Object.entries(j.mz_range || {}).map(([l, v]) => `MS${l}: ${num(v[0], 2)} – ${num(v[1], 2)}`).join(" · ");
    b.innerHTML = `<h4 style="margin-top:0">${I18N.t("side.file")}</h4>` + kv([[I18N.t("lib.col.name"), j.file], [I18N.t("bn.f.date"), j.start || I18N.t("bn.f.noDate")], [I18N.t("bn.f.rtRange"), `${num(j.rt[0], 2)} – ${num(j.rt[1], 2)}`], [I18N.t("bn.f.mzRange"), rg], [I18N.t("lib.all.scans"), `${j.scans} (${Object.entries(j.levels).map(([l, n]) => `MS${l}: ${n}`).join(", ")})`]])
      + `<h4>${I18N.t("bn.f.instrument")}</h4>` + kv([[I18N.t("bn.f.model"), j.instrument], [I18N.t("bn.f.serial"), j.serial], [I18N.t("bn.f.analyzers"), (j.analyzers || []).join(", ")], [I18N.t("bn.f.components"), (j.components || []).join(", ")]])
      + `<h4>${I18N.t("bn.f.software")}</h4>` + kv([["Software", (j.software || []).join(", ")], [I18N.t("bn.f.source"), (j.source_files || []).join(", ")], [I18N.t("bn.f.conversion"), (j.conversion || []).join(", ")]])
      + `<h4>${I18N.t("bn.f.byType")}</h4>` + kv(j.filters.map(g => [g.key, `${g.n} · RT ${num(g.rt0, 2)}–${num(g.rt1, 2)}`])) + `<div class="muted" style="margin-top:6px">${I18N.t("bn.f.note")}</div>`;
  }
  // the spectra already drawn by the cell: m/z of its highest peak, for the launchers
  const topMz = p => { const d = p && p._a && p._a.data && p._a.data[0] && p._a.data[0].d; if (!d || !d.y || !d.y.length) return null; let j = 0; d.y.forEach((v, i) => { if (v > d.y[j]) j = i; }); return d.mz[j]; };
  const specOf = p => (p.type === "spec" ? p : E.panels.find(q => q.type === "spec" && q.link === p.id));
  function tCmp(b, p) {
    const g = window.ELEM && ELEM.takeCtx(), sp0 = specOf(p), sp = (g && g.sp) || sp0, d = sp && sp._a && sp._a.data && sp._a.data[0] && sp._a.data[0].d;
    ELEM.panel(b, { mz: g ? g.mz : topMz(sp0), spec: (g && g.spec) || d, polarity: g ? g.polarity : (cFile(p) || {}).polarity, sp });
  }
  const ISO_AD = ["[M+H]+", "[M+Na]+", "[M+NH4]+", "[M+K]+", "[M]+", "[M-H]-", "[M]-"];
  function tIso(b, p) {
    const sp = specOf(p), iso = (sp && sp.iso) || {}, fw = iso.fw || {}, cd = iso.cd || {};
    const sel = (id, opts, cur) => `<select id="${id}">${opts.map(([v, l]) => `<option value="${v}"${String(cur) === String(v) ? " selected" : ""}>${l}</option>`).join("")}</select>`;
    const adRow = (a, c, k) => `<div class="row hri-ad"><label>${k ? "" : I18N.t("lib.col.adduct")} <select data-ad aria-label="${I18N.t("lib.col.adduct")}">${ISO_AD.map(x => `<option${x === a ? " selected" : ""}>${x}</option>`).join("")}</select></label><label>${I18N.t("bn.iso.conc")} <input data-conc type="number" min="0" max="1000" step="any" value="${c}" style="width:56px"></label>${k ? `<button type="button" data-delad aria-label="${I18N.t("bn.iso.delAd")}">×</button>` : ""}</div>`;
    const adds = iso.adds && iso.adds.length ? iso.adds : [{ ad: iso.ad || "[M+H]+", c: 100 }];
    b.innerHTML = `<div class="muted">${I18N.t("bn.iso.note")}</div>
      <div class="row"><label>${I18N.t("bn.iso.formula")} <input id="hri-f" placeholder="C13H24N4O3S" value="${iso.formula ? esc(iso.formula) : ""}" style="width:130px"></label></div>
      <div id="hri-ads">${adds.map((a, k) => adRow(a.ad, a.c == null ? 100 : a.c, k)).join("")}</div>
      <div class="row"><button type="button" id="hri-addad">${I18N.t("bn.iso.addAd")}</button></div>
      <h4>${I18N.t("bn.iso.cd")}</h4>
      <div class="row"><label title="${I18N.t("bn.iso.cdTitle")}">${I18N.t("bn.iso.z0")} <input id="hri-z0" type="number" min="1" max="60" step="any" value="${cd.z0 ?? ""}" placeholder="1" style="width:52px"></label><label>${I18N.t("bn.iso.hw")} <input id="hri-hw" type="number" min="0" max="60" step="any" value="${cd.hw ?? ""}" placeholder="0" style="width:52px"></label></div>
      <h4>${I18N.t("bn.iso.output")}</h4>
      <div class="row"><label>${sel("hri-st", [["barre", I18N.t("bn.iso.bars")], ["profilo", I18N.t("bn.iso.profile")], ["centroidi", I18N.t("bn.iso.centroids")]], iso.style || "centroidi")}</label><label title="${I18N.t("bn.iso.ptsTitle")}">${I18N.t("bn.iso.pts")} <input id="hri-pts" value="${iso.pts || 10}" style="width:36px"></label></div>
      <h4>${I18N.t("bn.iso.resolution")}</h4>
      <div class="row"><label>${sel("hri-rm", [["R", I18N.t("bn.iso.rm.R")], ["da", I18N.t("bn.iso.rm.da")], ["ppm", I18N.t("bn.iso.rm.ppm")]], fw.mode || "R")} <input id="hri-r" placeholder="${I18N.t("bn.iso.auto")}" value="${fw.v != null ? fw.v : iso.R != null ? iso.R : ""}" style="width:72px"></label>
        <label title="${I18N.t("bn.iso.defTitle")}">${I18N.t("bn.iso.defAs")} ${sel("hri-df", [["fwhm", I18N.t("bn.iso.fwhm")], ["10", I18N.t("bn.iso.v10")], ["5", I18N.t("bn.iso.v5")]], iso.def || "fwhm")}</label></div>
      <div class="row"><button type="button" id="hri-go" class="go" title="${I18N.t("bn.iso.overTitle")}">${I18N.t("bn.iso.over")}</button><button type="button" id="hri-new" title="${I18N.t("bn.iso.newTitle")}">${I18N.t("bn.iso.new")}</button><button type="button" id="hri-rep" title="${I18N.t("bn.iso.repTitle")}">${I18N.t("bn.iso.rep")}</button><button type="button" id="hri-off">${I18N.t("lib.remove")}</button></div>
      <div class="muted sm">${I18N.t("bn.iso.hint")}</div>`;
    const wire = () => b.querySelectorAll("[data-delad]").forEach(x => { x.onclick = () => { x.closest(".hri-ad").remove(); }; });
    wire();
    b.querySelector("#hri-addad").onclick = () => {
      const rows = b.querySelectorAll(".hri-ad"); if (rows.length >= 4) return;
      const used = [...rows].map(r => r.querySelector("[data-ad]").value), nx = ISO_AD.find(x => !used.includes(x)) || ISO_AD[0];
      b.querySelector("#hri-ads").insertAdjacentHTML("beforeend", adRow(nx, 10, rows.length)); wire();
    };
    const read = async () => {
      const f = b.querySelector("#hri-f").value.trim(); if (!f) { nearMsg(I18N.t("bn.iso.typeFormula")); return null; }
      let r; try { r = await getFormula(f, ""); } catch (e) { nearMsg(I18N.t("bn.iso.failed", { message: e.message })); return null; }
      const ads = [...b.querySelectorAll(".hri-ad")].map(w => ({ ad: w.querySelector("[data-ad]").value, c: Math.max(0, numIt(w.querySelector("[data-conc]").value) || 0) })).filter(a => a.c > 0);
      if (!ads.length) { nearMsg(I18N.t("bn.iso.noSeries")); return null; }
      const o = { formula: r.formula, ad: ads[0].ad, style: b.querySelector("#hri-st").value, def: b.querySelector("#hri-df").value, pts: Math.max(3, parseInt(b.querySelector("#hri-pts").value) || 10) };
      if (ads.length > 1 || ads[0].c !== 100) o.adds = ads;
      const z0 = numIt(b.querySelector("#hri-z0").value), hw = numIt(b.querySelector("#hri-hw").value);
      if (z0 > 0 && (z0 > 1 || hw > 0)) o.cd = { z0, hw: hw > 0 ? hw : 0 };
      const mode = b.querySelector("#hri-rm").value, rv = b.querySelector("#hri-r").value.trim().replace(",", "."), x = parseFloat(rv);
      if (rv !== "" && x >= 0) { if (mode === "R") o.R = x; else if (x > 0) o.fw = { mode, v: x }; }
      return o;
    };
    const apply = (s2, o, only) => { s2.iso = { ...o, only: !!only }; let pat; try { pat = HR.isoPat(o, 0); } catch (e) { return nearMsg(e.message); } s2.zoom = [pat[0].mz - 4, pat[pat.length - 1].mz + 4]; s2._isoKey = JSON.stringify(s2.zoom); draw(s2); };
    b.querySelector("#hri-go").onclick = async () => { if (!sp) return nearMsg(I18N.t("bn.needSpec")); const o = await read(); if (o) apply(sp, o, false); };
    b.querySelector("#hri-rep").onclick = async () => { if (!sp) return nearMsg(I18N.t("bn.needSpec")); const o = await read(); if (o) apply(sp, o, true); };
    b.querySelector("#hri-new").onclick = async () => {
      if (!sp) return nearMsg(I18N.t("bn.needSpec")); const o = await read(); if (!o) return;
      const n = addPanel("spec", { tab: sp.tab, k: sp.k, level: sp.level, r0: sp.r0, r1: sp.r1, filt: sp.filt || null, title: PT.isosim, link: null, x: 0, y: E.panels.reduce((m, q) => Math.max(m, q.y + q.h + pnlGap()), 0), w: hostWidth(), h: 300, full: true });
      relayout(); fitHost(); apply(n, o, false); setActive(n); n.el.scrollIntoView({ block: "center", behavior: "smooth" }); uiSave();
    };
    b.querySelector("#hri-off").onclick = () => { if (sp) { sp.iso = null; draw(sp); } };
  }
  function tIdn(b, p) {
    const sp = specOf(p), f = cFile(p);
    b.innerHTML = `<div class="muted">${I18N.t("bn.idn.note")}</div><div class="row"><button type="button" id="hri-s">${I18N.t("bn.idn.search")}</button><button type="button" id="hri-a">${I18N.t("bn.idn.all")}</button></div><div class="muted sm">${I18N.t("bn.idn.hint")}</div>`;
    b.querySelector("#hri-s").onclick = () => { if (sp && sp.level === 2 && window.LIB) LIB.searchFrom(sp); else nearMsg(I18N.t("bn.idn.needMs2")); };
    b.querySelector("#hri-a").onclick = () => { const f2 = f && (f.kind === "ms2" ? f : E.files.find(x => !x.gone && baseOf(x) === baseOf(f) && x.kind === "ms2")); if (f2 && window.LIB) LIB.identifyAll(f2.k); else nearMsg(I18N.t("lib.noMs2")); };
  }
  const BODY = { hdr: tHdr, lst: tLst, fil: tFil, cmp: tCmp, iso: tIso, idn: tIdn, disp: (b, p) => DISP.panel(b, p), alb: (b, p, tok) => INF.render(b, p, tok) };
  let infoRaf = 0;
  function renderInfo() {
    const card = Q("#hrinfo"); if (!card || card.hidden) return;
    FRAME.cancel(infoRaf);
    infoRaf = FRAME.next(() => {
      const p = act(), b = Q("#hri-body"); if (!b) return;
      Q("#hri-ttl").textContent = (I9.find(x => x[0] === ist.tab) || [])[1] + (p ? "" : "");
      if (!p) return void (b.innerHTML = `<div class="muted">${I18N.t("bn.noCellText")}</div>`);
      const tok = ist.tok = (ist.tok || 0) + 1; b.innerHTML = `<div class="muted">…</div>`;
      Promise.resolve(BODY[ist.tab](b, p, tok)).catch(e => { if (tok === ist.tok) b.innerHTML = `<span class="fail">${esc(e.message)}</span>`; });
    });
  }
  function setInfoTab(t) {
    if (I9.some(x => x[0] === t)) {
      ist.tab = t;
      isave();
    }
  }
  function infoBar() {
    const aside = Q("#dfiles"); if (!aside) return;
    let c = Q("#hrinfo");
    if (!sync()) {
      if (c) c.hidden = true;
      if (window.BARRA) BARRA.syncMode();
      return;
    }
    if (!c) {
      c = document.createElement("div"); c.id = "hrinfo"; c.className = "card"; c.setAttribute("aria-label", I18N.t("side.info.aria"));
      c.innerHTML = `<div id="hri-ttl"></div><div id="hri-body"></div>`;
      const pnlContainer = Q("#sb-panels") || aside;
      pnlContainer.appendChild(c);
      c.style.height = ist.h + "px";
      FRAME.ro(c, () => { if (c.offsetHeight > 120) { ist.h = c.offsetHeight; isave(); } });
    }
    if (window.BARRA) {
      BARRA.syncMode();
      c.hidden = !(BARRA.isOpen() && ["hdr", "lst", "fil", "cmp", "iso", "idn", "disp", "alb"].includes(BARRA.curTab));
    } else {
      c.hidden = false;
    }
    renderInfo();
  }
  function bar() {
    const host = Q("#tools"); if (!host) return;
    let b = Q("#hrbar");
    if (!sync()) { if (b) b.hidden = true; const ic = Q("#hrinfo"); if (ic) ic.hidden = true; return; }
    if (!b) {
      b = document.createElement("div"); b.id = "hrbar"; b.setAttribute("role", "toolbar"); b.setAttribute("aria-label", I18N.t("bn.toolbar"));
      host.insertAdjacentElement("beforebegin", b);
      b.innerHTML = barBtn("prev", I18N.t("bn.bar.prev"), ICON.prev) + barBtn("next", I18N.t("bn.bar.next"), ICON.next) + `<span class="sepv"></span>` +
        barBtn("zundo", I18N.t("bn.bar.zundo"), ICON.zundo) + barBtn("zredo", I18N.t("bn.bar.zredo"), ICON.zredo) + barBtn("zall", I18N.t("bn.bar.zall"), ICON.zall) +
        barBtn("norm", I18N.t("bn.bar.norm"), ICON.norm) + barBtn("pin", I18N.t("bn.bar.pinbtn"), IC_PIN) + `<span class="sepv"></span>` +
        barBtn("add", I18N.t("bn.bar.addcell"), ICON.add, null, I18N.t("bn.cell")) + barBtn("type", I18N.t("bn.bar.type"), ICON.type) + barBtn("del", I18N.t("bn.bar.del"), ICON.del) + `<span class="sepv"></span>` +
        barBtn("peaks", I18N.t("bn.bar.peaks"), ICON.peaks) +
        barBtn("lib", I18N.t("bn.bar.lib"), ICON.lib) + barBtn("tree", I18N.t("bn.bar.tree"), ICON.tree) + `<span class="sepv"></span>` +
        barBtn("copy", I18N.t("bn.bar.copy"), ICON.copy) + barBtn("xls", I18N.t("bn.bar.xls"), ICON.xls) + `<span class="act"></span>`;
      b.querySelectorAll("button[data-hb]").forEach(x => {
        const id = x.dataset.hb;
        x.onclick = e => {
          e.stopPropagation();
          if (id === "add") return menu(e, [["chrom", I18N.t("bn.m.chrom")], ["spec", I18N.t("bn.m.spec")], ["map", I18N.t("bn.m.map")], ["xic", I18N.t("bn.m.xic")], ["sub", I18N.t("bn.m.sub")]].map(([t, l]) => ({ label: l, fn: () => (t === "xic" ? openXic() : t === "sub" ? subXicCell() : addCell(t)) })));
          if (id === "type") return menu(e, [["chrom", I18N.t("bn.m.chrom")], ["spec", I18N.t("bn.m.spec")], ["map", I18N.t("bn.m.map")]].map(([t, l]) => ({ label: l, fn: () => retype(t) })));
          if (id === "copy") { const a = act(); if (!a) return; return menu(e, [{ label: I18N.t("bn.m.png"), fn: () => copyPng(a) }, { label: I18N.t("bn.m.data"), fn: () => copyText(a) }]); }
          if (FN[id]) FN[id]();
        };
      });
    }
    b.hidden = false; sync2(); infoBar();
  }
  return { token: () => ist.tok, infoBar, renderInfo, setInfoTab, afterLayout, bar, sync2, on, sync, header, decorate, groupsOf, setFilter, setPin, eligible, rangeSeries, addRange, dropRange, filesFor };
})();
window.BANCO = BANCO;
