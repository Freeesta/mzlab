// High-resolution mass spectrometry (HRMS) target screening calculation core.
"use strict";

const LISTE_TARGET_CORE = (() => {
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

  function parseRow(cols, headIndices, idx) {
    const [cId, cName, cNameIt, cFormula, cMass, cMh, cMna, cMnh4, cSmiles, cInchi, cEffect, cChem] = headIndices;
    const name = cName >= 0 && cols[cName] ? cols[cName].replace(/^"|"$/g, "") : `Compound_${idx}`;
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

    return {
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
    };
  }

  function parseTargetCsv(text) {
    const lines = String(text).replace(/^\uFEFF/, "").split(/\r?\n/).filter(l => l.trim() && !l.startsWith("#"));
    if (!lines.length) return [];
    const sep = (lines[0].match(/;/g) || []).length > (lines[0].match(/,/g) || []).length ? ";" : ",";
    const head = csvSplit(lines[0], sep);

    const headIndices = [
      matchCol(head, ["id", "identifier", "slug"]),
      matchCol(head, ["name_en", "name", "compound_name", "substance", "nome"]),
      matchCol(head, ["name_it", "nome_it"]),
      matchCol(head, ["neutral_formula", "molecular_formula", "neutralformula", "formula"]),
      matchCol(head, ["monoisotopic_mass", "exact_mass", "mass", "monoisotopicmass", "exactmass"]),
      matchCol(head, ["mz_mh", "mz_h", "m+h", "mh"]),
      matchCol(head, ["mz_mna", "mz_na", "m+na", "mna"]),
      matchCol(head, ["mz_mnh4", "mz_nh4", "m+nh4", "mnh4"]),
      matchCol(head, ["smiles"]),
      matchCol(head, ["inchikey"]),
      matchCol(head, ["effect_class", "effect", "classe"]),
      matchCol(head, ["chemical_class", "class"])
    ];

    const entries = [];
    for (let i = 1; i < lines.length; i++) {
      const cols = csvSplit(lines[i], sep);
      if (cols.length >= 2) entries.push(parseRow(cols, headIndices, i));
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
        const count = m[2] ? parseInt(m[2], 10) : 1;
        counts[m[1]] = (counts[m[1]] || 0) + count;
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

  function peakAt(rt, y, i, j, noise) {
    const apexIdx = Math.floor((i + j) / 2);
    const height = y[apexIdx];
    const snRatio = height / noise;
    const fracH = 0.05 * height;

    let left = i;
    while (left > 0 && y[left - 1] <= y[left] && y[left - 1] > fracH) left--;
    if (left > 0 && y[left - 1] <= fracH) left--;

    let right = j;
    while (right < y.length - 1 && y[right + 1] <= y[right] && y[right + 1] > fracH) right++;
    if (right < y.length - 1 && y[right + 1] <= fracH) right++;

    let area = 0.0;
    for (let k = left; k < right; k++) {
      const dt = rt[k + 1] - rt[k];
      area += (y[k] + y[k + 1]) * 0.5 * dt;
    }
    return { apex_idx: apexIdx, rt_apex: rt[apexIdx], rt_start: rt[left], rt_end: rt[right], height, area, sn_ratio: snRatio, right };
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
          const pk = peakAt(rt, y, i, j, noise);
          if (pk.area >= minArea && pk.sn_ratio >= minSn) peaks.push(pk);
          i = pk.right + 1;
          continue;
        }
      }
      i++;
    }
    return peaks;
  }

  function extractXic(ms1Spectra, lo, hi) {
    const rt = [], y = [];
    let maxY = 0.0;
    for (let i = 0; i < ms1Spectra.length; i++) {
      const s = ms1Spectra[i];
      rt.push(s.rt);
      let sumI = 0.0;
      const mzs = s.mz || [];
      const intensities = s.intensity || s.y || [];
      for (let j = 0; j < mzs.length; j++) {
        if (mzs[j] >= lo && mzs[j] <= hi) sumI += intensities[j];
      }
      if (sumI > maxY) maxY = sumI;
      y.push(sumI);
    }
    return { rt, y, maxY };
  }

  function expectedIsotopeRatio(neutralFormula, adduct) {
    const counts = parseFormulaCounts(neutralFormula);
    if (adduct === "[M+H]+") counts["H"] = (counts["H"] || 0) + 1;
    else if (adduct === "[M+Na]+") counts["Na"] = (counts["Na"] || 0) + 1;
    else if (adduct === "[M+NH4]+") {
      counts["N"] = (counts["N"] || 0) + 1;
      counts["H"] = (counts["H"] || 0) + 4;
    }
    const nC = counts["C"] || 0, nN = counts["N"] || 0;
    return Math.max(0.01, nC * 0.0108 + nN * 0.0037);
  }

  function computeIsotopeScore(neutralFormula, adduct, targetMz, spec, tolerancePpm) {
    if (!spec || !spec.mz) return null;
    const expM1Ratio = expectedIsotopeRatio(neutralFormula, adduct);
    const expM1Mz = targetMz + 1.003355;
    const tolM0 = targetMz * tolerancePpm * 1e-6, tolM1 = expM1Mz * tolerancePpm * 1e-6;
    let m0Int = 0.0, m1Int = 0.0, m1Obs = targetMz;
    const mzs = spec.mz, intensities = spec.intensity || spec.y || [];

    for (let i = 0; i < mzs.length; i++) {
      const m = mzs[i], inten = intensities[i];
      if (Math.abs(m - targetMz) <= tolM0 && inten > m0Int) m0Int = inten;
      if (Math.abs(m - expM1Mz) <= tolM1 && inten > m1Int) { m1Int = inten; m1Obs = m; }
    }

    if (m0Int > 0.0 && m1Int > 0.0) {
      const obsRatio = m1Int / m0Int;
      const dPpm = (m1Obs - expM1Mz) / expM1Mz * 1e6;
      const tm = Math.exp(-0.5 * Math.pow(dPpm / tolerancePpm, 2));
      const ti = Math.exp(-0.5 * Math.pow(Math.log(obsRatio / expM1Ratio) / 0.35, 2));
      return Math.round(tm * ti * 100) / 100;
    }
    return null;
  }

  function checkMs2Match(targetMz, rtApex, peakWidth, ms2Spectra, tolerancePpm) {
    const windowRt = Math.max(peakWidth, 0.05);
    for (let i = 0; i < ms2Spectra.length; i++) {
      const s = ms2Spectra[i];
      if (Math.abs(s.rt - rtApex) <= windowRt) {
        const prec = s.precursor != null ? s.precursor : s.prec;
        if (prec != null && Math.abs((prec - targetMz) / targetMz * 1e6) <= tolerancePpm) {
          return true;
        }
      }
    }
    return false;
  }

  function formatProposalAndMotif(peak, ppmError, isotopeScore, ms2Matched, tolerancePpm) {
    const isoVal = isotopeScore != null ? isotopeScore : 0.0;
    let proposal = "";
    if (peak.sn_ratio >= 5.0 && isoVal >= 0.7 && ms2Matched) proposal = "Prev. 1";
    else if (peak.sn_ratio >= 5.0 && isoVal >= 0.7) proposal = "Prev. 2";
    else if (peak.sn_ratio >= 3.0 && Math.abs(ppmError) <= tolerancePpm) proposal = "Prev. 3";
    else return null;

    const isoStr = isotopeScore != null && isotopeScore >= 0.7
      ? `isotope pattern confirmed (score ${isotopeScore.toFixed(2)})`
      : isotopeScore != null ? `isotope pattern unconfirmed (score ${isotopeScore.toFixed(2)})` : "isotope pattern unconfirmed";
    const ms2Str = ms2Matched ? "MS2 precursor matched" : "no MS2 precursor matched";
    const motif = `Peak at RT ${peak.rt_apex.toFixed(2)} min, S/N ${peak.sn_ratio.toFixed(1)}, mass error ${ppmError >= 0 ? "+" : ""}${ppmError.toFixed(1)} ppm, ${isoStr}, ${ms2Str}`;
    return { proposal, motif };
  }

  function evaluatePeak(entry, adduct, targetMz, ms1Spectra, ms2Spectra, peak, opts) {
    const spec = ms1Spectra[peak.apex_idx];
    const tolDa = targetMz * opts.tolerance_ppm * 1e-6;
    let observedMz = targetMz, maxApexI = 0.0;
    const mzs = spec.mz || [], intensities = spec.intensity || spec.y || [];

    for (let i = 0; i < mzs.length; i++) {
      if (mzs[i] >= targetMz - tolDa && mzs[i] <= targetMz + tolDa && intensities[i] > maxApexI) {
        maxApexI = intensities[i];
        observedMz = mzs[i];
      }
    }

    const ppmError = (observedMz - targetMz) / targetMz * 1e6;
    const peakWidth = peak.rt_end - peak.rt_start;
    const ms2Matched = checkMs2Match(targetMz, peak.rt_apex, peakWidth, ms2Spectra, opts.tolerance_ppm);
    const isotopeScore = computeIsotopeScore(entry.neutral_formula, adduct, targetMz, spec, opts.tolerance_ppm);
    const pm = formatProposalAndMotif(peak, ppmError, isotopeScore, ms2Matched, opts.tolerance_ppm);
    if (!pm) return null;

    return {
      id: entry.id, name_en: entry.name_en, neutral_formula: entry.neutral_formula, adduct,
      target_mz: targetMz, observed_mz: observedMz, ppm_error: ppmError,
      rt_apex: peak.rt_apex, rt_start: peak.rt_start, rt_end: peak.rt_end,
      height: peak.height, area: peak.area, sn_ratio: peak.sn_ratio,
      isotope_score: isotopeScore, ms2_matched: ms2Matched,
      proposal: pm.proposal, motif: pm.motif
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
      const ra = proposalRank(a.proposal), rb = proposalRank(b.proposal);
      return ra !== rb ? ra - rb : b.sn_ratio - a.sn_ratio;
    });
  }

  function cercaTargets(listEntries, spectra, options = {}) {
    const opts = {
      tolerance_ppm: options.tolerance_ppm || 5.0,
      min_sn: options.min_sn || 3.0,
      min_area: options.min_area || 1000.0,
      adducts: options.adducts || ["[M+H]+", "[M+Na]+", "[M+NH4]+"]
    };
    const ms1 = spectra.filter(s => s.level === 1), ms2 = spectra.filter(s => s.level === 2);
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
    const rows = matches.map(m => [
      m.id, `"${m.name_en.replace(/"/g, '""')}"`, m.neutral_formula, m.adduct,
      m.target_mz.toFixed(5), m.observed_mz.toFixed(5), m.ppm_error.toFixed(2),
      m.rt_apex.toFixed(4), m.rt_start.toFixed(4), m.rt_end.toFixed(4),
      m.height.toFixed(1), m.area.toFixed(1), m.sn_ratio.toFixed(1),
      m.isotope_score != null ? m.isotope_score.toFixed(2) : "",
      m.ms2_matched ? "true" : "false", m.proposal, `"${m.motif.replace(/"/g, '""')}"`
    ].join(","));
    return header.concat(rows).join("\n");
  }

  return {
    ADDUCT_DELTAS,
    csvSplit,
    matchCol,
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
    proposalRank,
    sortMatches,
    cercaTargets,
    csvExport
  };
})();

if (typeof module !== "undefined") module.exports = LISTE_TARGET_CORE;
if (typeof window !== "undefined") /** @type {any} */ (window).LISTE_TARGET_CORE = LISTE_TARGET_CORE;
