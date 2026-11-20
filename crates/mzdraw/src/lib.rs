//! mzDraw: the core of a 2D molecule editor (docs/agenti/mzdraw.md). The document `mzdraw/1`, the edit operations with their exact
//! inverses (`ops`), the interactive editor (`editor`: tools, hit-testing, view), the display list every surface paints (`scene`) and
//! its SVG form (`svg`). It depends on `mzlab-mol` and serde only, so it can leave this workspace; the mzLab glue lives in `mzdraw-wasm`.

pub mod build;
pub mod doc;
pub mod editor;
mod editor_tools;
pub mod io;
pub mod layout;
mod m;
pub mod ops;
pub mod replay;
pub mod scene;
pub mod svg;

pub use doc::{Atom, Bond, Document, FORMAT};
pub use editor::Editor;
pub use ops::{Op, OpError, Session};

/// The product name, kept here only (the page shows the i18n key `draw2.name`).
pub const NAME: &str = "mzDraw";
