//! The pure part of the regulatory lists (no network, no disk): the readers of the user's own CSV (`svhc`, `ca_prop65`), the `resolve` core, the rows of a list,
//! the writer of the `listings` package and `import_official`, which turns an official file into a private `.mzsus`. `mzlab-cli normative build` and `pack`
//! (native only) call it for the sources it fetches; the app (route `normative/import-official`) and the lazy `normative` .wasm (`mzlab-normative-wasm`) call `official`.
//! Area file: `docs/agenti/normative.md`.

pub mod csvrd;
pub mod entry;
pub mod import_csv;
pub mod official;
pub mod pack;
pub mod resolve;
pub mod rows;
pub mod sidecar;

/// An error with the list or step it belongs to; `mzlab-cli` maps it to its `err.normative.parse`.
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub list: String,
    pub msg: String,
}

impl Error {
    pub fn parse(list: &str, msg: &str) -> Error {
        Error {
            list: list.into(),
            msg: msg.into(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.list, self.msg)
    }
}
