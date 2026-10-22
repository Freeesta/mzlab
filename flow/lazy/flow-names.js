// @ts-check
// Flow (administrator mode): the words of the canvas. Plain names, a line of «why» and a category for every step; the base chain that is always there, the add-on
// analyses (each a short list of steps), the starting templates (lists of add-ons). A step without an entry keeps the title of its descriptor.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  /** category → colour variable value (Okabe-Ito), icon of F.icon, and the word shown beside the colour */
  F.CATS = {
    input: { color: "#0072B2", icon: "import", word: "Data in" },
    process: { color: "#009E73", icon: "flow", word: "Process" },
    search: { color: "#E69F00", icon: "search", word: "Match & identify" },
    output: { color: "#56B4E9", icon: "export", word: "Data out" },
  };
  /** @type {Record<string, [string, string, string]>} [plain name, category, why] */
  F.NAMES = {
    "import.raw": ["Import raw files", "input", "Reads the instrument files you chose."],
    "convert.mzml": ["Convert to mzML", "input", "Writes the spectra in an open format other tools can read."],
    "index.mzidx": ["Import data", "input", "Reads your files once and keeps a fast index, so later steps open only the scans they need."],
    "features.estimate": ["Estimate settings", "process", "Measures the noise and peak width of your files and proposes settings."],
    "features.detect": ["Find features", "process", "Finds the peaks that rise above the noise in each file."],
    "features.align": ["Align samples", "process", "Lines up the same compound across files, correcting small shifts in time."],
    "features.gapfill": ["Fill gaps", "process", "Looks for the signal again where a file missed a compound that others have."],
    "table.filter": ["Keep reliable rows", "process", "Drops rows seen too rarely, too small, or no bigger than the blank."],
    "features.ms2_link": ["Link fragment spectra", "process", "Attaches the fragmentation spectra to the peak they came from."],
    "export.table_csv": ["Save table as CSV", "output", "Writes the feature table to a file you can open in Excel."],
    "spectra.chimeric": ["Check mixed spectra", "process", "Flags fragmentation spectra that contain more than one compound."],
    "formula.ms1": ["Molecular formulas", "search", "Suggests chemical formulas that fit each mass and its isotope pattern."],
    "formula.ms2": ["Formulas from fragments", "search", "Checks the suggested formulas against the fragments that were measured."],
    "spectra.match": ["Match to library", "search", "Compares your fragment spectra with a library of known compounds."],
    "spectra.insilico": ["Predicted fragments", "search", "Predicts how a candidate should break and compares it with the measured spectrum."],
    "rt.predict": ["Check retention time", "search", "Asks whether the time a compound came out is plausible for its structure."],
    "annot.level": ["Confidence level", "search", "Says how sure each identification is, from a guess to a confirmed standard."],
    "annot.regulatory": ["Regulatory lists", "search", "Checks whether a compound is on a list of regulated or suspected substances."],
    "annot.hazard": ["Aquatic hazard", "search", "Estimates the hazard of a compound to water life."],
    "tp.generate": ["Predict transformation products", "search", "Lists the products a parent compound could turn into."],
    "tp.filter": ["Filter products", "search", "Keeps the products that show up in your data."],
    "tp.link": ["Link product spectra", "search", "Attaches the fragment spectra to each product."],
    "tp.network": ["Product network", "search", "Links the products that come from each other, with the evidence for each link."],
    "tp.position": ["Where it changed", "search", "Proposes which part of the molecule was modified."],
    "structure.render": ["Draw the change", "output", "Draws the parent with the modified position marked."],
    "structure.draw": ["Draw a structure", "input", "Lets you draw the parent compound."],
    "structure.reactivity": ["Where it reacts", "search", "Estimates which parts of the parent are most reactive."],
  };
  F.INPUT_NAMES = { raw: "Raw files", mzml: "mzML files", mzidx: "Scan index", spectra: "MS2 spectra", library: "Spectral library", structure: "Parent structure", samples: "Sample list", table: "Feature table" };

  /** the base: always present, configurable, never removable. The first entry is also the node that holds the raw files */
  F.BASE = [
    { key: "import", title: "Import data", why: F.NAMES["index.mzidx"][2], steps: ["index.mzidx"] },
    { key: "table", title: "Build feature table", why: "Finds the peaks in every file, lines them up across files and fills the gaps: one table, one row per compound.", steps: ["features.detect", "features.align", "features.gapfill"] },
  ];
  /** the analyses a user adds to the base */
  F.ADDONS = [
    { id: "formulas", name: "Molecular formulas", line: "Suggest formulas for each feature.", steps: ["formula.ms1"] },
    { id: "fragformulas", name: "Formulas from fragments", line: "Check formulas against measured fragments.", steps: ["formula.ms1", "formula.ms2"] },
    { id: "chimeric", name: "Check mixed spectra", line: "Flag fragment spectra that contain more than one compound.", steps: ["spectra.chimeric"] },
    { id: "match", name: "Match to a spectral library", line: "Compare fragment spectra with known compounds.", steps: ["spectra.match"] },
    { id: "insilico", name: "Predicted fragments", line: "Compare a candidate's predicted breakage with the measured spectrum.", steps: ["spectra.insilico"] },
    { id: "rt", name: "Check retention time", line: "Ask whether the time a compound came out is plausible.", steps: ["rt.predict"] },
    { id: "level", name: "Confidence level", line: "Say how sure each identification is.", steps: ["annot.level"] },
    { id: "regulatory", name: "Regulatory lists", line: "Flag regulated and suspected substances.", steps: ["annot.regulatory"] },
    { id: "hazard", name: "Aquatic hazard", line: "Estimate the hazard of each compound to water life.", steps: ["annot.hazard"] },
    { id: "tp", name: "Transformation products", line: "Predict and look for the products of a parent.", steps: ["tp.generate", "tp.link"] },
    { id: "reliable", name: "Keep reliable rows", line: "Drop the rows seen too rarely, too small or no bigger than the blank.", steps: ["table.filter"] },
    { id: "network", name: "Product network", line: "Link the products that come from each other.", steps: ["tp.network"] },
    { id: "tpfilter", name: "Filter products", line: "Keep the products that show up in your data.", steps: ["tp.filter"] },
    { id: "position", name: "Where it changed", line: "Propose which part of the molecule was modified.", steps: ["tp.position"] },
    { id: "reactivity", name: "Where it reacts", line: "Estimate which parts of the parent are most reactive.", steps: ["structure.reactivity"] },
  ];
  // `steps`: a plan of its own (no feature-table base), the steps in order, each wired to what fits
  // `more`: kept under «More templates» instead of on the first row
  F.TEMPLATES = [
    { id: "tp", name: "Find transformation products", good: "Good for: degradation or metabolism studies.", addons: ["tp"] },
    { id: "table", name: "Feature table", good: "Good for: a first look at your data.", addons: [] },
    { id: "identify", name: "Untargeted analysis", good: "Good for: unknown compounds in real samples, with formulas and library matches.", addons: ["formulas", "match", "level"] },
    { id: "msn", name: "Fragmentation tree of a standard", good: "Good for: an MSn infusion of a known compound: its fragments, neutral losses and sub-formulas, as a library and a table.", help: "Fragmentation rules: CID vs HCD (see the MSn manual).", steps: ["spectra.msn_graph", "msn.library_export", "msn.table_export"], params: { "spectra.msn_graph": { interpreta: true } }, addons: [] },
    { id: "targeted", name: "Targeted check", good: "Good for: a list of compounds you already suspect.", addons: ["formulas", "match", "level", "regulatory"], more: true },
    { id: "tpseries", name: "TP time series", good: "Good for: how products rise and fall over the minutes of a treatment (needs a sample list with times).", addons: ["tp", "tpfilter", "reliable", "network"], more: true },
    { id: "standards", name: "Standards check", good: "Good for: checking that the standards and the blanks behave before you trust a table (needs a sample list).", addons: ["reliable"], params: { "table.filter": { qc_cv_max: 30 } }, more: true },
    { id: "blankfree", name: "Blank-corrected table", good: "Good for: a table without what the blanks also show (needs a sample list with blanks).", addons: ["reliable"], params: { "table.filter": { blank_ratio: 3 } }, more: true },
  ];
  /** @param {string} id */
  F.nameOf = id => (F.NAMES[id] || [id, "process", ""]);
})();
