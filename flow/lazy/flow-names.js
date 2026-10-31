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
    "spectra.centroid": ["Centroid spectra", "input", "Turns profile spectra into peaks, file by file; files that are already centroided pass through unchanged."],
    "index.mzidx": ["Import data", "input", "Reads your files once and keeps a fast index."],
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
    "screen.suspects": ["Find suspects", "search", "Looks for the compounds of your suspect lists in the feature table, by mass and formula."],
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
    "spectra.msn_graph": ["Fragmentation graph", "process", "Builds the tree of fragments and neutral losses of one infusion."],
    "msn.library_export": ["Save as library", "output", "Writes the spectra of the tree as a library file."],
    "msn.table_export": ["Save as tables", "output", "Writes the ions and the losses of the graph as CSV tables."],
    "chrom.uv_peaks": ["Find UV peaks", "process", "Integrates the peaks of the UV trace of each file."],
    "features.table": ["Feature table", "process", "Detects and aligns the features of the files and writes the table."],
    "quant.calibrate": ["Apply calibration curves", "process", "Turns the signal of each compound into a concentration with the curves you imported."],
    "qc.mass_accuracy": ["Check mass accuracy", "process", "Measures the mass error of each file on known ions and, if you choose a model, corrects the m/z of the feature table."],
    "spectra.ms2": ["Read MS2 spectra", "input", "Reads the MS2 scans of data-dependent runs as spectra."],
    "qc.run_order": ["Check run order", "process", "Counts the QC, blank and standard files of each batch along the injection order and warns about gaps."],
    "export.annotations": ["Export annotations", "output", "Writes one CSV with the candidates of every identification method for each feature, with the level and the evidence."],
    "report.methods": ["Methods text", "output", "Writes the methods of a run, with every parameter, from its run record."],
    "table.drift": ["Correct drift", "process", "Brings each batch to the level of the QC files along the injection order."],
    "table.kmd": ["Kendrick mass defect", "search", "Lists the series of ions that share the same mass defect."],
    "table.pca": ["Explore with PCA", "process", "Shows how the samples group along the main components."],
    "table.stats": ["Compare conditions", "process", "Tests each compound between the conditions of the design."],
    "tp.dig_lr": ["Dig unit-resolution series", "search", "Scores the possible products of a parent in a series at unit resolution."],
    "table.plausible": ["Check the values", "process", "Runs seven sanity checks on the numbers of the table and reports what looks wrong."],
    "export.inclusion": ["Save inclusion list", "output", "Writes the m/z and time windows of the expected products as a CSV for the next acquisition."],
    "export.mztab": ["Save table as mzTab-M", "output", "Writes the feature table as an mzTab-M file, an open format for sharing results."],
    "tp.review": ["Decide the levels", "search", "Keeps your decision for each row: a level, a note, or not a product."],
    "msn.figure": ["Save graph as figure", "output", "Draws the fragmentation graph as an SVG, an HTML page or a PNG."],
    "meta.compare": ["Compare algorithms", "process", "Runs a step once per variant on the same data and measures how the outputs differ."],
    "spectra.lookup": ["Find spectra in a library", "search", "Pulls from a library the entries with a given name, accession or precursor m/z."],
    "table.match": ["Match table rows to library", "search", "Compares the fragment spectrum of every table row with a library of known compounds."],
  };
  F.INPUT_NAMES = { raw: "Raw files", mzml: "mzML files", mzidx: "Scan index", spectra: "MS2 spectra", library: "Spectral library", structure: "Parent structure", samples: "Sample list", table: "Feature table" };

  /** the base: always present, configurable, never removable. The first entry is also the node that holds the raw files */
  F.BASE = [
    { key: "import", title: "Import data", why: F.NAMES["index.mzidx"][2], steps: ["spectra.centroid", "index.mzidx"] },
    { key: "table", title: "Build feature table", why: "Finds peaks in every file, aligns them and fills gaps: one row per compound.", steps: ["features.detect", "features.align", "features.gapfill"] },
  ];
  /** groups of steps that Simple draws as one card beside the base (Advanced shows the steps apart); a group is drawn only when the plan has one of its steps */
  F.GROUPS = [
    { key: "export", title: "Export", why: "Saves the graph as a library and as tables.", steps: ["msn.library_export", "msn.table_export"], cat: "output" },
  ];
  /** steps that read the infusion open in Data with no file in the plan: the canvas joins them to the raw files */
  F.INFUSION_STEPS = ["spectra.msn_graph"];
  /** steps with a view: a card beside the step, off until the run made a result of this type (a view is not a step of the plan) */
  F.STEP_VIEWS = { "spectra.msn_graph": { title: "View graph", why: "Opens the fragmentation graph.", result: "report" } };
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
  // `presets`: {stepId: preset} written in the plan; `tail`: steps added after the add-ons; `note`: the card carries F.UNIT_NOTE
  // `more`: kept under «More templates» instead of on the first row
  F.TEMPLATES = [
    { id: "tp", name: "Find transformation products", good: "Good for: degradation or metabolism studies.", addons: ["tp"] },
    { id: "table", name: "Feature table", good: "Good for: a first look at your data.", addons: [] },
    { id: "lrtable", name: "Unit-resolution feature table", good: "Good for: full-scan files from a triple quadrupole or an ion trap (unit resolution): a table of nominal-mass candidates, no accurate-mass formulas.", note: true, addons: ["reliable"], presets: { "features.detect": "lr" }, params: { "table.filter": { blank_ratio: 3, dup_da: 0.3 } }, tail: ["features.estimate", "export.table_csv"] },
    { id: "identify", name: "Untargeted analysis", good: "Good for: unknown compounds in real samples, with formulas and library matches.", addons: ["formulas", "match", "level"] },
    { id: "msn", name: "Fragmentation tree of a standard", good: "Good for: an MSn infusion of a known compound: its fragments, neutral losses and sub-formulas, as a library and a table.", help: "Fragmentation rules: CID vs HCD (see the MSn manual).", steps: ["spectra.msn_graph", "msn.library_export", "msn.table_export"], params: { "spectra.msn_graph": { interpreta: true } }, addons: [] },
    { id: "bentazone", name: "Bentazone / transformation products", good: "Good for: a parent degraded over time (bentazone or any other), positive or negative, unit or high resolution: the files decide the polarity and the resolution.", addons: ["tp", "tpfilter", "position", "network"], more: true },
    { id: "targeted", name: "Targeted check", good: "Good for: a list of compounds you already suspect.", addons: ["formulas", "match", "level", "regulatory"], more: true },
    { id: "tpseries", name: "TP time series", good: "Good for: how products rise and fall over the minutes of a treatment (needs a sample list with times).", addons: ["tp", "tpfilter", "reliable", "network"], more: true },
    { id: "standards", name: "Standards check", good: "Good for: checking that the standards and the blanks behave before you trust a table (needs a sample list).", addons: ["reliable"], params: { "table.filter": { qc_cv_max: 30 } }, more: true },
    { id: "blankfree", name: "Blank-corrected table", good: "Good for: a table without what the blanks also show (needs a sample list with blanks).", addons: ["reliable"], params: { "table.filter": { blank_ratio: 3 } }, more: true },
  ];
  /** the honest label of the unit-resolution template: shown in its card and in the issues panel */
  F.UNIT_NOTE = "Unit resolution: m/z values are nominal (± the measured window); a row is a candidate, at most level 4 from MS1 alone (level 3 with an MS2 match, level 1 only with a standard).";
  /** @param {string} id */
  F.nameOf = id => (F.NAMES[id] || [id, "process", ""]);
})();
