// High-resolution mass spectrometry (HRMS) target and suspect screening engine.
"use strict";

const LISTE_TARGET = (() => {
  const ADDUCT_DELTAS = {
    "[M+H]+": 1.007276,
    "[M+Na]+": 22.989218,
    "[M+NH4]+": 18.033823,
    "[M-H]-": -1.007276,
    "[M+HCOO]-": 44.998201,
    "[M+Cl]-": 34.969402
  };

  function csvSplit(line, sep) {
    const out = [];
    let cur = "", q = false;
    for (let i = 0; i < line.length; i++) {
      const c = line[i];
      if (q) {
        if (c === '"' && line[i + 1] === '"') { cur += '"'; i++; }
        else if (c === '"') q = false;
        else cur += c;
      } else if (c === '"') q = true;
      else if (c === sep) { out.push(cur); cur = ""; }
      else cur += c;
    }
    out.push(cur);
    return out.map(s => s.trim());
  }

  function matchCol(head, candidates) {
    const clean = s => s.toLowerCase().replace(/[\s_\-\/]/g, "");
    const clHead = head.map(clean);
    for (const cand of candidates) {
      const idx = clHead.indexOf(clean(cand));
      if (idx >= 0) return idx;
    }
    return -1;
  }

  function parseTargetCsv(text) {
    const lines = String(text).replace(/^\uFEFF/, "").split(/\r?\n/).filter(l => l.trim() && !l.startsWith("#"));
    if (!lines.length) return [];
    const sep = (lines[0].match(/;/g) || []).length > (lines[0].match(/,/g) || []).length ? ";" : ",";
    const head = csvSplit(lines[0], sep);

    const cId = matchCol(head, ["id", "identifier", "slug"]);
    const cName = matchCol(head, ["name_en", "name", "compound_name", "substance", "nome"]);
    const cNameIt = matchCol(head, ["name_it", "nome_it"]);
    const cFormula = matchCol(head, ["neutral_formula", "molecular_formula", "neutralformula", "formula"]);
    const cMass = matchCol(head, ["monoisotopic_mass", "exact_mass", "mass", "monoisotopicmass", "exactmass"]);
    const cMh = matchCol(head, ["mz_mh", "mz_h", "m+h", "mh"]);
    const cMna = matchCol(head, ["mz_mna", "mz_na", "m+na", "mna"]);
    const cMnh4 = matchCol(head, ["mz_mnh4", "mz_nh4", "m+nh4", "mnh4"]);
    const cSmiles = matchCol(head, ["smiles"]);
    const cInchi = matchCol(head, ["inchikey"]);
    const cEffect = matchCol(head, ["effect_class", "effect", "classe"]);
    const cChem = matchCol(head, ["chemical_class", "class"]);

    const entries = [];
    for (let i = 1; i < lines.length; i++) {
      const cols = csvSplit(lines[i], sep);
      if (cols.length < 2) continue;
      const name = cName >= 0 && cols[cName] ? cols[cName].replace(/^"|"$/g, "") : `Compound_${i}`;
      const id = cId >= 0 && cols[cId] ? cols[cId] : name.toLowerCase().replace(/[^a-z0-9]+/g, "-");
      const neutral_formula = cFormula >= 0 && cols[cFormula] ? cols[cFormula].trim() : "";
      let mass = cMass >= 0 && cols[cMass] ? parseFloat(cols[cMass].replace(",", ".")) : NaN;
      let mzMh = cMh >= 0 && cols[cMh] ? parseFloat(cols[cMh].replace(",", ".")) : NaN;
      let mzMna = cMna >= 0 && cols[cMna] ? parseFloat(cols[cMna].replace(",", ".")) : NaN;
      let mzMnh4 = cMnh4 >= 0 && cols[cMnh4] ? parseFloat(cols[cMnh4].replace(",", ".")) : NaN;

      if (!isFinite(mass) && isFinite(mzMh)) mass = mzMh - ADDUCT_DELTAS["[M+H]+"];
      if (isFinite(mass)) {
        if (!isFinite(mzMh)) mzMh = mass + ADDUCT_DELTAS["[M+H]+"];
        if (!isFinite(mzMna)) mzMna = mass + ADDUCT_DELTAS["[M+Na]+"];
        if (!isFinite(mzMnh4)) mzMnh4 = mass + ADDUCT_DELTAS["[M+NH4]+"];
      }

      entries.push({
        id,
        name_en: name,
        name_it: cNameIt >= 0 && cols[cNameIt] ? cols[cNameIt] : name,
        neutral_formula,
        monoisotopic_mass: mass,
        mz_mh: mzMh,
        mz_mna: mzMna,
        mz_mnh4: mzMnh4,
        smiles: cSmiles >= 0 ? cols[cSmiles] : "",
        inchikey: cInchi >= 0 ? cols[cInchi] : "",
        effect_class: cEffect >= 0 ? cols[cEffect] : "",
        chemical_class: cChem >= 0 ? cols[cChem] : ""
      });
    }
    return entries;
  }

  function parseFormulaCounts(formula) {
    const counts = {};
    if (!formula) return counts;
    const regex = /([A-Z][a-z]*)(\d*)/g;
    let m;
    while ((m = regex.exec(formula)) !== null) {
      if (m[1]) {
        const el = m[1];
        const count = m[2] ? parseInt(m[2], 10) : 1;
        counts[el] = (counts[el] || 0) + count;
      }
    }
    return counts;
  }

  function calcTargetMz(entry, adduct) {
    if (adduct === "[M+H]+" && isFinite(entry.mz_mh)) return entry.mz_mh;
    if (adduct === "[M+Na]+" && isFinite(entry.mz_mna)) return entry.mz_mna;
    if (adduct === "[M+NH4]+" && isFinite(entry.mz_mnh4)) return entry.mz_mnh4;
    const delta = ADDUCT_DELTAS[adduct] || 0;
    if (isFinite(entry.monoisotopic_mass)) return entry.monoisotopic_mass + delta;
    return NaN;
  }

  function estimateNoise(y) {
    if (!y || !y.length) return 1.0;
    const sorted = y.slice().sort((a, b) => a - b);
    const mid = Math.floor(sorted.length / 2);
    const med = sorted.length % 2 === 0 && sorted.length > 1
      ? (sorted[mid - 1] + sorted[mid]) * 0.5
      : sorted[mid];
    const dev = y.map(v => Math.abs(v - med)).sort((a, b) => a - b);
    const devMid = Math.floor(dev.length / 2);
    const mad = dev.length % 2 === 0 && dev.length > 1
      ? (dev[devMid - 1] + dev[devMid]) * 0.5
      : dev[devMid];
    const n = mad * 1.4826;
    return n > 1e-6 ? n : 1.0;
  }

  function detectChromPeaks(rt, y, minSn, minArea, noise) {
    const n = y.length;
    if (n < 3) return [];
    const threshold = minSn * noise;
    const peaks = [];
    let i = 1;
    while (i < n - 1) {
      if (y[i] > y[i - 1] && y[i] >= y[i + 1] && y[i] >= threshold) {
        let j = i;
        while (j < n - 1 && y[j + 1] === y[i]) j++;
        if (j === n - 1 || y[j + 1] < y[i]) {
          const apexIdx = Math.floor((i + j) / 2);
          const height = y[apexIdx];
          const snRatio = height / noise;
          const fracH = 0.05 * height;

          let left = i;
          while (left > 0 && y[left - 1] <= y[left] && y[left - 1] > fracH) left--;
          if (left > 0 && y[left - 1] <= fracH) left--;

          let right = j;
          while (right < n - 1 && y[right + 1] <= y[right] && y[right + 1] > fracH) right++;
          if (right < n - 1 && y[right + 1] <= fracH) right++;

          let area = 0.0;
          for (let k = left; k < right; k++) {
            const dt = rt[k + 1] - rt[k];
            const avgY = (y[k] + y[k + 1]) * 0.5;
            area += avgY * dt;
          }

          if (area >= minArea && snRatio >= minSn) {
            peaks.push({
              apex_idx: apexIdx,
              rt_apex: rt[apexIdx],
              rt_start: rt[left],
              rt_end: rt[right],
              height,
              area,
              sn_ratio: snRatio
            });
          }
          i = right + 1;
          continue;
        }
      }
      i++;
    }
    return peaks;
  }

  function extractXic(ms1Spectra, lo, hi) {
    const rt = [];
    const y = [];
    let maxY = 0.0;
    for (let i = 0; i < ms1Spectra.length; i++) {
      const s = ms1Spectra[i];
      rt.push(s.rt);
      let sumI = 0.0;
      const mzs = s.mz || [];
      const intensities = s.intensity || s.y || [];
      for (let j = 0; j < mzs.length; j++) {
        const m = mzs[j];
        if (m >= lo && m <= hi) {
          sumI += intensities[j];
        }
      }
      if (sumI > maxY) maxY = sumI;
      y.push(sumI);
    }
    return { rt, y, maxY };
  }

  function computeIsotopeScore(neutralFormula, adduct, targetMz, spec, tolerancePpm) {
    if (!spec || !spec.mz) return null;
    const counts = parseFormulaCounts(neutralFormula);
    if (adduct === "[M+H]+") counts["H"] = (counts["H"] || 0) + 1;
    else if (adduct === "[M+Na]+") counts["Na"] = (counts["Na"] || 0) + 1;
    else if (adduct === "[M+NH4]+") {
      counts["N"] = (counts["N"] || 0) + 1;
      counts["H"] = (counts["H"] || 0) + 4;
    }

    const nC = counts["C"] || 0;
    const nN = counts["N"] || 0;
    const expM1Ratio = Math.max(0.01, nC * 0.0108 + nN * 0.0037);
    const expM1Mz = targetMz + 1.003355;
    const tolM0 = targetMz * tolerancePpm * 1e-6;
    const tolM1 = expM1Mz * tolerancePpm * 1e-6;

    let m0Int = 0.0;
    let m1Int = 0.0;
    let m1Obs = targetMz;
    const mzs = spec.mz;
    const intensities = spec.intensity || spec.y || [];

    for (let i = 0; i < mzs.length; i++) {
      const m = mzs[i];
      const inten = intensities[i];
      if (Math.abs(m - targetMz) <= tolM0 && inten > m0Int) {
        m0Int = inten;
      }
      if (Math.abs(m - expM1Mz) <= tolM1 && inten > m1Int) {
        m1Int = inten;
        m1Obs = m;
      }
    }

    if (m0Int > 0.0 && m1Int > 0.0) {
      const obsRatio = m1Int / m0Int;
      const dPpm = (m1Obs - expM1Mz) / expM1Mz * 1e6;
      const tm = Math.exp(-0.5 * Math.pow(dPpm / tolerancePpm, 2));
      const ti = Math.exp(-0.5 * Math.pow(Math.log(obsRatio / expM1Ratio) / 0.35, 2));
      const s = tm * ti;
      return Math.round(s * 100) / 100;
    }
    return null;
  }

  function checkMs2Match(targetMz, rtApex, peakWidth, ms2Spectra, tolerancePpm) {
    const windowRt = Math.max(peakWidth, 0.05);
    for (let i = 0; i < ms2Spectra.length; i++) {
      const s = ms2Spectra[i];
      if (Math.abs(s.rt - rtApex) <= windowRt) {
        const prec = s.precursor != null ? s.precursor : s.prec;
        if (prec != null) {
          const diffPpm = Math.abs((prec - targetMz) / targetMz * 1e6);
          if (diffPpm <= tolerancePpm) return true;
        }
      }
    }
    return false;
  }

  function formatProposalAndMotif(peak, ppmError, isotopeScore, ms2Matched, tolerancePpm) {
    const isoVal = isotopeScore != null ? isotopeScore : 0.0;
    let proposal = "";
    if (peak.sn_ratio >= 5.0 && isoVal >= 0.7 && ms2Matched) {
      proposal = "Prev. 1";
    } else if (peak.sn_ratio >= 5.0 && isoVal >= 0.7) {
      proposal = "Prev. 2";
    } else if (peak.sn_ratio >= 3.0 && Math.abs(ppmError) <= tolerancePpm) {
      proposal = "Prev. 3";
    } else {
      return null;
    }

    const isoStr = isotopeScore != null && isotopeScore >= 0.7
      ? `isotope pattern confirmed (score ${isotopeScore.toFixed(2)})`
      : isotopeScore != null
        ? `isotope pattern unconfirmed (score ${isotopeScore.toFixed(2)})`
        : "isotope pattern unconfirmed";
    const ms2Str = ms2Matched ? "MS2 precursor matched" : "no MS2 precursor matched";
    const motif = `Peak at RT ${peak.rt_apex.toFixed(2)} min, S/N ${peak.sn_ratio.toFixed(1)}, mass error ${ppmError >= 0 ? "+" : ""}${ppmError.toFixed(1)} ppm, ${isoStr}, ${ms2Str}`;

    return { proposal, motif };
  }

  function evaluatePeak(entry, adduct, targetMz, ms1Spectra, ms2Spectra, peak, opts) {
    const spec = ms1Spectra[peak.apex_idx];
    const tolDa = targetMz * opts.tolerance_ppm * 1e-6;
    let observedMz = targetMz;
    let maxApexI = 0.0;
    const mzs = spec.mz || [];
    const intensities = spec.intensity || spec.y || [];

    for (let i = 0; i < mzs.length; i++) {
      const m = mzs[i];
      const inten = intensities[i];
      if (m >= targetMz - tolDa && m <= targetMz + tolDa && inten > maxApexI) {
        maxApexI = inten;
        observedMz = m;
      }
    }

    const ppmError = (observedMz - targetMz) / targetMz * 1e6;
    const peakWidth = peak.rt_end - peak.rt_start;
    const ms2Matched = checkMs2Match(targetMz, peak.rt_apex, peakWidth, ms2Spectra, opts.tolerance_ppm);
    const isotopeScore = computeIsotopeScore(entry.neutral_formula, adduct, targetMz, spec, opts.tolerance_ppm);

    const pm = formatProposalAndMotif(peak, ppmError, isotopeScore, ms2Matched, opts.tolerance_ppm);
    if (!pm) return null;

    return {
      id: entry.id,
      name_en: entry.name_en,
      neutral_formula: entry.neutral_formula,
      adduct,
      target_mz: targetMz,
      observed_mz: observedMz,
      ppm_error: ppmError,
      rt_apex: peak.rt_apex,
      rt_start: peak.rt_start,
      rt_end: peak.rt_end,
      height: peak.height,
      area: peak.area,
      sn_ratio: peak.sn_ratio,
      isotope_score: isotopeScore,
      ms2_matched: ms2Matched,
      proposal: pm.proposal,
      motif: pm.motif
    };
  }

  function proposalRank(p) {
    if (p === "Prev. 1") return 1;
    if (p === "Prev. 2") return 2;
    if (p === "Prev. 3") return 3;
    return 4;
  }

  function sortMatches(matches) {
    return matches.slice().sort((a, b) => {
      const ra = proposalRank(a.proposal);
      const rb = proposalRank(b.proposal);
      if (ra !== rb) return ra - rb;
      return b.sn_ratio - a.sn_ratio;
    });
  }

  function cercaTargets(listEntries, spectra, options = {}) {
    const opts = {
      tolerance_ppm: options.tolerance_ppm || 5.0,
      min_sn: options.min_sn || 3.0,
      min_area: options.min_area || 1000.0,
      adducts: options.adducts || ["[M+H]+", "[M+Na]+", "[M+NH4]+"]
    };

    const ms1 = spectra.filter(s => s.level === 1);
    const ms2 = spectra.filter(s => s.level === 2);
    if (!ms1.length || !listEntries.length) return [];

    const matches = [];
    for (let i = 0; i < listEntries.length; i++) {
      const entry = listEntries[i];
      for (let j = 0; j < opts.adducts.length; j++) {
        const adduct = opts.adducts[j];
        const targetMz = calcTargetMz(entry, adduct);
        if (!isFinite(targetMz)) continue;
        const tolDa = targetMz * opts.tolerance_ppm * 1e-6;
        const xic = extractXic(ms1, targetMz - tolDa, targetMz + tolDa);
        if (xic.maxY <= 0.0) continue;

        const noise = estimateNoise(xic.y);
        const peaks = detectChromPeaks(xic.rt, xic.y, opts.min_sn, opts.min_area, noise);
        for (let k = 0; k < peaks.length; k++) {
          const m = evaluatePeak(entry, adduct, targetMz, ms1, ms2, peaks[k], opts);
          if (m) matches.push(m);
        }
      }
    }
    return sortMatches(matches);
  }

  function csvExport(matches) {
    const header = [
      "# mzLab Target screening export - " + new Date().toISOString().replace("T", " ").slice(0, 19),
      "id,name,neutral_formula,adduct,theoretical_mz,observed_mz,ppm_error,rt_apex,rt_start,rt_end,height,area,sn_ratio,isotope_score,ms2_matched,proposal,motif"
    ];
    const rows = matches.map(m => {
      const fields = [
        m.id,
        `"${m.name_en.replace(/"/g, '""')}"`,
        m.neutral_formula,
        m.adduct,
        m.target_mz.toFixed(5),
        m.observed_mz.toFixed(5),
        m.ppm_error.toFixed(2),
        m.rt_apex.toFixed(4),
        m.rt_start.toFixed(4),
        m.rt_end.toFixed(4),
        m.height.toFixed(1),
        m.area.toFixed(1),
        m.sn_ratio.toFixed(1),
        m.isotope_score != null ? m.isotope_score.toFixed(2) : "",
        m.ms2_matched ? "true" : "false",
        m.proposal,
        `"${m.motif.replace(/"/g, '""')}"`
      ];
      return fields.join(",");
    });
    return header.concat(rows).join("\n");
  }

  // ---------------------------------------------------------------- UI & Session
  let builtinCache = null;
  async function loadBuiltinList() {
    if (builtinCache) return builtinCache;
    try {
      const resp = await fetch("sostanze_hrms.csv");
      if (resp.ok) {
        const text = await resp.text();
        builtinCache = parseTargetCsv(text);
        return builtinCache;
      }
    } catch (_) {}
    return [];
  }

  function showMatch(m) {
    if (typeof xicDirect === "function") {
      xicDirect(m.target_mz);
    }
    if (typeof E !== "undefined" && E.panels) {
      let sp = E.panels.find(p => p.type === "spec");
      if (!sp && typeof addPanel === "function") {
        sp = addPanel("spec", { r0: m.rt_apex - 0.02, r1: m.rt_apex + 0.02 });
      }
      if (sp) {
        sp.r0 = m.rt_apex - 0.02;
        sp.r1 = m.rt_apex + 0.02;
        sp.si = null;
        if (typeof draw === "function") draw(sp);
      }
    }
    if (typeof toast === "function") {
      toast(`Target: ${m.name_en} (${m.adduct}) at RT ${m.rt_apex.toFixed(2)} min`);
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
          <span id="ts-count" class="muted" style="margin-left:4px">0 matches</span>
        </div>

        <div id="ts-table-wrap" style="flex:1;min-height:220px;overflow:auto;border:1px solid var(--line,#e2e8f0);border-radius:6px;background:var(--panel,#fff)">
          <table class="lct" style="width:100%;border-collapse:collapse;font-size:12px">
            <thead>
              <tr style="position:sticky;top:0;background:var(--soft,#f1f5f9);z-index:2">
                <th style="width:75px">Proposal</th>
                <th>Substance</th>
                <th>Neutral formula</th>
                <th>Adduct</th>
                <th class="num" style="width:75px"><i>m/z</i> calc</th>
                <th class="num" style="width:75px"><i>m/z</i> obs</th>
                <th class="num" style="width:60px">&Delta; ppm</th>
                <th class="num" style="width:65px">RT (min)</th>
                <th class="num" style="width:60px">Height</th>
                <th class="num" style="width:60px">Area</th>
                <th class="num" style="width:50px">S/N</th>
                <th class="num" style="width:80px">Isotope score</th>
                <th style="width:55px">MS2</th>
                <th>Motif</th>
              </tr>
            </thead>
            <tbody id="ts-tbody">
              <tr><td colspan="14" class="muted" style="text-align:center;padding:24px">No target screening performed yet. Click &laquo;Start Screening&raquo; to search.</td></tr>
            </tbody>
          </table>
        </div>
      </div>
    `;
  }

  function mount(container) {
    if (!container) return;
    container.innerHTML = panelHtml();

    let allMatches = [];
    let customList = [];
    let isRunning = false;
    let shouldStop = false;

    const elListSel = container.querySelector("#ts-list-sel");
    const elCsvFile = container.querySelector("#ts-csv-file");
    const elUploadBtn = container.querySelector("#ts-upload-btn");
    const elTolPpm = container.querySelector("#ts-tol-ppm");
    const elMinSn = container.querySelector("#ts-min-sn");
    const elStart = container.querySelector("#ts-start");
    const elStop = container.querySelector("#ts-stop");
    const elExport = container.querySelector("#ts-export");
    const elStatus = container.querySelector("#ts-status");
    const elFilterTxt = container.querySelector("#ts-filter-txt");
    const elFilterProp = container.querySelector("#ts-filter-prop");
    const elCount = container.querySelector("#ts-count");
    const elTbody = container.querySelector("#ts-tbody");

    let currentFilterProp = "all";
    let currentFilterTxt = "";

    function renderRows() {
      let filtered = allMatches;
      if (currentFilterProp !== "all") {
        filtered = filtered.filter(m => m.proposal === currentFilterProp);
      }
      if (currentFilterTxt) {
        const q = currentFilterTxt.toLowerCase();
        filtered = filtered.filter(m => (m.name_en + " " + m.neutral_formula + " " + m.adduct).toLowerCase().includes(q));
      }

      elCount.textContent = `${filtered.length} matches`;
      if (!filtered.length) {
        elTbody.innerHTML = `<tr><td colspan="14" class="muted" style="text-align:center;padding:16px">${allMatches.length ? "No targets match current filter." : "No target hits found."}</td></tr>`;
        return;
      }

      const badgeClass = p => p === "Prev. 1" ? "color:#166534;background:#dcfce7;font-weight:bold" : p === "Prev. 2" ? "color:#1e40af;background:#dbeafe" : "color:#854d0e;background:#fef9c3";

      elTbody.innerHTML = filtered.map((m, idx) => `
        <tr data-m-idx="${idx}" style="cursor:pointer" title="Click to view XIC and spectrum at RT ${m.rt_apex.toFixed(2)} min">
          <td><span style="display:inline-block;padding:1px 6px;border-radius:4px;font-size:11px;${badgeClass(m.proposal)}">${m.proposal}</span></td>
          <td><b>${m.name_en}</b></td>
          <td class="font-mono">${m.neutral_formula || "&ndash;"}</td>
          <td>${m.adduct}</td>
          <td class="num font-mono">${m.target_mz.toFixed(4)}</td>
          <td class="num font-mono">${m.observed_mz.toFixed(4)}</td>
          <td class="num sm">${m.ppm_error >= 0 ? "+" : ""}${m.ppm_error.toFixed(1)}</td>
          <td class="num">${m.rt_apex.toFixed(2)}</td>
          <td class="num sm">${Math.round(m.height)}</td>
          <td class="num sm">${Math.round(m.area)}</td>
          <td class="num">${m.sn_ratio.toFixed(1)}</td>
          <td class="num">${m.isotope_score != null ? m.isotope_score.toFixed(2) : "&ndash;"}</td>
          <td>${m.ms2_matched ? "<span style='color:#166534'>Yes</span>" : "<span class='muted'>No</span>"}</td>
          <td class="sm muted">${m.motif}</td>
        </tr>
      `).join("");

      elTbody.querySelectorAll("tr[data-m-idx]").forEach(row => {
        row.onclick = () => {
          const m = filtered[+row.dataset.mIdx];
          if (m) showMatch(m);
        };
      });
    }

    async function getActiveEntries() {
      const mode = elListSel.value;
      if (mode === "upload") return customList;
      if (mode === "keller") {
        if (typeof LISTE !== "undefined" && typeof LISTE.ensure === "function") {
          const idx = LISTE.ensure();
          if (idx && idx.pos) {
            return idx.pos.map(it => ({
              id: it.id || it.name,
              name_en: it.name,
              name_it: it.name,
              neutral_formula: it.formula || "",
              monoisotopic_mass: it.mz,
              mz_mh: it.mz,
              mz_mna: NaN,
              mz_mnh4: NaN
            }));
          }
        }
      }
      return loadBuiltinList();
    }

    async function runScreening() {
      if (isRunning) return;
      const entries = await getActiveEntries();
      if (!entries.length) {
        if (typeof toast === "function") toast("No target entries available to screen.");
        return;
      }

      const tolPpm = parseFloat(elTolPpm.value) || 5.0;
      const minSn = parseFloat(elMinSn.value) || 3.0;
      const adducts = [];
      if (container.querySelector("#ts-ad-mh").checked) adducts.push("[M+H]+");
      if (container.querySelector("#ts-ad-mna").checked) adducts.push("[M+Na]+");
      if (container.querySelector("#ts-ad-mnh4").checked) adducts.push("[M+NH4]+");

      allMatches = [];
      isRunning = true;
      shouldStop = false;
      elStart.style.display = "none";
      elStop.style.display = "inline-block";
      elStatus.style.display = "block";
      elStatus.textContent = `Screening ${entries.length} substances...`;

      const k = typeof E !== "undefined" && E.cur != null ? E.cur : 0;
      let ms2List = [];
      try {
        if (typeof J === "function") {
          const dda = await J(`api/dda?k=${k}`);
          if (dda && dda.sid) {
            ms2List = dda.sid.map((sid, idx) => ({ sid, rt: dda.rt[idx], prec: dda.prec[idx] }));
          }
        }
      } catch (_) {}

      renderRows();

      for (let i = 0; i < entries.length; i++) {
        if (shouldStop) break;
        const entry = entries[i];
        if (i % 5 === 0) {
          elStatus.textContent = `Screening ${i + 1}/${entries.length}: ${entry.name_en}... (${allMatches.length} matches found)`;
          await new Promise(r => setTimeout(r, 0));
        }

        for (let j = 0; j < adducts.length; j++) {
          const adduct = adducts[j];
          const targetMz = calcTargetMz(entry, adduct);
          if (!isFinite(targetMz)) continue;
          const tolDa = targetMz * tolPpm * 1e-6;

          let xicData = null;
          try {
            if (typeof J === "function") {
              const res = await J(`api/xic?k=${k}&mz=${targetMz}&tol=${tolDa}&level=1`);
              if (res && res.traces && res.traces[0]) xicData = res.traces[0];
            }
          } catch (_) {}

          if (!xicData || !xicData.y || !xicData.y.length) continue;
          const noise = estimateNoise(xicData.y);
          const peaks = detectChromPeaks(xicData.rt, xicData.y, minSn, 1000.0, noise);
          if (!peaks.length) continue;

          for (let pIdx = 0; pIdx < peaks.length; pIdx++) {
            const peak = peaks[pIdx];
            let apexSpec = null;
            try {
              if (typeof J === "function") {
                apexSpec = await J(`api/spectrum?k=${k}&rt0=${peak.rt_apex - 0.01}&rt1=${peak.rt_apex + 0.01}&level=1`);
              }
            } catch (_) {}

            const peakWidth = peak.rt_end - peak.rt_start;
            const ms2Matched = checkMs2Match(targetMz, peak.rt_apex, peakWidth, ms2List, tolPpm);
            const isoScore = apexSpec ? computeIsotopeScore(entry.neutral_formula, adduct, targetMz, apexSpec, tolPpm) : null;

            let observedMz = targetMz;
            if (apexSpec && apexSpec.mz) {
              let maxI = 0.0;
              for (let mIdx = 0; mIdx < apexSpec.mz.length; mIdx++) {
                const m = apexSpec.mz[mIdx];
                const inten = (apexSpec.intensity || apexSpec.y || [])[mIdx];
                if (Math.abs(m - targetMz) <= tolDa && inten > maxI) {
                  maxI = inten;
                  observedMz = m;
                }
              }
            }
            const ppmError = (observedMz - targetMz) / targetMz * 1e6;
            const pm = formatProposalAndMotif(peak, ppmError, isoScore, ms2Matched, tolPpm);
            if (pm) {
              allMatches.push({
                id: entry.id,
                name_en: entry.name_en,
                neutral_formula: entry.neutral_formula,
                adduct,
                target_mz: targetMz,
                observed_mz: observedMz,
                ppm_error: ppmError,
                rt_apex: peak.rt_apex,
                rt_start: peak.rt_start,
                rt_end: peak.rt_end,
                height: peak.height,
                area: peak.area,
                sn_ratio: peak.sn_ratio,
                isotope_score: isoScore,
                ms2_matched: ms2Matched,
                proposal: pm.proposal,
                motif: pm.motif
              });
              allMatches = sortMatches(allMatches);
              renderRows();
            }
          }
        }
      }

      isRunning = false;
      elStart.style.display = "inline-block";
      elStop.style.display = "none";
      elStatus.textContent = shouldStop
        ? `Screening stopped. ${allMatches.length} matches found.`
        : `Screening completed across ${entries.length} substances. ${allMatches.length} matches found.`;
    }

    elStart.onclick = runScreening;
    elStop.onclick = () => { shouldStop = true; };
    elUploadBtn.onclick = () => elCsvFile.click();
    elCsvFile.onchange = async e => {
      const f = e.target.files[0];
      if (!f) return;
      const text = await f.text();
      customList = parseTargetCsv(text);
      elListSel.value = "upload";
      if (typeof toast === "function") toast(`Loaded custom list (${customList.length} substances)`);
      elStatus.style.display = "block";
      elStatus.textContent = `Custom list loaded: ${customList.length} substances.`;
    };

    elExport.onclick = () => {
      if (!allMatches.length) {
        if (typeof toast === "function") toast("No target matches to export.");
        return;
      }
      const csv = csvExport(allMatches);
      const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `mzlab_target_screening_${Date.now()}.csv`;
      a.click();
      URL.revokeObjectURL(url);
    };

    elFilterTxt.oninput = e => { currentFilterTxt = e.target.value.trim(); renderRows(); };
    elFilterProp.querySelectorAll("button").forEach(b => {
      b.onclick = () => {
        elFilterProp.querySelectorAll("button").forEach(x => x.classList.toggle("on", x === b));
        currentFilterProp = b.dataset.prop;
        renderRows();
      };
    });
  }

  return {
    ADDUCT_DELTAS,
    csvSplit,
    parseTargetCsv,
    parseFormulaCounts,
    calcTargetMz,
    estimateNoise,
    detectChromPeaks,
    extractXic,
    computeIsotopeScore,
    checkMs2Match,
    formatProposalAndMotif,
    evaluatePeak,
    sortMatches,
    cercaTargets,
    csvExport,
    loadBuiltinList,
    showMatch,
    panelHtml,
    mount
  };
})();

if (typeof module !== "undefined") module.exports = LISTE_TARGET;
if (typeof window !== "undefined") window.LISTE_TARGET = LISTE_TARGET;
