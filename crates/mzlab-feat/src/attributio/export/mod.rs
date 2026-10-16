//! Exports of Attributio: the sheets and files a chemist takes away (evidence, Schymanski folder, publication table, quantification, gradient).

pub mod cells;
pub mod ctx;
pub mod publication;
pub mod publication_ev;
pub mod publication_flags;
pub mod publication_ids;
pub mod publication_readme;
pub mod publication_text;
pub mod pyfmt;
pub mod run;
pub mod schymanski;
pub mod schymanski_delta;
pub mod schymanski_notes;
pub mod schymanski_rows;
pub mod sha1;
pub mod wide;

/// The version of Attributio the exports follow.
pub const VERSION: &str = "1.12.0";
