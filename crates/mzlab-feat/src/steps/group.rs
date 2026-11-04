//! `features.group`: the descriptor of the grouping of a per-ion table into compounds (`ionnet`). Defaults after CAMERA (apex within 0.6 of the peak
//! width, correlation 0.75, 5 ppm) and the ion species of the ion identity networking of mzmine/GNPS; the sources are the references of the step.

use mzlab_core::step::{DataType, Facets, Param, Port, Reference, RunsIn, Step};

use super::FEAT_CEILING;
use crate::ionnet::ions::{DEFAULT_NEG, DEFAULT_POS};

pub(super) fn group() -> Step {
    let params = vec![
        Param::choice("polarity", "Polarity", &["auto", "positive", "negative"])
            .default_str("auto")
            .doc("Which list of ion species is searched; auto = the polarity of the files of the table"),
        Param::text("ions_pos", "Positive ion species")
            .default_str(DEFAULT_POS)
            .doc("Comma-separated species searched in positive mode, among [M+H]+ [M+Na]+ [M+NH4]+ [M+K]+ [M+H-H2O]+ [M+H-2H2O]+ [M+2H]2+ [M+H+Na]2+ [2M+H]+ [2M+Na]+ [2M+NH4]+"),
        Param::text("ions_neg", "Negative ion species")
            .default_str(DEFAULT_NEG)
            .doc("Comma-separated species searched in negative mode, among [M-H]- [M+Cl]- [M+HCOO]- [M+CH3COO]- [M-H-H2O]- [M-2H]2- [2M-H]- [2M+HCOO]-"),
        Param::number("rt_tol", "RT tolerance")
            .min(0.0)
            .default_num(0.1)
            .unit("min")
            .doc("Largest distance between the RT of two rows of one compound"),
        Param::number("apex_fwhm", "Apex distance")
            .min(0.0)
            .max(2.0)
            .default_num(0.6)
            .unit("fraction")
            .doc("In each file that detects both rows, the apexes are at most this share of the wider peak apart (CAMERA perfwhm)")
            .advanced(),
        Param::number("min_corr", "Height correlation")
            .min(-1.0)
            .max(1.0)
            .default_num(0.75)
            .doc("Smallest Pearson correlation of the heights of two rows over the files (CAMERA cor_eic_th)"),
        Param::integer("min_corr_samples", "Files for the correlation")
            .min(2.0)
            .default_num(3.0)
            .unit("files")
            .doc("The heights are compared only when both rows are measured in at least this many files")
            .advanced(),
        Param::number("ppm", "m/z tolerance")
            .min(0.0)
            .default_num(5.0)
            .unit("ppm"),
        Param::number("mz_abs", "Smallest m/z tolerance")
            .min(0.0)
            .default_num(0.001)
            .unit("Da")
            .doc("Floor of the m/z tolerance at low mass")
            .advanced(),
        Param::integer("max_charge", "Largest charge")
            .min(1.0)
            .max(3.0)
            .default_num(2.0)
            .doc("Largest charge an isotope step is read with")
            .advanced(),
    ];
    let hr = Facets {
        resolution: vec!["hr"],
        ..Facets::default()
    };
    Step {
        ins: vec![Port::new("table", DataType::Table, "Per-ion feature table").requires(hr)],
        outs: vec![Port::new("groups", DataType::Report, "Groups, ion species and edges (ionnet/1)")],
        params,
        runs_in: RunsIn {
            wasm: Some("flowtab"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["err.flow.group.ion"],
        references: vec![
            Reference {
                doi: "10.1038/s41467-021-23953-9",
                citation_key: "schmid2021ion",
                note: "Ion identity molecular networking: ion species of one molecule linked by peak correlation; the edge columns of GNPS",
            },
            Reference {
                doi: "10.1021/ac202450g",
                citation_key: "kuhl2012camera",
                note: "CAMERA: grouping by apex within a share of the FWHM and by correlation, isotopes and adducts by mass difference",
            },
        ],
        ..Step::core(
            "features.group",
            "Group ions",
            "Groups the rows of a per-ion table that are one compound: same RT, heights that correlate over the files, then isotope steps and ion species that give one neutral mass. Every row stays, with a group id; the edges can go to a molecular network.",
        )
    }
}
