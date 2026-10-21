//! What the readers of this crate leave out of the mzML (`Lettore::non_portati`), with the reason; written in `conv/1` (docs/agenti/estendere.md).

use mzlab_core::conv::{FUORI_MODELLO, NON_DECODIFICATO, SENZA_TERMINE};

/// Gaps of the numpress reader.
pub const NUMPRESS: &[(&str, &str)] = &[
    (
        "dataProcessing, referenceableParamGroup and scanSettings of the source",
        FUORI_MODELLO,
    ),
    (
        "cvParams the writer does not know (carried as userParam)",
        SENZA_TERMINE,
    ),
];

/// Gaps of the bruker reader.
pub const BRUKER: &[(&str, &str)] = &[
    ("inverse mobility 1/K0 of each point", NON_DECODIFICATO),
    (
        "image positions, regions and tables beyond Frames and GlobalMetadata (TSF)",
        FUORI_MODELLO,
    ),
    (
        "chromatography-data.sqlite, .mcf and .baf next to the data",
        FUORI_MODELLO,
    ),
];
