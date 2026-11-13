//! What the readers of this crate leave out of the mzML (`Lettore::not_carried`), with the reason; written in `conv/1` (docs/agenti/estendere.md).

use mzlab_core::conv::{NOT_DECODED, OUT_OF_MODEL, WITHOUT_TERM};

/// Gaps of the numpress reader.
pub const NUMPRESS: &[(&str, &str)] = &[
    (
        "dataProcessing, referenceableParamGroup and scanSettings of the source",
        OUT_OF_MODEL,
    ),
    (
        "cvParams the writer does not know (carried as userParam)",
        WITHOUT_TERM,
    ),
];

/// Gaps of the bruker reader.
pub const BRUKER: &[(&str, &str)] = &[
    ("inverse mobility 1/K0 of each point", NOT_DECODED),
    (
        "image positions, regions and tables beyond Frames and GlobalMetadata (TSF)",
        OUT_OF_MODEL,
    ),
    (
        "chromatography-data.sqlite, .mcf and .baf next to the data",
        OUT_OF_MODEL,
    ),
];
