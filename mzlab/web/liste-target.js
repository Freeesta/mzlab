// High-resolution mass spectrometry (HRMS) target and suspect screening UI and coordination.
"use strict";

const LISTE_TARGET = (function() {
  const CORE = (typeof window !== "undefined" && /** @type {any} */ (window).LISTE_TARGET_CORE) ||
    (typeof globalThis !== "undefined" && globalThis["require"] ? globalThis["require"]("./liste-target-core.js") : {});

  const EH = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

  let builtinCache = null;
  async function loadBuiltinList() {
    if (builtinCache) return builtinCache;
    try {
      const resp = await fetch("sostanze_hrms.csv");
      if (resp.ok) {
        const text = await resp.text();
        builtinCache = (CORE.parseTargetCsv || parseTargetCsv)(text);
        return builtinCache;
      }
    } catch (_) {}
    return [];
  }

  function showMatch(m) {
    if (typeof /** @type {any} */ (window).xicDirect === "function") {
      /** @type {any} */ (window).xicDirect(m.target_mz);
    }
    const win = /** @type {any} */ (window);
    if (win.E && win.E.panels) {
      let sp = win.E.panels.find((/** @type {any} */ p) => p.type === "spec");
      if (!sp && typeof win.addPanel === "function") {
        sp = win.addPanel("spec", { r0: m.rt_apex - 0.02, r1: m.rt_apex + 0.02 });
      }
      if (sp) {
        sp.r0 = m.rt_apex - 0.02;
        sp.r1 = m.rt_apex + 0.02;
        sp.si = null;
        if (typeof win.draw === "function") win.draw(sp);
      }
    }
    if (typeof win.toast === "function") {
      win.toast(`Target: ${m.name_en} (${m.adduct}) at RT ${m.rt_apex.toFixed(2)} min`);
    }
  }

  function panelHtml() {
    return `
      <div class="target-panel" style="display:flex;flex-direction:column;gap:8px;padding:8px 10px;height:100%;box-sizing:border-box;overflow:auto">
        <div style="display:flex;align-items:center;gap:10px;flex-wrap:wrap;background:var(--soft,#f1f5f9);padding:8px 10px;border-radius:6px">
          <label style="font-size:12px;display:inline-flex;align-items:center;gap:4px">
            <b>Target list:</b>
            <select id="ts-list-sel" style="font-size:12px;padding:2px 4px">
              <option value="psychoactive">Psychoactive substances (645)</option>
              <option value="keller">Keller 2008 Contaminants</option>
              <option value="upload">Upload custom CSV...</option>
            </select>
          </label>
          <input type="file" id="ts-csv-file" accept=".csv,.txt" style="display:none">
          <button type="button" id="ts-upload-btn" style="font-size:12px" title="Load custom target list from CSV">Upload CSV</button>

          <label style="font-size:12px;display:inline-flex;align-items:center;gap:4px">
            <span>Tol &plusmn; ppm</span>
            <input type="number" id="ts-tol-ppm" value="5" step="0.5" min="0.1" style="width:50px;font-size:12px">
          </label>
          <label style="font-size:12px;display:inline-flex;align-items:center;gap:4px">
            <span>Min S/N</span>
            <input type="number" id="ts-min-sn" value="3" step="0.5" min="1" style="width:45px;font-size:12px">
          </label>

          <span style="font-size:12px;display:inline-flex;gap:6px;align-items:center">
            <b>Adducts:</b>
            <label><input type="checkbox" id="ts-ad-mh" checked> [M+H]+</label>
            <label><input type="checkbox" id="ts-ad-mna" checked> [M+Na]+</label>
            <label><input type="checkbox" id="ts-ad-mnh4" checked> [M+NH4]+</label>
          </span>

          <div style="display:inline-flex;gap:6px;margin-left:auto">
            <button type="button" id="ts-start" class="go" style="font-size:12px;font-weight:bold">Start Screening</button>
            <button type="button" id="ts-stop" style="display:none;font-size:12px">Stop</button>
            <button type="button" id="ts-export" style="font-size:12px">Export CSV</button>
          </div>
        </div>

        <div id="ts-status" style="display:none;font-size:12px;padding:4px 8px;background:var(--panel,#fff);border:1px solid var(--line,#cbd5e1);border-radius:4px"></div>

        <div style="display:flex;align-items:center;gap:8px;font-size:12px">
          <input type="text" id="ts-filter-txt" placeholder="Filter targets (name, Neutral formula)..." style="flex:1;padding:4px 6px">
          <span class="seg" id="ts-filter-prop">
            <button type="button" data-prop="all" class="on">All</button>
            <button type="button" data-prop="Prev. 1">Prev. 1</button>
            <button type="button" data-prop="Prev. 2">Prev. 2</button>
            <button type="button" data-prop="Prev. 3">Prev. 3</button>
          </span>
          <span id="ts-count" style="color:var(--muted,#64748b);font-weight:bold">0 matches</span>
        </div>

        <div style="flex:1;min-height:220px;overflow:auto;border:1px solid var(--line,#cbd5e1);border-radius:6px;background:var(--panel,#fff)">
          <table style="width:100%;border-collapse:collapse;font-size:12px">
            <thead>
              <tr style="position:sticky;top:0;background:var(--soft,#f1f5f9);z-index:2;border-bottom:1px solid var(--line,#cbd5e1)">
                <th style="width:72px;text-align:left;padding:4px 6px">Proposal</th>
                <th style="text-align:left;padding:4px 6px">Name</th>
                <th style="text-align:left;padding:4px 6px">Neutral formula</th>
                <th style="text-align:left;padding:4px 6px">Adduct</th>
                <th class="num" style="padding:4px 6px">Target <i>m/z</i></th>
                <th class="num" style="padding:4px 6px">Obs. <i>m/z</i></th>
                <th class="num" style="padding:4px 6px">&Delta; ppm</th>
                <th class="num" style="padding:4px 6px">RT (min)</th>
                <th class="num" style="padding:4px 6px">Height</th>
                <th class="num" style="padding:4px 6px">Area</th>
                <th class="num" style="padding:4px 6px">S/N</th>
                <th class="num" style="padding:4px 6px">Isotope</th>
                <th style="text-align:left;padding:4px 6px">MS2</th>
                <th style="text-align:left;padding:4px 6px">Motif</th>
              </tr>
            </thead>
            <tbody id="ts-tbody"></tbody>
          </table>
        </div>
      </div>
    `;
  }

  function renderRow(m, idx) {
    const badgeStyle = m.proposal === "Prev. 1" ? "color:#166534;background:#dcfce7;font-weight:bold"
      : m.proposal === "Prev. 2" ? "color:#1e40af;background:#dbeafe" : "color:#854d0e;background:#fef9c3";
    const deltaStr = (m.ppm_error >= 0 ? "+" : "") + m.ppm_error.toFixed(1);
    const ms2Badge = m.ms2_matched ? "<span style='color:#166534'>Yes</span>" : "<span class='muted'>No</span>";
    return `<tr data-m-idx="${idx}" style="cursor:pointer" title="Click to view XIC and spectrum at RT ${m.rt_apex.toFixed(2)} min">
      <td><span style="display:inline-block;padding:1px 6px;border-radius:4px;font-size:11px;${badgeStyle}">${EH(m.proposal)}</span></td>
      <td><b>${EH(m.name_en)}</b></td>
      <td class="font-mono">${EH(m.neutral_formula) || "&ndash;"}</td>
      <td>${EH(m.adduct)}</td>
      <td class="num font-mono">${m.target_mz.toFixed(4)}</td>
      <td class="num font-mono">${m.observed_mz.toFixed(4)}</td>
      <td class="num sm">${deltaStr}</td>
      <td class="num">${m.rt_apex.toFixed(2)}</td>
      <td class="num sm">${Math.round(m.height)}</td>
      <td class="num sm">${Math.round(m.area)}</td>
      <td class="num">${m.sn_ratio.toFixed(1)}</td>
      <td class="num">${m.isotope_score != null ? m.isotope_score.toFixed(2) : "&ndash;"}</td>
      <td>${ms2Badge}</td>
      <td class="sm muted">${EH(m.motif)}</td>
    </tr>`;
  }

  function getKellerEntries() {
    const win = /** @type {any} */ (window);
    if (win.LISTE && typeof win.LISTE.ensure === "function") {
      const idx = win.LISTE.ensure();
      if (idx && idx.pos) {
        return idx.pos.map((/** @type {any} */ it) => ({
          id: it.id || it.name,
          name_en: it.name,
          name_it: it.name,
          neutral_formula: it.formula || "",
          monoisotopic_mass: it.mz,
          mz_mh: it.mz,
          mz_mna: it.mz + (CORE.ADDUCT_DELTAS ? CORE.ADDUCT_DELTAS["[M+Na]+"] - CORE.ADDUCT_DELTAS["[M+H]+"] : 21.9819),
          mz_mnh4: it.mz + (CORE.ADDUCT_DELTAS ? CORE.ADDUCT_DELTAS["[M+NH4]+"] - CORE.ADDUCT_DELTAS["[M+H]+"] : 17.0265)
        }));
      }
    }
    return [];
  }

  function setupFilterEvents(container, refresh) {
    const txt = container.querySelector("#ts-filter-txt");
    const propSeg = container.querySelector("#ts-filter-prop");
    if (txt) txt.oninput = () => refresh();
    if (propSeg) {
      propSeg.querySelectorAll("button").forEach((/** @type {any} */ b) => {
        b.onclick = () => {
          propSeg.querySelectorAll("button").forEach((/** @type {any} */ x) => x.classList.remove("on"));
          b.classList.add("on");
          refresh();
        };
      });
    }
  }

  function setupUploadHandler(container, onCustomLoaded) {
    const btn = container.querySelector("#ts-upload-btn");
    const fileIn = container.querySelector("#ts-csv-file");
    const sel = container.querySelector("#ts-list-sel");
    if (!btn || !fileIn || !sel) return;
    btn.onclick = () => fileIn.click();
    fileIn.onchange = async () => {
      const file = fileIn.files && fileIn.files[0];
      if (!file) return;
      const text = await file.text();
      const fn = CORE.parseTargetCsv || parseTargetCsv;
      const customList = fn(text);
      let opt = sel.querySelector('option[value="upload"]');
      if (opt) opt.textContent = `${file.name} (${customList.length} substances)`;
      sel.value = "upload";
      onCustomLoaded(customList);
    };
  }

  function getSelectedAdducts(container) {
    const adducts = [];
    if (container.querySelector("#ts-ad-mh")?.checked) adducts.push("[M+H]+");
    if (container.querySelector("#ts-ad-mna")?.checked) adducts.push("[M+Na]+");
    if (container.querySelector("#ts-ad-mnh4")?.checked) adducts.push("[M+NH4]+");
    return adducts.length ? adducts : ["[M+H]+"];
  }

  async function executeScreening(entries, opts, callbacks) {
    const spectra = await fetchSessionSpectra();
    const ms1 = spectra.filter((/** @type {any} */ s) => s.level === 1);
    const ms2 = spectra.filter((/** @type {any} */ s) => s.level === 2);
    const stepBatch = Math.max(1, Math.floor(entries.length / 50));
    const hits = [];

    for (let i = 0; i < entries.length; i++) {
      if (callbacks.shouldAbort()) break;
      const entry = entries[i];
      for (const adduct of opts.adducts) {
        const targetMz = (CORE.calcTargetMz || calcTargetMz)(entry, adduct);
        if (!isFinite(targetMz)) continue;
        const tolDa = targetMz * opts.tolerance_ppm * 1e-6;
        const xic = (CORE.extractXic || extractXic)(ms1, targetMz - tolDa, targetMz + tolDa);
        if (xic.maxY <= 0.0) continue;
        const noise = (CORE.estimateNoise || estimateNoise)(xic.y);
        const peaks = (CORE.detectChromPeaks || detectChromPeaks)(xic.rt, xic.y, opts.min_sn, opts.min_area, noise);
        for (const pk of peaks) {
          const m = (CORE.evaluatePeak || evaluatePeak)(entry, adduct, targetMz, ms1, ms2, pk, opts);
          if (m) hits.push(m);
        }
      }
      if (i % stepBatch === 0 || i === entries.length - 1) {
        callbacks.onProgress(i + 1, entries.length, hits);
        await new Promise(r => setTimeout(r, 0));
      }
    }
    return hits;
  }

  function setupExport(btn, getMatches) {
    if (!btn) return;
    btn.onclick = () => {
      const csv = (CORE.csvExport || csvExport)(getMatches());
      const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
      const u = URL.createObjectURL(blob), a = document.createElement("a");
      a.href = u; a.download = `target_screening_${Date.now()}.csv`;
      document.body.appendChild(a); a.click(); a.remove();
    };
  }

  function mount(container) {
    if (!container) return;
    container.innerHTML = panelHtml();

    let allMatches = [], customList = [], abortScreening = false;
    const elTbody = container.querySelector("#ts-tbody");
    const elCount = container.querySelector("#ts-count");
    const elStatus = container.querySelector("#ts-status");
    const elListSel = container.querySelector("#ts-list-sel");
    const btnStart = container.querySelector("#ts-start");
    const btnStop = container.querySelector("#ts-stop");
    const btnExport = container.querySelector("#ts-export");

    function renderMatches() {
      if (!elTbody || !elCount) return;
      const q = (container.querySelector("#ts-filter-txt")?.value || "").trim().toLowerCase();
      const activeProp = container.querySelector("#ts-filter-prop button.on")?.dataset.prop || "all";
      let filtered = allMatches.filter(m => activeProp === "all" || m.proposal === activeProp);
      if (q) filtered = filtered.filter(m => (m.name_en + " " + m.neutral_formula + " " + m.adduct).toLowerCase().includes(q));
      elCount.textContent = `${filtered.length} matches`;
      if (!filtered.length) {
        const msg = allMatches.length ? "No targets match current filter." : "No target hits found.";
        elTbody.innerHTML = "<tr><td colspan=\"14\" class=\"muted\" style=\"text-align:center;padding:16px\">" + msg + "</td></tr>";
        return;
      }
      elTbody.innerHTML = filtered.map(renderRow).join("");
      elTbody.querySelectorAll("tr[data-m-idx]").forEach((/** @type {any} */ row) => {
        row.onclick = () => { const m = filtered[+row.dataset.mIdx]; if (m) showMatch(m); };
      });
    }

    async function startScreening() {
      abortScreening = false; allMatches = []; renderMatches();
      btnStart.style.display = "none"; btnStop.style.display = "inline-block";
      elStatus.style.display = "block"; elStatus.textContent = "Loading substance list and spectra...";
      const mode = elListSel?.value;
      const entries = mode === "upload" ? customList : (mode === "keller" ? getKellerEntries() : await loadBuiltinList());
      const opts = {
        tolerance_ppm: parseFloat(container.querySelector("#ts-tol-ppm")?.value) || 5.0,
        min_sn: parseFloat(container.querySelector("#ts-min-sn")?.value) || 3.0,
        min_area: 1000.0,
        adducts: getSelectedAdducts(container)
      };
      const callbacks = {
        shouldAbort: () => abortScreening,
        onProgress: (done, tot, hits) => {
          allMatches = (CORE.sortMatches || sortMatches)(hits);
          renderMatches();
          elStatus.textContent = `Screened ${done}/${tot} substances (${allMatches.length} proposed hits)...`;
        }
      };
      allMatches = await executeScreening(entries, opts, callbacks);
      allMatches = (CORE.sortMatches || sortMatches)(allMatches);
      renderMatches();
      btnStart.style.display = "inline-block"; btnStop.style.display = "none";
      elStatus.textContent = `Screening finished. ${allMatches.length} hits found in ${entries.length} target substances.`;
    }

    btnStart.onclick = startScreening;
    btnStop.onclick = () => { abortScreening = true; };
    setupExport(btnExport, () => allMatches);
    setupFilterEvents(container, renderMatches);
    setupUploadHandler(container, list => { customList = list; });
    renderMatches();
  }

  async function fetchSessionSpectra() {
    const win = /** @type {any} */ (window);
    if (win.MZLAB_SPECTRA_CACHE) return win.MZLAB_SPECTRA_CACHE;
    try {
      const r = await fetch("api/spectra?k=0&i0=0&i1=99999&level=0");
      if (r.ok) {
        const j = await r.json();
        if (j && j.scans) return j.scans;
      }
    } catch (_) {}
    return [];
  }

  // Fallbacks forwarding to CORE
  const parseTargetCsv = (t) => CORE.parseTargetCsv ? CORE.parseTargetCsv(t) : [];
  const calcTargetMz = (e, a) => CORE.calcTargetMz ? CORE.calcTargetMz(e, a) : NaN;
  const estimateNoise = (y) => CORE.estimateNoise ? CORE.estimateNoise(y) : 1.0;
  const detectChromPeaks = (rt, y, s, a, n) => CORE.detectChromPeaks ? CORE.detectChromPeaks(rt, y, s, a, n) : [];
  const extractXic = (sp, lo, hi) => CORE.extractXic ? CORE.extractXic(sp, lo, hi) : { rt: [], y: [], maxY: 0 };
  const evaluatePeak = (e, a, mz, ms1, ms2, pk, op) => CORE.evaluatePeak ? CORE.evaluatePeak(e, a, mz, ms1, ms2, pk, op) : null;
  const sortMatches = (m) => CORE.sortMatches ? CORE.sortMatches(m) : m;
  const cercaTargets = (l, s, o) => CORE.cercaTargets ? CORE.cercaTargets(l, s, o) : [];
  const csvExport = (m) => CORE.csvExport ? CORE.csvExport(m) : "";

  return {
    mount,
    loadBuiltinList,
    showMatch,
    cercaTargets,
    parseTargetCsv,
    csvExport,
    calcTargetMz,
    estimateNoise,
    detectChromPeaks,
    extractXic,
    evaluatePeak,
    sortMatches,
    core: () => CORE
  };
})();

if (typeof module !== "undefined") module.exports = LISTE_TARGET;
if (typeof window !== "undefined") /** @type {any} */ (window).LISTE_TARGET = LISTE_TARGET;
