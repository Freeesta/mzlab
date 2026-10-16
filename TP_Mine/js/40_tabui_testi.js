// @ts-check
// TPMINE-PRIVATE  TABUI_T / TT: the help texts of Tabui, in English only whatever the language of the page (labels stay short; the explanation lives here). TT(key, params)
// replaces {name}.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  /** @type {{ en: Record<string, string> }} */
  const T = {
  en: {
    "ft.col.level": "Identification level (Schymanski scale): L5 appears with the treatment, L4 unique formula, L3 MS2 linked to the parent. Only the chemist writes 2b.",
    "ft.col.pred": "Prediction (Löffler et al. 2025): how expected the transformation is, from 4 generic rule to 1 confirmed by the data. A different scale: «Prev. 3» is not «L3».",
    "ft.col.identity": "What it might be: formula and transformation from the parent, or «m/z … (unexpected)» for a new ion.",
    "ft.col.mz": "Measured m/z (apex). At unit resolution it is a candidate, not an identification.",
    "ft.col.rt": "Retention time of the apex, in minutes.",
    "ft.col.trend": "Area in the treated samples in time order; the symbols below tell how each area was obtained.",
    "ft.col.cells": "One symbol per sample: how the area was obtained. Open the row, Samples tab, for the numbers.",
    "ft.col.maxarea": "Highest area among the samples (engine units), in scientific notation: 821.43e+6 = 821 430 000. The export also has the true number.",
    "ft.col.status": "«Detected» = a peak found in at least one sample. «Searched, not detected» = an expected transformation that was searched and has no peak: not a product, only the expected m/z.",
    "ft.col.tpname": "Name of the product from the ion m/z: TP 362. Isomers (same formula, other RT): A, B... in RT order; same nominal mass but another formula: _31, _22 (decimals of the m/z).",
    "ft.col.net": "Formula only: what the product loses and gains against the parent, e.g. −C2H3 +O3. «—» for the parent.",
    "ft.col.modif": "The transformation in plain words, from the name of the rule that proposed it (e.g. «Oxidation to carbonyl»).",
    "ft.col.score": "Ranking score (0-100): share of the criteria passed. Not a probability.",
    "ft.col.ppm": "Mass error of the proposed formula, in ppm.",
    "ft.col.drt": "RT difference from the parent, in minutes: a more polar product elutes earlier.",
    "ft.col.sim": "Similarity of the MS2 with the parent's: equal fragments, shifted ones, common neutral losses.",
    "ft.col.ms2": "Number of MS2 scans inside the peak.",
    "ft.col.tmax": "Treatment time at which the area is highest, in minutes.",
    "ft.col.class": "Shape of the kinetics: rises, goes through a maximum, falls.",
    "ft.col.region": "Part of the molecule where the MS2 places the modification.",
    "ft.col.rank": "Place in the order of the engine: level, then score.",
    "ft.state.detected": "● detected: the peak was found by the detection in this sample.",
    "ft.state.filled": "◐ filled: no peak was found; the area was re-read where the feature was expected.",
    "ft.state.targeted": "◌ targeted: searched on purpose (hypothesis or list), not found by the free detection.",
    "ft.state.absent": "○ absent: no signal.",
    "ft.chip.changed": "Only the rows that appear or grow with the treatment.",
    "ft.chip.noblank": "Hide the rows found in the blank or dark control.",
    "ft.chip.not0": "Hide the rows already present at t0.",
    "ft.chip.ms2": "Only the rows with at least one MS2.",
    "ft.chip.decided": "Only the rows where you decided a level.",
    "ft.chip.hidenottp": "Hides the rows that you marked as not a TP.",
    "ft.decided.note": "Your own note on this row: kept in the browser with the decision and written in the exports.",
    "ft.chip.level": "Only the rows at this level or better.",
    "ft.chip.diagnostic": "Diagnostic evidence: site determined with margin, consistent RT and kinetics. Only the chemist can turn it into 2b.",
    "ft.chip.unexpected": "New ions that do not come from a transformation rule.",
    "ft.chip.strong": "Candidates with a strong score (75 or more): almost all the criteria are passed. Still a candidate, not an identification.",
    "ft.chip.possible": "Candidates with a possible score (50 to 74): some criteria are not passed.",
    "ft.chip.weak": "Weak evidence: rows that fail a criterion a product cannot do without (a real chromatographic peak, persistence, formation over time, a parent that falls) or score below 50. Hidden by default, never removed: one click shows them.",
    "ft.alt": "Other candidate identities with the same mass (\"+2\" = two more). Click to open the Identity tab of the row with the list.",
    "ft.col.formula": "Molecular formula proposed for the ion, if there is one.",
    "ft.col.ms2match": "Number of MS2 experiments matched to this row. The list is in the «MS2 matches» section under the table.",
    "ft.lr.ms2link": "Opens the «MS2 matches» section under the table, where the matches can be corrected.",
    "ft.chip.region": "Only the rows whose MS2 places the modification in a part of the molecule.",
    "ft.chip.wait": "Becomes active when the engine has computed t0 and blank for this series; without it there are no numbers.",
    "ft.export.view": "Export the rows and columns you see, in the order you see them, with the filters written at the top.",
    "ft.pin.parent": "The parent stays at the top as a reference: filters and sorting do not move it.",
    "ft.chip.det2": "Only rows with a detected (not filled) peak in at least two files.",
    "ft.more.filters": "Opens the numeric filters: files, area, blank ratio, m/z and RT.",
    "ft.norm": "How the areas of the files are compared: none, sum or median of the areas.",
    "ft.col.id": "Number of the row in the table of the engine.",
    "ft.col.charge": "Charge of the ion, when the engine found it.",
    "ft.col.detn": "In how many files the peak was detected (not filled or targeted).",
    "ft.cols": "Add or remove optional columns; the first two always stay.",
    "ft.row.open": "Click or Enter: the row expands right below with everything that is known. Esc closes it.",
    "ft.tab.overview": "XIC of all the samples with the peak borders, and the spectrum (MS1 with isotopes, MS2).",
    "ft.tab.samples": "For every sample: type and time, area, state, apex RT and peak borders.",
    "ft.tab.identity": "Neutral mass, adduct, candidates with their evidence, proposed level and decided level.",
    "ft.tab.quality": "Peak shape, signal/noise, width, M+1 isotope, presence in blank and t0.",
    "ft.peak.borders": "Start and end of the peak (median of the files that detected it).",
    "ft.q.flags": "Peak warnings: irregular shape, low signal/noise, few points.",
    "ft.adduct": "Ion the engine chose for this row ([M+H]+, [M+Na]+…) and the neutral mass it gives.",
    "ft.showdata": "Opens the file in the Data view with the XIC of this m/z.",
    "ft.decided": "The level you decide. Kept in the browser for this series and written in the exports.",
    "ft.more": "Other results of the series, one open at a time.",
    "ft.f.name": "Name of the parent compound: type 2 letters to search the list, or leave it empty and give the formula.",
    "ft.f.formula": "Neutral molecular formula of the parent, for example C10H12N2O3S. If missing, the engine uses the SMILES.",
    "ft.f.smiles": "Structure of the parent (optional when there is a formula): used by the fragments and the groups of the rules.",
    "ft.f.adduct": "Ion with which the parent appears in the spectrum.",
    "ft.f.prt": "Retention time of the parent, in minutes. Empty = automatic.",
    "ft.f.tol": "XIC window around every m/z, in Da (or in ppm at high resolution).",
    "ft.f.rtt": "How far the RT of a peak may move between files, in minutes.",
    "ft.f.steps": "How many transformations can be combined in one product (1-3).",
    "ft.f.minf": "Low resolution: a peak must appear in at least this many treated samples, or in two consecutive ones at the same RT. 0 = off.",
    "ft.f.rtmin": "Ignore what elutes before this RT (unretained front), in minutes.",
    "ft.f.disc": "Also look for ions that grow over time without coming from a transformation rule. Slower.",
    "ft.f.chg": "Formula change of the rule, for example +O-H2.",
    "ft.chip.l5": "Only level 5 rows: exact mass only, no MS2 or library that explains them.",
    "ft.chip.pol": "Only the rows of this polarity (POS = positive ions, NEG = negative).",
    "ft.col.cand": "Best library candidate for the MS2 of the feature: a hypothesis with its evidence, not an identification. Without a candidate the m/z is shown.",
    "ft.col.entropy": "Spectral entropy: similarity of the MS2 with the library record, from 0 to 1.",
    "ft.col.ext": "Names that the source table (mzMine, GNPS2, MS-DIAL) gives the feature: evidence with its source, never a level.",
    "ft.col.members": "Group of features of the same compound (adducts, isotopes, in-source fragments) and how many there are.",
    "ft.col.nmass": "Neutral mass that explains most features of the group: a hypothesis, open it for the evidence of every link.",
    "ft.col.pol": "Polarity of the table: POS = positive ions, NEG = negative.",
    "ft.col.basis": "What the level rests on: spectral library, standard, predicted structure, the chemist's judgement.",
    "ft.col.reasons": "Why the level is this one: the checks passed or failed, in words.",
    "ft.tab.members": "The features of the group, with the role of each in the neutral mass; a button opens the feature.",
    "ft.go.banco": "Opens the Banco 2b on this feature, to review it and decide the level."
  }
  };
  /** @param {string} key @param {Record<string, string | number>} [params] */
  const TT = (key, params) => {
    let s = T.en[key] || "";
    if (params) s = s.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m));
    return s;
  };
  /** the messages of the engine and of the pipeline (errors, notices), English only: no `lang/` key, no language catalog */
  const E = {
    "error.featFailed": "The pipeline failed.",
    "error.featMemory": "Not enough memory for the «{fase}» phase on {files} files (about {stima_mb} MB needed of {budget_mb} MB). The phases already finished are saved; split the series into smaller groups.",
    "error.featWasmHash": "The pipeline engine did not download correctly (its checksum is wrong). Reload the page.",
    "error.featState": "The state of the hypotheses is not valid: search the hypotheses again.",
    "error.featWorker": "A worker of the pipeline stopped working. Try again; if it happens again, close other tabs to free memory.",
    "error.featNoHypotheses": "The hypotheses of the parent must be searched before the proofs are built.",
    "error.memoryBudget": "This file is too big for the memory the browser gives the calculation engine.",
    "error.badRecipe": "The parameters of the pipeline are not valid.",
    "error.badMzfeat": "This is not a valid .mzfeat file.",
    "error.io": "A file could not be read.",
    "tpmine.feat.degraded": "Table without proofs: not enough memory for the proofs on {files} files (phase {phase}, about {stima_mb} MB needed of {budget_mb} MB). The phases already finished are saved; split the series into smaller groups.",
    "tpmine.feat.restarted": "A worker of the pipeline stopped and was restarted: the step runs again.",
    "tpmine.feat.cancelled": "The pipeline was stopped.",
    "tpmine.centroided": "{name} was in profile: Tabui works on centroids and used a centroided copy.",
  };
  /** English text of a key of the engine or of the pipeline; an unknown key shows the message of the engine, or the key itself @param {string} key @param {Record<string, any>} [params] */
  const TE = (key, params) => {
    const p = params || {};
    if (!E[key]) return String(p.message || p.detail || key);
    return E[key].replace(/\{(\w+)\}/g, (m, k) => (k in p ? String(p[k]) : m));
  };
  G.TABUI_T = T; G.TT = TT; G.TABUI_E = E; G.TE = TE;
})();
