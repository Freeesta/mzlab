//! Readers of the tables of other programs, in Rust: mzMine (full table, annotations, FBMN quant + IIN edges), the MGF of FBMN, MS-DIAL (alignment) and
//! GNPS2 (library matches and network). They recognise the columns by NAME and give ONE model of the features (`model::Table`), the same one the page
//! has always read: `join::build` takes the files of a table and returns it. The rules are those of Attributio 1.12.0 (readers/mzmine.py, gnps.py, ms2.py).

mod chem;
mod delim;
mod gnps;
mod join;
mod mgf;
mod model;
mod msdial;
mod mzmine;

pub use chem::{check_structure, clean_name, formula_mass, is_propagated, norm_adduct, Check};
pub use delim::{each_row, num, split_quoted};
pub use join::{build, detect, InFile};
pub use model::{pol_of, table_json, Table};

/// An error of a reader: `key` is the stable name the page maps to a message («columns», «empty», «noTable»).
#[derive(Clone, Debug, PartialEq)]
pub struct ImportError {
    pub key: &'static str,
    pub msg: String,
}

impl ImportError {
    pub fn new(key: &'static str, msg: &str) -> ImportError {
        ImportError {
            key,
            msg: msg.into(),
        }
    }
}

pub use gnps::{read_gnps_library, read_gnps_network};
pub use mgf::{attach_ms2, read_mgf};
pub use msdial::{parse_spectrum, read_msdial};
pub use mzmine::{merge_annotations, read_edges, read_full, read_quant};
