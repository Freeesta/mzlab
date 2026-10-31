//! `normative build`, the rows of one list: the rows of `mzlab_feat::normative::rows` plus the inputs the list was read from. Used by `build.rs`.

use mzlab_feat::normative::rows::{self, ListSrc};

pub use mzlab_feat::normative::rows::{Coverage, EXTRA};

use super::entry::Entry;
use super::readers::Input;
use super::resolve::Resolver;
use super::sources::ListSrc as Src;

pub struct Built {
    pub id: String,
    pub csv: String,
    pub coverage: Coverage,
    pub inputs: Vec<Input>,
    pub unresolved: Vec<[String; 5]>,
}

pub fn build_list(
    src: &Src,
    entries: &[Entry],
    inputs: Vec<Input>,
    res: &Resolver,
    resolve: bool,
) -> Built {
    let access = inputs
        .iter()
        .map(|x| x.retrieved.as_str())
        .max()
        .unwrap_or("");
    let b = rows::build_list(
        &ListSrc {
            id: &src.id,
            tags: &src.tags,
        },
        entries,
        access,
        res,
        resolve,
    );
    Built {
        id: b.id,
        csv: b.csv,
        coverage: b.coverage,
        inputs,
        unresolved: b.unresolved,
    }
}
