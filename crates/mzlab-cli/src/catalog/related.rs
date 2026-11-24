//! Task words, `see_also` and bulk equivalents of commands and Flow steps: what `commands --find` matches beyond the summaries, and what
//! `commands --json PATH` adds to a command. Written from an audit of agent tasks in a low-resolution LC-MS campaign (the fixtures of
//! `find_tests.rs`): every row names a command path or a step id that exists (`find_tests.rs` checks it).

pub(crate) struct Rel {
    /// A command path (`peak table`) or a Flow step id (`chrom.transitions`).
    pub name: &'static str,
    /// Words of the tasks it does that its summary does not say.
    pub words: &'static str,
    /// Related commands or steps.
    pub also: &'static [&'static str],
    /// The one call to use instead of many calls of this one; empty when there is none.
    pub bulk: &'static str,
    /// The command line `commands --find` shows; empty: the first example of the command, or the template line of the step.
    pub example: &'static str,
}

const fn r(name: &'static str, words: &'static str, also: &'static [&'static str]) -> Rel {
    Rel {
        name,
        words,
        also,
        bulk: "",
        example: "",
    }
}

impl Rel {
    const fn bulk(mut self, b: &'static str) -> Self {
        self.bulk = b;
        self
    }
    const fn ex(mut self, e: &'static str) -> Self {
        self.example = e;
        self
    }
    /// Whether `bulk` is another command (`mzlab-cli OTHER ...`) rather than this one's own batch use (`trace xic --mz M1,M2`).
    pub(crate) fn elsewhere(&self) -> bool {
        self.bulk
            .strip_prefix("mzlab-cli ")
            .is_some_and(|b| !b.starts_with(&format!("{} ", self.name)))
    }
}

const PEAK_BULK: &str = "mzlab-cli peak table FILE... --auto: every peak of every channel of every file in one call; a target list: step chrom.transitions";

pub(crate) static RELATED: &[Rel] = &[
    r("file list", "inventory folder directory what is there", &["file inventory", "file info", "sample list"]),
    r("file inventory", "inventory samples checksum", &["file list", "sample list"]),
    r("file info", "inventory what kind experiment sim mrm full scan", &["file channels", "file list"]),
    r("raw info", "profile centroid label ion counting quantum grid nominal stepped mass offset what kind of data", &["file info", "raw cache"]),
    r("raw cache", "index cache build check prebuild entries", &["file index", "raw info"]),
    r("file channels", "sim srm mrm channel list transitions", &["trace chrom", "peak table"]),
    r("file convert", "convert mzml vendor wiff raw sample names", &["sample list", "sample select", "file validate", "convert.mzml"])
        .ex("mzlab-cli file convert run.wiff --out mzml/ --json"),
    r("sample list", "sample names wiff", &["sample select", "file convert"]),
    r("sample table", "roles blank qc standard control time point names", &["sample import", "sample design propose", "table.filter"]),
    r("sample import", "metadata csv tsv sample names roles group blank dark wash qc standard from a spreadsheet unmatched duplicate", &["sample table", "sample design propose"]),
    r("features", "non-targeted untargeted feature table csv peak picking all files", &["features.table", "features.detect", "flow run"]),
    r("estimate", "estimate parameters from the data noise floor minimum height fwhm rt tolerance mass defect blank level set lines before a run", &["flow defaults", "features", "flow run"]),
    r("quant union", "one target list for many series conditions replicates polarities join merge feature tables campaign across series", &["quant targets", "features"]),
    r("table compare", "blank reference ratio subtraction candidate products", &["table.filter", "tp.dig_lr"]),
    r("trace xic", "extracted ion chromatogram eic xic many ions list mz", &["peak table", "trace chrom", "chrom.transitions"])
        .bulk("one call per file: --mz M1,M2,... (or --sweep) gives every ion; never one call per ion")
        .ex("mzlab-cli trace xic s.mzML --mz 364.4,194.2 --da 0.3 --out xic.json"),
    r("trace chrom", "chromatogram channel tic bpc sim srm trace csv", &["file channels", "peak table", "trace xic"]).bulk(PEAK_BULK),
    r("peak auto-lr", "integrate peaks one trace sim mrm area snr", &["peak table", "chrom.transitions", "trace chrom"]).bulk(PEAK_BULK),
    r("peak auto", "integrate peaks one trace high resolution", &["features", "features.detect", "peak auto-lr"])
        .bulk("mzlab-cli features FILE... --out table.csv: every high-resolution peak of many files in one call (peak table --auto is the low-resolution engine)"),
    r("peak area", "integrate one peak area", &["peak table"])
        .bulk("mzlab-cli peak table FILE... --windows PATH: the same fixed windows in every trace of every file, one call"),
    r("peak table", "integrate integration quantify area snr every peak all traces channels many files batch sim srm mrm xic target windows", &["chrom.transitions", "peak auto-lr", "trace xic", "quant.curve"])
        .ex("mzlab-cli peak table a.mzML b.mzML --auto --out peaks.json"),
    r("spectrum get", "mass spectrum scan blank subtract", &["spectrum list", "spectra.centroid"]),
    r("ion pattern", "isotope pattern chlorine bromine", &["features.group", "ion mz"]),
    r("ion match", "adduct neutral loss contaminant", &["features.group", "features.companions", "ion mz"]),
    r("mol draw", "draw depict structure picture image figure svg png cdxml ket molfile smiles 2d", &["structure.render", "tp.network", "mol export"])
        .ex("mzlab-cli mol draw 'smiles:CCO' --format svg --out ethanol.svg"),
    r("flow record", "provenance record reproducible", &["flow verify", "flow report"]),
    r("flow verify", "provenance check fingerprints reproducible", &["flow record"]),
    r("flow report", "provenance report parameters times", &["flow record"]),
    r("flow steps", "plan template step parameters", &["flow defaults", "flow check", "flow run"]),
    r("flow defaults", "default parameter value limit unit set key off when empty what can i change", &["flow steps", "flow check"]),
    r("commands", "find search help catalog task", &["schema", "flow steps"]),
    r("chrom.transitions", "sim srm mrm target list targeted integrate quantify snr channels triple quadrupole many files batch", &["peak table", "quant.curve", "trace xic"]),
    r("table.filter", "blank subtraction threshold background remove filter qc cv replicate", &["table compare", "features.align"]),
    r("features.group", "correlated correlation ions isotopes adducts in-source fragments group compound neutral mass", &["ion pattern", "ion match", "features.align"]),
    r("features.companions", "companions companion co-eluting coeluting adducts in-source fragments isotopes m+1 m+2 unit mass nominal low resolution edges", &["features.group", "ion match", "features.align"]),
    r("features.align", "align table samples families ions", &["features.group", "features.gapfill"]),
    r("spectra.centroid", "centroid profile peak picking spectra", &["convert.mzml", "spectrum get"]),
    r("tp.dig_lr", "kinetics kinetic series time course dig degradation transformation products parent unit low resolution one ion noise floor", &["tp.generate", "table compare", "peak noise"])
        .bulk("one run digs the whole series: every file in index, one table; other settings: rerun in the same --out with --set (cached)"),
    r("tp.generate", "hypotheses transformation products candidates suspects parent", &["tp.dig_lr", "tp.filter", "tp.network"]),
    r("tp.network", "pathway network scheme products draw", &["mol draw", "structure.render"]),
    r("structure.render", "draw modification structure product", &["mol draw", "tp.network"]),
    r("export.table_csv", "export table csv", &["export.mztab", "features"]),
    r("quant.curve", "calibration curve quantification lod loq", &["quant.calibrate", "peak table", "chrom.transitions"]),
];

/// The row of `name`, if it has one.
pub(crate) fn of(name: &str) -> Option<&'static Rel> {
    RELATED.iter().find(|r| r.name == name)
}
