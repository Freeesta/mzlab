// @ts-check
"use strict";
// Data file «livelli/1»: the confidence levels (Schymanski et al. 2014; MSI 2007) and the profiles of thresholds. Pure data (JSON inside the assignment): the module
// livelli.js reads it, tests/test_livelli.py reads it, nothing here is code. A new version of the format is a new file (livelli-2.js), never an edit of the meaning of a key.
// A profile is a set of thresholds ("soglie") plus the gates of the cascade; the gate labels and the sentences are i18n keys (lang/it.js, lang/en.js).
// Profile "mzlab": the default. Profile "attributio": rules, thresholds and wording of Attributio 1.12.0 (annotate.py), hidden in the interface.
window.LIVELLI_DATI = {
 "formato": "livelli/1",
 "predefinito": "mzlab",
 "riferimento": "Schymanski et al., Environ. Sci. Technol. 2014, 48, 2097",
 "livelli": ["1", "2a", "2b", "3", "4", "5", "0"],
 "profili": {
  "mzlab": {
   "visibile": true,
   "soglie": {
    "entropia_min": 0.75,
    "picchi_min": 6,
    "picchi_min_pochi": 3,
    "precursore_ppm_max": 5,
    "scansioni_min": 2,
    "margine_min": 0.05,
    "entropia_provvisorio": 0.60,
    "analogo_mcos_min": 0.60,
    "std_rt_max": 0.1,
    "std_entropia_min": 0.8,
    "std_ppm_max": 5
   },
   "cancelli": [
    {"id": "prec", "campo": "ppm", "op": "<=", "soglia": "precursore_ppm_max", "assoluto": true, "unita": "ppm", "ruolo": "source"},
    {"id": "score", "campo": "ent", "op": ">=", "soglia": "entropia_min", "ruolo": "source"},
    {"id": "shared", "campo": "shared", "op": ">=", "soglia": "picchi_min", "ridotta": {"se_campo": "nlib", "minore_di": 6, "soglia": "picchi_min_pochi"}, "ruolo": "source"},
    {"id": "scans", "campo": "nScans", "op": ">=", "soglia": "scansioni_min", "ruolo": "confirm"},
    {"id": "distinct", "campo": "margin", "op": ">=", "soglia": "margine_min", "ruolo": "confirm"}
   ],
   "standard": [
    {"id": "stdrt", "campo": "drt", "op": "<=", "soglia": "std_rt_max", "assoluto": true, "unita": "min"},
    {"id": "stdscore", "campo": "ent", "op": ">=", "soglia": "std_entropia_min"},
    {"id": "stdppm", "campo": "ppm", "op": "<=", "soglia": "std_ppm_max", "assoluto": true, "unita": "ppm"}
   ],
   "provvisorio": {"campo": "ent", "soglia": "entropia_provvisorio"},
   "senza_prove": "5",
   "punti": {
    "versione": 1,
    "riferimento": "stepped points per proof after Alygizakis et al., TrAC 2023, as an explanation and never as a learned model: the total orders the candidates, the level stays the cascade above (2b and 1 only by the chemist). A proof 'against' earns no points and is never offset. Steps are listed best first; a string is the key of a threshold of 'soglie'.",
    "tolleranze": {"massa_ppm": "precursore_ppm_max", "margine_ms2": "margine_min"},
    "prove": {
     "massa": {"op": "<=", "assoluto": true, "unita": "ppm", "gradini": [{"soglia": 2, "punti": 2}, {"soglia": 5, "punti": 1}]},
     "isotopi": {"op": ">=", "provvisorio": true, "gradini": [{"soglia": 0.9, "punti": 2}, {"soglia": 0.7, "punti": 1}]},
     "MS2": {"op": ">=", "richiede": {"op": ">=", "soglia": "picchi_min"}, "gradini": [{"soglia": "entropia_min", "punti": 3}, {"soglia": "entropia_provvisorio", "punti": 1}]},
     "frammenti": {"op": ">=", "provvisorio": true, "gradini": [{"soglia": 0.8, "punti": 2}, {"soglia": 0.5, "punti": 1}]},
     "rete": {"op": ">=", "provvisorio": true, "gradini": [{"soglia": 0.5, "punti": 1}]},
     "cinetica": {"op": "<=", "massimo": 0, "massimo_se_pesata": 1, "gradini": [{"soglia": 0.693, "punti": 1}]},
     "controlli": {"op": ">=", "gradini": []},
     "RT": {"op": ">=", "gradini": []},
     "frequenza": {"op": ">=", "provvisorio": true, "gradini": [{"soglia": 0.75, "punti": 1}]},
     "esterne": {"op": ">=", "gradini": [{"soglia": 0.64, "punti": 2}]}
    }
   }
  },
  "attributio": {
   "visibile": false,
   "fonte": "attributio 1.12.0 (commit 2bde634), annotate.py: AnnotationSettings, LEVEL_BASIS_LABEL, level_reasons; only thresholds, rules and wording",
   "soglie": {
    "coseno_min": 0.70,
    "ioni_comuni_min": 4,
    "precursore_ppm_max": 15.0,
    "margine_conflitto_coseno": 0.05,
    "cosmic_min": 0.64,
    "canopus_prob_min": 0.70,
    "struttura_libreria_ppm_max": 20.0,
    "rt_gruppo_tol_min": 0.20
   },
   "cancelli": [
    {"id": "score", "campo": "cos", "op": ">=", "soglia": "coseno_min", "ruolo": "source"},
    {"id": "shared", "campo": "shared", "op": ">=", "soglia": "ioni_comuni_min", "ruolo": "source"},
    {"id": "prec", "campo": "ppm", "op": "<=", "soglia": "precursore_ppm_max", "assoluto": true, "unita": "ppm", "ruolo": "source"},
    {"id": "distinct", "campo": "marginCos", "op": ">=", "soglia": "margine_conflitto_coseno", "ruolo": "confirm"}
   ],
   "standard": [],
   "provvisorio": null,
   "senza_prove": "0",
   "cascata": [
    {"livello": "1", "base": "reference standard", "regola": "user standard: mass + RT + MS/MS (cosine with the spectrum of the standard of the SAME ion)"},
    {"livello": "2a", "base": "spectral library", "regola": "at least one valid library source: cosine >= coseno_min AND shared ions >= ioni_comuni_min AND precursor within precursore_ppm_max (beyond = analogue, never 2a)"},
    {"livello": "2b", "base": "diagnostic evidence", "regola": "ONLY by decision of the chemist (from 3 or from 2a), never automatic; refused on 4 or 0 with a warning; the RT rule wins over 2b"},
    {"livello": "3", "base": "name conflict", "regola": "valid sources that name different structures (first InChIKey block) and the best ahead by less than margine_conflitto_coseno"},
    {"livello": "3", "base": "isomers not discriminated", "regola": "the same library compound at two or more RT (beyond rt_gruppo_tol_min)"},
    {"livello": "3", "base": "RT isomer", "regola": "the user standard elutes at another RT: another isomer"},
    {"livello": "3", "base": "expert review", "regola": "the chemist judges a 2a match insufficient"},
    {"livello": "3", "base": "CSI:FingerID", "regola": "CSI:FingerID structure with COSMIC (ConfidenceScoreApproximate) >= cosmic_min (~10% FDR, Hoffmann et al. 2022); never downgrades a library match"},
    {"livello": "4", "base": "CANOPUS class", "regola": "CANOPUS class only (NPClassifier pathway or ClassyFire superclass) with probability >= canopus_prob_min"},
    {"livello": "0", "base": "suspect-RT", "regola": "standard found by mass + RT but MS/MS not confirmed: outside the counts"},
    {"livello": "0", "base": "none", "regola": "detected, no annotation"}
   ],
   "fonti_escluse": [
    "propagated GNPS2 annotations (name starting with 'Candidate', or containing '(known structural isomers' or '(delta mass:'): never a source",
    "mzMine lipids with method = 'matchedlipid': not spectral matches",
    "structure (SMILES/InChI/InChIKey/formula) of the mzMine library: used only if the formula explains the precursor of the entry within struttura_libreria_ppm_max with the declared adduct or a common one (known bug of mzMine exports: reused keys)"
   ],
   "pulizia_nomi": {
    "prefissi": ["Massbank:", "MassbankEU:", "MoNA:", "ReSpect:", "Spectral Match to "],
    "taglio": "only the part before '|'",
    "suffissi": [" from NIST14", " - 30.0 eV", " - 40.0 eV", " - 20.0 eV"],
    "accessione_iniziale_regex": "^(?:[A-Z]{2,3}\\d{5,}|\\d{5,})\\s+",
    "energia_finale_regex": "_CE\\d+(?:\\.\\d+)?$"
   },
   "testi_basi_en": {
    "reference standard": "Confirmed by reference standard: mass + RT + MS/MS (Lvl 1)",
    "spectral library": "Spectral library match (Lvl 2a)",
    "diagnostic evidence": "Structure supported by diagnostic evidence, judged by the chemist on the MS/MS spectrum (Lvl 2b)",
    "name conflict": "Spectral library matches of different structures that the spectrum does not tell apart (best match ahead by less than the margin): named by the best match (Lvl 3)",
    "isomers not discriminated": "Spectral library match of the same compound at two or more retention times: the spectrum does not tell the isomers apart (Lvl 3)",
    "RT isomer": "Spectral library match at a retention time other than the reference standard's: another isomer (Lvl 3)",
    "expert review": "Spectral library match judged insufficient by the chemist (Lvl 3)",
    "CSI:FingerID": "CSI:FingerID structure (COSMIC conf >= {cosmic}, ~10% FDR, Lvl 3)",
    "CANOPUS class": "CANOPUS class only, probability >= {canopus} (Lvl 4)",
    "suspect-RT": "Reference standard, mass + RT only, MS/MS not confirmed (suspect-RT, Lvl 0)",
    "none": "Unannotated (DETECTED)"
   }
  }
 }
};
