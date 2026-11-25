//! The algorithms of `features.align` and `features.gapfill`: the groups and the integration of mzLab (the defaults) and the join aligner and peak
//! finder of mzmine 3 (`align_mzmine`, run by `untargeted::flow_align`).

use mzlab_core::step::{Algorithm, Applicability, Param, Reference, Status};

/// `mz_offset` of `features.align` (`lrscreen_offset.rs`).
pub(super) fn mz_offset_param() -> Param {
    Param::text("mz_offset", "m/z offset")
        .default_str("0")
        .doc("The instrument's m/z offset, observed minus true, in Da (|DA| up to 0.6): a number, or auto:MZ[@RT] to measure it per file on the known ion at m/z MZ (near RT min). It is subtracted from each file's m/z before the grouping, so rows hold corrected m/z; the gap filling reads the raw scans at m/z plus the offset; recenter then works on the corrected m/z")
        .tip("Subtracts the instrument's m/z offset (Da, observed minus true) from each file; auto:MZ[@RT] measures it on a known ion.")
        .advanced()
}

/// `dedup` and `dedup_overlap` of `features.gapfill` (`lrscreen_dedup.rs`): the two rules that merge the rows of one ion and the threshold of the second.
pub(super) fn dedup_params() -> [Param; 2] {
    [
        Param::choice("dedup", "Merge duplicate rows", &["auto", "off"])
            .default_str("auto")
            .doc("Unit resolution only: of two rows within 0.6 Da and the RT tolerance whose areas correlate (r of at least 0.9 over 3 files) keeps the one with the higher maximum height; areas are never summed")
            .tip("Unit resolution only: merges close rows whose areas correlate or peaks overlap, keeping the taller; areas are never summed.")
            .advanced(),
        Param::number("dedup_overlap", "Overlap to merge")
            .min(0.0)
            .max(1.0)
            .default_num(0.5)
            .doc("Unit resolution only: rows within 0.6 Da and the RT tolerance whose peaks, in the files where both were detected, cover at least this share of the narrower one (and half of it of the wider one), in 2 files or the only file, are one ion (a shoulder or broad top cut in two); 0 = off")
            .tip("Unit resolution only: share of the narrower peak two close rows must cover to be merged; 0 keeps every row.")
            .advanced(),
    ]
}

fn mzmine3() -> Reference {
    Reference {
        doi: "10.1038/s41587-023-01690-2",
        citation_key: "schmid2023integrative",
        note: "MZmine 3: the join aligner and the peak finder",
    }
}

fn any_data() -> Applicability {
    Applicability {
        resolution: vec!["hr", "lr"],
        spectrum_mode: vec!["centroid", "profile"],
        ..Applicability::default()
    }
}

fn abs_da(default: f64) -> Param {
    Param::number("abs_da", "Absolute m/z tolerance")
        .min(0.0)
        .default_num(default)
        .unit("Da")
        .doc("Lower bound of the m/z tolerance; the larger of it and ppm wins")
        .advanced()
}

/// `groups` (default) and `join` of `features.align`.
pub(super) fn align() -> Vec<Algorithm> {
    let groups = Algorithm {
        id: "groups",
        version: 1,
        title: "Groups by m/z and RT gaps",
        doc: "Groups the features of the files by m/z and by the RT gaps between neighbours, per family or per ion (`famalign`, `align`).",
        references: Vec::new(),
        applicability: any_data(),
        params: Vec::new(),
        status: Status::Available,
    };
    let join = Algorithm {
        id: "join",
        version: 1,
        title: "Join aligner",
        doc: "The join aligner of mzmine 3: the file with the most features left gives the rows, each feature of the others goes to the best-scored row inside the tolerances. One row per ion whatever mode says; ppm is the m/z tolerance; rt_gap, max_ppm_span, drop_artefacts, split_gap, rt_warp and the family parameters do not apply.",
        references: vec![mzmine3()],
        applicability: Applicability {
            resolution: vec!["hr"],
            note: "High resolution feature tables, as mzmine expects",
            ..any_data()
        },
        params: vec![
            Param::number("mz_weight", "m/z weight").min(0.0).default_num(3.0),
            Param::number("rt_weight", "RT weight").min(0.0).default_num(1.0),
            Param::number("rt_tol", "RT tolerance")
                .min(0.0)
                .default_num(0.2)
                .unit("min")
                .doc("Half width of the RT range of a row"),
            abs_da(0.0015),
        ],
        status: Status::Available,
    };
    vec![groups, join]
}

/// `integrate` (default) and `peak_finder` of `features.gapfill`.
pub(super) fn gapfill() -> Vec<Algorithm> {
    let integrate = Algorithm {
        id: "integrate",
        version: 1,
        title: "Trace integration",
        doc: "Integrates the raw trace of the file in the window of the row at the noise floor (`gapfill`); the cell keeps an area only.",
        references: Vec::new(),
        applicability: any_data(),
        params: Vec::new(),
        status: Status::Available,
    };
    let finder = Algorithm {
        id: "peak_finder",
        version: 1,
        title: "Peak finder",
        doc: "The peak finder of mzmine 3: in the m/z and RT range of the row, the most intense point of each scan, then a peak around an interior apex while the shape stays within the intensity tolerance. floor is the noise of the mass list; max_gap and long_trace do not apply.",
        references: vec![mzmine3()],
        applicability: Applicability {
            resolution: vec!["hr"],
            spectrum_mode: vec!["centroid"],
            note: "High resolution centroided data, as mzmine expects",
            ..Applicability::default()
        },
        params: vec![
            Param::number("intensity_tol", "Intensity tolerance")
                .min(0.0)
                .default_num(0.2)
                .doc("A point joins the peak while it is not above its neighbour toward the apex by more than this fraction"),
            abs_da(0.0015),
        ],
        status: Status::Available,
    };
    vec![integrate, finder]
}
